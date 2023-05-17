use alloc::{borrow::ToOwned, string::String};
use core::ops::{Deref, DerefMut};

use error_stack::{Report, Result};
use onlyerror::Error;
use serde::Serialize;
use serde_json::Value;

use crate::{
    url, DataType, DataTypeMut, DataTypeRef, Type, TypeMut, TypeRef, TypeUrl, VersionedUrlRef,
};

#[derive(Debug, Clone, Error)]
pub enum TextError {
    #[error("`{0:?}` is not text")]
    NotText(Value),
}

#[derive(Debug, Clone, Serialize)]
pub struct Text(String);

impl Text {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl<T> From<T> for Text
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Text {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl TypeUrl for Text {
    type InheritsFrom = ();

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/object/" / v / 1);
}

impl Type for Text {
    type Mut<'a> = TextMut<'a> where Self: 'a;
    type Ref<'a> = TextRef<'a> where Self: 'a;

    fn as_mut(&mut self) -> Self::Mut<'_> {
        TextMut(&mut self.0)
    }

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

#[derive(Debug, Copy, Clone, Serialize)]
pub struct TextRef<'a>(&'a str);

impl<'a> TextRef<'a> {
    #[must_use]
    pub const fn as_str(&self) -> &'a str {
        self.0
    }
}

impl Deref for TextRef<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl TypeUrl for TextRef<'_> {
    type InheritsFrom = ();

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/object/" / v / 1);
}

impl TypeRef for TextRef<'_> {
    type Owned = Text;

    fn into_owned(self) -> Self::Owned {
        Text(self.0.to_owned())
    }
}

impl<'a> DataTypeRef<'a> for TextRef<'a> {
    type Error = TextError;

    fn try_from_value(value: &'a Value) -> Result<Self, Self::Error> {
        value.as_str().map_or_else(
            || Err(Report::new(TextError::NotText(value.clone()))),
            |value| Ok(Self(value)),
        )
    }
}

#[derive(Debug, Serialize)]
pub struct TextMut<'a>(&'a mut str);

impl Deref for TextMut<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for TextMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl TypeUrl for TextMut<'_> {
    type InheritsFrom = ();

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/object/" / v / 1);
}

impl TypeMut for TextMut<'_> {
    type Owned = Text;

    fn into_owned(self) -> Self::Owned {
        Text(self.0.to_owned())
    }
}

impl<'a> DataTypeMut<'a> for TextMut<'a> {
    type Error = TextError;

    fn try_from_value(value: &'a mut Value) -> Result<Self, Self::Error> {
        if let Value::String(value) = value {
            Ok(Self(value.as_mut_str()))
        } else {
            Err(Report::new(TextError::NotText(value.clone())))
        }
    }
}
