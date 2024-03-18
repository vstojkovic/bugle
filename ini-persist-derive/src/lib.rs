use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(IniLoad, attributes(ini))]
pub fn derive_ini_load(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);

    let output = quote!();
    output.into()
}
