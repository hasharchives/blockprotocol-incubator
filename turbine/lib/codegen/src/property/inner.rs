use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use type_system::{url::VersionedUrl, PropertyValues};

use crate::{
    name::{Location, NameResolver},
    property::State,
    shared::Variant,
};

pub(super) struct InnerGenerator<'a> {
    pub(super) id: &'a VersionedUrl,
    pub(super) variant: Variant,

    pub(super) values: &'a [PropertyValues],

    pub(super) resolver: &'a NameResolver<'a>,
    pub(super) locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,

    pub(super) state: &'a mut State,
}

impl<'a> InnerGenerator<'a> {
    pub(super) fn finish(mut self) -> TokenStream {
        let n = self.state.inner.len();
        let name = format_ident!("{}{n}", self.state.inner_name);

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
