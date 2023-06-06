use crate::{
    select::{path::JsonPath, value::Value},
    EntityNode, View,
};

// new module
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

impl PropertyMatch<'_> {
    pub(crate) fn matches(&self, view: &View, node: &EntityNode) -> bool {
        todo!()
    }
}
