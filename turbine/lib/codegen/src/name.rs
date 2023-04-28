use std::collections::HashMap;

use type_system::url::VersionedUrl;

use crate::{analysis::DependencyAnalyzer, AnyType};

// TODO: I don't like the name
pub(crate) struct Namer<'a> {
    analyzer: &'a DependencyAnalyzer<'a>,
    lookup: HashMap<VersionedUrl, AnyType>,

    overrides: HashMap<String, String>,
}

impl<'a> Namer<'a> {
    pub(crate) fn new(
        analyzer: &'a DependencyAnalyzer<'a>,
        lookup: HashMap<VersionedUrl, AnyType>,
    ) -> Self {
        Self {
            analyzer,
            lookup,

            overrides: HashMap::new(),
        }
    }

    pub(crate) fn with_override(
        &mut self,
        prefix: impl Into<String>,
        replace_with: impl Into<String>,
    ) {
        self.overrides.insert(prefix.into(), replace_with.into());
    }

    /// Return the module location for the structure or enum for the specified URL
    pub(crate) fn location(id: &VersionedUrl) {
        todo!()
    }

    /// Return the name of the structure or enum for the specified URL
    pub(crate) fn name(id: &VersionedUrl) {
        todo!()
    }

    /// Returns the name for the accessor or property for the specified URL
    pub(crate) fn property_name(id: &VersionedUrl) {
        todo!()
    }
}
