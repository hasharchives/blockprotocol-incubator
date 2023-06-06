use alloc::borrow::Cow;

use funty::Numeric;

use crate::{
    select::{path::JsonPath, value::Value},
    EntityNode, View,
};

pub trait SatisfiesEquals<'a, Rhs> {
    fn into_match(self, rhs: Rhs) -> PropertyMatch<'a>;
}

impl<'a, T, U> SatisfiesEquals<'a, U> for T
where
    T: Into<Value<'a>>,
    U: Into<Value<'a>>,
{
    fn into_match(self, rhs: U) -> PropertyMatch<'a> {
        PropertyMatch {
            lhs: PathOrValue::Value(self.into()),
            condition: Condition::Equals,
            rhs: PathOrValue::Value(rhs.into()),
        }
    }
}

impl<'a, T> SatisfiesEquals<'a, JsonPath<'a>> for T
where
    T: Into<Value<'a>>,
{
    fn into_match(self, rhs: JsonPath<'a>) -> PropertyMatch<'a> {
        PropertyMatch {
            lhs: PathOrValue::Value(self.into()),
            condition: Condition::Equals,
            rhs: PathOrValue::Path(rhs),
        }
    }
}

impl<'a> SatisfiesEquals<'a, JsonPath<'a>> for JsonPath<'a> {
    fn into_match(self, rhs: JsonPath<'a>) -> PropertyMatch<'a> {
        PropertyMatch {
            lhs: PathOrValue::Path(self),
            condition: Condition::Equals,
            rhs: PathOrValue::Path(rhs),
        }
    }
}

impl<'a, U> SatisfiesEquals<'a, U> for JsonPath<'a>
where
    U: Into<Value<'a>>,
{
    fn into_match(self, rhs: U) -> PropertyMatch<'a> {
        PropertyMatch {
            lhs: PathOrValue::Path(self),
            condition: Condition::Equals,
            rhs: PathOrValue::Value(rhs.into()),
        }
    }
}

trait SatisfiesNotEquals<Lhs> {}

impl<T, U> SatisfiesNotEquals<U> for T {}

trait SatisfiesLessThan<Lhs> {}

impl<T, U> SatisfiesLessThan<U> for T
where
    T: Numeric,
    U: Numeric,
{
}

pub enum Condition {
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEquals,
    GreaterThan,
    GreaterThanOrEquals,
    Contains,
    StartsWith,
    EndsWith,
}

// TODO: JsonPath should be done via turbine :thinking:
//  (or untyped as alternative)
//  (properly typed is possible in turbine ~> path to value)
pub enum PathOrValue<'a> {
    Path(JsonPath<'a>),
    Value(Value<'a>),
}

pub struct PropertyMatch<'a> {
    lhs: PathOrValue<'a>,
    condition: Condition,
    rhs: PathOrValue<'a>,
}

impl<'a> PropertyMatch<'a> {
    pub fn equals<T, U>(lhs: T, rhs: U)
    where
        T: SatisfiesEquals<'a, U>,
    {
        lhs.into_match(rhs);
    }
}

impl PropertyMatch<'_> {
    pub(crate) fn matches(&self, view: &View, node: &EntityNode) -> bool {
        let Some(entity) = view.entity(node.id) else {
            return false;
        };

        let lhs = match &self.lhs {
            PathOrValue::Path(path) => path.traverse_entity(entity).map(Cow::Owned),
            PathOrValue::Value(value) => Some(Cow::Borrowed(value)),
        };

        let rhs = match &self.rhs {
            PathOrValue::Path(path) => path.traverse_entity(entity).map(Cow::Owned),
            PathOrValue::Value(value) => Some(Cow::Borrowed(value)),
        };

        let Some(lhs) = lhs else {
            return false;
        };

        let Some(rhs) = rhs else {
            return false;
        };

        match self.condition {
            Condition::Equals => lhs == rhs,
            Condition::NotEquals => lhs != rhs,
            Condition::LessThan => lhs < rhs,
            Condition::LessThanOrEquals => lhs <= rhs,
            Condition::GreaterThan => lhs > rhs,
            Condition::GreaterThanOrEquals => lhs >= rhs,
            Condition::Contains => lhs.contains(rhs.as_ref()),
            Condition::StartsWith => lhs.starts_with(rhs.as_ref()),
            Condition::EndsWith => lhs.ends_with(rhs.as_ref()),
        }
    }
}
