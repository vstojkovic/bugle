pub mod local;
pub mod mpsc;

#[cfg(feature = "derive")]
pub use dynabus_derive::Event;

#[cfg(feature = "crossbeam")]
mod crossbeam;

pub trait Event {}

pub trait Bus {
    fn publish<E: Event + 'static>(&self, event: E) -> bool;
    fn subscribe_transform<E: Event + 'static, F: Fn(E) -> Option<E> + 'static>(
        &mut self,
        handler: F,
    );

    fn subscribe_observer<E: Event + 'static, F: Fn(&E) + 'static>(&mut self, handler: F) {
        self.subscribe_transform(move |event| {
            handler(&event);
            Some(event)
        })
    }

    fn subscribe_consumer<E: Event + 'static, F: Fn(E) + 'static>(&mut self, handler: F) {
        self.subscribe_transform(move |event| {
            handler(event);
            None
        })
    }
}
