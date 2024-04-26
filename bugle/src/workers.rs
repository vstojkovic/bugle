mod fls;
mod saved_games;
mod server_loader;

pub use fls::{FlsWorker, LoginComplete};
pub use saved_games::SavedGamesWorker;
pub use server_loader::{PongReceived, ServerLoaderWorker, ServersLoaded};

#[derive(Debug)]
pub enum TaskState<T> {
    Pending,
    Ready(T),
}

impl<T: Clone> Clone for TaskState<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Pending => Self::Pending,
            Self::Ready(value) => Self::Ready(value.clone()),
        }
    }
}

impl<T: Copy> Copy for TaskState<T> {}
