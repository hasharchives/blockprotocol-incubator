use std::collections::HashSet;

use type_system::url::VersionedUrl;

pub(crate) struct Facts {
    pub(crate) links: HashSet<VersionedUrl>,
}

impl Facts {
    pub fn new() -> Self {
        Self {
            links: HashSet::new(),
        }
    }
}
