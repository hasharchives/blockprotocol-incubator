use proc_macro2::TokenStream;
use quote::quote;
use type_system::{EntityType, ValueOrArray};

use crate::name::NameResolver;

pub(crate) fn generate(entity: &EntityType, resolver: &NameResolver) -> TokenStream {
    let url = entity.id();

    let location = resolver.location(url);

    let name = location.name.value;
    let alias = location.alias.map(|alias| quote!(pub type #alias = #name;));

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

    let properties = entity.properties().iter().map(|(base, value)| {
        let url = match value {
            ValueOrArray::Value(value) => value.url(),
            ValueOrArray::Array(value) => value.items().url(),
        };

        let name = &property_names[url].0;
        let location = &locations[url];

        let value_ty = location.alias.as_ref().unwrap_or(&location.name.value);
        let mut value_ty = quote!(#value_ty);

        if matches!(value, ValueOrArray::Array(_)) {
            value_ty = quote!(Vec<#value_ty>)
        }

        quote!(#name: #value_ty)
    });

    quote! {
        #import_alloc

        #[derive(Debug, Clone)]
        pub struct #name {
            #(#properties),*
        }

        // TODO: accessors
        // TODO: impl Ref
        // TODO: use && mod

        #alias
    }
}
