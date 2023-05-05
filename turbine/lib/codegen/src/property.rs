use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use serde_json::Value;
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
        use turbine::{Type, TypeRef, TypeMut};
        use turbine::{PropertyType, PropertyTypeRef, PropertyTypeMut};
        use turbine::{DataType, DataTypeRef, DataTypeMut};
        use turbine::url;
        use turbine::{VersionedUrlRef, GenericPropertyError};
        use error_stack::{Result, Report, ResultExt as _};

        #(#imports)*
    }
}

struct Type {
    def: TokenStream,
    impl_ty: TokenStream,
    impl_try_from_value: TokenStream,
    impl_conversion: TokenStream,
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
        let Body {
            def: body,
            try_from,
        } = generate_body(
            (id, variant),
            value,
            resolver,
            locations,
            &SelfType::struct_(),
            state,
        );
        let semicolon = semicolon.then_some(quote!(;));

        // TODO: as_ref, as_mut, into_owned (tho they should be relatively easy)

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

    // we cannot hoist and therefore need to create an enum
    let (body, try_from_variants): (Vec<_>, Vec<_>) = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let name = format_ident!("Variant{index}");
            let Body {
                def: body,
                try_from,
            } = generate_body(
                (id, variant),
                value,
                resolver,
                locations,
                &SelfType::enum_(&name.to_token_stream()),
                state,
            );

            (
                quote! {
                    #name #body
                },
                try_from,
            )
        })
        .unzip();

    let try_from = quote! {
        let mut errors: Result<(), GenericPropertyError> = Ok(());

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
        impl_conversion,
    } = generate_type(id, &name, variant, values, resolver, locations, state);

    let value_ref = match variant {
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

                #impl_conversion
            }
        ),
    });

    name
}

#[derive(Debug, Copy, Clone)]
struct SelfVariant<'a>(&'a TokenStream);

impl ToTokens for SelfVariant<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = self.0;
        tokens.extend(quote!(:: #name))
    }
}

#[derive(Debug, Copy, Clone)]
struct SelfType<'a> {
    variant: Option<SelfVariant<'a>>,
}

impl<'a> SelfType<'a> {
    fn hoist(&self) -> bool {
        self.variant.is_none()
    }

    fn hoisted_visibility(&self) -> Option<Visibility> {
        self.hoist().then_some(Visibility::Public(Pub::default()))
    }

    fn enum_(name: &'a TokenStream) -> Self {
        SelfType {
            variant: Some(SelfVariant(name)),
        }
    }

    fn struct_() -> Self {
        SelfType { variant: None }
    }
}

impl ToTokens for SelfType<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let variant = self.variant;
        tokens.extend(quote!(Self #variant));
    }
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

fn generate_body_data_type(
    variant: Variant,
    reference: &DataTypeReference,
    locations: &HashMap<&VersionedUrl, Location>,
    self_type: &SelfType,
    suffix: Option<TokenStream>,
) -> Body {
    let location = &locations[reference.url()];
    let vis = self_type.hoisted_visibility();

    let type_name = location
        .alias
        .value
        .as_ref()
        .unwrap_or(&location.name.value);
    let mut type_name = Ident::new(type_name, Span::call_site()).to_token_stream();

    if let Some(suffix) = suffix {
        type_name = quote!(<#type_name as Type>#suffix);
    }

    let cast = match variant {
        Variant::Owned => quote!(as DataType),
        Variant::Ref => quote!(as DataTypeRef<'a>),
        Variant::Mut => quote!(as DataTypeMut<'a>),
    };

    let try_from = quote!({
        let value = <#type_name #cast>::try_from_value(value)
            .change_context(GenericPropertyError::Data);

        value.map(#self_type)
    });

    let reference = variant.into_reference(IncludeLifetime::No);

    // we can either be called if we're hoisted (`destruct`) or we're in a match arm (`match_arm`),
    // either way the conversion code stays the same, but how we get to value is a bit different
    let match_arm = quote!(#self_type(value) =>);
    let destruct = quote!(let Self(value) = #reference self);

    let cast = match variant {
        Variant::Owned => quote!(as Type),
        Variant::Ref => quote!(as TypeRef),
        Variant::Mut => quote!(as TypeMut),
    };

    let variant = self_type.variant;

    // TODO: we might need to explicitly cast on all other variants as well
    // need to use explicit cast as there are multiple possibilities here, either `Ref` or `Mut`
    // if a `DataType` implements both
    let into_owned =
        quote!(<<Self as #cast>::Owned> #variant (<#type_name #cast>::into_owned(value)));
    let as_ref = quote!(<Self::Ref> #variant (<#type_name #cast>::as_ref(value)));
    let as_mut = quote!(<Self::Mut> #variant (<#type_name #cast>::as_mut(value)));

    Body {
        def: quote!((#vis #type_name)),
        try_from,
        conversion: Conversion {
            into_owned,
            as_ref,
            as_mut,
            match_arm,
            destruct,
        },
    }
}

fn generate_body_object(
    id: &VersionedUrl,
    variant: Variant,
    object: &Object<ValueOrArray<PropertyTypeReference>, 1>,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    self_type: &SelfType,
    state: &mut State,
) -> Body {
    let property_names =
        resolver.property_names(object.properties().values().map(|property| match property {
            ValueOrArray::Value(value) => value.url(),
            ValueOrArray::Array(value) => value.items().url(),
        }));

    let properties = properties(id, object, resolver, &property_names, locations);

    let try_from = shared::generate_properties_try_from_value(
        variant,
        &properties,
        &Ident::new("GenericPropertyError", Span::call_site()),
        &self_type.to_token_stream(),
    );

    let visibility = self_type.hoisted_visibility();
    let fields = properties.iter().map(|(base, property)| {
        generate_property(
            base,
            property,
            variant,
            visibility.as_ref(),
            &mut state.import,
        )
    });

    let mutability = match variant {
        Variant::Owned => Some(quote!(mut)),
        Variant::Ref | Variant::Mut => None,
    };

    let clone = match variant {
        Variant::Owned => Some(quote!(.clone())),
        Variant::Ref | Variant::Mut => None,
    };

    let reference = variant.into_reference(IncludeLifetime::No);

    let property_names: Vec<_> = properties
        .values()
        .map(|Property { name, .. }| name)
        .collect();
    let match_arm = quote!(#self_type { #(#property_names),* } =>);
    let destruct = quote!(let Self { #(#property_names),* } = #reference self);

    // we have already loaded the
    let variant = self_type.variant;
    let into_owned = quote!(<Self::Owned> #variant {
        #(#property_names: #property_names.into_owned()),*
    });
    let as_ref = quote!(<Self::Ref> #variant {
        #(#property_names: #property_names.as_ref()),*
    });
    let as_mut = quote!(<Self::Mut> #variant {
        #(#property_names: #property_names.as_mut()),*
    });

    Body {
        def: quote!({
            #(#fields),*
        }),
        try_from: quote!('variant: {
            let serde_json::Value::Object(#mutability properties) = value #clone else {
                break 'variant Err(Report::new(GenericPropertyError::ExpectedObject))
            };

            #try_from
        }),
        conversion: Conversion {
            into_owned,
            as_ref,
            as_mut,
            match_arm,
            destruct,
        },
    }
}

fn generate_body_array(
    id: &VersionedUrl,
    variant: Variant,
    array: &Array<OneOf<PropertyValues>>,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    self_type: &SelfType,
    state: &mut State,
) -> Body {
    let items = array.items();
    let inner = generate_inner(id, variant, items.one_of(), resolver, locations, state);

    let vis = self_type.hoisted_visibility();

    let lifetime = variant.into_lifetime().map(|lifetime| quote!(<#lifetime>));

    let suffix = match variant {
        Variant::Ref => Some(quote!(.map(|array| array.into_boxed_slice()))),
        _ => None,
    };

    let try_from = quote!({
        match value {
            serde_json::Value::Array(array) => turbine::fold_iter_reports(
                array.into_iter().map(|value| <#inner #lifetime>::try_from_value(value))
            )
            #suffix
            .map(#self_type)
            .change_context(GenericPropertyError::Array),
            _ => Err(Report::new(GenericPropertyError::ExpectedArray))
        }
    });

    let reference = variant.into_reference(IncludeLifetime::No);
    let match_arm = quote!(#self_type(value) =>);
    let destruct = quote!(let Self(value) = #reference self);

    // we have already loaded the
    let variant = self_type.variant;
    // TODO: depending on what it is, we might need to `.into()` or `.into_boxed_slice()`
    // we don't need to cast to a specific trait here, because we know that inner type cannot be the
    // same type (for now) as we do not directly hoist DataType etc. as inner value.
    let into_owned = quote!(<Self::Owned> #variant (value.into_iter().map(|value| value.into_owned())).collect());
    let as_ref = quote!(<Self::Ref> #variant (value.iter().map(|value| value.as_ref())).collect());
    // TODO: this might fail?
    let as_mut =
        quote!(<Self::Mut> #variant (value.iter_mut().map(|value| value.as_mut())).collect());

    // in theory we could do some more hoisting, e.g. if we have multiple OneOf that are
    // Array
    state.import.vec = true;

    Body {
        def: quote!((#vis Vec<#inner #lifetime>)),
        try_from,
        conversion: Conversion {
            into_owned,
            as_ref,
            as_mut,
            match_arm,
            destruct,
        },
    }
}

#[allow(clippy::too_many_lines)]
fn generate_body(
    (id, variant): (&VersionedUrl, Variant),
    value: &PropertyValues,
    resolver: &NameResolver,
    locations: &HashMap<&VersionedUrl, Location>,
    self_type: &SelfType,
    state: &mut State,
) -> Body {
    let suffix = match variant {
        Variant::Owned => None,
        Variant::Ref => Some(quote!(::Ref<'a>)),
        Variant::Mut => Some(quote!(::Mut<'a>)),
    };

    match value {
        PropertyValues::DataTypeReference(reference) => {
            generate_body_data_type(variant, reference, locations, self_type, suffix)
        }
        PropertyValues::PropertyTypeObject(object) => {
            generate_body_object(id, variant, object, resolver, locations, self_type, state)
        }
        // TODO: automatically flatten, different modes?, inner data-type reference should just be a
        //  newtype?
        PropertyValues::ArrayOfPropertyValues(array) => {
            generate_body_array(id, variant, array, resolver, locations, self_type, state)
        }
    }
}

fn generate_doc(property: &PropertyType) -> TokenStream {
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

    let doc = generate_doc(property);

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
        #doc
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

    let doc = generate_doc(property);

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
        #doc
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

    let doc = generate_doc(property);

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
        #doc
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
            phantom_data: false,
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
