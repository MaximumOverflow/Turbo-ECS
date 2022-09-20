use quote::{format_ident, quote};
use proc_macro::TokenStream;
use syn::DeriveInput;

pub fn impl_component(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let name_str = name.to_string().to_uppercase();
    let id_name = format_ident!("__COMPONENT_ID_OF_{}", name_str);

    let gen = quote! {
        turbo_ecs::lazy_static! {
            static ref #id_name: turbo_ecs::components::component_id::ComponentId = unsafe {
                turbo_ecs::components::component_id::get_next()
            };
        }

        impl turbo_ecs::components::Component for #name {
            #[inline(always)]
            fn component_id() -> turbo_ecs::components::component_id::ComponentId {
                *#id_name
            }
        }

        impl turbo_ecs::components::ComponentTypeInfo for #name {
            type ComponentType = #name;

            #[inline(always)]
            fn component_id() -> turbo_ecs::components::component_id::ComponentId {
                turbo_ecs::components::component_id::ComponentId::of::<#name>()
            }
        }
    };
    gen.into()
}