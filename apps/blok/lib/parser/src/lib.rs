use error_stack::{IntoReport, Result, ResultExt};
use swc_common::BytePos;
use swc_ecma_parser::{StringInput, Syntax, TsConfig};

use crate::error::Error;

mod cst;
mod error;

pub struct Parser {
    modules: Vec<cst::Module>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn parse(&mut self, text: &str) -> Result<(), Error> {
        let mut parser = swc_ecma_parser::Parser::new(
            Syntax::Typescript(TsConfig {
                tsx: false,
                decorators: false,
                dts: true,
                no_early_errors: false,
                disallow_ambiguous_jsx_like: false,
            }),
            StringInput::new(text, BytePos(0), BytePos(text.len() as u32)),
            None,
        );

        let module = parser
            .parse_typescript_module()
            .into_report()
            .change_context(Error::ParseError)?;

        self.modules.push(module);

        Ok(())
    }
}
