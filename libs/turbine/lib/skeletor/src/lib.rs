#![feature(error_in_core)]

mod cargo;
mod vfs;

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    process::Command,
};

use codegen::{AnyTypeRepr, Flavor, ModuleFlavor, Output, Override};
use error_stack::{IntoReport, Result, ResultExt};
use onlyerror::Error;
use pathdiff::diff_paths;

use crate::vfs::VirtualFolder;

#[derive(Debug, Clone)]
pub enum Dependency {
    Path(PathBuf),
    Git {
        url: String,
        rev: Option<String>,
        branch: Option<String>,
        tag: Option<String>,
    },
    CratesIo,
}

impl Dependency {
    pub(crate) fn make_relative_to(&mut self, parent: &Path) {
        if let Self::Path(path) = self {
            let cwd = std::env::current_dir().expect("unable to get current directory");

            let canon = cwd
                .join(&*path)
                .canonicalize()
                .expect("unable to canonicalize path");

            *path = diff_paths(canon, parent).expect("unable to diff paths");
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Style {
    Mod,
    Module,
}

impl From<Style> for ModuleFlavor {
    fn from(value: Style) -> Self {
        match value {
            Style::Mod => Self::ModRs,
            Style::Module => Self::ModuleRs,
        }
    }
}

pub struct Config {
    pub root: PathBuf,
    pub style: Style,
    pub name: Option<String>,

    pub overrides: Vec<Override>,
    pub flavors: Vec<Flavor>,
    pub force: bool,

    pub turbine: Dependency,
}

impl Config {
    fn resolve(&mut self) {
        self.root
            .canonicalize()
            .expect("unable to canonicalize root");
    }
}

#[derive(Debug, Copy, Clone, Error)]
pub enum Error {
    #[error("unable to generate code")]
    Codegen,
    #[error("cargo error")]
    Cargo,
    #[error("path error")]
    Path,
    #[error("io error")]
    Io,
    #[error("format error")]
    Format,
    #[error("http error")]
    Http,
    #[error("serde error")]
    Serde,
    #[error("unable to determine crate name")]
    Name,
    #[error("template error")]
    Template,
    #[error("directory is non-empty, and creation was not forced")]
    Exists,
}

pub fn generate(types: Vec<AnyTypeRepr>, mut config: Config) -> Result<(), Error> {
    cargo::init(&mut config)?;

    let Output {
        files: types,
        utilities,
    } = codegen::process(types, codegen::Config {
        module: Some(config.style.into()),
        overrides: config.overrides,
        flavors: config.flavors,
    })
    .change_context(Error::Codegen)?;

    let mut folder = VirtualFolder::new("src".to_owned());

    for (path, contents) in types {
        let (directories, file) = path.typed.into_parts();

        folder.insert(VecDeque::from(directories), file, contents);
    }

    folder.normalize_top_level(config.style, &utilities);

    folder
        .output(config.root.join("src"))
        .into_report()
        .change_context(Error::Io)?;

    let mut child = Command::new("cargo-fmt")
        .arg("--all")
        .arg("--")
        .arg("--config")
        .arg("normalize_doc_attributes=true")
        .current_dir(&config.root)
        .spawn()
        .into_report()
        .change_context(Error::Format)?;

    child.wait().into_report().change_context(Error::Format)?;

    Ok(())
}
