use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Fields, Result};

pub fn expand_load(input: DeriveInput) -> Result<TokenStream> {
    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("IniLoad can only be derived on a struct with named fields"),
    };
    let struct_name = input.ident;
    let section_name = struct_name.to_string();
    let load_calls = fields.into_iter().map(|field| {
        let span = field.span();
        let field_name = field.ident.as_ref().unwrap();
        let key = field_name.to_string();
        quote_spanned! { span =>
            self.#field_name.load_in(props, #key)?;
        }
    });
    let output = quote! {
        #[automatically_derived]
        impl ini_persist::load::IniLoad for #struct_name {
            fn load_from_ini(&mut self, ini: ini::Ini) -> ini_persist::Result<()> {
                use ini_persist::load::Property;
                if let Some(props) = ini.section(Some(#section_name)) {
                    #(#load_calls)*
                }
                Ok(())
            }
        }
    };
    Ok(output)
}
