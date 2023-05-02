use std::path::Path;

use codegen::AnyTypeRepr;
use error_stack::{Result, ResultExt};
use onlyerror::Error;

pub enum Style {
    Mod,
    Module,
}

#[derive(Debug, Copy, Clone, Error)]
pub enum Error {
    #[error("unable to generate code")]
    Codegen,
}

pub fn generate(root: impl AsRef<Path>, types: Vec<AnyTypeRepr>) -> Result<(), Error> {
    let root = root.as_ref();

    let types = codegen::process(types).change_context(Error::Codegen)?;

    todo!()
}
