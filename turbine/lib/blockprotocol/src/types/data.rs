use std::ops::Deref;

use crate::{url, Type, TypeRef, VersionedUrlRef};

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

pub struct TextRef<'a>(&'a str);

impl TypeRef for TextRef<'_> {
    type Owned = Text;

    fn into_owned(self) -> Self::Owned {
        Text(self.0.to_owned())
    }
}

impl Deref for TextRef<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
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

impl TypeRef for Boolean {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
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

impl TypeRef for Null {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
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

impl TypeRef for EmptyList {
    type Owned = Self;

    fn into_owned(self) -> Self::Owned {
        self
    }
}

pub struct Object(serde_json::Map<String, serde_json::Value>);

impl Type for Object {
    type Ref<'a> = ObjectRef<'a> where Self: 'a;

    const ID: VersionedUrlRef<'static> =
        url!("https://blockprotocol.org/@blockprotocol/types/data-type/object" / v / 1);

    fn as_ref(&self) -> Self::Ref<'_> {
        ObjectRef(&self.0)
    }
}

impl Deref for Object {
    type Target = serde_json::Map<String, serde_json::Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ObjectRef<'a>(&'a serde_json::Map<String, serde_json::Value>);

impl Deref for ObjectRef<'_> {
    type Target = serde_json::Map<String, serde_json::Value>;

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
