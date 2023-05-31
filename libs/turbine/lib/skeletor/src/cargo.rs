use std::fmt::{Display, Formatter};

use askama::Template;
use error_stack::{IntoReport, Report, Result, ResultExt};
use url::Url;

use crate::{Config, Dependency, Error};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Index {
    name: String,
    version: String,
}

fn fetch_version(name: &str) -> Result<String, Error> {
    let url = Url::parse("https://index.crates.io/").expect("failed to parse url");

    #[allow(clippy::string_slice)]
    let url = match name.len() {
        1 => url.join(&format!("1/{name}")).expect("failed to join url"),
        2 => url.join(&format!("2/{name}")).expect("failed to join url"),
        3 => url
            .join(&format!("3/{}/{}", &name[..1], name))
            .expect("failed to join url"),
        _ => url
            .join(&format!("{}/{}/{}", &name[..2], &name[2..4], name))
            .expect("failed to join url"),
    };

    let response = reqwest::blocking::get(url)
        .into_report()
        .change_context(Error::Http)?;

    let text = response.text().into_report().change_context(Error::Http)?;

    let latest = text
        .lines()
        .last()
        .ok_or_else(|| Report::new(Error::Http))?;

    let index: Index = serde_json::from_str(latest)
        .into_report()
        .change_context(Error::Serde)?;

    Ok(index.version)
}

#[derive(Debug, Clone)]
pub enum TurbineVersion {
    Path(String),
    Git {
        url: String,
        rev: Option<String>,
        branch: Option<String>,
        tag: Option<String>,
    },
}

// Reason: it is only fallible in a case which we don't support just yet.
#[allow(clippy::fallible_impl_from)]
impl From<Dependency> for TurbineVersion {
    fn from(value: Dependency) -> Self {
        match value {
            Dependency::Path(path) => Self::Path(path.to_string_lossy().into_owned()),
            Dependency::Git {
                url,
                rev,
                branch,
                tag,
            } => Self::Git {
                url,
                rev,
                branch,
                tag,
            },
            Dependency::CratesIo => {
                panic!("turbine crate not yet published to crates.io");
            }
        }
    }
}

impl Display for TurbineVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(path) => f.write_fmt(format_args!(r##"{{ path = "{path}" }}"##)),
            Self::Git {
                url,
                rev,
                branch,
                tag,
            } => {
                let mut spec = format!(r##"{{ git = "{url}" "##);

                if let Some(rev) = rev {
                    spec.push_str(&format!(r##", rev = "{rev}" "##));
                }

                if let Some(branch) = branch {
                    spec.push_str(&format!(r##", branch = "{branch}" "##));
                }

                if let Some(tag) = tag {
                    spec.push_str(&format!(r##", tag = "{tag}" "##));
                }

                spec.push_str(" }");

                f.write_str(&spec)
            }
        }
    }
}

#[derive(Debug)]
struct Versions {
    error_stack: String,
    hashbrown: String,
    serde: String,
    serde_json: String,
    turbine: TurbineVersion,
}

impl Versions {
    fn latest(config: &Config) -> Result<Self, Error> {
        let error_stack = fetch_version("error-stack")?;
        let hashbrown = fetch_version("hashbrown")?;
        let serde = fetch_version("serde")?;
        let serde_json = fetch_version("serde_json")?;
        let turbine = TurbineVersion::from(config.turbine.clone());

        Ok(Self {
            error_stack,
            hashbrown,
            serde,
            serde_json,
            turbine,
        })
    }
}

#[derive(Debug, Template)]
#[template(path = "Cargo.toml.askama", escape = "none")]
struct CargoTemplate {
    name: String,
    versions: Versions,
}

#[cfg(test)]
mod tests {}
