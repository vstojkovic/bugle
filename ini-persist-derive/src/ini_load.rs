use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Error, Fields, Result};

use crate::attr::{IniAttr, NoAttrSupport};

mod attr;

use self::attr::FieldAttr;

pub fn expand_ini_load(input: DeriveInput) -> Result<TokenStream> {
    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("IniLoad can only be derived on a struct with named fields"),
    };

    NoAttrSupport::from_ast(input.attrs.iter())?;

    let struct_name = input.ident;
    let load_calls = fields
        .into_iter()
        .map(|field| {
            let attr = FieldAttr::from_ast(field.attrs.iter())?;
            let field_name = field.ident.as_ref().unwrap();
            let span = field.ty.span();

            let section_name = attr.section.unwrap_or_else(|| Some(field_name.to_string()));
            let section = match section_name {
                Some(name) => quote!(Some(#name)),
                None => quote!(None::<String>),
            };

            Ok(quote_spanned! { span =>
                if let Some(section) = ini.section(#section) {
                    self.#field_name.load_in(section, "")?;
                }
            })
        })
        .map(|result| result.unwrap_or_else(Error::into_compile_error));

    let output = quote! {
        #[automatically_derived]
        impl ini_persist::load::IniLoad for #struct_name {
            fn load_from_ini(&mut self, ini: &ini::Ini) -> ini_persist::Result<()> {
                use ini_persist::load::LoadProperty;
                #(#load_calls)*
                ini_persist::Result::Ok(())
            }
        }
    };

    Ok(output)
}
