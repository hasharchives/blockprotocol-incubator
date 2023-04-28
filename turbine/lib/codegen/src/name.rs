use std::collections::HashMap;

use type_system::url::VersionedUrl;

use crate::{analysis::DependencyAnalyzer, AnyType};

// TODO: I don't like the name
pub(crate) struct NameResolver<'a> {
    analyzer: &'a DependencyAnalyzer<'a>,
    lookup: &'a HashMap<VersionedUrl, AnyType>,

    overrides: HashMap<String, String>,
}

impl<'a> NameResolver<'a> {
    pub(crate) fn new(
        analyzer: &'a DependencyAnalyzer<'a>,
        lookup: &'a HashMap<VersionedUrl, AnyType>,
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

    /// Same as [`Self::location`], but is aware of name clashes and will resolve those properly
    pub(crate) fn locations(ids: &[&VersionedUrl]) {
        todo!()
    }

    /// Return the name of the structure or enum for the specified URL
    pub(crate) fn name(id: &VersionedUrl) {
        todo!()
    }

    // TODO: name on multiple versions, and inner (cannot by done by the name resolver)

    /// Returns the name for the accessor or property for the specified URL
    pub(crate) fn property_name(id: &VersionedUrl) {
        todo!()
    }

    /// Same as [`Self::property_name`], but is aware of name clashes and will resolve those
    pub(crate) fn property_names(id: &[&VersionedUrl]) {
        todo!()
    }
}
