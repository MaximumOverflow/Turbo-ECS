use proc_macro::TokenStream;
use syn::DeriveInput;
use quote::quote;

pub fn impl_component(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl turbo_ecs::components::Component for #name {}

        impl turbo_ecs::components::ComponentTypeInfo for #name {
            type ComponentType = #name;
            fn component_id() -> turbo_ecs::components::ComponentId {
                turbo_ecs::components::ComponentId::of::<#name>()
            }
        }

        impl turbo_ecs::components::ComponentFrom<#name> for #name {
            unsafe fn convert(value: #name) -> Self { value.clone() }
        }

        impl turbo_ecs::components::ComponentFrom<*const #name> for #name {
            unsafe fn convert(value: *const #name) -> Self { unsafe { *value } }
        }

        impl turbo_ecs::components::ComponentFrom<*const #name> for &#name {
            unsafe fn convert(value: *const #name) -> Self { unsafe { &*value } }
        }

        impl turbo_ecs::components::ComponentFrom<*mut #name> for #name {
            unsafe fn convert(value: *mut #name) -> Self { unsafe { *value } }
        }

        impl turbo_ecs::components::ComponentFrom<*mut #name> for &#name {
            unsafe fn convert(value: *mut #name) -> Self { unsafe { &mut *value } }
        }

        impl turbo_ecs::components::ComponentFrom<*mut #name> for &mut #name {
            unsafe fn convert(value: *mut #name) -> Self { unsafe { &mut *value } }
        }

        impl turbo_ecs::components::ComponentSet for #name {
            #[inline(always)]
            fn get_bitfield() -> std::sync::Arc<turbo_ecs::data_structures::BitField> {
                <(#name, ) as turbo_ecs::components::ComponentSet>::get_bitfield()
            }
        }

        impl turbo_ecs::components::ComponentSet for &#name {
            #[inline(always)]
            fn get_bitfield() -> std::sync::Arc<turbo_ecs::data_structures::BitField> {
                <(&#name, ) as turbo_ecs::components::ComponentSet>::get_bitfield()
            }
        }

        impl turbo_ecs::components::ComponentSet for &mut #name {
            #[inline(always)]
            fn get_bitfield() -> std::sync::Arc<turbo_ecs::data_structures::BitField> {
                <(&mut #name, ) as turbo_ecs::components::ComponentSet>::get_bitfield()
            }
        }
    };
    gen.into()
}