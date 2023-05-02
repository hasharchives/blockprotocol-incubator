#![feature(error_in_core)]

mod vfs;

use std::path::{Path, PathBuf};

use cargo::{
    core::{Dependency, Shell, SourceId, Workspace},
    ops::{
        cargo_add::{AddOptions, DepOp},
        NewOptions,
    },
};
use codegen::AnyTypeRepr;
use error_stack::{IntoReport, IntoReportCompat, Result, ResultExt};
use onlyerror::Error;

#[derive(Debug, Copy, Clone)]
pub enum Style {
    Mod,
    Module,
}

pub struct Config {
    pub root: PathBuf,
    pub style: Style,
    pub name: Option<String>,
}

#[derive(Debug, Copy, Clone, Error)]
pub enum Error {
    #[error("unable to generate code")]
    Codegen,
    #[error("cargo error")]
    Cargo,
    #[error("path error")]
    Path,
}

fn setup(root: impl AsRef<Path>, name: Option<String>) -> Result<(), Error> {
    let root = root.as_ref();
    let abs_root = std::fs::canonicalize(root)
        .into_report()
        .change_context(Error::Path)?;

    let cargo_init = NewOptions::new(None, false, true, root.to_owned(), name, None, None)
        .into_report()
        .change_context(Error::Cargo)?;
    let cargo_config = cargo::Config::default()
        .into_report()
        .change_context(Error::Cargo)?;

    cargo::ops::init(&cargo_init, &cargo_config)
        .into_report()
        .change_context(Error::Cargo)?;

    let source_id = SourceId::for_path(&abs_root)
        .into_report()
        .change_context(Error::Cargo)?;
    let (package, _) = cargo::ops::read_package(&abs_root, source_id, &cargo_config)
        .into_report()
        .change_context(Error::Cargo)?;

    let workspace = Workspace::new(&root.join("Cargo.toml"), &cargo_config)
        .into_report()
        .change_context(Error::Codegen)?;

    // add all required dependencies
    // TODO: blockprotocol, but that is kinda, idk...?
    let cargo_add = AddOptions {
        config: &cargo_config,
        spec: &package,
        dependencies: vec![
            DepOp {
                crate_spec: Some("hashbrown".to_owned()),
                rename: None,
                features: Some(
                    ["core", "alloc", "ahash", "inline-more"]
                        .into_iter()
                        .map(ToOwned::to_owned)
                        .collect(),
                ),
                default_features: Some(false),
                optional: Some(false),
                registry: None,
                path: None,
                git: None,
                branch: None,
                rev: None,
                tag: None,
            },
            DepOp {
                crate_spec: Some("error-stack".to_owned()),
                rename: None,
                features: None,
                default_features: Some(false),
                optional: Some(false),
                registry: None,
                path: None,
                git: None,
                branch: None,
                rev: None,
                tag: None,
            },
            DepOp {
                crate_spec: Some("serde".to_owned()),
                rename: None,
                features: Some(
                    ["derive", "alloc"]
                        .into_iter()
                        .map(ToOwned::to_owned)
                        .collect(),
                ),
                default_features: Some(false),
                optional: Some(false),
                registry: None,
                path: None,
                git: None,
                branch: None,
                rev: None,
                tag: None,
            },
        ],
        section: Default::default(),
        dry_run: false,
    };

    cargo::ops::cargo_add::add(&workspace, &cargo_add)
        .into_report()
        .change_context(Error::Cargo)?;

    Ok(())
}

pub fn generate(types: Vec<AnyTypeRepr>, config: Config) -> Result<(), Error> {
    let types = codegen::process(types).change_context(Error::Codegen)?;

    setup(&config.root, config.name)?;

    // TODO: generate the intermediate `mod.rs` and `module.rs` files, put all files onto the fs
    // TODO: rustfmt

    todo!()
}

#[test]
fn compile() {}
