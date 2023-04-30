use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use type_system::{
    url::{BaseUrl, VersionedUrl},
    DataTypeReference, Object, PropertyType, PropertyTypeReference, PropertyValues, ValueOrArray,
};

use crate::{
    name::{Location, NameResolver, PropertyName},
    shared,
    shared::{Property, PropertyKind},
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Variant {
    Owned,
    Ref,
    Mut,
}

const RESERVED: &[&str] = &[
    "Type",
    "TypeRef",
    "PropertyType",
    "PropertyTypeRef",
    "VersionedUrlRef",
    "GenericPropertyError",
    "Serialize",
    // TODO: clashes here need to be handled specifically ~> problem we increment them ourselves
    "Inner",
];

struct Inner {
    name: Ident,
    stream: TokenStream,
}

impl ToTokens for Inner {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.stream.clone());
    }
}

fn properties<'a>(
    id: &VersionedUrl,
    object: &'a Object<ValueOrArray<PropertyTypeReference>, 1>,
    resolver: &NameResolver,
    property_names: &HashMap<&VersionedUrl, PropertyName>,
    locations: &HashMap<&VersionedUrl, Location>,
) -> BTreeMap<&'a BaseUrl, Property> {
    shared::properties(
        id,
        object.properties(),
        object.required(),
        resolver,
        property_names,
        locations,
    )
}

fn generate_type(
    id: &VersionedUrl,
    name: &Ident,
    variant: Variant,
    values: &[PropertyValues],
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    inner: &mut Vec<Inner>,
) -> TokenStream {
    let lifetime = match variant {
        Variant::Ref | Variant::Mut => Some(quote!(<'a>)),
        Variant::Owned => None,
    };

    if let [value] = values {
        let semicolon = match value {
            PropertyValues::PropertyTypeObject(_) => false,
            PropertyValues::ArrayOfPropertyValues(_) | PropertyValues::DataTypeReference(_) => true,
        };

        // we can hoist!
        let body = generate_contents(id, variant, value, resolver, locations, true, inner);
        let semicolon = semicolon.then_some(quote!(;));

        return quote! {
            // TODO: try_from_value (depending on variant)
            pub struct #name #lifetime #body #semicolon
        };
    }

    // we cannot hoist and therefore need to create an enum
    let body = values.iter().enumerate().map(|(index, value)| {
        let body = generate_contents(id, variant, value, resolver, locations, false, inner);
        let name = format_ident!("Variant{index}");

        // TODO: try_from_value
        quote! {
            #name #body
        }
    });

    // TODO: try_from_value
    quote! {
        pub enum #name #lifetime {
            #(#body),*
        }
    }
}

fn generate_inner(
    id: &VersionedUrl,
    variant: Variant,
    values: &[PropertyValues],
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    inner: &mut Vec<Inner>,
) -> Ident {
    let n = inner.len();
    let name = format_ident!("Inner{n}");

    let type_ = generate_type(id, &name, variant, values, resolver, locations, inner);

    inner.push(Inner {
        name: name.clone(),
        stream: type_,
    });

    name
}

fn generate_contents(
    id: &VersionedUrl,
    variant: Variant,
    value: &PropertyValues,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    hoist: bool,
    inner: &mut Vec<Inner>,
) -> TokenStream {
    match value {
        PropertyValues::DataTypeReference(reference) => {
            let location = &locations[reference.url()];
            let vis = hoist.then_some(quote!(pub));

            let name = location
                .alias
                .value
                .as_ref()
                .unwrap_or(&location.name.value);
            let name = Ident::new(name, Span::call_site());

            match variant {
                Variant::Owned => quote!((#vis #name)),
                Variant::Ref => quote!((#vis #name::Ref<'a>)),
                Variant::Mut => quote!((#vis #name::Mut<'a>)),
            }
        }
        PropertyValues::PropertyTypeObject(object) => {
            let property_names = resolver.property_names(object.properties().values().map(
                |property| match property {
                    ValueOrArray::Value(value) => value.url(),
                    ValueOrArray::Array(value) => value.items().url(),
                },
            ));

            let properties = properties(id, object, resolver, &property_names, locations);

            let properties = properties.iter().map(|(base, property)| {
                let url = base.as_str();
                let Property {
                    name,
                    type_,
                    kind,
                    required,
                } = property;

                let type_ = match variant {
                    Variant::Owned => type_.to_token_stream(),
                    Variant::Ref => quote!(#type_::Ref<'a>),
                    Variant::Mut => quote!(#type_::Mut<'a>),
                };

                let mut type_ = match kind {
                    PropertyKind::Array if variant == Variant::Owned || variant == Variant::Mut => {
                        quote!(Vec<#type_>)
                    }
                    PropertyKind::Array => quote!(Box<[#type_]>),
                    PropertyKind::Plain => type_,
                    PropertyKind::Boxed => quote!(Box<#type_>),
                };

                if !required {
                    type_ = quote!(Option<#type_>);
                }

                let vis = hoist.then_some(quote!(pub));

                quote! {
                    #[serde(rename = #url)]
                    #vis #name: #type_
                }
            });

            quote! {
                {
                    #(#properties),*
                }
            }
        }
        // TODO: automatically flatten, different modes?, inner data-type reference should just be a
        //  newtype?
        // TODO: needs a `generate_object` in that case ~> not really tho
        PropertyValues::ArrayOfPropertyValues(array) => {
            let items = array.items();
            let inner = generate_inner(id, variant, items.one_of(), resolver, locations, inner);

            let vis = hoist.then_some(quote!(pub));

            let lifetime = match variant {
                Variant::Ref | Variant::Mut => Some(quote!(<'a>)),
                Variant::Owned => None,
            };

            // in theory we could do some more hoisting, e.g. if we have multiple OneOf that are
            // Array
            quote!((#vis Vec<#inner #lifetime>))
        }
    }
}

// id: &VersionedUrl,
//     name: &Ident,
//     variant: Variant,
//     values: &[PropertyValues],
//     resolver: &NameResolver,
//     locations: &HashMap<&VersionedUrl, Location>,
//     inner: &mut Vec<Inner>,
// TODO: locations should be generated from union of all types!
fn generate_owned(
    property: &PropertyType,
    location: &Location,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    inner: &mut Vec<Inner>,
) -> TokenStream {
    let name = Ident::new(location.name.value.as_str(), Span::call_site());

    // TODO: this is incomplete!
    generate_type(
        property.id(),
        &name,
        Variant::Owned,
        property.one_of(),
        resolver,
        locations,
        inner,
    )
}

fn generate_ref(
    property: &PropertyType,
    location: &Location,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    inner: &mut Vec<Inner>,
) -> TokenStream {
    let name = Ident::new(location.name_ref.value.as_str(), Span::call_site());

    generate_type(
        property.id(),
        &name,
        Variant::Ref,
        property.one_of(),
        resolver,
        locations,
        inner,
    )
}

fn generate_mut(
    property: &PropertyType,
    location: &Location,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    inner: &mut Vec<Inner>,
) -> TokenStream {
    let name = Ident::new(location.name_mut.value.as_str(), Span::call_site());

    generate_type(
        property.id(),
        &name,
        Variant::Mut,
        property.one_of(),
        resolver,
        locations,
        inner,
    )
}

// Generate the code for all oneOf, depending (with the () vs. {}) and extra types required,
// then if oneOf is one use a struct instead, inner types (`Inner`) should be
// generated via a mutable vec
pub(crate) fn generate(property: &PropertyType, resolver: &NameResolver) -> TokenStream {
    let location = resolver.location(property.id());
    let name = Ident::new(location.name.value.as_str(), Span::call_site());

    let mut references: Vec<_> = property
        .property_type_references()
        .into_iter()
        .map(PropertyTypeReference::url)
        .chain(
            property
                .data_type_references()
                .into_iter()
                .map(DataTypeReference::url),
        )
        .collect();
    // need to sort, as otherwise results might vary between invocations
    references.sort();

    let locations = resolver.locations(references.iter().map(Deref::deref), RESERVED);

    let mut inner = vec![];

    let owned = generate_owned(property, &location, resolver, &locations, &mut inner);
    let ref_ = generate_ref(property, &location, resolver, &locations, &mut inner);
    let mut_ = generate_mut(property, &location, resolver, &locations, &mut inner);

    quote! {
        #(#inner)*

        #owned
        #ref_
        #mut_
    }
}

// TODO: mod handling, use handling (data-type), `Inner` DENY_LIST, alias
// TODO: test multiple versions
