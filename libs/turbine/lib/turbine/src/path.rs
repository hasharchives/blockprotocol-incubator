use alloc::{borrow::Cow, boxed::Box, vec::Vec};
use core::slice;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment<'a> {
    Field(Cow<'a, str>),
    Index(usize),
}

pub enum Next<'a> {
    Borrowed(&'a EntityPath<'a>),
    Owned(Box<EntityPath<'a>>),
}

impl<'a> Next<'a> {
    fn as_ref(&self) -> &EntityPath<'a> {
        match self {
            Self::Borrowed(path) => path,
            Self::Owned(path) => path,
        }
    }
}

pub struct LinkedPathSegment<'a> {
    this: &'a PathSegment<'a>,
    next: Option<Next<'a>>,
}

impl<'a> LinkedPathSegment<'a> {
    #[must_use]
    pub const fn new(this: &'a PathSegment<'a>, next: Option<Next<'a>>) -> Self {
        Self { this, next }
    }
}

pub enum EntityPath<'a> {
    Borrowed(&'a [PathSegment<'a>]),
    Owned(Vec<PathSegment<'a>>),
    Linked(LinkedPathSegment<'a>),
}

impl<'a> EntityPath<'a> {
    #[must_use]
    pub const fn new(path: &'a [PathSegment]) -> Self {
        Self::Borrowed(path)
    }

    #[must_use]
    pub fn new_owned(path: Vec<PathSegment<'a>>) -> Self {
        Self::Owned(path)
    }

    #[must_use]
    pub const fn new_linked(path: LinkedPathSegment<'a>) -> Self {
        Self::Linked(path)
    }
}

pub enum EntityPathIterator<'a> {
    Slice(slice::Iter<'a, PathSegment<'a>>),
    Linked(&'a LinkedPathSegment<'a>),
    Empty,
}

impl<'a> Iterator for EntityPathIterator<'a> {
    type Item = &'a PathSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EntityPathIterator::Slice(slice) => slice.next(),
            EntityPathIterator::Linked(node) => {
                let next = node.this;

                if let Some(next) = &node.next {
                    *self = match next.as_ref() {
                        EntityPath::Borrowed(slice) => EntityPathIterator::Slice(slice.iter()),
                        EntityPath::Owned(owned) => EntityPathIterator::Slice(owned.iter()),
                        EntityPath::Linked(linked) => EntityPathIterator::Linked(linked),
                    };
                } else {
                    *self = EntityPathIterator::Empty;
                }

                Some(next)
            }
            EntityPathIterator::Empty => None,
        }
    }
}
