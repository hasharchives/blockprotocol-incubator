use alloc::{borrow::Cow, vec::Vec};

pub struct Array<'a> {
    pub values: Vec<Value<'a>>,
}

pub struct Object<'a> {
    pub properties: Vec<(Value<'a>, Value<'a>)>,
}

pub enum Value<'a> {
    Null,
    Bool(bool),
    Integer(i128),
    String(Cow<'a, str>),
    Array(Array<'a>),
    Object(Object<'a>),
}
