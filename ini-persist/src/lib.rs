pub mod error;
pub mod load;

use self::error::Error;

pub type Result<T> = std::result::Result<T, Error>;
