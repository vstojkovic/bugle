use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Error, Fields, Ident, Result};

use crate::attr::{FieldAttr, LoadFn, StructAttr};

pub fn expand_load(input: DeriveInput) -> Result<TokenStream> {
    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("IniLoad can only be derived on a struct with named fields"),
    };

    let attr = StructAttr::from_ast(input.attrs.iter())?;
    let struct_name = input.ident;
    let section_name = attr
        .section
        .unwrap_or_else(|| Some(struct_name.to_string()));
    let section_quote = match section_name {
        Some(name) => quote!(Some(#name)),
        None => quote!(None::<String>),
    };

    let load_calls = fields
        .into_iter()
        .map(|field| {
            let attr = FieldAttr::from_ast(field.attrs.iter())?;
            let field_name = field.ident.as_ref().unwrap();
            let span = field.ty.span();

            if attr.flatten.is_some() {
                expand_flattened_field(field_name, span)
            } else {
                expand_field(field_name, attr, span)
            }
        })
        .map(|result| result.unwrap_or_else(Error::into_compile_error));

    let output = quote! {
        #[automatically_derived]
        impl ini_persist::load::IniLoad for #struct_name {
            fn load_from_ini(&mut self, ini: &ini::Ini) -> ini_persist::Result<()> {
                use ini_persist::load::Property;
                if let Some(props) = ini.section(#section_quote) {
                    #(#load_calls)*
                }
                Ok(())
            }
        }
    };

    Ok(output)
}

fn expand_field(name: &Ident, attr: FieldAttr, span: Span) -> Result<TokenStream> {
    let key = attr.key.unwrap_or_else(|| name.to_string());
    Ok(match attr.load_fn {
        None => quote_spanned! { span =>
            self.#name.load_in(props, #key)?;
        },
        Some(LoadFn::InPlace(path)) => quote_spanned! { span =>
            #path(&mut self.#name, props, #key)?;
        },
        Some(LoadFn::Constructed(path)) => quote_spanned! { span =>
            if let Some(value) = #path(props, #key)? {
                self.#name = value;
            }
        },
        Some(LoadFn::Parsed(path)) => quote_spanned! { span =>
            if let Some(value) = props.get(#key) {
                self.#name = #path(value)?;
            }
        },
    })
}

fn expand_flattened_field(name: &Ident, span: Span) -> Result<TokenStream> {
    Ok(quote_spanned! { span =>
        self.#name.load_from_ini(ini)?;
    })
}
