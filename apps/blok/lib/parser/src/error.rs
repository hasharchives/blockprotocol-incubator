use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("parse error")]
    ParseError,
    #[error("unsupported declaration")]
    UnsupportedDeclaration,
    #[error("unsupported statement")]
    UnsupportedStatement,
    #[error("id of enum member must be an identifier")]
    EnumMemberIdMustBeIdentifier,
    #[error("enum member cannot have an initializer")]
    EnumMemberCannotHaveInitializer,
}
