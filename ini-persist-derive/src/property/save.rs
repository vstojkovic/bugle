use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{DeriveInput, Ident, Result, Type};

use super::attr::FieldAttr;
use super::expand_property_impl;

pub fn expand_save_property(input: DeriveInput) -> Result<TokenStream> {
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
    typ: &Type,
    key: TokenStream,
    _attr: FieldAttr,
    span: Span,
) -> (TokenStream, TokenStream) {
    let remove = quote_spanned! { span =>
        <#typ as ini_persist::save::SaveProperty>::remove(section, #key);
    };
    let append = quote_spanned! { span =>
        self.#name.append(section, #key);
    };
    (remove, append)
}

fn expand_struct_trait(
    struct_name: &Ident,
    field_expansions: Vec<Result<(TokenStream, TokenStream)>>,
) -> TokenStream {
    let (remove_calls, append_calls): (Vec<_>, Vec<_>) = field_expansions
        .into_iter()
        .map(|expansion| {
            expansion.unwrap_or_else(|err| {
                let err = err.into_compile_error();
                (err.clone(), err)
            })
        })
        .unzip();
    quote! {
        #[automatically_derived]
        impl ini_persist::save::SaveProperty for #struct_name {
            fn remove(section: &mut ini::Properties, key: &str) {
                #(#remove_calls)*
            }

            fn append(&self, section: &mut ini::Properties, key: &str) {
                #(#append_calls)*
            }
        }
    }
}

fn expand_named_variant_match(name: &Ident, enum_name: &Ident, span: Span) -> TokenStream {
    quote_spanned!(span => #enum_name::#name => stringify!(#name).to_string())
}

fn expand_repr_variant_match(name: &Ident, enum_name: &Ident, span: Span) -> TokenStream {
    quote_spanned!(span => #enum_name::#name => format!("{}", Discriminants::#name))
}

fn expand_enum_trait(
    enum_name: &Ident,
    _repr_type: Option<&Ident>,
    prelude: TokenStream,
    match_arms: Vec<TokenStream>,
) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl ini_persist::save::DisplayProperty for #enum_name {
            fn display(&self) -> String {
                #prelude
                match self {
                    #(#match_arms,)*
                }
            }
        }
    }
}
