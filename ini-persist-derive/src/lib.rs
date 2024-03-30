use syn::{parse_macro_input, DeriveInput, Error};

mod attr;
mod ini;
mod property;

use self::ini::load::expand_ini_load;
use self::ini::save::expand_ini_save;
use self::property::load::expand_load_property;
use self::property::save::expand_save_property;

#[proc_macro_derive(IniLoad, attributes(ini))]
pub fn derive_ini_load(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_ini_load(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(IniSave, attributes(ini))]
pub fn derive_ini_save(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_ini_save(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(LoadProperty, attributes(ini))]
pub fn derive_load_property(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_load_property(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(SaveProperty, attributes(ini))]
pub fn derive_save_property(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_save_property(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
