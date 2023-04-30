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
    shared,
    shared::{generate_mod, imports, Property, PropertyKind},
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

#[derive(Debug, Copy, Clone)]
struct Import {
    vec: bool,
    box_: bool,
}

struct State {
    is_link: bool,
    import: Import,
}

fn properties<'a>(
    entity: &'a EntityType,
    resolver: &NameResolver,
    property_names: &HashMap<&VersionedUrl, PropertyName>,
    locations: &HashMap<&VersionedUrl, Location>,
) -> BTreeMap<&'a BaseUrl, Property> {
    shared::properties(
        entity.id(),
        entity.properties(),
        entity.required(),
        resolver,
        property_names,
        locations,
    )
}

// TODO: adapt code from properties
fn generate_use(
    references: &[&VersionedUrl],
    locations: &HashMap<&VersionedUrl, Location>,
    state: State,
) -> TokenStream {
    let mut imports: Vec<_> = imports(references, locations).collect();

    if state.is_link {
        imports.push(quote!(
            use blockprotocol::entity::LinkData;
        ));
    }

    if state.import.box_ {
        imports.push(quote!(
            use alloc::boxed::Box;
        ));
    }

    if state.import.vec {
        imports.push(quote!(
            use alloc::vec::Vec;
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

        #(#imports)*
    }
}

fn generate_owned(
    entity: &EntityType,
    location: &Location,
    properties: &BTreeMap<&BaseUrl, Property>,
    state: &mut State,
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
            PropertyKind::Array => {
                state.import.vec = true;
                quote!(Vec<#type_>)
            }
            PropertyKind::Plain => type_.to_token_stream(),
            PropertyKind::Boxed => {
                state.import.box_ = true;
                quote!(Box<#type_>)
            }
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

    if state.is_link {
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
    state: &mut State,
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
            PropertyKind::Array => {
                state.import.box_ = true;
                quote!(Box<[#type_]>)
            }
            PropertyKind::Plain => type_,
            PropertyKind::Boxed => {
                state.import.box_ = true;
                quote!(Box<#type_>)
            }
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

    if state.is_link {
        fields.push(quote!(pub link_data: &'a LinkData));
    }

    let name = Ident::new(&location.name.value, Span::call_site());
    let name_ref = Ident::new(&location.name_ref.value, Span::call_site());

    let alias = location.name_ref.alias.as_ref().map(|alias| {
        let alias = Ident::new(alias, Span::call_site());

        quote!(pub type #alias<'a> = #name_ref<'a>;)
    });

    quote! {
        #[derive(Debug, Clone, Serialize)]
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
    state: &mut State,
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
            PropertyKind::Array => {
                state.import.vec = true;
                quote!(Vec<#type_>)
            }
            PropertyKind::Plain => type_,
            PropertyKind::Boxed => {
                state.import.box_ = true;
                quote!(Box<#type_>)
            }
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

    if state.is_link {
        fields.push(quote!(pub link_data: &'a mut LinkData));
    }

    let name = Ident::new(&location.name.value, Span::call_site());
    let name_mut = Ident::new(&location.name_mut.value, Span::call_site());

    let alias = location.name_mut.alias.as_ref().map(|alias| {
        let alias = Ident::new(alias, Span::call_site());

        quote!(pub type #alias<'a> = #name_mut<'a>;)
    });

    quote! {
        #[derive(Debug, Serialize)]
        pub struct PropertiesMut<'a> {
            #(#properties),*
        }

        #[derive(Debug, Serialize)]
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

    let mut state = State {
        is_link,
        import: Import {
            vec: false,
            box_: false,
        },
    };

    let owned = generate_owned(entity, &location, &properties, &mut state);
    let ref_ = generate_ref(&location, &properties, &mut state);
    let mut_ = generate_mut(&location, &properties, &mut state);

    let mod_ = generate_mod(&location.kind, resolver);
    let use_ = generate_use(&references, &locations, state);

    quote! {
        #use_

        #owned
        #ref_
        #mut_

        #mod_
    }
}

// TODO: test builtin import name clash resolve!
