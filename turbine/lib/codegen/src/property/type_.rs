use std::collections::HashMap;

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use type_system::{url::VersionedUrl, PropertyValues};

use crate::{
    name::{Location, NameResolver},
    property::{
        property_value::{PropertyValue, PropertyValueGenerator, SelfType},
        PathSegment, State,
    },
    shared::Variant,
};

pub(super) struct Type {
    pub(super) def: TokenStream,
    // TODO: rename
    pub(super) impl_ty: TokenStream,
    pub(super) impl_try_from_value: TokenStream,
    pub(super) impl_conversion: TokenStream,
}

pub(super) struct TypeGenerator<'a> {
    pub(super) id: &'a VersionedUrl,
    pub(super) name: &'a Ident,
    pub(super) variant: Variant,

    pub(super) values: &'a [PropertyValues],
    pub(super) resolver: &'a NameResolver<'a>,
    pub(super) locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,

    pub(super) state: &'a mut State,
}

impl<'a> TypeGenerator<'a> {
    pub(super) fn finish(self) -> Type {
        let derive = match self.variant {
            Variant::Owned | Variant::Ref => quote!(#[derive(Debug, Clone, Serialize)]),
            Variant::Mut => quote!(#[derive(Debug, Serialize)]),
        };

        let lifetime = match self.variant {
            Variant::Ref | Variant::Mut => Some(quote!(<'a>)),
            Variant::Owned => None,
        };

        let name = self.name;

        if let [value] = self.values {
            let semicolon = match value {
                PropertyValues::PropertyTypeObject(_) => false,
                PropertyValues::ArrayOfPropertyValues(_) | PropertyValues::DataTypeReference(_) => {
                    true
                }
            };

            self.state.stack.push(PathSegment::OneOf { index: 0 });
            let PropertyValue { body, try_from } = PropertyValueGenerator {
                id: self.id,
                variant: self.variant,
                self_type: SelfType::struct_(),
                resolver: self.resolver,
                locations: self.locations,
                value,
                state: self.state,
            }
            .finish();
            self.state.stack.pop();

            let semicolon = semicolon.then_some(quote!(;));

            let def = quote! {
                #derive
                pub struct #name #lifetime #body #semicolon
            };

            return Type {
                def,
                impl_ty: quote!(#name #lifetime),
                impl_try_from_value: try_from,
                impl_conversion: quote!(todo!();),
            };
        }

        // we cannot hoist and therefore need to create an enum

        // N.B.:
        // in the enum we could in theory name the variant by the name of the struct, problem here
        // is ofc that we would still need to name the other variants and then we have
        // potential name conflicts... Do we need to box on Ref and Mut self-referential?
        let (body, try_from_variants, conversion): (Vec<_>, Vec<_>, Vec<_>) = self
            .values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let name = format_ident!("Variant{index}");

                self.state.stack.push(PathSegment::OneOf { index });
                let PropertyValue { body, try_from } = PropertyValueGenerator {
                    id: self.id,
                    variant: self.variant,
                    self_type: SelfType::enum_(&name.to_token_stream()),
                    resolver: self.resolver,
                    locations: self.locations,
                    value,
                    state: self.state,
                }
                .finish();
                self.state.stack.pop();

                (
                    quote! {
                        #name #body
                    },
                    try_from,
                    quote!(todo!();),
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
        // let conversion = match self.variant {
        //     Variant::Owned => {
        //         let as_ref = conversion.iter().map(
        //             |Conversion {
        //                  as_ref, match_arm, ..
        //              }| quote!(#match_arm #as_ref),
        //         );
        //         let as_mut = conversion.iter().map(
        //             |Conversion {
        //                  as_mut, match_arm, ..
        //              }| quote!(#match_arm #as_mut),
        //         );
        //
        //         quote! {
        //             fn as_ref(&self) -> Self::Ref<'_> {
        //                 match &self {
        //                     #(#as_ref),*
        //                 }
        //             }
        //
        //             fn as_mut(&mut self) -> Self::Mut<'_> {
        //                 match &mut self {
        //                     #(#as_mut),*
        //                 }
        //             }
        //         }
        //     }
        //     Variant::Ref | Variant::Mut => {
        //         let match_arms = conversion.into_iter().map(
        //             |Conversion {
        //                  into_owned,
        //                  match_arm,
        //                  ..
        //              }| quote!(#match_arm #into_owned),
        //         );
        //
        //         quote! {
        //             fn into_owned(self) -> Self::Owned {
        //                 match self {
        //                     #(#match_arms),*
        //                 }
        //             }
        //         }
        //     }
        // };

        Type {
            def,
            impl_ty: quote!(#name #lifetime),
            impl_try_from_value: try_from,
            impl_conversion: quote!(todo!();),
        }
    }
}
