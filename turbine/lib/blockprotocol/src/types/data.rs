use alloc::{borrow::ToOwned, string::String};
use core::ops::Deref;

use error_stack::{Report, Result};
use onlyerror::Error;
use serde_json::Value;

use crate::{url, DataType, DataTypeRef, Type, TypeRef, VersionedUrlRef};

#[derive(Debug, Clone, Error)]
pub enum TextError {
    #[error("`{0:?}` is not text")]
    NotText(Value),
}

pub struct Text(String);

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Type for Text {
    type Ref<'a> = TextRef<'a> where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/text" / v / 1);

    fn as_ref(&self) -> Self::Ref<'_> {
        TextRef(&self.0)
    }
}

impl DataType for Text {
    type Error = TextError;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        if let Value::String(value) = value {
            Ok(Self(value))
        } else {
            Err(Report::new(TextError::NotText(value)))
        }
    }
}

pub struct TextRef<'a>(&'a str);

impl TypeRef for TextRef<'_> {
    type Owned = Text;

    fn into_owned(self) -> Self::Owned {
        Text(self.0.to_owned())
    }
}

impl<'a> DataTypeRef<'a> for TextRef<'a> {
    type Error = TextError;

    fn try_from_value(value: &'a Value) -> Result<Self, Self::Error> {
        if let Some(value) = value.as_str() {
            Ok(Self(value))
        } else {
            Err(Report::new(TextError::NotText(value.clone())))
        }
    }
}

impl Deref for TextRef<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[derive(Debug, Clone, Error)]
pub enum NumberError {
    #[error("`{0:?}` is not a number")]
    NotANumber(Value),
}

pub struct Number(serde_json::Number);

impl Deref for Number {
    type Target = serde_json::Number;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Type for Number {
    type Ref<'a> = NumberRef<'a> where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/number" / v / 1);

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

#[derive(Debug, Clone, Error)]
pub enum BooleanError {
    #[error("`{0:?}` is not a bool")]
    NotABoolean(Value),
}

#[derive(Debug, Copy, Clone)]
pub struct Boolean(bool);

impl Deref for Boolean {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Type for Boolean {
    type Ref<'a> = Self where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/boolean" / v / 1);

    fn as_ref(&self) -> Self::Ref<'_> {
        *self
    }
}

impl DataType for Boolean {
    type Error = BooleanError;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        if let Value::Bool(value) = value {
            Ok(Self(value))
        } else {
            Err(Report::new(BooleanError::NotABoolean(value)))
        }
    }
}

impl<'a> DataTypeRef<'a> for Boolean {
    type Error = BooleanError;

    fn try_from_value(value: &'a Value) -> Result<Self, Self::Error> {
        if let Some(value) = value.as_bool() {
            Ok(Self(value))
        } else {
            Err(Report::new(BooleanError::NotABoolean(value.clone())))
        }
    }
}

impl TypeRef for Boolean {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
}

#[derive(Debug, Clone, Error)]
pub enum NullError {
    #[error("`{0:?}` is not `null`")]
    NotNull(Value),
}

#[derive(Debug, Copy, Clone)]
pub struct Null;

impl Type for Null {
    type Ref<'a> = Self where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/null" / v / 1);

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

impl TypeRef for Null {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
}

#[derive(Debug, Clone, Error)]
pub enum EmptyListError {
    #[error("`{0:?}` is not an array")]
    NotAnArray(Value),

    #[error("array is not empty")]
    NotEmpty,
}

#[derive(Debug, Copy, Clone)]
pub struct EmptyList;

impl Type for EmptyList {
    type Ref<'a> = Self where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/emptyList" / v / 1);

    fn as_ref(&self) -> Self::Ref<'_> {
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

impl<'a> DataTypeRef<'a> for EmptyList {
    type Error = EmptyListError;

    fn try_from_value(value: &'a Value) -> Result<Self, Self::Error> {
        if let Some(value) = value.as_array() {
            if value.is_empty() {
                Ok(Self)
            } else {
                Err(Report::new(EmptyListError::NotEmpty))
            }
        } else {
            Err(Report::new(EmptyListError::NotAnArray(value.clone())))
        }
    }
}

impl TypeRef for EmptyList {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
}

#[derive(Debug, Clone, Error)]
pub enum ObjectError {
    #[error("`{0:?}` is not an object")]
    NotAnObject(Value),
}

pub struct Object(serde_json::Map<String, Value>);

impl Type for Object {
    type Ref<'a> = ObjectRef<'a> where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/object" / v / 1);

    fn as_ref(&self) -> Self::Ref<'_> {
        ObjectRef(&self.0)
    }
}

impl DataType for Object {
    type Error = ObjectError;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        if let Value::Object(value) = value {
            Ok(Self(value))
        } else {
            Err(Report::new(ObjectError::NotAnObject(value)))
        }
    }
}

impl Deref for Object {
    type Target = serde_json::Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ObjectRef<'a>(&'a serde_json::Map<String, Value>);

impl Deref for ObjectRef<'_> {
    type Target = serde_json::Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl TypeRef for ObjectRef<'_> {
    type Owned = Object;

    fn into_owned(self) -> Self::Owned {
        Object(self.0.clone())
    }
}
impl<'a> DataTypeRef<'a> for ObjectRef<'a> {
    type Error = ObjectError;

    fn try_from_value(value: &'a Value) -> Result<Self, Self::Error> {
        value
            .as_object()
            .map(Self)
            .ok_or_else(|| Report::new(ObjectError::NotAnObject(value.clone())))
    }
}
