use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
};

use clap::{Args, ValueEnum, ValueHint};
use codegen::AnyTypeRepr;
use error_stack::{IntoReport, Result, ResultExt};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use skeletor::{Config, Style};
use thiserror::Error;
use url::Url;

#[derive(ValueEnum, Copy, Clone, Debug)]
pub enum LibStyle {
    Mod,
    Module,
}

impl Display for LibStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LibStyle::Mod => f.write_str("mod"),
            LibStyle::Module => f.write_str("module"),
        }
    }
}

impl From<LibStyle> for Style {
    fn from(value: LibStyle) -> Self {
        match value {
            LibStyle::Mod => Self::Mod,
            LibStyle::Module => Self::Module,
        }
    }
}

#[derive(Args, Debug)]
#[group(required = true)]
pub struct LibOrigin {
    #[arg(long)]
    remote: Option<Url>,

    #[arg(long, value_hint = ValueHint::FilePath)]
    local: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub(crate) struct Lib {
    #[arg(value_hint = ValueHint::DirPath)]
    root: PathBuf,

    #[command(flatten)]
    origin: LibOrigin,

    #[arg(long, default_value_t = LibStyle::Mod)]
    style: LibStyle,

    #[arg(long)]
    name: Option<String>,
}

enum Origin {
    Remote(Url),
    Local(PathBuf),
}

impl From<LibOrigin> for Origin {
    fn from(value: LibOrigin) -> Self {
        match (value.remote, value.local) {
            (Some(remote), None) => Self::Remote(remote),
            (None, Some(local)) => Self::Local(local),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Error)]
pub enum Error {
    #[error("unable to join URL with path")]
    Url,
    #[error("unable to send request to remote")]
    Http,
    #[error("unable to traverse the result")]
    Traverse,
    #[error("io error")]
    Io,
    #[error("unable to deserialize into type")]
    Serde,
    #[error("unable to create new project")]
    Skeletor,
}

fn call_remote(url: Url) -> Result<Vec<AnyTypeRepr>, Error> {
    let url = url
        .join("entity-types/query")
        .into_report()
        .change_context(Error::Url)?;

    let query = json!({
      "filter": {
        "all": []
      },
      "graphResolveDepths": {
        "inheritsFrom": {
          "outgoing": 1
        },
        "constrainsValuesOn": {
          "outgoing": 128
        },
        "constrainsPropertiesOn": {
          "outgoing": 128
        },
        "constrainsLinksOn": {
          "outgoing": 1
        },
        "constrainsLinkDestinationsOn": {
          "outgoing": 1
        },
        "isOfType": {
          "outgoing": 0
        },
        "hasLeftEntity": {
          "outgoing": 0,
          "incoming": 0
        },
        "hasRightEntity": {
          "outgoing": 0,
          "incoming": 0
        }
      },
      "temporalAxes": {
        "pinned": {
          "axis": "transactionTime",
          "timestamp": null
        },
        "variable": {
          "axis": "decisionTime",
          "interval": {
            "start": null,
            "end": null
          }
        }
      }
    });

    let client = Client::new();
    let response = client
        .post(url)
        .json(&query)
        .send()
        .into_report()
        .change_context(Error::Http)?;

    // Do the same as:
    // .vertices | .[] | .[] | .inner.schema
    let response: Value = response.json().into_report().change_context(Error::Http)?;

    // TODO: propagate error?!
    let types = response["vertices"]
        .as_object()
        .expect("should conform to format")
        .values()
        .flat_map(|value| {
            value
                .as_object()
                .expect("should conform to format")
                .values()
        })
        .map(|value| value["inner"]["schema"].clone())
        .map(|value| serde_json::from_value::<AnyTypeRepr>(value).expect("should be valid type"))
        .collect();

    Ok(types)
}

pub(crate) fn execute(lib: Lib) -> Result<(), Error> {
    let origin = Origin::from(lib.origin);

    let types = match origin {
        Origin::Remote(remote) => call_remote(remote)?,
        Origin::Local(local) => {
            let types = std::fs::read_to_string(local)
                .into_report()
                .change_context(Error::Io)?;

            serde_json::from_str::<Vec<AnyTypeRepr>>(&types)
                .into_report()
                .change_context(Error::Serde)?
        }
    };

    skeletor::generate(types, Config {
        root: lib.root,
        style: lib.style.into(),
        name: lib.name,
    })
    .change_context(Error::Skeletor)
}
