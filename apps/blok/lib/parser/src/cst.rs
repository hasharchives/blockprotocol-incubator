//! Concrete syntax tree.
//!
//! This is the output of the parser, which is then used to generate the AST.
//! This is a strict subset of valid TypeScript.

use error_stack::Report;
use text_size::TextRange;

use crate::error::Error;

struct Ident(String);

struct FileReference(String);

struct Import {
    from: FileReference,
    items: Vec<Ident>,
}

struct Export {
    items: Vec<Ident>,
}

struct Type {
    ident: Spanned<Ident>,

    generics: Vec<Spanned<Ident>>,
}

struct RecordField {
    ident: Spanned<Ident>,

    expr: Spanned<TypeExpr>,
}

enum Literal {
    String(String),
    Integer(i128),
    Float(f64),
}

enum TypeExpr {
    /// Type.
    ///
    /// Example: `T` or `T<U>`
    Type(Spanned<Type>),

    /// Literal value
    ///
    /// Example: `"1234"` or `1234`
    Literal(Literal),

    /// The `|` operator.
    ///
    /// Example: `string | number`
    Union(Vec<Spanned<TypeExpr>>),

    /// The `&` operator.
    ///
    /// Example: `string & number`
    Intersection(Vec<Spanned<TypeExpr>>),

    /// The `[]` operator.
    ///
    /// Example: `string[]`
    Array(Box<Spanned<TypeExpr>>),

    /// The `{...}` operator.
    Record(Vec<Spanned<RecordField>>),
}

struct TypeDef {
    ident: Spanned<Type>,

    expr: Spanned<TypeExpr>,
}

struct InterfaceField {
    ident: Spanned<Ident>,

    expr: Spanned<TypeExpr>,
}

struct Interface {
    ident: Spanned<Type>,

    extends: Vec<Spanned<Type>>,

    fields: Vec<Spanned<InterfaceField>>,
}

enum Statement {
    TypeDef(Spanned<TypeDef>),
    Interface(Spanned<Interface>),
}

struct Spanned<T> {
    position: TextRange,

    value: T,
}

pub(crate) struct Module {
    imports: Vec<Import>,
    exports: Vec<Export>,

    nodes: Vec<Spanned<Statement>>,
}
