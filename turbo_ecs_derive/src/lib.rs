mod component;

use proc_macro::TokenStream;
use syn;

#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    component::impl_component(&ast)
}