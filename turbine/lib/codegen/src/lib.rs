#![feature(result_option_inspect)]

mod analysis;

use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use error_stack::{IntoReport, Result, ResultExt};
use quote::__private::TokenStream;
use serde_json::Value;
use thiserror::Error;
use type_system::{repr, url::VersionedUrl, DataType, EntityType, PropertyType};

// what we need to do:
// 1) Configuration:
//      - URL to get entity types
//      - style of module (mod.rs vs. module.rs)
//
// 2) fetch all types
// 3) categorize into:
//      - data types (if built-in refer to those, otherwise error out)
//      - property types
//      - entity types
// 4) create modules for each type, they are designated by
//      if hash: url base (backwards) / org / entity|property / id.rs
//      if blockprotocol: bp / org / entity|property / id.rs
// 5) if there are multiple versions transform into a module, put the current one in mod, there
//      others in v1.rs etc and suffix name w/ V1
// 6) for property types inner types should be named Inner (if multiple Inner1, Inner2, etc.)
// 7) when referring to those just use crate::<URL>::...
// 8) generate the code required: 2 variants: Owned and Ref (ref is lightweight)
//      with proper accessors, id converted to snake_case,
//          if duplicate error out,
//              sort properties,
//                  then increment
//              same for import problems, just alias with the name we want
//
// internally we also need to keep track which entity is in which file
// todo: generate code, that selects Ref out of all verticies of a specific type, should not be
// generated, but generic code instead
//
// result: BTreeMap<File, String>
// where File is the module name as a Path, so it can be created by e.g. CLI
//
// Problematic: multi layered objects/properties (validating them correctly ~> needs intermediate
// types (with names (?)))
//
// If multiple versions, the latest version is named Example, while the others are called ExampleV1
//
// TODO: entities can also have link data associated with them! (important on self?)
//
// TODO: tests?
fn fetch() {}

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct File {
    pub path: String,
}

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("unable to parse type from repr")]
    Parse,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum AnyTypeRepr {
    Data(repr::DataType),
    Property(repr::PropertyType),
    Entity(repr::EntityType),
}

#[derive(Debug, Clone)]
pub enum AnyType {
    Data(DataType),
    Property(PropertyType),
    Entity(EntityType),
}

impl AnyType {
    fn id(&self) -> &VersionedUrl {
        match self {
            AnyType::Data(ty) => ty.id(),
            AnyType::Property(ty) => ty.id(),
            AnyType::Entity(ty) => ty.id(),
        }
    }
}

pub fn process(values: Vec<AnyTypeRepr>) -> Result<BTreeMap<File, TokenStream>, Error> {
    let values: Result<Vec<_>, _> = values
        .into_iter()
        .map(|any| match any {
            AnyTypeRepr::Data(data) => DataType::try_from(data)
                .into_report()
                .map(AnyType::Data)
                .change_context(Error::Parse),
            AnyTypeRepr::Property(property) => PropertyType::try_from(property)
                .into_report()
                .map(AnyType::Property)
                .change_context(Error::Parse),
            AnyTypeRepr::Entity(entity) => EntityType::try_from(entity)
                .into_report()
                .map(AnyType::Entity)
                .change_context(Error::Parse),
        })
        .collect();

    let values = values?;

    todo!()
}
