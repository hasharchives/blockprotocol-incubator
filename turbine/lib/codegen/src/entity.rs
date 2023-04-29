use std::{collections::HashMap, ops::Deref};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use type_system::{url::VersionedUrl, EntityType, ValueOrArray};

use crate::{
    analysis::EdgeKind,
    name::{Location, LocationKind, NameResolver, PropertyName},
};

fn imports<'a>(
    references: impl IntoIterator<Item = &'a &'a VersionedUrl> + 'a,
    locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,
) -> impl Iterator<Item = TokenStream> + 'a {
    references.into_iter().map(|reference| {
        // explicit type not needed here, but CLion otherwise complains
        let location: &Location = &locations[reference];

        let mut path: Vec<_> = location
            .path
            .directories()
            .iter()
            .map(|directory| Ident::new(directory.name(), Span::call_site()))
            .collect();

        // only add to path if we're not a mod.rs file, otherwise it will lead to import errors
        if !location.path.file().is_mod() {
            path.push(Ident::new(location.path.file().name(), Span::call_site()));
        }

        let mut name = Ident::new(&location.name.value, Span::call_site()).to_token_stream();

        if let Some(alias) = &location.alias.owned {
            let alias = Ident::new(alias, Span::call_site());
            name = quote!(#name as #alias);
        }

        let mut ref_name =
            Ident::new(&location.ref_name.value, Span::call_site()).to_token_stream();

        if let Some(alias) = &location.alias.reference {
            let alias = Ident::new(alias, Span::call_site());
            ref_name = quote!(#ref_name as #alias);
        }

        quote! {
            use crate #(:: #path)* :: #name;
            use crate #(:: #path)* :: #ref_name;
        }
    })
}

fn properties(
    entity: &EntityType,
    resolver: &NameResolver,
    property_names: &HashMap<&VersionedUrl, PropertyName>,
    locations: &HashMap<&VersionedUrl, Location>,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let required = entity.required();

    entity
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

            let reference = Ident::new(
                location
                    .alias
                    .reference
                    .as_ref()
                    .unwrap_or(&location.ref_name.value),
                Span::call_site(),
            );
            let mut reference = quote!(#reference<'a>);

            if matches!(value, ValueOrArray::Array(_)) {
                owned = quote!(Vec<#owned>);
                reference = quote!(Vec<#reference);
            } else if resolver.analyzer().edge(entity.id(), url).kind == EdgeKind::Boxed {
                owned = quote!(Box<#owned>);
                reference = quote!(Box<#reference);
            }

            if !required.contains(base) {
                owned = quote!(Option<#owned>);
                reference = quote!(Option<#reference>);
            }

            (quote!(pub #name: #owned), quote!(pub #name: #reference))
        })
        .unzip()
}

fn versions(kind: LocationKind, resolver: &NameResolver) -> Vec<TokenStream> {
    match kind {
        LocationKind::Latest { other } => {
            other
                .iter()
                .map(|url| {
                    let location = resolver.location(url);
                    let file = Ident::new(location.path.file().name(), Span::call_site());

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
    }
}

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

        quote!(pub type #alias<'a> = #name<'a>;)
    });

    let property_type_references = entity.property_type_references();

    let mut references: Vec<_> = property_type_references
        .iter()
        .map(|reference| reference.url())
        .collect();
    // need to sort, as otherwise results might vary between invocations
    references.sort();

    let property_names = resolver.property_names(references.iter().map(Deref::deref));
    let locations = resolver.locations(references.iter().map(Deref::deref));

    let import_alloc = entity
        .properties()
        .values()
        .any(|value| matches!(value, ValueOrArray::Array(_)))
        .then(|| {
            quote!(
                use alloc::vec::Vec;
            )
        });

    let imports = imports(&references, &locations);

    let (properties, properties_ref) = properties(entity, resolver, &property_names, &locations);

    // TODO: is_link!

    let version = entity.id().version;
    let base_url = entity.id().base_url.as_str();

    let versions = versions(location.kind, resolver);

    quote! {
        #import_alloc

        #(#imports)*

        use blockprotocol::{Type, EntityType, TypeRef, EntityTypeRef, GenericEntityError, VersionedUrlRef};
        use blockprotocol::entity::Entity;
        use blockprotocol::url;

        #[derive(Debug, Clone)]
        pub struct #name {
            #(#properties),*
        }

        // TODO: accessors?

        impl Type for #name {
            type Ref<'a> = #ref_name<'a> where Self: 'a;

            const ID: VersionedUrlRef<'static>  = url!(#base_url / v / #version);

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
