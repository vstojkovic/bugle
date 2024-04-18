use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::{Bus, Event};

pub struct LocalBus {
    dispatch_map: HashMap<TypeId, Box<dyn Any>>,
}

struct Handlers<E: Event> {
    list: Vec<Box<dyn Fn(E) -> Option<E>>>,
}

impl LocalBus {
    pub fn new() -> Self {
        Self {
            dispatch_map: HashMap::new(),
        }
    }
}

impl Bus for LocalBus {
    fn publish<E: Event + 'static>(&self, event: E) {
        let Some(erased) = self.dispatch_map.get(&TypeId::of::<E>()) else {
            return;
        };
        let handlers = erased.downcast_ref::<Handlers<E>>().unwrap();
        handlers.handle(event);
    }

    fn subscribe_transform<E: Event + 'static, F: Fn(E) -> Option<E> + 'static>(
        &mut self,
        handler: F,
    ) {
        let erased = self
            .dispatch_map
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(Handlers::<E>::new()));
        let handlers = erased.downcast_mut::<Handlers<E>>().unwrap();
        handlers.add(handler);
    }
}

impl<E: Event> Handlers<E> {
    fn new() -> Self {
        Self { list: Vec::new() }
    }

    fn add<F: Fn(E) -> Option<E> + 'static>(&mut self, handler: F) {
        self.list.push(Box::new(handler));
    }

    fn handle(&self, mut event: E) {
        for handler in self.list.iter() {
            let Some(next) = handler(event) else {
                return;
            };
            event = next;
        }
    }
}
