use ini::{Ini, Properties};

use crate::Result;

pub trait IniLoad {
    fn load_from_ini(&mut self, ini: &Ini) -> Result<()>;
}

pub trait Property {
    fn load_in(&mut self, section: &Properties, key: &str) -> Result<()>;
}

pub trait ConstructedProperty: Sized {
    fn load(section: &Properties, key: &str) -> Result<Option<Self>>;
}

impl<P: ConstructedProperty> Property for P {
    fn load_in(&mut self, section: &Properties, key: &str) -> Result<()> {
        if let Some(value) = P::load(section, key)? {
            *self = value;
        }
        Ok(())
    }
}

pub trait ParsedProperty: Sized {
    fn parse(text: &str) -> Result<Self>;
}

impl<P: ParsedProperty> ConstructedProperty for P {
    fn load(section: &Properties, key: &str) -> Result<Option<Self>> {
        match section.get(key) {
            Some(text) => Ok(Some(P::parse(text)?)),
            None => Ok(None),
        }
    }
}

impl ParsedProperty for String {
    fn parse(text: &str) -> Result<Self> {
        Ok(text.to_string())
    }
}

macro_rules! impl_from_str_properties {
    ($($type:ty),+ $(,)?) => {
        $(
        impl ParsedProperty for $type {
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
