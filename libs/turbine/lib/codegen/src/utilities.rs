use proc_macro2::TokenStream;
use quote::quote;
use type_system::EntityType;

use crate::{name::NameResolver, shared::determine_import_path};

fn generate_find_inherits_from<'a>(
    entities: impl IntoIterator<Item = &'a EntityType>,
    resolver: &NameResolver,
) -> TokenStream {
    let arms = resolver
        .locations(entities.into_iter().map(EntityType::id), &[])
        .into_values()
        .map(|location| {
            let path = determine_import_path(&location);
            let name = location.name.value;

            let ident = quote!(crate #(:: #path)* #name);

            quote!(
                #ident::ID => #ident::InheritsFrom::resolve().collect()
            )
        });

    quote! {
        pub fn find_inherits_from(url: turbine::VersionedUrlRef) -> alloc::collections::BTreeSet<turbine::VersionedUrlRef<'static>> {
            match url {
                #(#arms),*
                _ => alloc::collections::BTreeSet::new()
            }
        }
    }
}

pub(crate) fn generate<'a>(
    entities: impl IntoIterator<Item = &'a EntityType> + Clone,
    resolver: &NameResolver,
) -> TokenStream {
    generate_find_inherits_from(entities, resolver)
}
