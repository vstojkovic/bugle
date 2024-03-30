mod error;
pub mod load;
pub mod save;

pub use self::error::Error;
pub type Result<T> = std::result::Result<T, Error>;
