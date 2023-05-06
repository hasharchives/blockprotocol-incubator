mod inner;
mod property_value;
mod type_;

use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
};

use itertools::Itertools;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{token::Pub, Visibility};
use type_system::{
    url::{BaseUrl, VersionedUrl},
    Array, DataTypeReference, Object, OneOf, PropertyType, PropertyTypeReference, PropertyValues,
    ValueOrArray,
};

use crate::{
    name::{Location, NameResolver, PropertyName},
    shared,
    shared::{
        generate_mod, generate_property, imports, Import, IncludeLifetime, Property, Variant,
    },
};

struct State {
    inner: Vec<Inner>,
    import: Import,
    inner_name: String,
}

const RESERVED: &[&str] = &[
    "Type",
    "TypeRef",
    "PropertyType",
    "PropertyTypeRef",
    "PropertyTypeMut",
    "DataType",
    "DataTypeRef",
    "DataTypeMut",
    "VersionedUrlRef",
    "GenericPropertyError",
    "Serialize",
    "Report",
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

struct PropertyTypeGenerator<'a> {
    property: &'a PropertyType,
    resolver: &'a NameResolver<'a>,

    location: Location<'a>,

    locations: HashMap<&'a VersionedUrl, Location<'a>>,
    references: Vec<&'a VersionedUrl>,

    state: State,
}

impl<'a> PropertyTypeGenerator<'a> {
    fn new(property: &'a PropertyType, resolver: &'a NameResolver<'a>) -> Self {
        let location = resolver.location(property.id());

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

        let mut reserved = RESERVED.to_vec();
        reserved.push(&location.name.value);
        reserved.push(&location.name_ref.value);
        reserved.push(&location.name_mut.value);

        if let Some(name) = &location.name.alias {
            reserved.push(name);
        }
        if let Some(name) = &location.name_ref.alias {
            reserved.push(name);
        }
        if let Some(name) = &location.name_mut.alias {
            reserved.push(name);
        }

        // TODO: fix
        let mut inner = "Inner".to_owned();
        // we need to clone here, otherwise we're in ownership kerfuffle
        let locations = resolver.locations(references.clone(), &reserved);

        for location in locations.values() {
            let name = location
                .alias
                .value
                .as_ref()
                .unwrap_or(&location.name.value);
            let name_ref = location
                .alias
                .value_ref
                .as_ref()
                .unwrap_or(&location.name_ref.value);
            let name_mut = location
                .alias
                .value_mut
                .as_ref()
                .unwrap_or(&location.name_mut.value);

            // ensures that we test if the new identifier is also a collision
            loop {
                if name.starts_with(inner.as_str())
                    || name_ref.starts_with(inner.as_str())
                    || name_mut.starts_with(inner.as_str())
                {
                    inner = format!("_{inner}");
                } else {
                    break;
                }
            }
        }

        let state = State {
            inner: vec![],
            import: Import {
                vec: false,
                box_: false,
                phantom_data: false,
            },
            inner_name: inner,
        };

        Self {
            property,
            resolver,
            location,
            locations,
            references,
            state,
        }
    }

    fn use_(&self) -> TokenStream {
        let mut imports: Vec<_> = imports(&self.references, &self.locations).collect();

        if self.state.import.box_ {
            imports.push(quote!(
                use alloc::boxed::Box;
            ));
        }

        if self.state.import.vec {
            imports.push(quote!(
                use alloc::vec::Vec;
            ));
        }

        quote! {
            use serde::Serialize;
            use turbine::{Type, TypeRef, TypeMut};
            use turbine::{PropertyType, PropertyTypeRef, PropertyTypeMut};
            use turbine::{DataType, DataTypeRef, DataTypeMut};
            use turbine::url;
            use turbine::{VersionedUrlRef, GenericPropertyError};
            use error_stack::{Result, Report, ResultExt as _};

            #(#imports)*
        }
    }

    fn mod_(&self) -> Option<TokenStream> {
        generate_mod(&self.location.kind, self.resolver)
    }

    fn doc(&self) -> TokenStream {
        let property = self.property;
        let title = property.title();
        // mimic #()?
        let description = property.description().into_iter();

        quote!(
            #[doc = #title]
            #(
                #[doc = ""]
                #[doc = #description]
            )*
        )
    }

    fn owned(&mut self) -> TokenStream {
        let name = Ident::new(self.location.name.value.as_str(), Span::call_site());
        let name_ref = Ident::new(self.location.name_ref.value.as_str(), Span::call_site());
        let name_mut = Ident::new(self.location.name_mut.value.as_str(), Span::call_site());

        let base_url = self.property.id().base_url.as_str();
        let version = self.property.id().version;

        let alias = self.location.name.alias.as_ref().map(|alias| {
            let alias = Ident::new(alias, Span::call_site());

            quote!(pub type #alias = #name;)
        });

        let doc = self.doc();

        let Type {
            def,
            impl_try_from_value,
            impl_conversion,
            ..
        } = generate_type(
            self.property.id(),
            &name,
            Variant::Owned,
            self.property.one_of(),
            self.resolver,
            &self.locations,
            &mut self.state,
        );

        quote! {
            #doc
            #def

            impl Type for #name {
                type Mut<'a> = #name_mut<'a> where Self: 'a;
                type Ref<'a> = #name_ref<'a> where Self: 'a;

                const ID: VersionedUrlRef<'static>  = url!(#base_url / v / #version);

                #impl_conversion
            }

            impl PropertyType for #name {
                type Error = GenericPropertyError;

                fn try_from_value(value: serde_json::Value) -> Result<Self, Self::Error> {
                    #impl_try_from_value
                }
            }

            #alias
        }
    }

    fn ref_(&mut self) -> TokenStream {
        let name = Ident::new(self.location.name.value.as_str(), Span::call_site());
        let name_ref = Ident::new(self.location.name_ref.value.as_str(), Span::call_site());

        let alias = self.location.name_ref.alias.as_ref().map(|alias| {
            let alias = Ident::new(alias, Span::call_site());

            quote!(pub type #alias<'a> = #name_ref<'a>;)
        });

        let doc = self.doc();

        let Type {
            def,
            impl_try_from_value,
            impl_conversion,
            ..
        } = generate_type(
            self.property.id(),
            &name_ref,
            Variant::Ref,
            self.property.one_of(),
            self.resolver,
            &self.locations,
            &mut self.state,
        );

        quote! {
            #doc
            #def

            impl TypeRef for #name_ref<'_> {
                type Owned = #name;

                #impl_conversion
            }

            impl<'a> PropertyTypeRef<'a> for #name_ref<'a> {
                type Error = GenericPropertyError;

                fn try_from_value(value: &'a serde_json::Value) -> Result<Self, Self::Error> {
                    #impl_try_from_value
                }
            }

            #alias
        }
    }

    fn mut_(&mut self) -> TokenStream {
        let name = Ident::new(self.location.name.value.as_str(), Span::call_site());
        let name_mut = Ident::new(self.location.name_mut.value.as_str(), Span::call_site());

        let alias = self.location.name_mut.alias.as_ref().map(|alias| {
            let alias = Ident::new(alias, Span::call_site());

            quote!(pub type #alias<'a> = #name_mut<'a>;)
        });

        let doc = self.doc();

        let Type {
            def,
            impl_try_from_value,
            impl_conversion,
            ..
        } = generate_type(
            self.property.id(),
            &name_mut,
            Variant::Mut,
            self.property.one_of(),
            self.resolver,
            &self.locations,
            &mut self.state,
        );

        quote! {
            #doc
            #def

            impl TypeMut for #name_mut<'_> {
                type Owned = #name;

                #impl_conversion
            }

            impl<'a> PropertyTypeMut<'a> for #name_mut<'a> {
                type Error = GenericPropertyError;

                fn try_from_value(value: &'a mut serde_json::Value) -> Result<Self, Self::Error> {
                    #impl_try_from_value
                }
            }

            #alias
        }
    }

    fn finish(mut self) -> TokenStream {
        let owned = self.owned();
        let ref_ = self.ref_();
        let mut_ = self.mut_();

        let use_ = self.use_();
        let mod_ = self.mod_();

        let inner = self.state.inner;

        quote! {
            #use_

            #(#inner)*

            #owned
            #ref_
            #mut_

            #mod_
        }
    }
}

// TODO: into correct generator!?
struct PropertyValueGenerator {}

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

struct Type {
    def: TokenStream,
    impl_ty: TokenStream,
    impl_try_from_value: TokenStream,
    impl_conversion: TokenStream,
}

struct Conversion {
    into_owned: TokenStream,
    as_ref: TokenStream,
    as_mut: TokenStream,
    match_arm: TokenStream,
    destruct: TokenStream,
}

struct Body {
    def: TokenStream,
    try_from: TokenStream,
    conversion: Conversion,
}

// Generate the code for all oneOf, depending (with the () vs. {}) and extra types required,
// then if oneOf is one use a struct instead, inner types (`Inner`) should be
// generated via a mutable vec
pub(crate) fn generate(property: &PropertyType, resolver: &NameResolver) -> TokenStream {
    let generator = PropertyTypeGenerator::new(property, resolver);

    generator.finish()
}

// N.B.:
// in the enum we could in theory name the variant by the name of the struct, problem here is ofc
// that we would still need to name the other variants and then we have potential name conflicts...
// Do we need to box on Ref and Mut self-referential?
