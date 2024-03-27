use lazy_static::lazy_static;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use regex::Regex;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Ident, Meta, Result,
    Token, Variant,
};

use crate::attr::{IniAttr, NoAttrSupport};

mod attr;

use self::attr::{EnumAttr, FieldAttr, LoadFn, StructAttr};

pub fn expand_load_property(input: DeriveInput) -> Result<TokenStream> {
    match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => expand_struct_impl(input.ident, fields.named.into_iter().collect(), input.attrs),
        Data::Enum(DataEnum { variants, .. }) => {
            expand_enum_impl(input.ident, variants.into_iter().collect(), input.attrs)
        }
        _ => panic!("Property can only be derived on an enum or a struct with named fields"),
    }
}

fn expand_struct_impl(
    ident: Ident,
    fields: Vec<Field>,
    attrs: Vec<Attribute>,
) -> Result<TokenStream> {
    let attr = StructAttr::from_ast(attrs.iter())?;
    let struct_name = &ident;
    let default_format = attr
        .key_format
        .as_ref()
        .map(String::as_str)
        .unwrap_or("{prefix}{name}");

    let load_calls = fields
        .into_iter()
        .map(|field| {
            let attr = FieldAttr::from_ast(field.attrs.iter())?;
            let field_name = field.ident.as_ref().unwrap();
            let span = field.ty.span();

            expand_field(field_name, attr, default_format, span)
        })
        .map(|result| result.unwrap_or_else(Error::into_compile_error));

    let output = quote! {
        #[automatically_derived]
        impl ini_persist::load::LoadProperty for #struct_name {
            fn load_in(&mut self, section: &ini::Properties, key: &str) -> ini_persist::Result<()> {
                #(#load_calls)*
                ini_persist::Result::Ok(())
            }
        }
    };

    Ok(output)
}

fn expand_field(
    name: &Ident,
    attr: FieldAttr,
    default_format: &str,
    span: Span,
) -> Result<TokenStream> {
    let key_name = attr.key_name.unwrap_or_else(|| name.to_string());
    let key_format = attr
        .key_format
        .as_ref()
        .map(String::as_str)
        .unwrap_or_else(|| default_format);
    let key = if attr.flatten.is_some() { quote!(key) } else { expand_key(key_format, &key_name) };

    Ok(match attr.load_fn {
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
    })
}

fn expand_key(format: &str, name: &str) -> TokenStream {
    let uses_prefix = PREFIX_REGEX.is_match(format);
    let uses_name = NAME_REGEX.is_match(format);
    match (uses_prefix, uses_name) {
        (true, true) => quote!(&format!(#format, prefix=key, name=#name)),
        (true, false) => quote!(&format!(#format, prefix=key)),
        (false, true) => quote!(&format!(#format, name=#name)),
        (false, false) => quote!(#format),
    }
}

fn expand_enum_impl(
    ident: Ident,
    variants: Vec<Variant>,
    attrs: Vec<Attribute>,
) -> Result<TokenStream> {
    for variant in variants.iter() {
        NoAttrSupport::from_ast(variant.attrs.iter())?;
        if is_non_unit_variant(variant) {
            panic!("Only unit variants are supported");
        }
    }

    let attr = EnumAttr::from_ast(attrs.iter())?;
    let enum_name = &ident;
    let repr_type = enum_repr(&attrs)?.unwrap_or_else(|| Ident::new("isize", ident.span()));

    let prelude = attr
        .repr
        .map(|_| {
            let constants = variants.iter().map(|variant| {
                expand_variant_const(&variant.ident, enum_name, &repr_type, variant.span())
            });
            quote! {
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
        .iter()
        .map(|variant| {
            NoAttrSupport::from_ast(variant.attrs.iter())?;
            Ok(match_expander(&variant.ident, enum_name, variant.span()))
        })
        .map(|result| result.unwrap_or_else(Error::into_compile_error));

    let output = quote! {
        #[automatically_derived]
        impl ini_persist::load::ParseProperty for #enum_name {
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

fn enum_repr(attrs: &[Attribute]) -> Result<Option<Ident>> {
    const SUPPORTED_REPRS: &[&str] = &[
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
    ];

    for attr in attrs.iter() {
        if !attr.path().is_ident("repr") {
            continue;
        }
        let reprs = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for repr in reprs {
            match repr {
                Meta::Path(path) if path.is_ident("packed") => continue,
                Meta::Path(path) if SUPPORTED_REPRS.into_iter().any(|id| path.is_ident(id)) => {
                    return Ok(Some(path.get_ident().unwrap().clone()));
                }
                Meta::List(meta) if meta.path.is_ident("align") => continue,
                meta @ _ => return Err(Error::new_spanned(meta, "unsupported repr")),
            }
        }
    }
    Ok(None)
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

lazy_static! {
    static ref PREFIX_REGEX: Regex = Regex::new(r"(^|[^{])\{prefix[:}]").unwrap();
    static ref NAME_REGEX: Regex = Regex::new(r"(^|[^{])\{name[:}]").unwrap();
}
