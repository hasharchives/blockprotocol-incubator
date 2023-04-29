use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use type_system::{EntityType, ValueOrArray};

use crate::name::{LocationKind, NameResolver};

pub(crate) fn generate(entity: &EntityType, resolver: &NameResolver) -> TokenStream {
    let url = entity.id();

    let location = resolver.location(url);

    let name = Ident::new(&location.name.value, Span::call_site());
    let ref_name = Ident::new(&location.ref_name.value, Span::call_site());

    let alias = location.name.alias.map(|alias| {
        let alias = Ident::new(&alias, Span::call_site());

        quote!(pub type #alias = #name;)
    });
    let ref_alias = location.ref_name.alias.map(|alias| {
        let alias = Ident::new(&alias, Span::call_site());

        quote!(pub type #alias = #name;)
    });

    let property_type_references = entity.property_type_references();

    let property_names = resolver.property_names(
        property_type_references
            .iter()
            .map(|reference| reference.url()),
    );

    let locations = resolver.locations(
        property_type_references
            .iter()
            .map(|reference| reference.url()),
    );

    let mut import_alloc = entity
        .properties()
        .values()
        .any(|value| matches!(value, ValueOrArray::Array(_)))
        .then(|| {
            quote!(
                use alloc::vec::Vec;
            )
        });

    let imports = property_type_references.iter().map(|reference| {
        let location = &locations[reference.url()];

        let mut path: Vec<_> = location
            .path
            .0
            .iter()
            .map(|directory| Ident::new(&directory.0, Span::call_site()))
            .collect();

        // only add to path if we're not a mod.rs file, otherwise it will lead to import errors
        if !location.path.1.is_mod() {
            path.push(Ident::new(&location.path.1.0, Span::call_site()));
        }

        let name = Ident::new(
            location
                .alias
                .owned
                .as_ref()
                .unwrap_or(&location.name.value),
            Span::call_site(),
        );
        let ref_name = Ident::new(
            location
                .alias
                .reference
                .as_ref()
                .unwrap_or(&location.ref_name.value),
            Span::call_site(),
        );

        quote! {
            use crate #(:: #path)* :: #name;
            use crate #(:: #path)* :: #ref_name;
        }
    });

    let (properties, properties_ref): (Vec<_>, Vec<_>) = entity
        .properties()
        .iter()
        .map(|(base, value)| {
            let url = match value {
                ValueOrArray::Value(value) => value.url(),
                ValueOrArray::Array(value) => value.items().url(),
            };

            let name = Ident::new(&property_names[url].0, Span::call_site());
            let location = &locations[url];

            let owned = Ident::new(
                location
                    .alias
                    .owned
                    .as_ref()
                    .unwrap_or(&location.name.value),
                Span::call_site(),
            );
            let mut owned = quote!(#owned);

            let mut reference = Ident::new(
                location
                    .alias
                    .reference
                    .as_ref()
                    .unwrap_or(&location.ref_name.value),
                Span::call_site(),
            );
            let mut reference = quote!(#reference);

            if matches!(value, ValueOrArray::Array(_)) {
                owned = quote!(Vec<#owned>);
                reference = quote!(Vec<#reference>);
            }

            (quote!(pub #name: #owned), quote!(pub #name: #reference))
        })
        .unzip();

    // TODO: is_link!

    let version = entity.id().version;
    let base_url = entity.id().base_url.as_str();

    let versions = match location.kind {
        LocationKind::Latest { other } => {
            other
                .iter()
                .map(|url| {
                    let location = resolver.location(url);
                    let file = Ident::new(&location.path.1.0, Span::call_site());

                    let name = Ident::new(&location.name.value, Span::call_site());
                    let ref_name = Ident::new(&location.ref_name.value, Span::call_site());

                    // optional aliases
                    let name_alias = location.name.alias.as_ref().map(|alias| {
                        let alias = Ident::new(alias, Span::call_site());
                        quote!(pub use #file::#alias;)
                    });
                    let ref_name_alias = location.ref_name.alias.as_ref().map(|alias| {
                        let alias = Ident::new(alias, Span::call_site());
                        quote!(pub use #file::#alias;)
                    });

                    quote! {
                        pub mod #file;
                        pub use #file::#name;
                        pub use #file::#ref_name;
                        #name_alias
                        #ref_name_alias
                    }
                })
                .collect::<Vec<_>>()
        }
        LocationKind::Version => vec![],
    };

    // TODO: required vs. not required (`Option`) vs no `Option`

    quote! {
        #import_alloc

        #(#imports);*

        use blockprotocol::{Type, EntityType, TypeRef, EntityTypeRef, GenericEntityError};
        use blockprotocol::entity::Entity;
        use blockprotocol::url;

        #[derive(Debug, Clone)]
        pub struct #name {
            #(#properties),*
        }

        // TODO: accessors?

        impl Type for #name {
            type Ref<'a> = #ref_name where Self: 'a;

            const ID = url!(#base_url / v / #version);

            fn as_ref(&self) -> Self::Ref<'_> {
                // TODO!
                todo!()
            }
        }

        impl EntityType for #name {
            type Error = GenericEntityError;

            fn try_from_entity(value: Entity) -> Result<Self, Self::Error> {
                // TODO!
                todo!()
            }
        }

        #[derive(Debug, Clone)]
        pub struct #ref_name<'a> {
            #(#properties_ref),*
        }

        // TODO: accessors?

        impl TypeRef for #ref_name<'_> {
            type Owned = #name;

            fn into_owned(self) -> Self::Owned {
                // TODO!
                todo!();
            }
        }

        impl<'a> EntityTypeRef<'a> for #ref_name<'a> {
            type Error = GenericEntityError;

            fn try_from_entity(value: &'a Entity) -> Result<Self, Self::Error> {
                // TODO!
                todo!()
            }
        }

        #alias
        #ref_alias

        #(#versions)*
    }
}
