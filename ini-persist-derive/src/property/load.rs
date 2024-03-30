use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{DeriveInput, Error, Ident, Result, Type};

use super::attr::{FieldAttr, LoadFn};
use super::expand_property_impl;

pub fn expand_load_property(input: DeriveInput) -> Result<TokenStream> {
    expand_property_impl(
        input,
        expand_field,
        expand_struct_trait,
        expand_repr_variant_match,
        expand_named_variant_match,
        expand_enum_trait,
    )
}

fn expand_field(
    name: &Ident,
    _typ: &Type,
    key: TokenStream,
    attr: FieldAttr,
    span: Span,
) -> TokenStream {
    match attr.load_fn {
        None => quote_spanned! { span =>
            self.#name.load_in(section, #key)?;
        },
        Some(LoadFn::InPlace(path)) => quote_spanned! { span =>
            #path(&mut self.#name, section, #key)?;
        },
        Some(LoadFn::Constructed(path)) => quote_spanned! { span =>
            if let Some(value) = #path(section, #key)? {
                self.#name = value;
            }
        },
        Some(LoadFn::Parsed(path)) => quote_spanned! { span =>
            if let Some(value) = section.get(#key) {
                self.#name = #path(value)?;
            }
        },
    }
}

fn expand_struct_trait(
    struct_name: &Ident,
    field_expansions: Vec<Result<TokenStream>>,
) -> TokenStream {
    let load_calls = field_expansions
        .into_iter()
        .map(|expansion| expansion.unwrap_or_else(Error::into_compile_error));
    quote! {
        #[automatically_derived]
        impl ini_persist::load::LoadProperty for #struct_name {
            fn load_in(&mut self, section: &ini::Properties, key: &str) -> ini_persist::Result<()> {
                #(#load_calls)*
                ini_persist::Result::Ok(())
            }
        }
    }
}

fn expand_named_variant_match(name: &Ident, enum_name: &Ident, span: Span) -> TokenStream {
    quote_spanned!(span => stringify!(#name) => #enum_name::#name)
}

fn expand_repr_variant_match(name: &Ident, enum_name: &Ident, span: Span) -> TokenStream {
    quote_spanned!(span => Discriminants::#name => #enum_name::#name)
}

fn expand_enum_trait(
    enum_name: &Ident,
    repr_type: Option<&Ident>,
    prelude: TokenStream,
    match_arms: Vec<TokenStream>,
) -> TokenStream {
    let parse_repr = repr_type
        .map(|repr_type| {
            quote! {
                let value = #repr_type::parse(value)?;
            }
        })
        .unwrap_or_default();
    quote! {
        #[automatically_derived]
        impl ini_persist::load::ParseProperty for #enum_name {
            fn parse(value: &str) -> ini_persist::Result<Self> {
                #prelude
                #parse_repr
                ini_persist::Result::Ok(match value {
                    #(#match_arms,)*
                    _ => return ini_persist::Result::Err(ini_persist::Error::invalid_value(
                        format!("invalid value: {}", value)
                    ))
                })
            }
        }
    }
}
