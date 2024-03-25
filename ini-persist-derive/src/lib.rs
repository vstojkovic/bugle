use syn::{parse_macro_input, DeriveInput, Error};

mod attr;
mod load;
mod property;

use self::load::expand_load;
use self::property::expand_property;

#[proc_macro_derive(IniLoad, attributes(ini))]
pub fn derive_ini_load(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_load(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Property, attributes(ini))]
pub fn derive_property(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_property(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
