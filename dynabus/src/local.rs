use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::Instant;

use crate::{Bus, Event};

pub struct LocalBus {
    dispatch_map: HashMap<TypeId, Box<dyn Any>>,
}

#[derive(Debug)]
pub struct Subscription<E: Event> {
    key: HandlerKey,
    phantom: PhantomData<E>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct HandlerKey {
    timestamp: Instant,
    sequence: u64,
}

struct Handlers<E: Event> {
    handlers: Vec<Box<dyn Fn(E) -> Option<E>>>,
    keys: Vec<HandlerKey>,
    key_map: HashMap<HandlerKey, usize>,
    last_key: HandlerKey,
}

impl LocalBus {
    pub fn new() -> Self {
        Self {
            dispatch_map: HashMap::new(),
        }
    }
}

impl Bus for LocalBus {
    type Subscription<E: Event> = Subscription<E>;

    fn publish<E: Event + 'static>(&self, event: E) -> bool {
        let Some(erased) = self.dispatch_map.get(&TypeId::of::<E>()) else {
            return false;
        };
        let handlers = erased.downcast_ref::<Handlers<E>>().unwrap();
        handlers.handle(event)
    }

    fn subscribe_transform<E: Event + 'static, F: Fn(E) -> Option<E> + 'static>(
        &mut self,
        handler: F,
    ) -> Self::Subscription<E> {
        let type_id = TypeId::of::<E>();
        let erased = self
            .dispatch_map
            .entry(type_id)
            .or_insert_with(|| Box::new(Handlers::<E>::new()));
        let handlers = erased.downcast_mut::<Handlers<E>>().unwrap();
        let key = handlers.add(handler);
        Self::Subscription {
            key,
            phantom: PhantomData,
        }
    }

    fn unsubscribe<E: Event + 'static>(&mut self, subscription: Self::Subscription<E>) {
        let Some(erased) = self.dispatch_map.get_mut(&TypeId::of::<E>()) else {
            return;
        };
        let handlers = erased.downcast_mut::<Handlers<E>>().unwrap();
        handlers.remove(subscription.key);
    }
}

impl<E: Event> Handlers<E> {
    fn new() -> Self {
        Self {
            handlers: Vec::new(),
            keys: Vec::new(),
            key_map: HashMap::new(),
            last_key: HandlerKey::new(),
        }
    }

    fn add<F: Fn(E) -> Option<E> + 'static>(&mut self, handler: F) -> HandlerKey {
        self.last_key.advance();
        self.key_map.insert(self.last_key, self.handlers.len());
        self.keys.push(self.last_key);
        self.handlers.push(Box::new(handler));
        self.last_key
    }

    fn remove(&mut self, key: HandlerKey) {
        let Some(idx) = self.key_map.remove(&key) else {
            return;
        };
        let last_key = self.keys.last().copied().unwrap();
        let _ = self.handlers.swap_remove(idx);
        self.keys.swap_remove(idx);
        self.key_map
            .entry(last_key)
            .and_modify(|last_key_idx| *last_key_idx = idx);
    }

    fn handle(&self, mut event: E) -> bool {
        if self.handlers.is_empty() {
            return false;
        }
        for handler in self.handlers.iter() {
            let Some(next) = handler(event) else {
                return true;
            };
            event = next;
        }
        true
    }
}

impl HandlerKey {
    fn new() -> Self {
        Self {
            timestamp: Instant::now(),
            sequence: 0,
        }
    }

    fn advance(&mut self) {
        let timestamp = Instant::now();
        if timestamp != self.timestamp {
            self.timestamp = timestamp;
            self.sequence = 0;
        } else {
            self.sequence += 1;
        }
    }
}
