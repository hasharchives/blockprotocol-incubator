use proc_macro2::TokenStream;
use quote::quote;
use type_system::PropertyType;

use crate::name::NameResolver;

pub(crate) fn generate(_: &PropertyType, _: &NameResolver) -> TokenStream {
    quote!(unimplemented!();)
}
