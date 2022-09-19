use quote::{format_ident, quote};
use proc_macro::TokenStream;
use syn::DeriveInput;

pub fn impl_component(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let name_str = name.to_string().to_uppercase();
    let id_name = format_ident!("__COMPONENT_ID_OF_{}", name_str);

    let gen = quote! {
        turbo_ecs::lazy_static! {
            static ref #id_name: turbo_ecs::components::ComponentId = unsafe {
                turbo_ecs::components::component_id::get_next()
            };
        }

        impl turbo_ecs::components::Component for #name {}

        impl turbo_ecs::components::component_id::HasComponentId for #name {
            #[inline(always)]
            fn component_id() -> turbo_ecs::components::ComponentId {
                *#id_name
            }
        }

        impl turbo_ecs::components::ComponentTypeInfo for #name {
            type ComponentType = #name;

            #[inline(always)]
            fn component_id() -> turbo_ecs::components::ComponentId {
                turbo_ecs::components::ComponentId::of::<#name>()
            }
        }

        impl turbo_ecs::entities::ComponentQuery for #name {
            type Arguments = <(#name, ()) as turbo_ecs::entities::ComponentQuery>::Arguments;

            #[inline(always)]
            fn get_query() -> turbo_ecs::entities::EntityQuery {
                <(#name, ()) as turbo_ecs::entities::ComponentQuery>::get_query()
            }
        }
    };
    gen.into()
}