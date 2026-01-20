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

/// Builder for AST configuration
///
/// Provides a fluent API for configuring how the generated parser
/// integrates with external AST types.
#[derive(Debug, Default)]
pub struct AstConfigBuilder {
    config: AstConfig,
}

impl AstConfigBuilder {
    /// Create a new AstConfigBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an import statement (e.g., "crate::ast::*")
    pub fn import(mut self, import_path: &str) -> Self {
        self.config.imports.push(import_path.to_string());
        self
    }

    /// Set the return type of the parse() function
    pub fn result_type(mut self, result_type: &str) -> Self {
        self.config.result_type = Some(result_type.to_string());
        self
    }

    /// Set the external span type (disables internal Span generation)
    pub fn span_type(mut self, span_type: &str) -> Self {
        self.config.span_type = Some(span_type.to_string());
        self.config.generate_span = false;
        self
    }

    /// Set the external string type (e.g., "JsString")
    pub fn string_type(mut self, string_type: &str) -> Self {
        self.config.string_type = Some(string_type.to_string());
        self
    }

    /// Set the external error type (disables internal ParseError generation)
    pub fn error_type(mut self, error_type: &str) -> Self {
        self.config.error_type = Some(error_type.to_string());
        self.config.generate_parse_error = false;
        self
    }

    /// Disable ParseResult enum generation
    pub fn no_parse_result(mut self) -> Self {
        self.config.generate_parse_result = false;
        self
    }

    /// Enable AST mapping application
    ///
    /// When enabled, mapping closures defined with `.ast()` are embedded
    /// in the generated code and called inline during parsing.
    pub fn apply_mappings(mut self) -> Self {
        self.config.apply_mappings = true;
        self
    }

    /// Enable string dictionary integration for string interning
    ///
    /// When enabled:
    /// - Parser struct accepts `&'a mut StringDict` parameter
    /// - Token text is created via `string_dict.get_or_insert(slice)`
    /// - Requires `string_type` to be set (e.g., "JsString")
    ///
    /// # Example
    /// ```ignore
    /// .ast_config(|c| c
    ///     .import("crate::string_dict::StringDict")
    ///     .import("crate::value::JsString")
    ///     .string_type("JsString")
    ///     .string_dict("StringDict")
    /// )
    /// ```
    pub fn string_dict(mut self, dict_type: &str) -> Self {
        self.config.string_dict_type = Some(dict_type.to_string());
        self
    }

    /// Set custom method name for string interning (default: "get_or_insert")
    pub fn string_dict_method(mut self, method: &str) -> Self {
        self.config.string_dict_method = Some(method.to_string());
        self
    }

    /// Build the final AstConfig
    pub fn build(self) -> AstConfig {
        self.config
    }
}

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
    pub ast_config: AstConfig,
}

impl Grammar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure AST integration settings
    ///
    /// # Example
    /// ```ignore
    /// let grammar = Grammar::new()
    ///     .ast_config(|c| c
    ///         .import("crate::ast::*")
    ///         .import("crate::lexer::Span")
    ///         .span_type("Span")
    ///         .string_type("JsString")
    ///         .error_type("JsError")
    ///     )
    ///     .lexer(...)
    ///     .rule(...)
    ///     .build();
    /// ```
    pub fn ast_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(AstConfigBuilder) -> AstConfigBuilder,
    {
        let builder = AstConfigBuilder::new();
        let builder = f(builder);
        self.ast_config = builder.build();
        self
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
            ast_config: self.ast_config,
        }
    }
}

/// Compiled grammar ready for code generation
#[derive(Debug)]
pub struct CompiledGrammar {
    pub lexer: LexerDef,
    pub rules: Vec<RuleDef>,
    pub ast_config: AstConfig,
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
