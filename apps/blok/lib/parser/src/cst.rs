//! Concrete syntax tree.
//!
//! This is the output of the parser, which is then used to generate the AST.
//! This is a strict subset of valid TypeScript.

use text_size::TextRange;

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
    ident: Positional<Ident>,

    generics: Vec<Positional<Ident>>,
}

struct RecordField {
    ident: Positional<Ident>,

    expr: Positional<TypeExpr>,
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
    Type(Positional<Type>),

    /// Literal value
    ///
    /// Example: `"1234"` or `1234`
    Literal(Literal),

    /// The `|` operator.
    ///
    /// Example: `string | number`
    Union(Vec<Positional<TypeExpr>>),

    /// The `&` operator.
    ///
    /// Example: `string & number`
    Intersection(Vec<Positional<TypeExpr>>),

    /// The `[]` operator.
    ///
    /// Example: `string[]`
    Array(Box<Positional<TypeExpr>>),

    /// The `{...}` operator.
    Record(Vec<Positional<RecordField>>),
}

struct TypeDef {
    ident: Positional<Type>,

    expr: Positional<TypeExpr>,
}

struct InterfaceField {
    ident: Positional<Ident>,

    expr: Positional<TypeExpr>,
}

struct Interface {
    ident: Positional<Type>,

    extends: Vec<Positional<Type>>,

    fields: Vec<Positional<InterfaceField>>,
}

enum Statement {
    TypeDef(Positional<TypeDef>),
    Interface(Positional<Interface>),
}

struct Positional<T> {
    position: TextRange,

    value: T,
}

struct Module {
    imports: Vec<Import>,
    exports: Vec<Export>,

    nodes: Vec<Positional<Statement>>,
}
