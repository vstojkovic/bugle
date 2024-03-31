use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Error, Fields, Ident, Result, Type};

use crate::attr::{IniAttr, NoAttrSupport};

mod attr;
pub mod load;
pub mod save;

use self::attr::FieldAttr;

type FieldExpander = fn(name: &Ident, typ: &Type, section: TokenStream, span: Span) -> TokenStream;
type TraitExpander = fn(struct_name: &Ident, field_expansions: Vec<TokenStream>) -> TokenStream;

fn expand_ini_impl(
    input: DeriveInput,
    field_expander: FieldExpander,
    trait_expander: TraitExpander,
) -> Result<TokenStream> {
    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("IniLoad can only be derived on a struct with named fields"),
    };

    NoAttrSupport::from_ast(input.attrs.iter())?;

    let struct_name = input.ident;
    let field_expansions = fields
        .into_iter()
        .map(|field| {
            let attr = FieldAttr::from_ast(field.attrs.iter())?;
            let field_name = field.ident.as_ref().unwrap();
            let span = field.ty.span();

            let section_name = attr.section.unwrap_or_else(|| Some(field_name.to_string()));
            let section = match section_name {
                Some(name) => quote!(Some(#name.to_string())),
                None => quote!(None::<String>),
            };

            Ok(field_expander(field_name, &field.ty, section, span))
        })
        .map(|result| result.unwrap_or_else(Error::into_compile_error))
        .collect();

    Ok(trait_expander(&struct_name, field_expansions))
}
