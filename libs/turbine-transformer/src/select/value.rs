use alloc::{borrow::Cow, collections::BTreeMap, string::String, vec::Vec};

pub struct Array<'a> {
    pub values: Vec<Value<'a>>,
}

impl<'a> FromIterator<Value<'a>> for Array<'a> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Value<'a>>,
    {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

pub struct Object<'a> {
    pub properties: Vec<(Value<'a>, Value<'a>)>,
}

impl<'a, K, V> FromIterator<(K, V)> for Object<'a>
where
    K: Into<Value<'a>>,
    V: Into<Value<'a>>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let properties = iter
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();

        Self { properties }
    }
}

pub enum Value<'a> {
    Null,
    Bool(bool),
    Integer(i128),
    Float(f64),
    String(Cow<'a, str>),
    Array(Array<'a>),
    Object(Object<'a>),
}

macro_rules! impl_from {
    (Int => $($ty:ty),*) => {
        $(
            impl<'a> From<$ty> for Value<'a> {
                fn from(value: $ty) -> Self {
                    Self::Integer(value as i128)
                }
            }
        )*
    };

    (Float => $($ty:ty),*) => {
        $(
            impl<'a> From<$ty> for Value<'a> {
                fn from(value: $ty) -> Self {
                    Self::Float(value as f64)
                }
            }
        )*
    };
}

impl From<()> for Value<'_> {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl_from!(Int => i8, i16, i32, i64, i128, isize);
impl_from!(Float => f32, f64);

impl From<bool> for Value<'_> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Self::String(Cow::Borrowed(value))
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(value: String) -> Self {
        Self::String(Cow::Owned(value))
    }
}

impl<'a> From<Array<'a>> for Value<'a> {
    fn from(value: Array<'a>) -> Self {
        Self::Array(value)
    }
}

impl<'a> From<Vec<Value<'a>>> for Value<'a> {
    fn from(value: Vec<Value<'a>>) -> Self {
        Self::Array(Array { values: value })
    }
}

impl<'a> From<Object<'a>> for Value<'a> {
    fn from(value: Object<'a>) -> Self {
        Self::Object(value)
    }
}

impl<'a> From<BTreeMap<String, Value<'a>>> for Value<'a> {
    fn from(value: BTreeMap<String, Value<'a>>) -> Self {
        let object = value.into_iter().collect();

        Self::Object(object)
    }
}

impl<'a> From<serde_json::Value> for Value<'a> {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(value),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Self::Integer(value as i128)
                } else if let Some(value) = value.as_f64() {
                    Self::Float(value)
                } else {
                    unreachable!()
                }
            }
            serde_json::Value::String(value) => Self::String(Cow::Owned(value)),
            serde_json::Value::Array(array) => Self::Array(Array {
                values: array.into_iter().map(Value::from).collect(),
            }),
            serde_json::Value::Object(object) => Self::Object(Object {
                properties: object
                    .into_iter()
                    .map(|(k, v)| (Value::from(k), Value::from(v)))
                    .collect(),
            }),
        }
    }
}

impl<'a> From<&'a serde_json::Value> for Value<'a> {
    fn from(value: &'a serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(*value),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Self::Integer(value as i128)
                } else if let Some(value) = value.as_f64() {
                    Self::Float(value)
                } else {
                    unreachable!()
                }
            }
            serde_json::Value::String(value) => Self::String(Cow::Borrowed(value)),
            serde_json::Value::Array(array) => Self::Array(Array {
                values: array.iter().map(Value::from).collect(),
            }),
            serde_json::Value::Object(object) => Self::Object(Object {
                properties: object
                    .iter()
                    .map(|(k, v)| (Value::from(k.as_str()), Value::from(v)))
                    .collect(),
            }),
        }
    }
}
