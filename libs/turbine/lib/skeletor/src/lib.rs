#![feature(error_in_core)]

mod vfs;

use std::{
    collections::VecDeque,
    iter::once,
    path::{Path, PathBuf},
    process::Command,
};

use cargo::{
    core::{compiler::CompileMode, SourceId, Workspace},
    ops::{
        cargo_add::{AddOptions, DepOp},
        CompileOptions, FixOptions, NewOptions, VersionControl,
    },
    util::toml_mut::manifest::DepTable,
};
use codegen::{AnyTypeRepr, Flavor, ModuleFlavor, Override};
use error_stack::{IntoReport, IntoReportCompat, Result, ResultExt};
use onlyerror::Error;

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
}

#[allow(clippy::too_many_lines)]
fn setup(
    root: impl AsRef<Path>,
    name: Option<String>,
    force: bool,
    turbine: Dependency,
) -> Result<(PathBuf, cargo::Config), Error> {
    let root = root.as_ref();

    if force && root.exists() {
        std::fs::remove_dir_all(root)
            .into_report()
            .change_context(Error::Path)?;
    }

    std::fs::create_dir_all(root)
        .into_report()
        .change_context(Error::Path)?;
    let abs_root = std::fs::canonicalize(root)
        .into_report()
        .change_context(Error::Path)?;

    let cargo_init = NewOptions::new(
        Some(VersionControl::NoVcs),
        false,
        true,
        abs_root.clone(),
        name,
        None,
        None,
    )
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
    let (package, _) =
        cargo::ops::read_package(&abs_root.join("Cargo.toml"), source_id, &cargo_config)
            .into_report()
            .change_context(Error::Cargo)?;

    let workspace = Workspace::new(&abs_root.join("Cargo.toml"), &cargo_config)
        .into_report()
        .change_context(Error::Codegen)?;

    // add all required dependencies
    let cargo_add = AddOptions {
        config: &cargo_config,
        spec: &package,
        dependencies: vec![
            DepOp {
                crate_spec: Some("hashbrown".to_owned()),
                rename: None,
                features: Some(
                    ["ahash", "inline-more"]
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
            DepOp {
                crate_spec: Some("serde_json".to_owned()),
                rename: None,
                features: Some(once("alloc").map(ToOwned::to_owned).collect()),
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
                crate_spec: Some("turbine".to_owned()),
                rename: None,
                features: None,
                default_features: None,
                optional: Some(false),
                registry: None,
                git: match &turbine {
                    Dependency::Git { url, .. } => Some(url.clone()),
                    _ => None,
                },
                path: match &turbine {
                    Dependency::Path(path) => Some(path.to_string_lossy().to_string()),
                    _ => None,
                },
                branch: match &turbine {
                    Dependency::Git { branch, .. } => branch.clone(),
                    _ => None,
                },
                rev: match &turbine {
                    Dependency::Git { rev, .. } => rev.clone(),
                    _ => None,
                },
                tag: match turbine {
                    Dependency::Git { tag, .. } => tag,
                    _ => None,
                },
            },
        ],
        section: DepTable::default(),
        dry_run: false,
    };

    cargo::ops::cargo_add::add(&workspace, &cargo_add)
        .into_report()
        .change_context(Error::Cargo)?;

    Ok((abs_root, cargo_config))
}

pub fn generate(types: Vec<AnyTypeRepr>, config: Config) -> Result<(), Error> {
    let types = codegen::process(types, codegen::Config {
        module: Some(config.style.into()),
        overrides: config.overrides,
        flavors: config.flavors,
    })
    .change_context(Error::Codegen)?;

    let (abs_root, cargo_config) = setup(&config.root, config.name, config.force, config.turbine)?;

    let mut folder = VirtualFolder::new("src".to_owned());

    for (path, contents) in types {
        let (directories, file) = path.typed.into_parts();

        folder.insert(VecDeque::from(directories), file, contents);
    }

    folder.normalize_top_level(config.style);

    folder
        .output(config.root.join("src"))
        .into_report()
        .change_context(Error::Io)?;

    // let workspace = Workspace::new(&abs_root.join("Cargo.toml"), &cargo_config)
    //     .into_report()
    //     .change_context(Error::Codegen)?;

    // cargo::ops::fix(&workspace, &mut FixOptions {
    //     edition: true,
    //     idioms: true,
    //     compile_opts: CompileOptions::new(&cargo_config, CompileMode::Check { test: true })
    //         .into_report()
    //         .change_context(Error::Codegen)?,
    //     allow_dirty: true,
    //     allow_no_vcs: true,
    //     allow_staged: true,
    //     broken_code: false,
    // })
    // .into_report()
    // .change_context(Error::Codegen)?;

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
