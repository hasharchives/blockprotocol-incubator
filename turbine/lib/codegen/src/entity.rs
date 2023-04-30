use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    str::FromStr,
};

use once_cell::sync::Lazy;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use type_system::{url::VersionedUrl, EntityType, EntityTypeReference, ValueOrArray};

use crate::{
    analysis::EdgeKind,
    name::{Location, LocationKind, NameResolver, PropertyName},
};

const RESERVED: &[&str] = &[
    "Type",
    "TypeRef",
    "EntityType",
    "EntityTypeRef",
    "VersionedUrlRef",
    "GenericEntityError",
    "Entity",
    "LinkData",
    "Serialize",
    "Properties",
    "PropertiesRef",
];

static LINK_REF: Lazy<EntityTypeReference> = Lazy::new(|| {
    EntityTypeReference::new(
        VersionedUrl::from_str(
            "https://blockprotocol.org/@blockprotocol/types/entity-type/link/v/1",
        )
        .expect("should be valid url"),
    )
});

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

        if let Some(alias) = &location.alias.value {
            let alias = Ident::new(alias, Span::call_site());
            name = quote!(#name as #alias);
        }

        let mut ref_name =
            Ident::new(&location.name_ref.value, Span::call_site()).to_token_stream();

        if let Some(alias) = &location.alias.value_ref {
            let alias = Ident::new(alias, Span::call_site());
            ref_name = quote!(#ref_name as #alias);
        }

        quote! {
            use crate #(:: #path)* :: #name;
            use crate #(:: #path)* :: #ref_name;
        }
    })
}

// TODO: most of this code can likely be shared with object properties! ~> need to think about
//  hoisting
fn properties(
    entity: &EntityType,
    resolver: &NameResolver,
    property_names: &HashMap<&VersionedUrl, PropertyName>,
    locations: &HashMap<&VersionedUrl, Location>,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let required = entity.required();

    // we need consistent ordering, otherwise output is going to differ _every_ time
    let properties: BTreeMap<_, _> = entity.properties().iter().collect();

    properties
        .into_iter()
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
                    .value
                    .as_ref()
                    .unwrap_or(&location.name.value),
                Span::call_site(),
            );
            let mut owned = quote!(#owned);

            let reference = Ident::new(
                location
                    .alias
                    .value_ref
                    .as_ref()
                    .unwrap_or(&location.name_ref.value),
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

            let base = base.as_str();
            (
                quote! {
                    #[serde(rename = #base)]
                    pub #name: #owned
                },
                quote! {
                    #[serde(rename = #base)]
                    pub #name: #reference
                },
            )
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
                    let ref_name = Ident::new(&location.name_ref.value, Span::call_site());

                    // optional aliases
                    let name_alias = location.name.alias.as_ref().map(|alias| {
                        let alias = Ident::new(alias, Span::call_site());
                        quote!(pub use #file::#alias;)
                    });
                    let ref_name_alias = location.name_ref.alias.as_ref().map(|alias| {
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

// Reason: most of the lines are just generation code (TODO: we might want to break up in the
// future?)
#[allow(clippy::too_many_lines)]
pub(crate) fn generate(entity: &EntityType, resolver: &NameResolver) -> TokenStream {
    let url = entity.id();

    let location = resolver.location(url);

    let name = Ident::new(&location.name.value, Span::call_site());
    let ref_name = Ident::new(&location.name_ref.value, Span::call_site());

    let alias = location.name.alias.map(|alias| {
        let alias = Ident::new(&alias, Span::call_site());

        quote!(pub type #alias = #name;)
    });
    let ref_alias = location.name_ref.alias.map(|alias| {
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
    let locations = resolver.locations(references.iter().map(Deref::deref), RESERVED);

    let import_alloc = entity
        .properties()
        .values()
        .any(|value| matches!(value, ValueOrArray::Array(_)))
        .then(|| {
            quote!(
                use alloc::vec::Vec;
            )
        });

    let mut imports: Vec<_> = imports(&references, &locations).collect();

    let (properties, properties_ref) = properties(entity, resolver, &property_names, &locations);

    let is_link = entity
        .inherits_from()
        .all_of()
        .iter()
        .any(|reference| reference == &*LINK_REF);

    let mut fields = vec![quote!(pub properties: Properties)];
    let mut fields_ref = vec![quote!(pub properties: PropertiesRef<'a>)];

    if is_link {
        imports.push(quote! {
            use blockprotocol::entity::LinkData;
        });
        fields.push(quote!(pub link_data: LinkData));
        fields_ref.push(quote!(pub link_data: &'a LinkData));
    }

    let version = entity.id().version;
    let base_url = entity.id().base_url.as_str();

    let versions = versions(location.kind, resolver);

    quote! {
        use serde::Serialize;
        use blockprotocol::{Type, TypeRef, EntityType, EntityTypeRef, VersionedUrlRef, GenericEntityError};
        use blockprotocol::entity::Entity;
        use error_stack::Result;

        #import_alloc

        #(#imports)*

        #[derive(Debug, Clone, Serialize)]
        pub struct Properties {
            #(#properties),*
        }

        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #name {
            #(#fields),*
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

        impl blockprotocol::EntityType for #name {
            type Error = GenericEntityError;

            fn try_from_entity(value: Entity) -> Option<Result<Self, Self::Error>> {
                // TODO!
                todo!()
            }
        }

        pub struct PropertiesRef<'a> {
            #(#properties_ref),*
        }

        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #ref_name<'a> {
            #(#fields_ref),*
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

            fn try_from_entity(value: &'a Entity) -> Option<Result<Self, Self::Error>> {
                // TODO!
                todo!()
            }
        }

        #alias
        #ref_alias

        #(#versions)*
    }
}
