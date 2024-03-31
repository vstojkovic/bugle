use std::borrow::Cow;

use ini::{Ini, Properties};

#[cfg(feature = "derive")]
pub use ini_persist_derive::{IniSave, SaveProperty};

pub trait IniSave {
    fn save_to_ini(&self, ini: &mut Ini);
}

pub trait SaveProperty {
    fn remove(section: &mut Properties, key: &str) {
        default_remove(section, key);
    }
    fn append(&self, section: &mut Properties, key: &str);
}

pub fn default_remove(section: &mut Properties, key: &str) {
    let _ = section.remove_all(key);
}

pub trait DisplayProperty {
    fn display(&self) -> String;
}

impl<P: DisplayProperty> SaveProperty for P {
    fn append(&self, section: &mut Properties, key: &str) {
        section.append(key, self.display());
    }
}

macro_rules! impl_display_properties {
    ($($type:ty),+ $(,)?) => {
        $(
        impl DisplayProperty for $type {
            fn display(&self) -> String {
                self.to_string()
            }
        }
        )+
    };
}

impl_display_properties!(
    String, &str, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64, char,
    bool,
);

impl DisplayProperty for Cow<'_, str> {
    fn display(&self) -> String {
        self.to_string()
    }
}

impl<P: SaveProperty> SaveProperty for Option<P> {
    fn remove(section: &mut Properties, key: &str) {
        P::remove(section, key);
    }

    fn append(&self, section: &mut Properties, key: &str) {
        if let Some(value) = self.as_ref() {
            value.append(section, key);
        }
    }
}
