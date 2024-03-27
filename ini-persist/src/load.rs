use ini::{Ini, Properties};

use crate::Result;

#[cfg(feature = "derive")]
pub use ini_persist_derive::{IniLoad, LoadProperty};

pub trait IniLoad {
    fn load_from_ini(&mut self, ini: &Ini) -> Result<()>;
}

pub trait LoadProperty {
    fn load_in(&mut self, section: &Properties, key: &str) -> Result<()>;
}

pub trait ConstructProperty: Sized {
    fn load(section: &Properties, key: &str) -> Result<Option<Self>>;
}

impl<P: ConstructProperty> LoadProperty for P {
    fn load_in(&mut self, section: &Properties, key: &str) -> Result<()> {
        if let Some(value) = P::load(section, key)? {
            *self = value;
        }
        Ok(())
    }
}

pub trait ParseProperty: Sized {
    fn parse(text: &str) -> Result<Self>;
}

impl<P: ParseProperty> ConstructProperty for P {
    fn load(section: &Properties, key: &str) -> Result<Option<Self>> {
        match section.get(key) {
            Some(text) => Ok(Some(P::parse(text)?)),
            None => Ok(None),
        }
    }
}

impl ParseProperty for String {
    fn parse(text: &str) -> Result<Self> {
        Ok(text.to_string())
    }
}

macro_rules! impl_from_str_properties {
    ($($type:ty),+ $(,)?) => {
        $(
        impl ParseProperty for $type {
            fn parse(text: &str) -> $crate::Result<Self> {
                use std::str::FromStr;
                Self::from_str(text).map_err(|err| {
                    $crate::error::Error::invalid_type(format!(
                        concat!("failed to parse ", stringify!($type), " from: {}"),
                        text
                    ))
                    .with_cause(err)
                })
            }
        }
        )+
    };
}

impl_from_str_properties!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, bool, char,
);

impl<P: ConstructProperty> ConstructProperty for Option<P> {
    fn load(section: &ini::Properties, key: &str) -> Result<Option<Self>> {
        Ok(Some(P::load(section, key)?))
    }
}
