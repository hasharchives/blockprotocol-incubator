use alloc::{boxed::Box, vec, vec::Vec};

use crate::{
    select::{dynamic::DynamicMatch, property::PropertyMatch, type_::TypeMatch, Statement},
    EntityNode, View,
};

pub enum Clause<'a> {
    /// If empty, always true.
    All(Vec<Clause<'a>>),
    /// If empty, always false.
    Any(Vec<Clause<'a>>),
    Not(Box<Clause<'a>>),

    Type(TypeMatch<'a>),
    Dynamic(DynamicMatch),
    Property(PropertyMatch<'a>),
}

impl Clause<'_> {
    pub fn matches(&self, view: &View, node: &EntityNode) -> bool {
        match self {
            Self::All(clauses) => clauses.iter().all(|c| c.matches(view, node)),
            Self::Any(clauses) => clauses.iter().any(|c| c.matches(view, node)),
            Self::Not(clause) => !clause.matches(view, node),

            Self::Type(matches) => matches.matches(view, node),
            Self::Dynamic(matches) => matches.matches(view, node),
            Self::Property(matches) => matches.matches(view, node),
        }
    }

    pub fn or(self, other: impl Into<Self>) -> Self {
        let other = other.into();

        if let Self::Any(mut clauses) = self {
            clauses.push(other);
            return Self::Any(clauses);
        }

        Self::Any(vec![self, other])
    }

    pub fn and(self, other: impl Into<Self>) -> Self {
        let other = other.into();

        if let Self::All(mut clauses) = self {
            clauses.push(other);
            return Self::All(clauses);
        }

        Self::All(vec![self, other])
    }

    pub fn not(self) -> Self {
        Self::Not(Box::new(self))
    }
}

impl<'a> Clause<'a> {
    combinator!(with_links, into_statement);
}
