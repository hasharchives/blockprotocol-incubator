use error_stack::{Report, Result};
use onlyerror::Error;
use serde::Serialize;
use serde_json::Value;

use crate::{
    url, DataType, DataTypeMut, DataTypeRef, Type, TypeMut, TypeRef, TypeUrl, VersionedUrlRef,
};

#[derive(Debug, Clone, Error)]
pub enum NullError {
    #[error("`{0:?}` is not `null`")]
    NotNull(Value),
}

#[derive(Debug, Copy, Clone, Serialize)]
pub struct Null;

impl TypeUrl for Null {
    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/null/" / v / 1);
}

impl Type for Null {
    // `Null` is `Null`, you cannot change the value of it
    type Mut<'a> = Self where Self: 'a;
    type Ref<'a> = Self where Self: 'a;

    fn as_mut(&mut self) -> Self::Mut<'_> {
        *self
    }

    fn as_ref(&self) -> Self::Ref<'_> {
        *self
    }
}

impl DataType for Null {
    type Error = NullError;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        if value.is_null() {
            Ok(Self)
        } else {
            Err(Report::new(NullError::NotNull(value)))
        }
    }
}

impl TypeRef for Null {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl<'a> DataTypeRef<'a> for Null {
    type Error = NullError;

    fn try_from_value(value: &'a Value) -> Result<Self, Self::Error> {
        if value.is_null() {
            Ok(Self)
        } else {
            Err(Report::new(NullError::NotNull(value.clone())))
        }
    }
}

impl TypeMut for Null {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl<'a> DataTypeMut<'a> for Null {
    type Error = NullError;

    fn try_from_value(value: &'a mut Value) -> Result<Self, Self::Error> {
        <Self as DataTypeRef>::try_from_value(value)
    }
}
