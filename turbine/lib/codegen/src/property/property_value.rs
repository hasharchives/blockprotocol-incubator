use std::collections::{BTreeMap, HashMap};

use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Token, Visibility};
use type_system::{
    url::{BaseUrl, VersionedUrl},
    Array, DataTypeReference, Object, OneOf, PropertyTypeReference, PropertyValues, ValueOrArray,
};

use crate::{
    name::{Location, NameResolver, PropertyName},
    property::{inner::InnerGenerator, PathSegment, State},
    shared,
    shared::{IncludeLifetime, Property, Variant},
};

#[derive(Debug, Copy, Clone)]
struct SelfVariant<'a>(&'a TokenStream);

impl ToTokens for SelfVariant<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = self.0;
        tokens.extend(quote!(:: #name));
    }
}

#[derive(Debug, Copy, Clone)]
pub(super) struct SelfType<'a> {
    variant: Option<SelfVariant<'a>>,
}

impl<'a> SelfType<'a> {
    const fn hoist(self) -> bool {
        self.variant.is_none()
    }

    fn hoisted_visibility(self) -> Option<Visibility> {
        self.hoist()
            .then_some(Visibility::Public(<Token![pub]>::default()))
    }

    pub(super) const fn enum_(name: &'a TokenStream) -> Self {
        SelfType {
            variant: Some(SelfVariant(name)),
        }
    }

    pub(super) const fn struct_() -> Self {
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

pub(super) struct PropertyValue {
    pub(super) body: TokenStream,
    pub(super) try_from: TokenStream,
    // conversion: Conversion,
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

pub(super) struct PropertyValueGenerator<'a> {
    pub(super) id: &'a VersionedUrl,
    pub(super) variant: Variant,
    pub(super) self_type: SelfType<'a>,

    pub(super) resolver: &'a NameResolver<'a>,
    pub(super) locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,

    pub(super) value: &'a PropertyValues,

    pub(super) state: &'a mut State,
}

impl<'a> PropertyValueGenerator<'a> {
    fn data_type(&mut self, reference: &DataTypeReference) -> PropertyValue {
        let location = &self.locations[reference.url()];
        let vis = self.self_type.hoisted_visibility();

        let suffix = match self.variant {
            Variant::Owned => None,
            Variant::Ref => Some(quote!(::Ref<'a>)),
            Variant::Mut => Some(quote!(::Mut<'a>)),
        };

        let type_name_raw = location
            .alias
            .value
            .as_ref()
            .unwrap_or(&location.name.value);
        let mut type_name = Ident::new(type_name_raw, Span::call_site()).to_token_stream();

        if let Some(suffix) = suffix {
            type_name = quote!(<#type_name as Type>#suffix);
        }

        let cast = match self.variant {
            Variant::Owned => quote!(as DataType),
            Variant::Ref => quote!(as DataTypeRef<'a>),
            Variant::Mut => quote!(as DataTypeMut<'a>),
        };

        let self_type = self.self_type;
        let try_from = quote!({
            let value = <#type_name #cast>::try_from_value(value)
                .change_context(GenericPropertyError::Data);

            value.map(#self_type)
        });

        let reference = self.variant.into_reference(IncludeLifetime::No);

        // we can either be called if we're hoisted (`destruct`) or we're in a match arm
        // (`match_arm`), either way the conversion code stays the same, but how we get to
        // value is a bit different
        let match_arm = quote!(#self_type(value) =>);
        let destruct = quote!(let Self(value) = #reference self);

        let cast = match self.variant {
            Variant::Owned => quote!(as Type),
            Variant::Ref => quote!(as TypeRef),
            Variant::Mut => quote!(as TypeMut),
        };

        let mut type_name = Ident::new(type_name_raw, Span::call_site()).to_token_stream();
        type_name = match self.variant {
            Variant::Owned => type_name,
            Variant::Ref => quote!(<#type_name as Type> :: Ref),
            Variant::Mut => quote!(<#type_name as Type> :: Mut),
        };

        let variant = self_type.variant;

        // TODO: we might need to explicitly cast on all other variants as well
        // need to use explicit cast as there are multiple possibilities here, either `Ref` or `Mut`
        // if a `DataType` implements both
        let into_owned =
            quote!(<Self #cast>::Owned #variant (<#type_name #cast>::into_owned(value)));
        let as_ref = quote!(Self::Ref #variant (<#type_name #cast>::as_ref(value)));
        let as_mut = quote!(Self::Mut #variant (<#type_name #cast>::as_mut(value)));

        PropertyValue {
            body: quote!((#vis #type_name)),
            try_from,
        }
    }

    fn object(&mut self, object: &Object<ValueOrArray<PropertyTypeReference>, 1>) -> PropertyValue {
        let property_names = self
            .resolver
            .property_names(object.properties().values().map(|property| match property {
                ValueOrArray::Value(value) => value.url(),
                ValueOrArray::Array(value) => value.items().url(),
            }));

        let properties = properties(
            self.id,
            object,
            self.resolver,
            &property_names,
            self.locations,
        );

        let try_from = shared::generate_properties_try_from_value(
            self.variant,
            &properties,
            &Ident::new("GenericPropertyError", Span::call_site()),
            &self.self_type.to_token_stream(),
        );

        let visibility = self.self_type.hoisted_visibility();
        let fields = properties.iter().map(|(base, property)| {
            shared::generate_property(
                base,
                property,
                self.variant,
                visibility.as_ref(),
                &mut self.state.import,
            )
        });

        let mutability = match self.variant {
            Variant::Owned => Some(quote!(mut)),
            Variant::Ref | Variant::Mut => None,
        };

        let clone = match self.variant {
            Variant::Owned => Some(quote!(.clone())),
            Variant::Ref | Variant::Mut => None,
        };

        let reference = self.variant.into_reference(IncludeLifetime::No);

        let property_names: Vec<_> = properties
            .values()
            .map(|Property { name, .. }| name)
            .collect();
        let self_type = self.self_type;
        let match_arm = quote!(#self_type { #(#property_names),* } =>);
        // TODO: this is wrong, back to the drawing board
        // TODO: current challenges:
        //  1) we do not know what we do at this stage (do we destruct or are we an arm)
        //  2) we do not know what to generate
        //  3) we do not know what `Inner` does (who is the `Mut` variant)
        //      We need a path lookup which we trail (in state) and once `Inner` is accessed we
        //      generate it, we can then simply reference which one we need!
        //  The current approach is lacking, what we need to do instead is depending on the
        // `self_type`  either create a match_arm or destruct, how we destruct depends on
        // what we are trying to  achieve. `as_ref` is `&`, `as_mut` is `&mut`, `into_owned`
        // is nothing. We then return a  struct with all three, but as options. Not perfect
        // but good enough. These are either bodies  or just match arms.
        // TODO: integration tests on example project w/ bootstrapping and such
        let destruct = quote!(let Self { #(#property_names),* } = #reference self);

        // we have already loaded the
        let variant = self_type.variant;
        let into_owned = quote!(Self::Owned #variant {
            #(#property_names: #property_names.into_owned()),*
        });
        let as_ref = quote!(Self::Ref #variant {
            #(#property_names: #property_names.as_ref()),*
        });
        let as_mut = quote!(Self::Mut #variant {
            #(#property_names: #property_names.as_mut()),*
        });

        PropertyValue {
            body: quote!({
                #(#fields),*
            }),
            try_from: quote!('variant: {
                let serde_json::Value::Object(#mutability properties) = value #clone else {
                    break 'variant Err(Report::new(GenericPropertyError::ExpectedObject))
                };

                #try_from
            }),
        }
    }

    fn array(&mut self, array: &Array<OneOf<PropertyValues>>) -> PropertyValue {
        let items = array.items();

        self.state.stack.push(PathSegment::Array);
        let inner = InnerGenerator {
            id: self.id,
            variant: self.variant,
            values: items.one_of(),
            resolver: self.resolver,
            locations: self.locations,
            state: self.state,
        }
        .finish();
        self.state.stack.pop();

        let vis = self.self_type.hoisted_visibility();

        let lifetime = self
            .variant
            .into_lifetime()
            .map(|lifetime| quote!(<#lifetime>));

        let suffix = match self.variant {
            Variant::Ref => Some(quote!(.map(|array| array.into_boxed_slice()))),
            _ => None,
        };

        let self_type = self.self_type;
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

        let reference = self.variant.into_reference(IncludeLifetime::No);
        let match_arm = quote!(#self_type(value) =>);
        let destruct = quote!(let Self(value) = #reference self);

        // we have already loaded the
        let variant = self_type.variant;
        // TODO: depending on what it is, we might need to `.into()` or `.into_boxed_slice()`
        // we don't need to cast to a specific trait here, because we know that inner type cannot be
        // the same type (for now) as we do not directly hoist DataType etc. as inner value.
        let into_owned = quote!(Self::Owned #variant (value.into_iter().map(|value| value.into_owned())).collect());
        let as_ref =
            quote!(Self::Ref #variant (value.iter().map(|value| value.as_ref())).collect());
        // TODO: this might fail?
        let as_mut =
            quote!(Self::Mut #variant (value.iter_mut().map(|value| value.as_mut())).collect());

        // in theory we could do some more hoisting, e.g. if we have multiple OneOf that are
        // Array
        self.state.import.vec = true;

        PropertyValue {
            body: quote!((#vis Vec<#inner #lifetime>)),
            try_from,
        }
    }

    pub(super) fn finish(mut self) -> PropertyValue {
        match self.value {
            PropertyValues::DataTypeReference(reference) => self.data_type(reference),
            PropertyValues::PropertyTypeObject(object) => self.object(object),
            // TODO: automatically flatten, different modes?, inner data-type reference should just
            // be a  newtype?
            PropertyValues::ArrayOfPropertyValues(array) => self.array(array),
        }
    }
}
