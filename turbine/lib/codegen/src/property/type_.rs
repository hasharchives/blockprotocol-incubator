use std::collections::HashMap;

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use type_system::{
    url::VersionedUrl, Array, DataTypeReference, Object, OneOf, PropertyTypeReference,
    PropertyValues, ValueOrArray,
};

use crate::{
    name::Location,
    property::State,
    shared::{IncludeLifetime, Variant},
};

pub(super) struct Type {
    def: TokenStream,
    // TODO: rename
    impl_ty: TokenStream,
    impl_try_from_value: TokenStream,
    impl_conversion: TokenStream,
}

struct TypeGenerator<'a> {
    id: &'a VersionedUrl,
    name: &'a Ident,
    variant: Variant,

    values: &'a [PropertyValues],
    locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,

    state: &'a mut State,
}

impl<'a> TypeGenerator<'a> {
    fn finish(mut self) -> Type {
        let derive = match self.variant {
            Variant::Owned | Variant::Ref => quote!(#[derive(Debug, Clone, Serialize)]),
            Variant::Mut => quote!(#[derive(Debug, Serialize)]),
        };

        let lifetime = match self.variant {
            Variant::Ref | Variant::Mut => Some(quote!(<'a>)),
            Variant::Owned => None,
        };

        if let [value] = self.values {
            let semicolon = match value {
                PropertyValues::PropertyTypeObject(_) => false,
                PropertyValues::ArrayOfPropertyValues(_) | PropertyValues::DataTypeReference(_) => {
                    true
                }
            };

            // we can hoist!
            let Body {
                def: body,
                try_from,
                conversion:
                    Conversion {
                        into_owned,
                        as_ref,
                        as_mut,
                        destruct,
                        ..
                    },
            } = generate_body(
                (id, variant),
                value,
                resolver,
                locations,
                SelfType::struct_(),
                state,
            );
            let semicolon = semicolon.then_some(quote!(;));

            let def = quote! {
                #derive
                pub struct #name #lifetime #body #semicolon
            };

            let conversion = match variant {
                Variant::Owned => quote! {
                    fn as_ref(&self) -> Self::Ref<'_> {
                        #destruct;

                        #as_ref
                    }

                    fn as_mut(&self) -> Self::Mut<'_> {
                        #destruct;

                        #as_mut
                    }
                },
                Variant::Ref | Variant::Mut => {
                    quote! {
                        fn into_owned(self) -> Self::Owned {
                            #destruct;

                            #into_owned
                        }
                    }
                }
            };

            return Type {
                def,
                impl_ty: quote!(#name #lifetime),
                impl_try_from_value: try_from,
                impl_conversion: conversion,
            };
        }

        // we cannot hoist and therefore need to create an enum

        let (body, try_from_variants, conversion): (Vec<_>, Vec<_>, Vec<_>) = self
            .values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let name = format_ident!("Variant{index}");
                let Body {
                    def: body,
                    try_from,
                    conversion,
                } = generate_body(
                    (id, variant),
                    value,
                    resolver,
                    locations,
                    SelfType::enum_(&name.to_token_stream()),
                    state,
                );

                (
                    quote! {
                        #name #body
                    },
                    try_from,
                    conversion,
                )
            })
            .multiunzip();

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

        let name = self.name;
        let def = quote! {
            #derive
            #[serde(untagged)]
            pub enum #name #lifetime {
                #(#body),*
            }
        };

        // TODO: this breaks down on inner, where things do not have a `Self::Owned` partner
        // TODO: for every inner type we need to record their `Owned`, `Ref` and `Mut` counterpart
        // ~>  lookup is needed of some sort :/ ~> state with a path of some sorts
        let conversion = match self.variant {
            Variant::Owned => {
                let as_ref = conversion.iter().map(
                    |Conversion {
                         as_ref, match_arm, ..
                     }| quote!(#match_arm #as_ref),
                );
                let as_mut = conversion.iter().map(
                    |Conversion {
                         as_mut, match_arm, ..
                     }| quote!(#match_arm #as_mut),
                );

                quote! {
                    fn as_ref(&self) -> Self::Ref<'_> {
                        match &self {
                            #(#as_ref),*
                        }
                    }

                    fn as_mut(&mut self) -> Self::Mut<'_> {
                        match &mut self {
                            #(#as_mut),*
                        }
                    }
                }
            }
            Variant::Ref | Variant::Mut => {
                let match_arms = conversion.into_iter().map(
                    |Conversion {
                         into_owned,
                         match_arm,
                         ..
                     }| quote!(#match_arm #into_owned),
                );

                quote! {
                    fn into_owned(self) -> Self::Owned {
                        match self {
                            #(#match_arms),*
                        }
                    }
                }
            }
        };

        Type {
            def,
            impl_ty: quote!(#name #lifetime),
            impl_try_from_value: try_from,
            impl_conversion: conversion,
        }
    }
}
