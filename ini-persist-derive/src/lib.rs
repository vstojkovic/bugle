use syn::{parse_macro_input, DeriveInput, Error};

mod attr;
mod ini_load;
mod load_property;

use self::ini_load::expand_ini_load;
use self::load_property::expand_load_property;

#[proc_macro_derive(IniLoad, attributes(ini))]
pub fn derive_ini_load(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_ini_load(input)
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
