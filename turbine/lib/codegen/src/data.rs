use proc_macro2::TokenStream;
use quote::quote;
use type_system::DataType;

use crate::name::NameResolver;

// TODO: we need to special case data types from blockprotocol, those should be references via
//  blockprotocol crate
pub(crate) fn generate(_: &DataType, _: &NameResolver) -> TokenStream {
    quote!(unimplemented!())
}
