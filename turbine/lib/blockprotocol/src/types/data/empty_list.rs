use error_stack::{Report, Result};
use onlyerror::Error;
use serde::Serialize;
use serde_json::Value;

use crate::{url, DataType, DataTypeMut, DataTypeRef, Type, TypeMut, TypeRef, VersionedUrlRef};

#[derive(Debug, Clone, Error)]
pub enum EmptyListError {
    #[error("`{0:?}` is not an array")]
    NotAnArray(Value),

    #[error("array is not empty")]
    NotEmpty,
}

#[derive(Debug, Copy, Clone, Serialize)]
pub struct EmptyList;

impl Type for EmptyList {
    // `EmptyList` is `EmptyList`, you cannot change the value of it
    type Mut<'a> = Self where Self: 'a;
    type Ref<'a> = Self where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/emptyList/" / v / 1);

    fn as_ref(&self) -> Self::Ref<'_> {
        *self
    }

    fn as_mut(&mut self) -> Self::Mut<'_> {
        *self
    }
}

impl DataType for EmptyList {
    type Error = EmptyListError;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        if let Value::Array(value) = value {
            if value.is_empty() {
                Ok(Self)
            } else {
                Err(Report::new(EmptyListError::NotEmpty))
            }
        } else {
            Err(Report::new(EmptyListError::NotAnArray(value)))
        }
    }
}

impl TypeRef for EmptyList {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl<'a> DataTypeRef<'a> for EmptyList {
    type Error = EmptyListError;

    fn try_from_value(value: &'a Value) -> Result<Self, Self::Error> {
        value.as_array().map_or_else(
            || Err(Report::new(EmptyListError::NotAnArray(value.clone()))),
            |value| {
                if value.is_empty() {
                    Ok(Self)
                } else {
                    Err(Report::new(EmptyListError::NotEmpty))
                }
            },
        )
    }
}

impl TypeMut for EmptyList {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
}

impl<'a> DataTypeMut<'a> for EmptyList {
    type Error = EmptyListError;

    fn try_from_value(value: &'a mut Value) -> Result<Self, Self::Error> {
        <Self as DataTypeRef<'a>>::try_from_value(value)
    }
}
