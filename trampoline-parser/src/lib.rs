//! Trampoline Parser Generator
//!
//! A DSL for generating fully trampoline-based lexers and parsers.
//!
//! # Example
//!
//! ```rust
//! use trampoline_parser::Grammar;
//!
//! let grammar = Grammar::new()
//!     .lexer(|l| {
//!         l.token("PLUS", "+")
//!          .token("NUMBER", "")
//!              .start_with(|c| c.match_class("digit"))
//!              .continue_with(|c| c.match_class("digit"))
//!              .build()
//!     })
//!     .rule("expr", |r| {
//!         r.token("NUMBER")
//!     })
//!     .build();
//!
//! let code = grammar.generate();
//! ```

mod codegen;
mod ir;
mod lexer_dsl;
mod parser_dsl;

pub use codegen::*;
pub use ir::*;
pub use lexer_dsl::*;
pub use parser_dsl::*;

/// Operator associativity for Pratt parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
}

/// Main grammar builder
#[derive(Debug, Default)]
pub struct Grammar {
    pub lexer: LexerDef,
    pub rules: Vec<RuleDef>,
}

impl Grammar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Define lexer tokens
    pub fn lexer<F>(mut self, f: F) -> Self
    where
        F: FnOnce(LexerBuilder) -> LexerBuilder,
    {
        let builder = LexerBuilder::new();
        let builder = f(builder);
        self.lexer = builder.build_def();
        self
    }

    /// Define a parser rule
    pub fn rule<F>(mut self, name: &str, f: F) -> Self
    where
        F: FnOnce(&RuleBuilder) -> Combinator,
    {
        let builder = RuleBuilder::new(name);
        let combinator = f(&builder);
        self.rules.push(RuleDef {
            name: name.to_string(),
            combinator,
        });
        self
    }

    /// Finalize and validate the grammar
    pub fn build(self) -> CompiledGrammar {
        // TODO: Validate that all referenced rules exist
        // TODO: Validate that all referenced tokens exist
        CompiledGrammar {
            lexer: self.lexer,
            rules: self.rules,
        }
    }
}

/// Compiled grammar ready for code generation
#[derive(Debug)]
pub struct CompiledGrammar {
    pub lexer: LexerDef,
    pub rules: Vec<RuleDef>,
}

impl CompiledGrammar {
    /// Generate Rust source code for the parser
    pub fn generate(&self) -> String {
        CodeGenerator::new(self).generate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_grammar() {
        let grammar = Grammar::new()
            .lexer(|l| {
                l.token("PLUS", "+").token("STAR", "*").token("NUMBER", "") // pattern-based, empty literal
            })
            .rule("expr", |r| {
                r.sequence((r.token("NUMBER"), r.token("PLUS"), r.token("NUMBER")))
            })
            .build();

        assert_eq!(grammar.lexer.tokens.len(), 3);
        assert_eq!(grammar.rules.len(), 1);
        assert_eq!(grammar.rules[0].name, "expr");
    }
}
