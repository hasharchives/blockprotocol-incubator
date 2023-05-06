use std::collections::HashMap;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use type_system::{url::VersionedUrl, PropertyValues};

use crate::{
    name::{Location, NameResolver},
    property::{
        type_::{Type, TypeGenerator},
        State,
    },
    shared::Variant,
};

pub(super) struct Inner {
    name: Ident,
    stream: TokenStream,
}

impl ToTokens for Inner {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.stream.clone());
    }
}

pub(super) struct InnerGenerator<'a> {
    pub(super) id: &'a VersionedUrl,
    pub(super) variant: Variant,

    pub(super) values: &'a [PropertyValues],

    pub(super) resolver: &'a NameResolver<'a>,
    pub(super) locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,

    pub(super) state: &'a mut State,
}

impl<'a> InnerGenerator<'a> {
    pub(super) fn finish(mut self) -> Ident {
        let n = self.state.inner.len();
        let name = format_ident!("{}{n}", self.state.inner_name);

        let Type {
            def,
            impl_ty,
            impl_try_from_value,
            impl_conversion,
        } = TypeGenerator {
            id: self.id,
            name: &name,
            variant: self.variant,
            values: self.values,
            resolver: self.resolver,
            locations: self.locations,
            state: &mut self.state,
        }
        .finish();

        let value_ref = match self.variant {
            Variant::Owned => None,
            Variant::Ref => Some(quote!(&'a)),
            Variant::Mut => Some(quote!(&'a mut)),
        };

        self.state.inner.push(Inner {
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
}
