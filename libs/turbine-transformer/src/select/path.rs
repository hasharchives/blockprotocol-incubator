use alloc::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Segment<'a> {
    Field(Cow<'a, str>),
    Index(usize),
}

pub struct JsonPath<'a>(Cow<'a, [Segment<'a>]>);
