use std::collections::HashMap;
use proc_macro2::TokenStream;
use quote::format_ident;
use type_system::PropertyValues;
use type_system::url::VersionedUrl;
use crate::name::{Location, NameResolver};
use crate::property::State;
use crate::shared::Variant;

struct InnerGenerator<'a> {
    id: &'a VersionedUrl,
    variant: Variant,
    
    values: &'a[PropertyValues],
    
    resolver: &'a NameResolver<'a>,
    locations: &'a HashMap<&'a VersionedUrl, Location<'a>>,
    
    state: &'a mut State
}

impl<'a> InnerGenerator<'a> {
    fn finish(mut self) -> TokenStream {
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
}