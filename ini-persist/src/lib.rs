pub mod error;
pub mod load;

use self::error::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "derive")]
pub use ini_persist_derive::{IniLoad, Property};
