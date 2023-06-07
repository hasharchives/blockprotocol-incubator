use alloc::{
    borrow::{Cow, ToOwned},
    vec::Vec,
};

use turbine::{entity::Entity, BaseUrl, BaseUrlRef};

use crate::select::value::{Object, Value};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Segment<'a> {
    Field(Cow<'a, str>),
    Index(usize),
}

impl<'a> From<BaseUrlRef<'a>> for Segment<'a> {
    fn from(value: BaseUrlRef<'a>) -> Self {
        Self::Field(Cow::Borrowed(value.as_str()))
    }
}

impl<'a> From<BaseUrl> for Segment<'a> {
    fn from(value: BaseUrl) -> Self {
        Self::Field(Cow::Owned(value.as_str().to_owned()))
    }
}

impl<'a> From<usize> for Segment<'a> {
    fn from(value: usize) -> Self {
        Self::Index(value)
    }
}

pub struct JsonPath<'a>(Cow<'a, [Segment<'a>]>);

impl<'a> JsonPath<'a> {
    pub fn new() -> Self {
        Self(Cow::Owned(Vec::new()))
    }

    pub fn from_slice(segments: &'a [Segment<'a>]) -> Self {
        Self(Cow::Borrowed(segments))
    }

    pub fn then(mut self, segment: impl Into<Segment<'a>>) -> Self {
        self.0.to_mut().push(segment.into());
        self
    }

    pub(crate) fn traverse_entity<'b>(&self, entity: &'b Entity) -> Option<Value<'b>> {
        let value = entity.properties.properties();

        if self.0.is_empty() {
            return Some(
                value
                    .iter()
                    .map(|(key, value)| (Value::from(key.as_str()), Value::from(value)))
                    .collect::<Object>()
                    .into(),
            );
        }

        let (first, rest) = self.0.split_first()?;

        let value = match first {
            Segment::Field(field) => value.get(field.as_ref())?,
            Segment::Index(_) => {
                return None;
            }
        };

        JsonPath(Cow::Borrowed(rest)).traverse(value)
    }

    fn traverse<'b>(&self, value: &'b serde_json::Value) -> Option<Value<'b>> {
        let mut value = value;

        for segment in self.0.iter() {
            match segment {
                Segment::Field(field) => {
                    value = value.get(field.as_ref())?;
                }
                Segment::Index(index) => {
                    value = value.get(index)?;
                }
            }
        }

        Some(value.into())
    }
}
