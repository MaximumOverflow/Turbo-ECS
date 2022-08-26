use proc_macro::TokenStream;
use syn::DeriveInput;
use quote::quote;

pub fn impl_system(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let gen = quote!{};
    gen.into()
}