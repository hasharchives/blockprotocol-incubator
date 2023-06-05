use alloc::{collections::BTreeSet, vec::Vec};

use petgraph::{graph::NodeIndex, visit::IntoNodeReferences};
use turbine::{entity::EntityId, TypeUrl, VersionedUrlRef};

use crate::{EntityNode, View};

pub struct Matches<'a> {
    ids: BTreeSet<EntityId>,
    types: BTreeSet<VersionedUrlRef<'static>>,

    inherits_from: BTreeSet<VersionedUrlRef<'a>>,
}

impl Matches<'_> {
    fn matches(&self, view: &View, node: &EntityNode) -> bool {
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
        common > 0
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

impl<'a> Matches<'a> {
    pub fn or(mut self, other: Matches<'a>) -> Self {
        self.ids.extend(other.ids);
        self.types.extend(other.types);
        self.inherits_from.extend(other.inherits_from);

        self
    }

    pub fn with_links(self) -> Statement<'a> {
        Statement::from(self)
    }
}

pub struct Statement<'a> {
    if_: Matches<'a>,

    left: Option<Matches<'a>>,
    right: Option<Matches<'a>>,
}

impl<'a> Statement<'a> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            if_: Matches::new(),
            left: None,
            right: None,
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Reason: false-positive
    pub fn with_if(mut self, if_: Matches<'a>) -> Self {
        self.if_ = if_;
        self
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Reason: false-positive
    pub fn with_left(mut self, left: Matches<'a>) -> Self {
        self.left = Some(left);
        self
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Reason: false-positive
    pub fn with_right(mut self, right: Matches<'a>) -> Self {
        self.right = Some(right);
        self
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Reason: false-positive
    pub fn or_if(mut self, if_: Matches<'a>) -> Self {
        self.if_ = self.if_.or(if_);
        self
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Reason: false-positive
    pub fn or_left(mut self, left: Matches<'a>) -> Self {
        if let Some(this_left) = self.left {
            self.left = Some(this_left.or(left));
        } else {
            self.left = Some(left);
        }
        self
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Reason: false-positive
    pub fn or_right(mut self, right: Matches<'a>) -> Self {
        if let Some(this_right) = self.right {
            self.right = Some(this_right.or(right));
        } else {
            self.right = Some(right);
        }
        self
    }
}

impl<'a> From<Matches<'a>> for Statement<'a> {
    fn from(value: Matches<'a>) -> Self {
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
    fn eval_link(view: &View, link: Option<EntityId>, if_: Option<&Matches>) -> bool {
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
