use core::ops::{Deref, DerefMut};

use error_stack::{Report, Result};
use onlyerror::Error;
use serde::Serialize;
use serde_json::Value;

use crate::{url, DataType, DataTypeMut, DataTypeRef, Type, TypeMut, TypeRef, VersionedUrlRef};

#[derive(Debug, Clone, Error)]
pub enum NumberError {
    #[error("`{0:?}` is not a number")]
    NotANumber(Value),
}

#[derive(Debug, Clone, Serialize)]
pub struct Number(serde_json::Number);

impl Number {
    #[must_use]
    pub fn new(value: impl Into<serde_json::Number>) -> Self {
        Self(value.into())
    }
}

impl Deref for Number {
    type Target = serde_json::Number;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Type for Number {
    type Mut<'a> = NumberMut<'a> where Self: 'a;
    type Ref<'a> = NumberRef<'a> where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/number/" / v / 1);

    fn as_mut(&mut self) -> Self::Mut<'_> {
        NumberMut(&mut self.0)
    }

    fn as_ref(&self) -> Self::Ref<'_> {
        NumberRef(&self.0)
    }
}

impl DataType for Number {
    type Error = NumberError;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        if let Value::Number(value) = value {
            Ok(Self(value))
        } else {
            Err(Report::new(NumberError::NotANumber(value)))
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize)]
pub struct NumberRef<'a>(&'a serde_json::Number);

impl Deref for NumberRef<'_> {
    type Target = serde_json::Number;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl TypeRef for NumberRef<'_> {
    type Owned = Number;

    fn into_owned(self) -> Self::Owned {
        Number(self.0.clone())
    }
}

impl<'a> DataTypeRef<'a> for NumberRef<'a> {
    type Error = NumberError;

    fn try_from_value(value: &'a Value) -> Result<Self, Self::Error> {
        if let Value::Number(value) = value {
            Ok(Self(value))
        } else {
            Err(Report::new(NumberError::NotANumber(value.clone())))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NumberMut<'a>(&'a mut serde_json::Number);

impl Deref for NumberMut<'_> {
    type Target = serde_json::Number;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for NumberMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl TypeMut for NumberMut<'_> {
    type Owned = Number;

    fn into_owned(self) -> Self::Owned {
        Number(self.0.clone())
    }
}

impl<'a> DataTypeMut<'a> for NumberMut<'a> {
    type Error = NumberError;

    fn try_from_value(value: &'a mut Value) -> Result<Self, Self::Error> {
        if let Value::Number(value) = value {
            Ok(Self(value))
        } else {
            Err(Report::new(NumberError::NotANumber(value.clone())))
        }
    }
}