use std::borrow::Cow;

use lazy_static::lazy_static;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use regex::Regex;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, Ident, Meta, Result,
    Token, Type, Variant,
};

use crate::attr::{IniAttr, NoAttrSupport};

mod attr;
pub mod load;
pub mod save;

use self::attr::{EnumAttr, FieldAttr, StructAttr};

type FieldExpander<F> =
    fn(name: &Ident, typ: &Type, key: TokenStream, attr: FieldAttr, span: Span) -> F;
type StructTraitExpander<F> =
    fn(struct_name: &Ident, field_expansions: Vec<Result<F>>) -> TokenStream;
type VariantExpander = fn(name: &Ident, enum_name: &Ident, span: Span) -> TokenStream;
type EnumTraitExpander = fn(
    enum_name: &Ident,
    enum_attr: &EnumAttr,
    repr_type: Option<&Ident>,
    prelude: TokenStream,
    match_arms: Vec<TokenStream>,
) -> TokenStream;

fn expand_property_impl<F>(
    input: DeriveInput,
    field_expander: FieldExpander<F>,
    struct_trait_expander: StructTraitExpander<F>,
    repr_variant_expander: VariantExpander,
    named_variant_expander: VariantExpander,
    caseless_variant_expander: VariantExpander,
    enum_trait_expander: EnumTraitExpander,
) -> Result<TokenStream> {
    match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => expand_struct_impl(
            input.ident,
            fields.named.into_iter().collect(),
            input.attrs,
            field_expander,
            struct_trait_expander,
        ),
        Data::Enum(DataEnum { variants, .. }) => expand_enum_impl(
            input.ident,
            variants.into_iter().collect(),
            input.attrs,
            repr_variant_expander,
            named_variant_expander,
            caseless_variant_expander,
            enum_trait_expander,
        ),
        _ => panic!("Property can only be derived on an enum or a struct with named fields"),
    }
}

fn expand_struct_impl<F>(
    ident: Ident,
    fields: Vec<Field>,
    attrs: Vec<Attribute>,
    field_expander: FieldExpander<F>,
    trait_expander: StructTraitExpander<F>,
) -> Result<TokenStream> {
    let attr = StructAttr::from_ast(attrs.iter())?;
    let struct_name = &ident;
    let default_format = attr
        .key_format
        .as_ref()
        .map(String::as_str)
        .unwrap_or("{prefix}{name}");

    let field_expansions = fields
        .iter()
        .map(|field| {
            let attr = FieldAttr::from_ast(field.attrs.iter())?;
            let field_name = field.ident.as_ref().unwrap();
            let span = field.span();

            let key_name = attr
                .key_name
                .as_ref()
                .map(Cow::from)
                .unwrap_or_else(|| field_name.to_string().into());
            let key_format = attr
                .key_format
                .as_ref()
                .map(String::as_str)
                .unwrap_or_else(|| default_format);
            let key = if attr.flatten.is_some() {
                quote!(key)
            } else {
                expand_key(key_format, &key_name)
            };

            Ok(field_expander(field_name, &field.ty, key, attr, span))
        })
        .collect();

    Ok(trait_expander(struct_name, field_expansions))
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
    repr_variant_expander: VariantExpander,
    named_variant_expander: VariantExpander,
    caseless_variant_expander: VariantExpander,
    trait_expander: EnumTraitExpander,
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
            }
        })
        .unwrap_or_default();

    let match_expander = match (attr.repr, attr.ignore_case) {
        (Some(()), _) => repr_variant_expander,
        (None, None) => named_variant_expander,
        (None, Some(())) => caseless_variant_expander,
    };
    let match_arms = variants
        .iter()
        .map(|variant| {
            NoAttrSupport::from_ast(variant.attrs.iter())?;
            Ok(match_expander(&variant.ident, enum_name, variant.span()))
        })
        .map(|result| result.unwrap_or_else(Error::into_compile_error))
        .collect();

    Ok(trait_expander(
        enum_name,
        &attr,
        attr.repr.map(|_| &repr_type),
        prelude,
        match_arms,
    ))
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

lazy_static! {
    static ref PREFIX_REGEX: Regex = Regex::new(r"(^|[^{])\{prefix[:}]").unwrap();
    static ref NAME_REGEX: Regex = Regex::new(r"(^|[^{])\{name[:}]").unwrap();
}
