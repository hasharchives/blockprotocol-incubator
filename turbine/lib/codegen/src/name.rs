use std::collections::{BTreeMap, HashMap, HashSet};

use once_cell::sync::Lazy;
use regex::Regex;
use type_system::url::VersionedUrl;

use crate::{analysis::DependencyAnalyzer, AnyType};

#[derive(Debug, Copy, Clone)]
pub(crate) enum ModuleFlavor {
    ModRs,
    ModuleRs,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Directory(String);

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct File(String);

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Path(Vec<Directory>, File);

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Name {
    name: String,
    alias: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct Location {
    path: Path,
    name: Name,

    alias: Option<String>,
}

// TODO: what if we create regex masks for this sort of thing with replacements in overrides?
//  like a blockprotocol mask, hash mask, custom mask, to extract the type, with a default mask that
//  simply calls heck
//  custom simply chooses a flat name with heck

// BP: https://blockprotocol.org/@blockprotocol/types/data-type/text/v/1
// HASH: http://localhost:3000/@alice/types/property-type/cbrsUuid/v/1
// I'VE LIVED A LIE FOR MONTHS

/// Pattern matching mode
///
/// We only match path and host/protocol, everything else is stripped
#[derive(Debug, Copy, Clone)]
pub(crate) enum Mode {
    MatchPath,
    MatchAll,
}

impl Mode {
    /// Verification step that panics as this will lead to corruption either way
    ///
    /// Will verify that all named groups required by the [`NameResolver`] are present depending on
    /// the name.
    ///
    /// ## Panics
    ///
    /// If the regex pattern is incomplete or does not have the required capture groups
    fn verify_pattern(self, regex: &Regex) {
        match self {
            Self::MatchPath => {
                // we do not check for extra groups, as they might be used, this is mostly just to
                // encourage future checks
                let mut optional: HashSet<_> = std::iter::once("namespace").collect();
                let mut required: HashSet<_> = ["kind", "id"].into_iter().collect();

                for name in regex.capture_names().flatten() {
                    required.remove(name);
                    optional.remove(name);
                }

                assert!(
                    required.is_empty(),
                    "match path pattern requires `kind` and `id` named groups"
                );
            }
            Self::MatchAll => {
                let mut optional: HashSet<_> = std::iter::once("namespace").collect();
                let mut required: HashSet<_> = ["host", "link", "id"].into_iter().collect();

                for name in regex.capture_names().flatten() {
                    required.remove(name);
                    optional.remove(name);
                }

                assert!(
                    required.is_empty(),
                    "match all pattern requires `host`, `kind` and `id` named groups"
                );
            }
        }
    }
}

pub(crate) struct Flavor {
    name: &'static str,
    mode: Mode,
    pattern: Regex,
}

impl Flavor {
    pub(crate) fn new(name: &'static str, mode: Mode, pattern: Regex) -> Self {
        mode.verify_pattern(&pattern);

        Self {
            name,
            mode,
            pattern,
        }
    }
}

static BLOCKPROTOCOL_FLAVOR: Lazy<Flavor> = Lazy::new(|| {
    let pattern = Regex::new(
        r"/@(?P<namespace>[\w-]+)/types/(?P<kind>data|property|entity)-type/(?P<id>[\w\-_%]+)/",
    )
    .expect("valid pattern");

    Flavor::new("block-protocol", Mode::MatchPath, pattern)
});

static BUILTIN_FLAVORS: &[&Lazy<Flavor>] = &[&BLOCKPROTOCOL_FLAVOR];

enum Kind {
    Property,
    Data,
    Entity,
}

struct SegmentedUrl<'a> {
    host: &'a str,
    namespace: Option<&'a str>,
    kind: Kind,
    id: &'a str,
}

pub(crate) struct OverrideAction {
    replace: String,
    with: String,
}

impl OverrideAction {
    pub(crate) fn new(replace: impl Into<String>, with: impl Into<String>) -> Self {
        Self {
            replace: replace.into(),
            with: with.into(),
        }
    }
}

pub(crate) struct Override {
    host: Option<OverrideAction>,
}

impl Override {
    pub(crate) const fn new() -> Self {
        Self { host: None }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn with_host(mut self, host: OverrideAction) -> Self {
        self.host = Some(host);

        self
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PropertyName {
    name: String,
}

pub(crate) struct NameResolver<'a> {
    lookup: &'a HashMap<VersionedUrl, AnyType>,
    analyzer: &'a DependencyAnalyzer<'a>,

    overrides: Vec<Override>,
    module: ModuleFlavor,
    flavors: Vec<Flavor>,
}

impl<'a> NameResolver<'a> {
    pub(crate) const fn new(
        lookup: &'a HashMap<VersionedUrl, AnyType>,
        analyzer: &'a DependencyAnalyzer<'a>,
    ) -> Self {
        Self {
            lookup,
            analyzer,

            overrides: Vec::new(),
            module: ModuleFlavor::ModRs,
            flavors: Vec::new(),
        }
    }

    pub(crate) fn with_override(&mut self, value: Override) {
        self.overrides.push(value);
    }

    pub(crate) fn with_module_flavor(&mut self, flavor: ModuleFlavor) {
        self.module = flavor;
    }

    pub(crate) fn with_flavor(&mut self, flavor: Flavor) {
        self.flavors.push(flavor);
    }

    /// Return the module location for the structure or enum for the specified URL
    ///
    /// We need to resolve the name and if there are multiple versions we need to make sure that
    /// those are in the correct file! (`mod.rs` vs `module.rs`)
    pub(crate) fn location(&self, id: &VersionedUrl) -> Location {
        let versions: BTreeMap<_, _> = self
            .lookup
            .iter()
            .filter(|(key, _)| key.base_url == id.base_url)
            .filter(|(key, _)| **key != *id)
            .map(|(key, value)| (key.version, value))
            .collect();

        let url = id.base_url.to_url().as_str();
        // example::entities::number::v1 <- I want this!
        // do we need to classify by type? This sounds super dodgy :/
        // what about subdomains?

        if versions.is_empty() {}

        todo!()
    }

    // TODO: pub use previous versions in mod.rs if multiple files

    /// Same as [`Self::location`], but is aware of name clashes and will resolve those properly
    pub(crate) fn locations<'b>(ids: &[&'b VersionedUrl]) -> HashMap<&'b VersionedUrl, Location> {
        todo!()
    }

    /// Return the name of the structure or enum for the specified URL, if there are multiple
    /// versions, older versions will have `V<n>` appended to their name
    // TODO: type alias for current version!
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
