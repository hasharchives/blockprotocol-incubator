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
    }
}
