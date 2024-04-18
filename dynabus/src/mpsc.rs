use std::any::{Any, TypeId};
use std::collections::HashMap;

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

pub struct ChannelBus<S: Sender, R: Receiver, B: Bus = LocalBus> {
    tx: BusSender<S>,
    rx: R,
    dispatch_map: HashMap<TypeId, fn(&ChannelBus<S, R, B>, Box<dyn Any>)>,
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

    pub fn recv(&self) -> Result<(), R::Error> {
        let Some(message) = self.rx.try_recv()? else {
            return Ok(());
        };
        let type_id = (&*message).type_id();
        let Some(receiver) = self.dispatch_map.get(&type_id) else {
            return Ok(());
        };
        receiver(self, message);
        Ok(())
    }

    fn ensure_dispatch<E: Event + 'static>(&mut self) {
        self.dispatch_map
            .entry(TypeId::of::<E>())
            .or_insert(|bus, message| {
                bus.publish(*message.downcast::<E>().unwrap());
            });
    }
}

impl<S: Sender, R: Receiver, B: Bus> Bus for ChannelBus<S, R, B> {
    fn publish<E: Event + 'static>(&self, event: E) {
        self.backer.publish(event);
    }

    fn subscribe_transform<E: Event + 'static, F: Fn(E) -> Option<E> + 'static>(
        &mut self,
        handler: F,
    ) {
        self.ensure_dispatch::<E>();
        self.backer.subscribe_transform(handler);
    }

    fn subscribe_observer<E: Event + 'static, F: Fn(&E) + 'static>(&mut self, handler: F) {
        self.ensure_dispatch::<E>();
        self.backer.subscribe_observer(handler);
    }

    fn subscribe_consumer<E: Event + 'static, F: Fn(E) + 'static>(&mut self, handler: F) {
        self.ensure_dispatch::<E>();
        self.backer.subscribe_consumer(handler);
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
