use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{DeriveInput, Ident, Result, Type};

use super::expand_ini_impl;

pub fn expand_ini_load(input: DeriveInput) -> Result<TokenStream> {
    expand_ini_impl(input, expand_field, expand_trait)
}

fn expand_field(name: &Ident, _typ: &Type, section: TokenStream, span: Span) -> TokenStream {
    quote_spanned! { span =>
        if let Some(section) = ini.section(#section) {
            self.#name.load_in(section, "")?;
        }
    }
}

fn expand_trait(struct_name: &Ident, field_expansions: Vec<TokenStream>) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl ini_persist::load::IniLoad for #struct_name {
            fn load_from_ini(&mut self, ini: &ini::Ini) -> ini_persist::Result<()> {
                use ini_persist::load::LoadProperty;
                #(#field_expansions)*
                ini_persist::Result::Ok(())
            }
        }
    }
}
