use std::collections::HashMap;

use type_system::url::VersionedUrl;

use crate::{analysis::DependencyAnalyzer, AnyType};

#[derive(Debug, Copy, Clone)]
pub(crate) enum ModuleFlavor {
    ModRs,
    ModuleRs,
}

#[derive(Debug, Clone)]
pub(crate) struct Name {
    name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Location {
    path: Vec<String>,
    name: Name,

    alias: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PropertyName {
    name: String,
}

// TODO: I don't like the name
pub(crate) struct NameResolver<'a> {
    lookup: &'a HashMap<VersionedUrl, AnyType>,
    analyzer: &'a DependencyAnalyzer<'a>,

    overrides: HashMap<String, String>,
    flavor: ModuleFlavor,
}

impl<'a> NameResolver<'a> {
    pub(crate) fn new(
        lookup: &'a HashMap<VersionedUrl, AnyType>,
        analyzer: &'a DependencyAnalyzer<'a>,
    ) -> Self {
        Self {
            lookup,
            analyzer,

            overrides: HashMap::new(),
            flavor: ModuleFlavor::ModRs,
        }
    }

    pub(crate) fn with_override(
        &mut self,
        prefix: impl Into<String>,
        replace_with: impl Into<String>,
    ) {
        self.overrides.insert(prefix.into(), replace_with.into());
    }

    pub(crate) fn with_flavor(&mut self, flavor: ModuleFlavor) {
        self.flavor = flavor;
    }

    /// Return the module location for the structure or enum for the specified URL
    ///
    /// We need to resolve the name and if there are multiple versions we need to make sure that
    /// those are in the correct file! (`mod.rs` vs `module.rs`)
    pub(crate) fn location(id: &VersionedUrl) -> Location {
        todo!()
    }

    /// Same as [`Self::location`], but is aware of name clashes and will resolve those properly
    pub(crate) fn locations<'b>(ids: &[&'b VersionedUrl]) -> HashMap<&'b VersionedUrl, Location> {
        todo!()
    }

    /// Return the name of the structure or enum for the specified URL, if there are multiple
    /// versions, later versions will have `V<n>` appended to their name
    pub(crate) fn name(id: &VersionedUrl) -> Name {
        todo!()
    }

    // TODO: we need to generate the code for `mod` also!

    // TODO: inner (cannot by done by the name resolver)

    /// Returns the name for the accessor or property for the specified URL
    pub(crate) fn property_name(id: &VersionedUrl) -> PropertyName {
        todo!()
    }

    /// Same as [`Self::property_name`], but is aware of name clashes and will resolve those
    pub(crate) fn property_names<'b>(
        id: &[&'b VersionedUrl],
    ) -> HashMap<&'b VersionedUrl, PropertyName> {
        todo!()
    }
}
