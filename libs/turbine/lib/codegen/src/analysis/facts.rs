use std::collections::HashSet;

use type_system::url::VersionedUrl;

pub(crate) struct Facts {
    pub(crate) links: HashSet<VersionedUrl>,
}

impl Facts {
    pub(crate) fn new() -> Self {
        Self {
            links: HashSet::new(),
        }
    }

    pub(crate) fn links(&self) -> &HashSet<VersionedUrl> {
        &self.links
    }
}
