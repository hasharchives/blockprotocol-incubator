use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    str::FromStr,
};

use once_cell::sync::Lazy;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use type_system::{
    url::{BaseUrl, VersionedUrl},
    EntityType, EntityTypeReference, ValueOrArray,
};

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
    "PropertiesMut",
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

        quote! {
            use crate #(:: #path)* :: #name;
        }
    })
}

enum PropertyKind {
    Array,
    Plain,
    Boxed,
}

struct Property {
    name: Ident,
    type_: Ident,

    kind: PropertyKind,

    required: bool,
}

// TODO: most of this code can likely be shared with object properties! ~> need to think about
//  hoisting
fn properties<'a>(
    entity: &'a EntityType,
    resolver: &NameResolver,
    property_names: &HashMap<&VersionedUrl, PropertyName>,
    locations: &HashMap<&VersionedUrl, Location>,
) -> BTreeMap<&'a BaseUrl, Property> {
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

            let type_ = location
                .alias
                .value
                .as_ref()
                .unwrap_or(&location.name.value);
            let type_ = Ident::new(type_, Span::call_site());

            let required = required.contains(base);

            let kind = if matches!(value, ValueOrArray::Array(_)) {
                PropertyKind::Array
            } else if resolver.analyzer().edge(entity.id(), url).kind == EdgeKind::Boxed {
                PropertyKind::Boxed
            } else {
                PropertyKind::Plain
            };

            (base, Property {
                name,
                type_,
                kind,
                required,
            })
        })
        .collect()
}

fn generate_mod(kind: &LocationKind, resolver: &NameResolver) -> Option<TokenStream> {
    let LocationKind::Latest {other} = kind else {
        return None;
    };

    let statements = other.iter().map(|url| {
        let location = resolver.location(url);
        let file = Ident::new(location.path.file().name(), Span::call_site());

        let name = Ident::new(&location.name.value, Span::call_site());

        // we do not surface the ref or mut variants, this is intentional, as they
        // should be accessed through `::Ref` and `::Mut` instead!
        // TODO: rethink this strategy!

        // optional aliases
        let name_alias = location.name.alias.as_ref().map(|alias| {
            let alias = Ident::new(alias, Span::call_site());
            quote!(pub use #file::#alias;)
        });

        quote! {
            pub mod #file;
            pub use #file::#name;
            #name_alias
        }
    });

    Some(quote!(#(#statements)*))
}

fn generate_imports(
    entity: &EntityType,
    references: &[&VersionedUrl],
    locations: &HashMap<&VersionedUrl, Location>,
    link: bool,
) -> TokenStream {
    let import_alloc = entity
        .properties()
        .values()
        .any(|value| matches!(value, ValueOrArray::Array(_)))
        .then(|| {
            quote!(
                use alloc::vec::Vec;
            )
        });

    let mut imports: Vec<_> = imports(references, locations).collect();

    if link {
        imports.push(quote!(
            use blockprotocol::entity::LinkData;
        ));
    }

    quote! {
        use serde::Serialize;
        use blockprotocol::{Type, TypeRef, TypeMut};
        use blockprotocol::{EntityType, EntityTypeRef, EntityTypeMut};
        use blockprotocol::PropertyType as _;
        use blockprotocol::{VersionedUrlRef, GenericEntityError};
        use blockprotocol::entity::Entity;
        use error_stack::Result;

        #import_alloc

        #(#imports)*
    }
}

fn generate_owned(
    entity: &EntityType,
    location: &Location,
    properties: &BTreeMap<&BaseUrl, Property>,
    link: bool,
) -> TokenStream {
    let properties = properties.iter().map(|(base, property)| {
        let url = base.as_str();
        let Property {
            name,
            type_,
            kind,
            required,
        } = property;

        let mut type_ = match kind {
            PropertyKind::Array => quote!(Vec<#type_>),
            PropertyKind::Plain => type_.to_token_stream(),
            PropertyKind::Boxed => quote!(Box<#type_>),
        };

        if !required {
            type_ = quote!(Option<#type_>);
        }

        quote! {
            #[serde(rename = #url)]
            pub #name: #type_
        }
    });

    let mut fields = vec![quote!(pub properties: Properties)];

    if link {
        fields.push(quote!(pub link_data: LinkData));
    }

    let name = Ident::new(&location.name.value, Span::call_site());
    let name_ref = Ident::new(&location.name_ref.value, Span::call_site());
    let name_mut = Ident::new(&location.name_mut.value, Span::call_site());

    let base_url = entity.id().base_url.as_str();
    let version = entity.id().version;

    let alias = location.name.alias.as_ref().map(|alias| {
        let alias = Ident::new(alias, Span::call_site());

        quote!(pub type #alias = #name;)
    });

    quote! {
        #[derive(Debug, Clone, Serialize)]
        pub struct Properties {
            #(#properties),*
        }

        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #name {
            #(#fields),*
        }

        impl Type for #name {
            type Mut<'a> = #name_mut<'a> where Self: 'a;
            type Ref<'a> = #name_ref<'a> where Self: 'a;

            const ID: VersionedUrlRef<'static>  = url!(#base_url / v / #version);

            fn as_ref(&self) -> Self::Ref<'_> {
                // TODO!
                todo!()
            }

            fn as_mut(&self) -> Self::Mut<'_> {
                // TODO!
                todo!()
            }
        }

        impl EntityType for #name {
            type Error = GenericEntityError;

            fn try_from_entity(value: Entity) -> Option<Result<Self, Self::Error>> {
                // TODO!
                todo!()
            }
        }

        #alias
    }
}

fn generate_ref(
    location: &Location,
    properties: &BTreeMap<&BaseUrl, Property>,
    link: bool,
) -> TokenStream {
    let properties = properties.iter().map(|(base, property)| {
        let url = base.as_str();
        let Property {
            name,
            type_,
            kind,
            required,
        } = property;

        let type_ = quote!(#type_::Ref<'a>);

        let mut type_ = match kind {
            PropertyKind::Array => quote!(Box<[#type_]>),
            PropertyKind::Plain => type_,
            PropertyKind::Boxed => quote!(Box<#type_>),
        };

        if !required {
            type_ = quote!(Option<#type_>);
        }

        quote! {
            #[serde(rename = #url)]
            pub #name: #type_
        }
    });

    let mut fields = vec![quote!(pub properties: PropertiesRef<'a>)];

    if link {
        fields.push(quote!(pub link_data: &'a LinkData));
    }

    let name = Ident::new(&location.name.value, Span::call_site());
    let name_ref = Ident::new(&location.name_ref.value, Span::call_site());

    let alias = location.name_ref.alias.as_ref().map(|alias| {
        let alias = Ident::new(alias, Span::call_site());

        quote!(pub type #alias<'a> = #name_ref<'a>;)
    });

    quote! {
        pub struct PropertiesRef<'a> {
            #(#properties),*
        }

        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #name_ref<'a> {
            #(#fields),*
        }

        impl TypeRef for #name_ref<'_> {
            type Owned = #name;

            fn into_owned(self) -> Self::Owned {
                // TODO!
                todo!();
            }
        }

        impl<'a> EntityTypeRef<'a> for #name_ref<'a> {
            type Error = GenericEntityError;

            fn try_from_entity(value: &'a Entity) -> Option<Result<Self, Self::Error>> {
                // TODO!
                todo!()
            }
        }

        #alias
    }
}

fn generate_mut(
    location: &Location,
    properties: &BTreeMap<&BaseUrl, Property>,
    link: bool,
) -> TokenStream {
    let properties = properties.iter().map(|(base, property)| {
        let url = base.as_str();
        let Property {
            name,
            type_,
            kind,
            required,
        } = property;

        let type_ = quote!(#type_::Mut<'a>);

        let mut type_ = match kind {
            PropertyKind::Array => quote!(Vec<#type_>),
            PropertyKind::Plain => type_,
            PropertyKind::Boxed => quote!(Box<#type_>),
        };

        if !required {
            type_ = quote!(Option<#type_>);
        }

        quote! {
            #[serde(rename = #url)]
            pub #name: #type_
        }
    });

    let mut fields = vec![quote!(pub properties: PropertiesMut<'a>)];

    if link {
        fields.push(quote!(pub link_data: &'a mut LinkData));
    }

    let name = Ident::new(&location.name.value, Span::call_site());
    let name_mut = Ident::new(&location.name_mut.value, Span::call_site());

    let alias = location.name_mut.alias.as_ref().map(|alias| {
        let alias = Ident::new(alias, Span::call_site());

        quote!(pub type #alias<'a> = #name_mut<'a>;)
    });

    quote! {
        pub struct PropertiesMut<'a> {
            #(#properties),*
        }

        #[derive(Debug, Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct #name_mut<'a> {
            #(#fields),*
        }

        impl TypeMut for #name_mut<'_> {
            type Owned = #name;

            fn into_owned(self) -> Self::Owned {
                // TODO!
                todo!();
            }
        }

        impl<'a> EntityTypeMut<'a> for #name_mut<'a> {
            type Error = GenericEntityError;

            fn try_from_entity(value: &'a mut Entity) -> Option<Result<Self, Self::Error>> {
                // TODO!
                todo!()
            }
        }

        #alias
    }
}

// Reason: most of the lines are just generation code (TODO: we might want to break up in the
// future?)
#[allow(clippy::too_many_lines)]
pub(crate) fn generate(entity: &EntityType, resolver: &NameResolver) -> TokenStream {
    let url = entity.id();

    let location = resolver.location(url);

    let property_type_references = entity.property_type_references();

    let mut references: Vec<_> = property_type_references
        .iter()
        .map(|reference| reference.url())
        .collect();
    // need to sort, as otherwise results might vary between invocations
    references.sort();

    let property_names = resolver.property_names(references.iter().map(Deref::deref));
    let locations = resolver.locations(references.iter().map(Deref::deref), RESERVED);

    let properties = properties(entity, resolver, &property_names, &locations);

    let is_link = entity
        .inherits_from()
        .all_of()
        .iter()
        .any(|reference| reference == &*LINK_REF);

    let imports = generate_imports(entity, &references, &locations, is_link);

    let owned = generate_owned(entity, &location, &properties, is_link);
    let ref_ = generate_ref(&location, &properties, is_link);
    let mut_ = generate_mut(&location, &properties, is_link);

    let mod_ = generate_mod(&location.kind, resolver);

    quote! {
        #imports

        #owned
        #ref_
        #mut_

        #mod_
    }
}
