use std::collections::{BTreeMap, HashMap};

use proc_macro2::{Ident, Span};
use type_system::{
    url::{BaseUrl, VersionedUrl},
    PropertyTypeReference, ValueOrArray,
};

use crate::{
    analysis::EdgeKind,
    name::{Location, NameResolver, PropertyName},
};

pub(crate) enum PropertyKind {
    Array,
    Plain,
    Boxed,
}

pub(crate) struct Property {
    pub(crate) name: Ident,
    pub(crate) type_: Ident,

    pub(crate) kind: PropertyKind,

    pub(crate) required: bool,
}

pub(crate) fn properties<'a>(
    id: &VersionedUrl,
    properties: &'a HashMap<BaseUrl, ValueOrArray<PropertyTypeReference>>,
    required: &[BaseUrl],
    resolver: &NameResolver,
    property_names: &HashMap<&VersionedUrl, PropertyName>,
    locations: &HashMap<&VersionedUrl, Location>,
) -> BTreeMap<&'a BaseUrl, Property> {
    properties
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
            } else if resolver.analyzer().edge(id, url).kind == EdgeKind::Boxed {
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
