use std::collections::{BTreeMap, HashMap};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use type_system::{
    url::{BaseUrl, VersionedUrl},
    PropertyTypeReference, ValueOrArray,
};

use crate::{
    analysis::EdgeKind,
    data,
    name::{Location, LocationKind, NameResolver, PropertyName},
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

pub(crate) fn imports<'a>(
    references: impl IntoIterator<Item = &'a &'a VersionedUrl> + 'a,
    locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,
) -> impl Iterator<Item = TokenStream> + 'a {
    // explicit type not needed here, but CLion otherwise complains

    references.into_iter().map(|reference: &&VersionedUrl| {
        let location = &locations[reference];

        // shortcut for builtin data-types as they are handled in a special way
        if let Some(builtin) = data::find_builtin(reference) {
            let mut tokens = builtin.to_token_stream();

            if let Some(alias) = &location.alias.value {
                let alias = Ident::new(alias, Span::call_site());

                tokens = quote!(#tokens as #alias);
            }

            return quote!(#tokens;);
        }

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

pub(crate) fn generate_mod(kind: &LocationKind, resolver: &NameResolver) -> Option<TokenStream> {
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
