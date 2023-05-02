use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{token::Pub, Visibility};
use type_system::{
    url::{BaseUrl, VersionedUrl},
    DataTypeReference, Object, PropertyType, PropertyTypeReference, PropertyValues, ValueOrArray,
};

use crate::{
    name::{Location, NameResolver, PropertyName},
    shared,
    shared::{generate_mod, generate_property, imports, Import, Property, PropertyKind, Variant},
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
    "VersionedUrlRef",
    "GenericPropertyError",
    "Serialize",
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

fn generate_use(
    references: &[&VersionedUrl],
    locations: &HashMap<&VersionedUrl, Location>,
    import: Import,
) -> TokenStream {
    let mut imports: Vec<_> = imports(references, locations).collect();

    if import.box_ {
        imports.push(quote!(
            use alloc::boxed::Box;
        ));
    }

    if import.vec {
        imports.push(quote!(
            use alloc::vec::Vec;
        ));
    }

    quote! {
        use serde::Serialize;
        use blockprotocol::{Type, TypeRef, TypeMut};
        use blockprotocol::{PropertyType, PropertyTypeRef, PropertyTypeMut};
        use blockprotocol::{DataType as _, DataTypeRef as _, DataTypeMut as _};
        use blockprotocol::url;
        use blockprotocol::{VersionedUrlRef, GenericPropertyError};
        use error_stack::{Result, ResultExt as _};

        #(#imports)*
    }
}

struct Type {
    def: TokenStream,
    impl_ty: TokenStream,
    impl_try_from_value: TokenStream,
}

fn generate_type(
    id: &VersionedUrl,
    name: &Ident,
    variant: Variant,
    values: &[PropertyValues],
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    state: &mut State,
) -> Type {
    let derive = match variant {
        Variant::Owned | Variant::Ref => quote!(#[derive(Debug, Clone, Serialize)]),
        Variant::Mut => quote!(#[derive(Debug, Serialize)]),
    };

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
        let (body, try_from) = generate_contents(
            (id, variant),
            value,
            resolver,
            locations,
            true,
            &quote!(Self),
            state,
        );
        let semicolon = semicolon.then_some(quote!(;));

        // TODO: as_ref, as_mut, into_owned (tho they should be relatively easy)
        //  TODO: do these only after we're done with project bootstrapping!

        let def = quote! {
            #derive
            pub struct #name #lifetime #body #semicolon
        };

        return Type {
            def,
            impl_ty: quote!(#name #lifetime),
            impl_try_from_value: try_from,
        };
    }

    // TODO: basically do the same as untagged, just run it through try and if none fit just return
    //  all errors
    // we cannot hoist and therefore need to create an enum
    let (body, try_from_variants): (Vec<_>, Vec<_>) = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let name = format_ident!("Variant{index}");
            let (body, try_from) = generate_contents(
                (id, variant),
                value,
                resolver,
                locations,
                false,
                &quote!(Self::#name),
                state,
            );

            // TODO: try_from_value
            (
                quote! {
                    #name #body
                },
                try_from,
            )
        })
        .unzip();

    let try_from = quote! {
        let errors: Result<(), GenericPropertyError> = Ok(());

        #(
            let this = #try_from_variants;

            match this {
                Ok(this) => return Ok(this),
                Err(error) => match &mut errors {
                    Err(errors) => errors.extend_one(error),
                    errors => *errors = Err(error)
                }
            }
        )*

        errors?;

        unreachable!();
    };

    let def = quote! {
        #derive
        #[serde(untagged)]
        pub enum #name #lifetime {
            #(#body),*
        }
    };

    Type {
        def,
        impl_ty: quote!(#name #lifetime),
        impl_try_from_value: try_from,
    }
}

fn generate_inner(
    id: &VersionedUrl,
    variant: Variant,
    values: &[PropertyValues],
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    state: &mut State,
) -> Ident {
    let n = state.inner.len();
    let name = format_ident!("{}{n}", state.inner_name);

    let Type {
        def,
        impl_ty,
        impl_try_from_value,
    } = generate_type(id, &name, variant, values, resolver, locations, state);

    let mut value_ref = match variant {
        Variant::Owned => None,
        Variant::Ref => Some(quote!(&'a)),
        Variant::Mut => Some(quote!(&'a mut)),
    };

    state.inner.push(Inner {
        name: name.clone(),
        stream: quote!(
            #def

            impl #impl_ty {
                fn try_from_value(value: #value_ref serde_json::Value) -> Result<Self, GenericPropertError> {
                    #impl_try_from_value
                }
            }
        ),
    });

    name
}

#[allow(clippy::too_many_lines)]
fn generate_contents(
    (id, variant): (&VersionedUrl, Variant),
    value: &PropertyValues,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    hoist: bool,
    type_: &TokenStream,
    state: &mut State,
) -> (TokenStream, TokenStream) {
    let suffix = match variant {
        Variant::Owned => None,
        Variant::Ref => Some(quote!(::Ref<'a>)),
        Variant::Mut => Some(quote!(::Mut<'a>)),
    };

    match value {
        PropertyValues::DataTypeReference(reference) => {
            let location = &locations[reference.url()];
            let vis = hoist.then_some(quote!(pub));

            let name = location
                .alias
                .value
                .as_ref()
                .unwrap_or(&location.name.value);
            let mut name = Ident::new(name, Span::call_site()).to_token_stream();

            if let Some(suffix) = suffix {
                name = quote!(<#name as Type>#suffix);
            }

            let try_from = quote!({
                let value = <#name>::try_from_value(value)
                    .change_context(GenericPropertyError::Data);

                value.map(#type_)
            });

            (quote!((#vis #name)), try_from)
        }
        PropertyValues::PropertyTypeObject(object) => {
            let property_names = resolver.property_names(object.properties().values().map(
                |property| match property {
                    ValueOrArray::Value(value) => value.url(),
                    ValueOrArray::Array(value) => value.items().url(),
                },
            ));

            let properties = properties(id, object, resolver, &property_names, locations);

            let try_from = shared::generate_properties_try_from_value(
                variant,
                &properties,
                &Ident::new("GenericPropertyError", Span::call_site()),
                type_,
            );

            let visibility = hoist.then(|| Visibility::Public(Pub::default()));
            let properties = properties.iter().map(|(base, property)| {
                generate_property(
                    base,
                    property,
                    variant,
                    visibility.as_ref(),
                    &mut state.import,
                )
            });

            (
                quote!({
                    #(#properties),*
                }),
                quote!({
                    #try_from
                }),
            )
        }
        // TODO: automatically flatten, different modes?, inner data-type reference should just be a
        //  newtype?
        // TODO: needs a `generate_object` in that case ~> not really tho
        PropertyValues::ArrayOfPropertyValues(array) => {
            let items = array.items();
            let inner = generate_inner(id, variant, items.one_of(), resolver, locations, state);

            let vis = hoist.then_some(quote!(pub));

            let lifetime = match variant {
                Variant::Ref | Variant::Mut => Some(quote!(<'a>)),
                Variant::Owned => None,
            };

            let suffix = match variant {
                Variant::Ref => Some(quote!(.map(|array| array.into_boxed_slice()))),
                _ => None,
            };

            let try_from = quote!({
                match value {
                    serde_json::Value::Array(array) => blockprotocol::fold_iter_reports(
                        array.into_iter().map(|value| <#inner #lifetime>::try_from_value(value))
                    )
                    #suffix
                    .map(#type_)
                    .change_context(GenericPropertyError::Array),
                    _ => Err(Report::new(GenericPropertyError::ExpectedArray))
                }
            });

            // in theory we could do some more hoisting, e.g. if we have multiple OneOf that are
            // Array
            state.import.vec = true;
            (quote!((#vis Vec<#inner #lifetime>)), try_from)
        }
    }
}

fn generate_owned(
    property: &PropertyType,
    location: &Location,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    state: &mut State,
) -> TokenStream {
    let name = Ident::new(location.name.value.as_str(), Span::call_site());
    let name_ref = Ident::new(location.name_ref.value.as_str(), Span::call_site());
    let name_mut = Ident::new(location.name_mut.value.as_str(), Span::call_site());

    let base_url = property.id().base_url.as_str();
    let version = property.id().version;

    let alias = location.name.alias.as_ref().map(|alias| {
        let alias = Ident::new(alias, Span::call_site());

        quote!(pub type #alias = #name;)
    });

    let Type {
        def,
        impl_try_from_value,
        ..
    } = generate_type(
        property.id(),
        &name,
        Variant::Owned,
        property.one_of(),
        resolver,
        locations,
        state,
    );

    quote! {
        #def

        impl Type for #name {
            type Mut<'a> = #name_mut<'a> where Self: 'a;
            type Ref<'a> = #name_ref<'a> where Self: 'a;

            const ID: VersionedUrlRef<'static>  = url!(#base_url / v / #version);

            fn as_mut(&mut self) -> Self::Mut<'_> {
                // TODO!
                todo!()
            }

            fn as_ref(&self) -> Self::Ref<'_> {
                // TODO!
                todo!()
            }
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

fn generate_ref(
    property: &PropertyType,
    location: &Location,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    state: &mut State,
) -> TokenStream {
    let name = Ident::new(location.name.value.as_str(), Span::call_site());
    let name_ref = Ident::new(location.name_ref.value.as_str(), Span::call_site());

    let alias = location.name_ref.alias.as_ref().map(|alias| {
        let alias = Ident::new(alias, Span::call_site());

        quote!(pub type #alias<'a> = #name_ref<'a>;)
    });

    let Type {
        def,
        impl_try_from_value,
        ..
    } = generate_type(
        property.id(),
        &name_ref,
        Variant::Ref,
        property.one_of(),
        resolver,
        locations,
        state,
    );

    quote! {
        #def

        impl TypeRef for #name_ref<'_> {
            type Owned = #name;

            fn into_owned(self) -> Self::Owned {
                // TODO
                todo!();
            }
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

fn generate_mut(
    property: &PropertyType,
    location: &Location,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    state: &mut State,
) -> TokenStream {
    let name = Ident::new(location.name.value.as_str(), Span::call_site());
    let name_mut = Ident::new(location.name_mut.value.as_str(), Span::call_site());

    let alias = location.name_mut.alias.as_ref().map(|alias| {
        let alias = Ident::new(alias, Span::call_site());

        quote!(pub type #alias<'a> = #name_mut<'a>;)
    });

    let Type {
        def,
        impl_try_from_value,
        ..
    } = generate_type(
        property.id(),
        &name_mut,
        Variant::Mut,
        property.one_of(),
        resolver,
        locations,
        state,
    );

    quote! {
        #def

        impl TypeMut for #name_mut<'_> {
            type Owned = #name;

            fn into_owned(self) -> Self::Owned {
                // TODO
                todo!();
            }
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

// Generate the code for all oneOf, depending (with the () vs. {}) and extra types required,
// then if oneOf is one use a struct instead, inner types (`Inner`) should be
// generated via a mutable vec
pub(crate) fn generate(property: &PropertyType, resolver: &NameResolver) -> TokenStream {
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

    let mut inner = "Inner".to_owned();
    let locations = resolver.locations(references.iter().map(Deref::deref), &reserved);

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

    let mut state = State {
        inner: vec![],
        import: Import {
            vec: false,
            box_: false,
        },
        inner_name: inner,
    };

    let owned = generate_owned(property, &location, resolver, &locations, &mut state);
    let ref_ = generate_ref(property, &location, resolver, &locations, &mut state);
    let mut_ = generate_mut(property, &location, resolver, &locations, &mut state);

    let inner = state.inner;

    let use_ = generate_use(&references, &locations, state.import);
    let mod_ = generate_mod(&location.kind, resolver);

    quote! {
        #use_

        #(#inner)*

        #owned
        #ref_
        #mut_

        #mod_
    }
}

// N.B.:
// in the enum we could in theory name the variant by the name of the struct, problem here is ofc
// that we would still need to name the other variants and then we have potential name conflicts...
// Do we need to box on Ref and Mut self-referential?

// TODO: intermediate mod.rs (/module.rs) generation
// TODO: try_from_*
// TODO: project scaffolding
