use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DataEnum, DeriveInput, Error, Fields, Ident, Meta, Result, Token, Variant};

use crate::attr::{EnumAttr, IniAttr};

pub fn expand_property(input: DeriveInput) -> Result<TokenStream> {
    let variants = match &input.data {
        Data::Enum(DataEnum { variants, .. }) => variants,
        _ => panic!("Property can only be derived on an enum"),
    };

    if variants.into_iter().any(is_non_unit_variant) {
        panic!("Only unit variants are supported");
    }

    let attr = EnumAttr::from_ast(input.attrs.iter())?;
    let enum_name = &input.ident;
    let repr_type = enum_repr(&input)?;

    let prelude = attr
        .repr
        .map(|_| {
            let constants = variants.into_iter().map(|variant| {
                expand_variant_const(&variant.ident, enum_name, &repr_type, variant.span())
            });
            quote! {
                use ini_persist::load::ParsedProperty;

                struct Discriminants;

                #[allow(non_upper_case_globals)]
                impl Discriminants {
                    #(#constants;)*
                }
                let value = #repr_type::parse(value)?;
            }
        })
        .unwrap_or_default();
    let match_expander = match attr.repr {
        Some(()) => expand_repr_variant_match,
        None => expand_named_variant_match,
    };
    let match_arms = variants
        .into_iter()
        .map(|variant| match_expander(&variant.ident, enum_name, variant.span()));

    let output = quote! {
        #[automatically_derived]
        impl ini_persist::load::ParsedProperty for #enum_name {
            fn parse(value: &str) -> ini_persist::Result<Self> {
                #prelude
                ini_persist::Result::Ok(match value {
                    #(#match_arms,)*
                    _ => return ini_persist::Result::Err(ini_persist::error::Error::invalid_value(
                        format!("invalid value: {}", value)
                    ))
                })
            }
        }
    };

    Ok(output)
}

fn is_non_unit_variant(variant: &Variant) -> bool {
    if let Fields::Unit = variant.fields {
        return false;
    }
    true
}

fn enum_repr(input: &DeriveInput) -> Result<Ident> {
    const SUPPORTED_REPRS: &[&str] = &[
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
    ];

    for attr in input.attrs.iter() {
        if !attr.path().is_ident("repr") {
            continue;
        }
        let reprs = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for repr in reprs {
            match repr {
                Meta::Path(path) if path.is_ident("packed") => continue,
                Meta::Path(path) if SUPPORTED_REPRS.into_iter().any(|id| path.is_ident(id)) => {
                    return Ok(path.get_ident().unwrap().clone());
                }
                Meta::List(meta) if meta.path.is_ident("align") => continue,
                meta @ _ => return Err(Error::new_spanned(meta, "unsupported repr")),
            }
        }
    }
    Ok(Ident::new("isize", input.ident.span()))
}

fn expand_variant_const(
    name: &Ident,
    enum_name: &Ident,
    repr_type: &Ident,
    span: Span,
) -> TokenStream {
    quote_spanned!(span => const #name: #repr_type = #enum_name::#name as #repr_type)
}

fn expand_named_variant_match(name: &Ident, enum_name: &Ident, span: Span) -> TokenStream {
    quote_spanned!(span => stringify!(#name) => #enum_name::#name)
}

fn expand_repr_variant_match(name: &Ident, enum_name: &Ident, span: Span) -> TokenStream {
    quote_spanned!(span => Discriminants::#name => #enum_name::#name)
}
