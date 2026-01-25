//! Trampoline Parser Generator
//!
//! A DSL for generating fully trampoline-based scannerless parsers.
//!
//! # Example
//!
//! ```rust
//! use trampoline_parser::Grammar;
//!
//! let grammar = Grammar::new()
//!     .rule("number", |r| {
//!         r.capture(r.one_or_more(r.digit()))
//!     })
//!     .rule("expr", |r| {
//!         r.sequence((
//!             r.parse("number"),
//!             r.lit("+"),
//!             r.parse("number"),
//!         ))
//!     })
//!     .build();
//!
//! let code = grammar.generate();
//! ```

mod codegen;
pub mod grammars;
mod ir;
mod parser_dsl;
pub mod prefix_factoring;
mod validation;

pub use codegen::*;
pub use ir::*;
pub use parser_dsl::*;
pub use prefix_factoring::{
    identify_memoization_candidates, BacktrackingSeverity, BacktrackingWarning,
};
pub use validation::{validate_grammar, ValidationError};

/// Builder for AST configuration
#[derive(Debug, Default)]
pub struct AstConfigBuilder {
    config: AstConfig,
}

impl AstConfigBuilder {
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
    pub fn apply_mappings(mut self) -> Self {
        self.config.apply_mappings = true;
        self
    }

    /// Enable string dictionary integration for string interning
    pub fn string_dict(mut self, dict_type: &str) -> Self {
        self.config.string_dict_type = Some(dict_type.to_string());
        self
    }

    /// Set custom method name for string interning (default: "get_or_insert")
    pub fn string_dict_method(mut self, method: &str) -> Self {
        self.config.string_dict_method = Some(method.to_string());
        self
    }

    /// Add helper code (functions, constants) to be included in the generated parser
    pub fn helper(mut self, code: &str) -> Self {
        self.config.helper_code.push(code.to_string());
        self
    }

    /// Add a custom ParseResult variant for typed AST nodes
    pub fn result_variant(mut self, name: &str, rust_type: &str) -> Self {
        self.config.result_variants.push(ir::ResultVariant {
            name: name.to_string(),
            rust_type: rust_type.to_string(),
            span_expr: None,
        });
        self
    }

    /// Add a custom ParseResult variant with custom span extraction
    pub fn result_variant_with_span(
        mut self,
        name: &str,
        rust_type: &str,
        span_expr: &str,
    ) -> Self {
        self.config.result_variants.push(ir::ResultVariant {
            name: name.to_string(),
            rust_type: rust_type.to_string(),
            span_expr: Some(span_expr.to_string()),
        });
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
    pub rules: Vec<RuleDef>,
    pub ast_config: AstConfig,
}

impl Grammar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure AST integration settings
    pub fn ast_config<F>(mut self, f: F) -> Self
    where
        F: FnOnce(AstConfigBuilder) -> AstConfigBuilder,
    {
        let builder = AstConfigBuilder::new();
        let builder = f(builder);
        self.ast_config = builder.build();
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
        let errors = validation::validate_grammar(&self.rules);
        for error in &errors {
            eprintln!("Grammar warning: {}", error);
        }

        CompiledGrammar {
            rules: self.rules,
            ast_config: self.ast_config,
        }
    }

    /// Build with automatic backtracking optimization
    ///
    /// This is equivalent to calling `.build().optimize_backtracking()`.
    /// It detects Choice nodes with shared prefixes containing recursive rules
    /// and rewrites them to factor out the common prefix.
    pub fn build_optimized(self) -> CompiledGrammar {
        self.build().optimize_backtracking()
    }

    /// Build with automatic memoization to avoid exponential backtracking.
    ///
    /// This analyzes the grammar to identify rules that would benefit from
    /// memoization and automatically wraps them. Use this when you have
    /// patterns that cause exponential backtracking (like TypeScript's
    /// `identifier<types>(args)` vs comparison operators).
    ///
    /// The process:
    /// 1. Analyze the grammar to find Choice nodes with shared recursive prefixes
    /// 2. Identify rule references in those prefixes
    /// 3. Wrap those rules with memoization
    /// 4. Build the grammar
    pub fn build_with_memoization(mut self) -> CompiledGrammar {
        let candidates = prefix_factoring::identify_memoization_candidates(&self.rules);

        if !candidates.is_empty() {
            // Assign unique memo IDs to each candidate
            let mut memo_id = 0;

            // Wrap candidate rules with memoization
            for rule in &mut self.rules {
                if candidates.contains(&rule.name) {
                    rule.combinator = Combinator::Memoize {
                        id: memo_id,
                        inner: Box::new(rule.combinator.clone()),
                    };
                    memo_id += 1;
                }
            }
        }

        self.build()
    }

    /// Build with both prefix factoring optimization and automatic memoization.
    ///
    /// This combines the benefits of both optimization strategies:
    /// 1. Prefix factoring rewrites Choice nodes to factor out common prefixes
    /// 2. Memoization caches results for rules that cause exponential backtracking
    ///
    /// Use this for the most comprehensive backtracking prevention.
    pub fn build_optimized_with_memoization(mut self) -> CompiledGrammar {
        let candidates = prefix_factoring::identify_memoization_candidates(&self.rules);

        if !candidates.is_empty() {
            let mut memo_id = 0;
            for rule in &mut self.rules {
                if candidates.contains(&rule.name) {
                    rule.combinator = Combinator::Memoize {
                        id: memo_id,
                        inner: Box::new(rule.combinator.clone()),
                    };
                    memo_id += 1;
                }
            }
        }

        self.build().optimize_backtracking()
    }
}

/// Compiled grammar ready for code generation
#[derive(Debug)]
pub struct CompiledGrammar {
    pub rules: Vec<RuleDef>,
    pub ast_config: AstConfig,
}

impl CompiledGrammar {
    /// Generate Rust source code for the parser
    pub fn generate(&self) -> String {
        CodeGenerator::new(self).generate()
    }

    /// Analyze the grammar for backtracking issues.
    ///
    /// Returns warnings for each Choice node that has alternatives sharing
    /// a common prefix containing recursive rules (causing O(2^n) parsing).
    pub fn analyze_backtracking(&self) -> Vec<BacktrackingWarning> {
        prefix_factoring::analyze_grammar(&self.rules)
    }

    /// Optimize the grammar by factoring out common prefixes in Choice nodes.
    ///
    /// This transforms patterns like:
    /// ```text
    /// Choice([Seq([A, B, C]), Seq([A, B, D])])
    /// ```
    /// Into:
    /// ```text
    /// Seq([A, B, Choice([C, D])])
    /// ```
    ///
    /// Only transforms choices with `Exponential` severity (shared prefix
    /// containing recursive rules).
    pub fn optimize_backtracking(self) -> Self {
        use std::collections::HashMap;

        // Build rule map from references before consuming self
        let rule_map: HashMap<&str, &Combinator> = self
            .rules
            .iter()
            .map(|r| (r.name.as_str(), &r.combinator))
            .collect();

        // Optimize each rule's combinator
        let optimized_rules: Vec<RuleDef> = self
            .rules
            .iter()
            .map(|rule| RuleDef {
                name: rule.name.clone(),
                combinator: prefix_factoring::optimize_combinator(&rule.combinator, &rule_map),
            })
            .collect();

        CompiledGrammar {
            rules: optimized_rules,
            ast_config: self.ast_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_grammar() {
        let grammar = Grammar::new()
            .rule("number", |r| r.capture(r.one_or_more(r.digit())))
            .rule("expr", |r| {
                r.sequence((r.parse("number"), r.lit("+"), r.parse("number")))
            })
            .build();

        assert_eq!(grammar.rules.len(), 2);
        assert_eq!(grammar.rules[0].name, "number");
        assert_eq!(grammar.rules[1].name, "expr");
    }
}
