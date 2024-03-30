use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{DeriveInput, Ident, Result, Type};

use super::expand_ini_impl;

pub fn expand_ini_save(input: DeriveInput) -> Result<TokenStream> {
    expand_ini_impl(input, expand_field, expand_trait)
}

fn expand_field(name: &Ident, typ: &Type, section: TokenStream, span: Span) -> TokenStream {
    quote_spanned! { span =>
        {
            let section = ini.entry(#section).or_insert_with(ini::Properties::default);
            #typ::remove(section, "");
            self.#name.append(section, "");
        }
    }
}

fn expand_trait(struct_name: &Ident, field_expansions: Vec<TokenStream>) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl ini_persist::save::IniSave for #struct_name {
            fn save_to_ini(&self, ini: &mut ini::Ini) {
                use ini_persist::save::SaveProperty;
                #(#field_expansions)*
            }
        }
    }
}
