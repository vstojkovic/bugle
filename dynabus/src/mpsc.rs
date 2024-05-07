use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::local::LocalBus;
use crate::{Bus, Event};

pub type Message = Box<dyn Any + Send>;

pub trait Sender: Clone {
    type Error;
    fn try_send(&self, message: Message) -> Result<(), Self::Error>;
}

pub trait Receiver {
    type Error;
    fn try_recv(&self) -> Result<Option<Message>, Self::Error>;
}

pub trait BlockingReceiver: Receiver {
    fn recv_deadline(&self, deadline: Instant) -> Result<Option<Message>, Self::Error>;
}

pub struct ChannelBus<S: Sender, R: Receiver, B: Bus = LocalBus> {
    tx: BusSender<S>,
    rx: R,
    dispatch_map: HashMap<TypeId, fn(&ChannelBus<S, R, B>, Box<dyn Any>) -> bool>,
    backer: B,
}

#[derive(Clone)]
pub struct BusSender<S: Sender>(S);

impl<S: Sender, R: Receiver, B: Bus> ChannelBus<S, R, B> {
    pub fn new(channel: (S, R), backer: B) -> Self {
        Self {
            tx: BusSender(channel.0),
            rx: channel.1,
            dispatch_map: HashMap::new(),
            backer,
        }
    }

    pub fn sender(&self) -> &BusSender<S> {
        &self.tx
    }

    pub fn recv(&self) -> Result<Option<bool>, R::Error> {
        let Some(message) = self.rx.try_recv()? else {
            return Ok(None);
        };
        Ok(Some(self.dispatch_message(message)))
    }

    pub fn recv_deadline(&self, deadline: Instant) -> Result<Option<bool>, R::Error>
    where
        R: BlockingReceiver,
    {
        let Some(message) = self.rx.recv_deadline(deadline)? else {
            return Ok(None);
        };
        Ok(Some(self.dispatch_message(message)))
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<Option<bool>, R::Error>
    where
        R: BlockingReceiver,
    {
        self.recv_deadline(Instant::now() + timeout)
    }

    fn ensure_dispatch<E: Event + 'static>(&mut self) {
        self.dispatch_map
            .entry(TypeId::of::<E>())
            .or_insert(|bus, message| bus.publish(*message.downcast::<E>().unwrap()));
    }

    fn dispatch_message(&self, message: Message) -> bool {
        let type_id = (&*message).type_id();
        let Some(receiver) = self.dispatch_map.get(&type_id) else {
            return false;
        };
        receiver(self, message);
        true
    }
}

impl<S: Sender, R: Receiver, B: Bus> Bus for ChannelBus<S, R, B> {
    type Subscription<E: Event> = B::Subscription<E>;

    fn publish<E: Event + 'static>(&self, event: E) -> bool {
        self.backer.publish(event)
    }

    fn subscribe_transform<E: Event + 'static, F: Fn(E) -> Option<E> + 'static>(
        &mut self,
        handler: F,
    ) -> Self::Subscription<E> {
        self.ensure_dispatch::<E>();
        self.backer.subscribe_transform(handler)
    }

    fn subscribe_observer<E: Event + 'static, F: Fn(&E) + 'static>(
        &mut self,
        handler: F,
    ) -> Self::Subscription<E> {
        self.ensure_dispatch::<E>();
        self.backer.subscribe_observer(handler)
    }

    fn subscribe_consumer<E: Event + 'static, F: Fn(E) + 'static>(
        &mut self,
        handler: F,
    ) -> Self::Subscription<E> {
        self.ensure_dispatch::<E>();
        self.backer.subscribe_consumer(handler)
    }

    fn unsubscribe<E: Event + 'static>(&mut self, subscription: Self::Subscription<E>) {
        self.backer.unsubscribe(subscription)
    }
}

impl<S: Sender> BusSender<S> {
    pub fn send<E: Event + Send + 'static>(&self, event: E) -> Result<(), S::Error> {
        self.0.try_send(Box::new(event))
    }
}

impl Sender for std::sync::mpsc::Sender<Message> {
    type Error = std::sync::mpsc::SendError<Message>;
    fn try_send(&self, message: Message) -> Result<(), Self::Error> {
        self.send(message)
    }
}

impl Receiver for std::sync::mpsc::Receiver<Message> {
    type Error = std::sync::mpsc::TryRecvError;
    fn try_recv(&self) -> Result<Option<Message>, Self::Error> {
        match self.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(Self::Error::Empty) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl BlockingReceiver for std::sync::mpsc::Receiver<Message> {
    fn recv_deadline(&self, deadline: Instant) -> Result<Option<Message>, Self::Error> {
        let timeout = deadline.duration_since(Instant::now());
        match self.recv_timeout(timeout) {
            Ok(message) => Ok(Some(message)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(Self::Error::Disconnected),
        }
    }
}
