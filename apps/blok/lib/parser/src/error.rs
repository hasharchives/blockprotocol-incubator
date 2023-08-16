use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error")]
    ParseError,
}
