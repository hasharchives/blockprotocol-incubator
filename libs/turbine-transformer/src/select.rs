use alloc::{boxed::Box, collections::BTreeSet, vec, vec::Vec};

use petgraph::{graph::NodeIndex, visit::IntoNodeReferences};
use turbine::{entity::EntityId, TypeUrl, VersionedUrlRef};

use crate::{EntityNode, View};

type DynamicMatchFn<'a> = dyn Fn(&View, &EntityNode) -> bool + 'a;
type BoxedDynamicMatchFn = Box<DynamicMatchFn<'static>>;

macro_rules! combinator {
    ($($v:lifetime ,)? or) => {
    pub fn or$(<$v>)?(self, other: impl Into<Clause<'a>>) -> Clause<'a> {
        let this = self.into();
        let other = other.into();

        if let Clause::Any(mut clauses) = other {
            clauses.insert(0, this);
            Clause::Any(clauses)
        } else {
            Clause::Any(vec![this, other])
        }
    }
    };

    ($($v:lifetime ,)? and) => {
    pub fn and$(<$v>)?(self, other: impl Into<Clause<'a>>) -> Clause<'a> {
        let this = self.into();
        let other = other.into();

        if let Clause::All(mut clauses) = other {
            clauses.insert(0, this);
            Clause::All(clauses)
        } else {
            Clause::All(vec![this, other])
        }
    }
    };

    ($($v:lifetime ,)? not) => {
    pub fn not$(<$v>)?(self) -> Clause<'a> {
        Clause::Not(Box::new(self.into()))
    }
    };

    ($($tt:ident $(<$v:lifetime>)?),+) => {
        $(combinator!($($v,)? $tt);)*
    };
}

pub struct DynamicMatch {
    dynamic: BoxedDynamicMatchFn,
}

impl DynamicMatch {
    combinator!(or<'a>, and<'a>, not<'a>);

    #[must_use]
    pub fn new(dynamic: impl Fn(&View, &EntityNode) -> bool + 'static) -> Self {
        Self {
            dynamic: Box::new(dynamic),
        }
    }

    pub(crate) fn matches(&self, view: &View, node: &EntityNode) -> bool {
        (self.dynamic)(view, node)
    }
}

impl<'a> From<DynamicMatch> for Clause<'a> {
    fn from(dynamic: DynamicMatch) -> Self {
        Clause::Dynamic(dynamic)
    }
}

pub struct TypeMatch<'a> {
    ids: BTreeSet<EntityId>,
    types: BTreeSet<VersionedUrlRef<'static>>,

    inherits_from: BTreeSet<VersionedUrlRef<'a>>,
}

impl TypeMatch<'_> {
    pub(crate) fn matches(&self, view: &View, node: &EntityNode) -> bool {
        if self.ids.contains(&node.id) {
            return true;
        }

        let Some(type_) = node.type_ else {
            return false;
        };

        if self.types.contains(&VersionedUrlRef::from(type_)) {
            return true;
        }

        let inherits_from = (view.lookup_inherits_from)(VersionedUrlRef::from(type_));

        let common = self.inherits_from.intersection(&inherits_from).count();
        if common > 0 {
            return true;
        }

        false
    }

    #[must_use]
    pub const fn new() -> Self {
        Self {
            ids: BTreeSet::new(),
            types: BTreeSet::new(),
            inherits_from: BTreeSet::new(),
        }
    }

    pub fn or_id(mut self, id: EntityId) -> Self {
        self.ids.insert(id);
        self
    }

    pub fn or_type<T: TypeUrl>(mut self) -> Self {
        self.types.insert(T::ID);
        self
    }

    pub fn or_inherits_from<T: TypeUrl>(mut self) -> Self {
        self.inherits_from.insert(T::ID);
        self
    }
}

impl<'a> TypeMatch<'a> {
    combinator!(or, and, not);

    pub fn with_links(self) -> Statement<'a> {
        Statement::from(self)
    }

    pub fn into_statement(self) -> Statement<'a> {
        Statement::from(self)
    }
}

// TODO: as types? ~> used during construction, but not after
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
pub enum ParameterOrValue<'a> {
    Parameter(JsonPath<'a>),
    Value(Value<'a>),
}

pub struct PropertyMatch<'a> {
    lhs: ParameterOrValue<'a>,
    condition: Condition,
    rhs: ParameterOrValue<'a>,
}

impl PropertyMatch<'_> {
    fn matches(&self, view: &View, node: &EntityNode) -> bool {
        todo!()
    }
}

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
    pub fn with_links(self) -> Statement<'a> {
        Statement::from(self)
    }

    pub fn into_statement(self) -> Statement<'a> {
        Statement::from(self)
    }
}

impl<'a> From<TypeMatch<'a>> for Clause<'a> {
    fn from(value: TypeMatch<'a>) -> Self {
        Self::Type(value)
    }
}

pub struct Statement<'a> {
    if_: Clause<'a>,

    left: Option<Clause<'a>>,
    right: Option<Clause<'a>>,
}

impl<'a> Statement<'a> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            if_: Clause::All(Vec::new()),
            left: None,
            right: None,
        }
    }

    #[must_use]
    pub fn with_if(mut self, if_: impl Into<Clause<'a>>) -> Self {
        self.if_ = if_.into();
        self
    }

    #[must_use]
    pub fn with_left(mut self, left: impl Into<Clause<'a>>) -> Self {
        self.left = Some(left.into());
        self
    }

    #[must_use]
    pub fn with_right(mut self, right: impl Into<Clause<'a>>) -> Self {
        self.right = Some(right.into());
        self
    }

    #[must_use]
    pub fn or_if(mut self, if_: impl Into<Clause<'a>>) -> Self {
        self.if_ = self.if_.or(if_);
        self
    }

    #[must_use]
    pub fn or_left(mut self, left: impl Into<Clause<'a>>) -> Self {
        let left = left.into();

        if let Some(this_left) = self.left {
            self.left = Some(this_left.or(left));
        } else {
            self.left = Some(left);
        }
        self
    }

    #[must_use]
    pub fn or_right(mut self, right: impl Into<Clause<'a>>) -> Self {
        let right = right.into();

        if let Some(this_right) = self.right {
            self.right = Some(this_right.or(right));
        } else {
            self.right = Some(right);
        }
        self
    }
}

impl<'a> From<TypeMatch<'a>> for Statement<'a> {
    fn from(value: TypeMatch<'a>) -> Self {
        Self::from(Clause::from(value))
    }
}

impl<'a> From<Clause<'a>> for Statement<'a> {
    fn from(value: Clause<'a>) -> Self {
        Self {
            if_: value,
            left: None,
            right: None,
        }
    }
}

struct Select<'a> {
    statements: Vec<Statement<'a>>,
}

impl Select<'_> {
    fn eval_link(view: &View, link: Option<EntityId>, if_: Option<&Clause>) -> bool {
        let Some(if_) = if_ else {
            // completely skip checks for links if we have no if_ statement
            // important(!) we do not check if link is None here, as we want to allow both
            // to ensure that links are not allowed at all, `if_` must be set to an empty set
            return true;
        };

        let Some(link) = link else {
            // if we have an if_ statement, but no link, we fail
            // contrast to above, as we're in a very different context here, we need to know
            // if there are any links and only want to allow those
            return false;
        };

        let Some(node) = view.lookup.get(&link) else {
            // unable to find entity, not in graph, so skip
            return false;
        };

        let Some(weight) = view.graph.node_weight(*node) else {
            // in theory infallible, but we're not going to panic here
            return false;
        };

        // We do not check if the link is ignored, because even if such a link exists, the node
        // connected is still valid.
        if_.matches(view, weight)
    }

    fn eval_statement(view: &View, weight: &EntityNode, statement: &Statement) -> bool {
        if !statement.if_.matches(view, weight) {
            return false;
        }

        if !Self::eval_link(
            view,
            weight.link_data.as_ref().map(|link| link.left_entity_id),
            statement.left.as_ref(),
        ) {
            return false;
        }

        if !Self::eval_link(
            view,
            weight.link_data.as_ref().map(|link| link.right_entity_id),
            statement.right.as_ref(),
        ) {
            return false;
        }

        true
    }

    fn eval(&self, view: &View, weight: &EntityNode) -> bool {
        for statement in &self.statements {
            if Self::eval_statement(view, weight, statement) {
                return true;
            }
        }

        false
    }

    fn run(self, view: &View) -> BTreeSet<NodeIndex> {
        let ignore = &view.exclude;

        let mut selected = BTreeSet::new();

        for (index, weight) in view.graph.node_references() {
            if ignore.contains(&index) {
                continue;
            }

            if self.eval(view, weight) {
                selected.insert(index);
            }
        }

        selected
    }
}

impl View<'_> {
    pub fn select(&mut self, statements: Vec<Statement>) {
        let nodes = Select { statements }.run(self);

        self.exclude_complement(&nodes);
    }

    pub fn select_complement(&mut self, statements: Vec<Statement>) {
        let nodes = Select { statements }.run(self);

        self.exclude(&nodes);
    }
}
