//! Concrete syntax tree.
//!
//! This is the output of the parser, which is then used to generate the AST.
//! This is a strict subset of valid TypeScript.

use error_stack::{Report, Result};
use shared::polyfill::ResultExtend;
use swc_common::Spanned as _;
use swc_ecma_ast::{Decl, ModuleDecl, ModuleItem, Stmt, TsEnumMemberId};
use text_size::{TextRange, TextSize};

use crate::error::Error;

#[derive(Debug, Clone)]
struct Ident {
    span: Span,
    name: String,
}

impl From<&swc_ecma_ast::Ident> for Ident {
    fn from(value: &swc_ecma_ast::Ident) -> Self {
        Self {
            span: Span::from(value.span),
            name: value.sym.to_string(),
        }
    }
}

struct FileReference {
    span: Span,
    path: String,
}

#[derive(Debug, Copy, Clone)]
struct Span(TextRange);

impl From<swc_common::Span> for Span {
    fn from(span: swc_common::Span) -> Self {
        Self(TextRange::new(
            TextSize::from(span.lo.0),
            TextSize::from(span.hi.0),
        ))
    }
}

impl<T> From<&T> for Span
where
    T: swc_common::Spanned,
{
    fn from(spanned: &T) -> Self {
        Self::from(spanned.span())
    }
}

struct Import {
    from: FileReference,
    items: Vec<Ident>,
}

struct Export {
    span: Span,

    item: Ident,
}

struct Type {
    span: Span,
    ident: Ident,

    generics: Vec<Ident>,
}

enum Literal {
    String(String),
    Integer(i128),
    Float(f64),
}

struct Union {
    span: Span,

    types: Vec<TypeExpr>,
}

struct Intersection {
    span: Span,

    types: Vec<TypeExpr>,
}

struct Array {
    span: Span,

    item: Box<TypeExpr>,
}

struct RecordField {
    ident: Ident,

    expr: TypeExpr,
    optional: bool,
}

struct Record {
    span: Span,

    fields: Vec<RecordField>,
}

enum TypeExpr {
    /// Type.
    ///
    /// Example: `T` or `T<U>`
    Type(Type),

    /// Literal value
    ///
    /// Example: `"1234"` or `1234`
    Literal(Literal),

    /// The `|` operator.
    ///
    /// Example: `string | number`
    Union(Union),

    /// The `&` operator.
    ///
    /// Example: `string & number`
    Intersection(Intersection),

    /// The `[]` operator.
    ///
    /// Example: `string[]`
    Array(Array),

    /// The `{...}` operator.
    Record(Record),
}

struct TypeAlias {
    span: Span,
    ident: Type,

    expr: TypeExpr,
}

impl TypeAlias {
    fn convert(declaration: swc_ecma_ast::TsTypeAliasDecl) -> Result<Self, Error> {
        let span = Span::from(&declaration);
        todo!()
    }
}

struct InterfaceField {
    span: Span,
    ident: Ident,

    expr: TypeExpr,
    optional: bool,
}

struct Interface {
    span: Span,
    ident: Type,

    extends: Vec<Type>,

    fields: Vec<InterfaceField>,
}

impl Interface {
    fn convert(declaration: swc_ecma_ast::TsInterfaceDecl) -> Result<Self, Error> {
        let span = Span::from(&declaration);
        todo!()
    }
}

struct EnumMember {
    // we do not support any enum expressions, they are simple mappings to be used for versioning.
    span: Span,
    ident: Ident,
}

impl EnumMember {
    fn convert(member: swc_ecma_ast::TsEnumMember) -> Result<Self, Error> {
        let span = Span::from(&member);

        let TsEnumMemberId::Ident(id) = member.id else {
            // TODO: position, reason, etc
            return Err(Report::new(Error::EnumMemberIdMustBeIdentifier));
        };

        let ident = Ident::from(&id);

        if member.init.is_some() {
            // TODO: position, reason, etc
            return Err(Report::new(Error::EnumMemberCannotHaveInitializer));
        }

        Ok(Self { span, ident })
    }
}

struct Enum {
    span: Span,
    ident: Ident,

    members: Vec<EnumMember>,
}

impl Enum {
    fn convert(declaration: swc_ecma_ast::TsEnumDecl) -> Result<Self, Error> {
        let span = Span::from(&declaration);
        let ident = Ident::from(&declaration.id);

        let mut errors = Ok(());
        let mut members = Vec::new();

        for member in declaration.members {
            let member = EnumMember::convert(member);

            match member {
                Ok(member) => members.push(member),
                Err(error) => {
                    errors.extend_one(error);
                }
            }
        }

        errors.map(|_| Self {
            span,
            ident,
            members,
        })
    }
}

enum Statement {
    TypeAlias(TypeAlias),
    Interface(Interface),
    Enum(Enum),
}

impl Statement {
    fn ident(&self) -> &Ident {
        match self {
            Self::TypeAlias(def) => &def.ident.ident,
            Self::Interface(interface) => &interface.ident.ident,
            Self::Enum(enm) => &enm.ident,
        }
    }

    fn convert(declaration: Decl) -> Result<Self, Error> {
        let span = Span::from(&declaration);

        match declaration {
            Decl::Class(_) => {
                // TODO: position, reason, etc
                Err(Report::new(Error::UnsupportedDeclaration))
            }
            Decl::Fn(_) => {
                // TODO: position, reason, etc
                Err(Report::new(Error::UnsupportedDeclaration))
            }
            Decl::Var(_) => {
                // TODO: position, reason, etc
                Err(Report::new(Error::UnsupportedDeclaration))
            }
            Decl::Using(_) => {
                // TODO: position, reason, etc
                Err(Report::new(Error::UnsupportedDeclaration))
            }
            Decl::TsInterface(declaration) => {
                Interface::convert(*declaration).map(Statement::Interface)
            }
            Decl::TsTypeAlias(declaration) => {
                TypeAlias::convert(*declaration).map(Statement::TypeAlias)
            }
            Decl::TsEnum(declaration) => Enum::convert(*declaration).map(Statement::Enum),
            Decl::TsModule(_) => {
                // TODO: position, reason, etc
                Err(Report::new(Error::UnsupportedDeclaration))
            }
        }
    }
}

pub(crate) struct Module {
    imports: Vec<Import>,
    exports: Vec<Export>,

    statements: Vec<Statement>,
}

struct Demand {}

impl Demand {
    fn process(&mut self, file: &FileReference) {}
}

impl Module {
    pub(crate) fn convert(
        module: swc_ecma_ast::Module,
        demand: &mut Demand,
    ) -> Result<Self, Error> {
        let mut exports = Vec::new();
        let mut errors = Ok(());
        let mut statements = Vec::new();

        for item in module.body {
            match item {
                ModuleItem::Stmt(Stmt::Decl(declaration)) => {
                    let statement = Statement::convert(declaration);

                    match statement {
                        Ok(statement) => {
                            statements.push(statement);
                        }
                        Err(error) => {
                            errors.extend_one(error);
                        }
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(declaration)) => {
                    let span = Span::from(&declaration);

                    let statement = Statement::convert(declaration.decl);

                    match statement {
                        Ok(statement) => {
                            exports.push(Export {
                                span,
                                item: statement.ident().clone(),
                            });

                            statements.push(statement);
                        }
                        Err(error) => {
                            errors.extend_one(error);
                        }
                    }
                }
                ModuleItem::ModuleDecl(ModuleDecl::Import(declaration)) => {
                    let reference = FileReference {
                        span: Span::from(&declaration.src),
                        path: declaration.src.value.to_string(),
                    };

                    demand.process(&reference);

                    todo!("items, into own function")
                }
                ModuleItem::ModuleDecl(other) => {
                    // TODO: position, reason, etc
                    return Err(Report::new(Error::UnsupportedDeclaration));
                }
                ModuleItem::Stmt(other) => {
                    // TODO: position, reason, etc
                    return Err(Report::new(Error::UnsupportedStatement));
                }
            }
        }

        todo!()
    }
}
