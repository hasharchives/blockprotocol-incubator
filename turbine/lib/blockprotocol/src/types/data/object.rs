use alloc::string::String;
use core::ops::{Deref, DerefMut};

use error_stack::{Report, Result};
use onlyerror::Error;
use serde::Serialize;
use serde_json::Value;

use crate::{url, DataType, DataTypeMut, DataTypeRef, Type, TypeMut, TypeRef, VersionedUrlRef};

#[derive(Debug, Clone, Error)]
pub enum ObjectError {
    #[error("`{0:?}` is not an object")]
    NotAnObject(Value),
}

#[derive(Debug, Clone, Serialize)]
pub struct Object(serde_json::Map<String, Value>);

impl Deref for Object {
    type Target = serde_json::Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Object {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Type for Object {
    type Mut<'a> = ObjectMut<'a> where Self: 'a;
    type Ref<'a> = ObjectRef<'a> where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/object/" / v / 1);

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

#[derive(Debug, Copy, Clone, Serialize)]
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
        value.as_object().map_or_else(
            || Err(Report::new(ObjectError::NotAnObject(value.clone()))),
            |value| Ok(Self(value)),
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ObjectMut<'a>(&'a mut serde_json::Map<String, Value>);

impl Deref for ObjectMut<'_> {
    type Target = serde_json::Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for ObjectMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl TypeMut for ObjectMut<'_> {
    type Owned = Object;

    fn into_owned(self) -> Self::Owned {
        Object(self.0.clone())
    }
}

impl<'a> DataTypeMut<'a> for ObjectMut<'a> {
    type Error = ObjectError;

    fn try_from_value(value: &'a mut Value) -> Result<Self, Self::Error> {
        if let Value::Object(value) = value {
            Ok(Self(value))
        } else {
            Err(Report::new(ObjectError::NotAnObject(value.clone())))
        }
    }
}
