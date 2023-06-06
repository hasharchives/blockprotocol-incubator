use alloc::{borrow::Cow, vec::Vec};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment<'a> {
    Field(Cow<'a, str>),
    Index(usize),
}

pub struct LinkedPathSegment<'a> {
    this: &'a PathSegment<'a>,
    next: Option<&'a EntityPath<'a>>,
}

pub enum EntityPath<'a> {
    Borrowed(&'a [PathSegment<'a>]),
    Owned(Vec<PathSegment<'a>>),
    Linked(LinkedPathSegment<'a>),
}

impl<'a> EntityPath<'a> {
    pub fn new(path: &'a [PathSegment]) -> Self {
        Self::Borrowed(path)
    }

    pub fn new_owned(path: Vec<PathSegment<'a>>) -> Self {
        Self::Owned(path)
    }

    pub fn new_linked(path: LinkedPathSegment<'a>) -> Self {
        Self::Linked(path)
    }
}
