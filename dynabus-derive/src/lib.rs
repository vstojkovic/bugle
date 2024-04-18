use quote::quote;
use syn::{parse_macro_input, ConstParam, DeriveInput, GenericParam, LifetimeParam, TypeParam};

#[proc_macro_derive(Event)]
pub fn derive_event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;
    let generic_decls = if !input.generics.params.is_empty() {
        let params = input.generics.params.iter();
        quote!(<#(#params),*>)
    } else {
        quote!()
    };
    let param_list = if !input.generics.params.is_empty() {
        let params: Vec<_> = input
            .generics
            .params
            .iter()
            .map(|param| match param {
                GenericParam::Lifetime(LifetimeParam { lifetime, .. }) => quote!(#lifetime),
                GenericParam::Type(TypeParam { ident, .. }) => quote!(#ident),
                GenericParam::Const(ConstParam { ident, .. }) => quote!(#ident),
            })
            .collect();
        quote!(<#(#params),*>)
    } else {
        quote!()
    };
    let where_clause =
        if let Some(clause) = input.generics.where_clause { quote!(#clause) } else { quote!() };

    let output = quote! {
        #[automatically_derived]
        impl #generic_decls dynabus::Event for #ident #param_list #where_clause {}
    };
    output.into()
}
