use std::ops::Deref;

pub struct Text(String);

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct TextRef<'a>(&'a str);

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

pub struct NumberRef<'a>(&'a serde_json::Number);

impl Deref for NumberRef<'_> {
    type Target = serde_json::Number;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

pub struct Boolean(bool);

impl Deref for Boolean {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Null;

pub struct EmptyList;

pub struct Object(serde_json::Map<String, serde_json::Value>);

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
