use std::fmt::{Display, Formatter};

use error_stack::{IntoReport, Report, ResultExt};
use url::Url;

use crate::Error;

#[derive(Debug, Clone, serde::Serialize)]
struct Index {
    name: String,
    version: String,
}

async fn fetch_version(name: &str) -> Result<String, Error> {
    let url = Url::parse("https://index.crates.io/").expect("failed to parse url");

    let url = match name.len() {
        1 => url
            .join(&format!("1/{}", name))
            .expect("failed to join url"),
        2 => url
            .join(&format!("2/{}", name))
            .expect("failed to join url"),
        3 => url
            .join(&format!("3/{}/{}", &name[..1], name))
            .expect("failed to join url"),
        _ => url
            .join(&format!("{}/{}/{}", &name[..2], &name[2..4], name))
            .expect("failed to join url"),
    };

    let response = reqwest::get(url)
        .await
        .into_report()
        .change_context(Error::Http)?;

    let text = response
        .text()
        .await
        .into_report()
        .change_context(Error::Http)?;

    let latest = text
        .lines()
        .last()
        .ok_or_else(|| Report::new(Error::Http))?;

    let index: Index = serde_json::from_str(latest)
        .into_report()
        .change_context(Error::Serde)?;

    Ok(index.version)
}

pub enum TurbineVersion {
    Path(String),
    Git {
        url: String,
        rev: Option<String>,
        branch: Option<String>,
        tag: Option<String>,
    },
}

impl Display for TurbineVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TurbineVersion::Path(path) => f.write_fmt(format_args!(r##"{{ path = "{path}" }}"##)),
            TurbineVersion::Git { .. } => {}
        }
    }
}

struct Versions {
    error_stack: String,
    hashbrown: String,
    serde: String,
    serde_json: String,
    turbine: TurbineVersion,
}

struct Template {
    name: String,
    versions: Versions,
}
