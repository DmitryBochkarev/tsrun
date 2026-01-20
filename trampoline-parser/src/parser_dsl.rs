//! Parser DSL for defining grammar rules
//!
//! # Example
//!
//! ```rust
//! use trampoline_parser::{Grammar, RuleBuilder};
//!
//! let grammar = Grammar::new()
//!     .rule("expr", |r| {
//!         r.sequence((
//!             r.token("NUMBER"),
//!             r.token("PLUS"),
//!             r.token("NUMBER"),
//!         ))
//!     });
//! ```

use crate::ir::{Combinator, InfixOp, PostfixOp, PrattDef, PrefixOp, TernaryOp};
use crate::Assoc;

/// Builder for parser rules
#[derive(Debug)]
pub struct RuleBuilder {
    #[allow(dead_code)]
    name: String,
}

impl RuleBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    /// Match a token by name
    pub fn token(&self, name: &str) -> Combinator {
        Combinator::Token(name.to_string())
    }

    /// Reference another rule by name
    pub fn parse(&self, rule_name: &str) -> Combinator {
        Combinator::Rule(rule_name.to_string())
    }

    /// Sequence of combinators
    pub fn sequence<T: IntoCombinatorsVec>(&self, items: T) -> Combinator {
        Combinator::Sequence(items.into_combinators_vec())
    }

    /// Ordered choice (first match wins, auto-backtrack)
    pub fn choice<T: IntoCombinatorsVec>(&self, items: T) -> Combinator {
        Combinator::Choice(items.into_combinators_vec())
    }

    /// Zero or more
    pub fn zero_or_more(&self, inner: Combinator) -> Combinator {
        Combinator::ZeroOrMore(Box::new(inner))
    }

    /// One or more
    pub fn one_or_more(&self, inner: Combinator) -> Combinator {
        Combinator::OneOrMore(Box::new(inner))
    }

    /// Optional (zero or one)
    pub fn optional(&self, inner: Combinator) -> Combinator {
        Combinator::Optional(Box::new(inner))
    }

    /// Parse but discard result
    pub fn skip(&self, inner: Combinator) -> Combinator {
        Combinator::Skip(Box::new(inner))
    }

    /// Positive lookahead (peek without consuming)
    pub fn lookahead(&self, inner: Combinator) -> Combinator {
        Combinator::Lookahead(Box::new(inner))
    }

    /// Negative lookahead (fail if matches)
    pub fn negative_lookahead(&self, inner: Combinator) -> Combinator {
        Combinator::NegativeLookahead(Box::new(inner))
    }

    /// Separated list: item (sep item)*
    pub fn separated_by(&self, item: Combinator, separator: Combinator) -> Combinator {
        Combinator::SeparatedBy {
            item: Box::new(item),
            separator: Box::new(separator),
            trailing: false,
        }
    }

    /// Separated list with optional trailing separator
    pub fn separated_by_trailing(&self, item: Combinator, separator: Combinator) -> Combinator {
        Combinator::SeparatedBy {
            item: Box::new(item),
            separator: Box::new(separator),
            trailing: true,
        }
    }

    /// Pratt expression parsing
    pub fn pratt<F>(&self, operand: Combinator, f: F) -> Combinator
    where
        F: FnOnce(PrattBuilder) -> PrattBuilder,
    {
        let builder = PrattBuilder::new(operand);
        let builder = f(builder);
        Combinator::Pratt(builder.build())
    }
}

/// Extension trait for Combinator to add AST mapping
pub trait CombinatorExt {
    fn ast(self, mapping: &str) -> Combinator;
}

impl CombinatorExt for Combinator {
    /// Apply AST mapping to this combinator
    fn ast(self, mapping: &str) -> Combinator {
        Combinator::Mapped {
            inner: Box::new(self),
            mapping: mapping.to_string(),
        }
    }
}

/// Trait for converting tuples to Vec<Combinator>
pub trait IntoCombinatorsVec {
    fn into_combinators_vec(self) -> Vec<Combinator>;
}

// Implement for various tuple sizes
impl IntoCombinatorsVec for (Combinator,) {
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![self.0]
    }
}

impl IntoCombinatorsVec for (Combinator, Combinator) {
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![self.0, self.1]
    }
}

impl IntoCombinatorsVec for (Combinator, Combinator, Combinator) {
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![self.0, self.1, self.2]
    }
}

impl IntoCombinatorsVec for (Combinator, Combinator, Combinator, Combinator) {
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![self.0, self.1, self.2, self.3]
    }
}

impl IntoCombinatorsVec for (Combinator, Combinator, Combinator, Combinator, Combinator) {
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![self.0, self.1, self.2, self.3, self.4]
    }
}

impl IntoCombinatorsVec
    for (
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
    )
{
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![self.0, self.1, self.2, self.3, self.4, self.5]
    }
}

impl IntoCombinatorsVec
    for (
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
    )
{
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![self.0, self.1, self.2, self.3, self.4, self.5, self.6]
    }
}

impl IntoCombinatorsVec
    for (
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
    )
{
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7,
        ]
    }
}

impl IntoCombinatorsVec
    for (
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
    )
{
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7, self.8,
        ]
    }
}

impl IntoCombinatorsVec
    for (
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
    )
{
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7, self.8, self.9,
        ]
    }
}

impl IntoCombinatorsVec
    for (
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
    )
{
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7, self.8, self.9, self.10,
        ]
    }
}

impl IntoCombinatorsVec
    for (
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
        Combinator,
    )
{
    fn into_combinators_vec(self) -> Vec<Combinator> {
        vec![
            self.0, self.1, self.2, self.3, self.4, self.5, self.6, self.7, self.8, self.9,
            self.10, self.11,
        ]
    }
}

impl IntoCombinatorsVec for Vec<Combinator> {
    fn into_combinators_vec(self) -> Vec<Combinator> {
        self
    }
}

/// Builder for Pratt parsing operators
#[derive(Debug)]
pub struct PrattBuilder {
    operand: Combinator,
    prefix_ops: Vec<PrefixOp>,
    infix_ops: Vec<InfixOp>,
    postfix_ops: Vec<PostfixOp>,
    ternary: Option<TernaryOp>,
}

impl PrattBuilder {
    fn new(operand: Combinator) -> Self {
        Self {
            operand,
            prefix_ops: Vec::new(),
            infix_ops: Vec::new(),
            postfix_ops: Vec::new(),
            ternary: None,
        }
    }

    /// Define a prefix operator
    pub fn prefix(mut self, token: &str, precedence: u8, mapping: &str) -> Self {
        self.prefix_ops.push(PrefixOp {
            token: token.to_string(),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define an infix operator
    pub fn infix(mut self, token: &str, precedence: u8, assoc: Assoc, mapping: &str) -> Self {
        self.infix_ops.push(InfixOp {
            token: token.to_string(),
            precedence,
            assoc,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a simple postfix operator (++, --)
    pub fn postfix(mut self, token: &str, precedence: u8, mapping: &str) -> Self {
        self.postfix_ops.push(PostfixOp::Simple {
            token: token.to_string(),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a call expression postfix: callee(args)
    pub fn postfix_call(mut self, open: &str, close: &str, precedence: u8, mapping: &str) -> Self {
        self.postfix_ops.push(PostfixOp::Call {
            open: open.to_string(),
            close: close.to_string(),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define an index expression postfix: obj[index]
    pub fn postfix_index(mut self, open: &str, close: &str, precedence: u8, mapping: &str) -> Self {
        self.postfix_ops.push(PostfixOp::Index {
            open: open.to_string(),
            close: close.to_string(),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a member access postfix: obj.prop
    pub fn postfix_member(mut self, token: &str, precedence: u8, mapping: &str) -> Self {
        self.postfix_ops.push(PostfixOp::Member {
            token: token.to_string(),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a ternary operator: cond ? then : else
    pub fn ternary(
        mut self,
        first_token: &str,
        second_token: &str,
        precedence: u8,
        mapping: &str,
    ) -> Self {
        self.ternary = Some(TernaryOp {
            first_token: first_token.to_string(),
            second_token: second_token.to_string(),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    fn build(self) -> PrattDef {
        PrattDef {
            operand: Box::new(Some(self.operand)),
            prefix_ops: self.prefix_ops,
            infix_ops: self.infix_ops,
            postfix_ops: self.postfix_ops,
            ternary: self.ternary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_combinators() {
        let builder = RuleBuilder::new("test");

        let seq = builder.sequence((builder.token("A"), builder.token("B")));
        assert!(matches!(seq, Combinator::Sequence(_)));

        let choice = builder.choice((builder.token("A"), builder.token("B")));
        assert!(matches!(choice, Combinator::Choice(_)));
    }

    #[test]
    fn test_pratt_builder() {
        let builder = RuleBuilder::new("expr");

        let pratt = builder.pratt(builder.parse("primary"), |ops| {
            ops.prefix("MINUS", 10, "|e| Expr::Neg(e)")
                .infix("PLUS", 5, Assoc::Left, "|l, r| Expr::Add(l, r)")
                .postfix("PLUS_PLUS", 15, "|e| Expr::PostInc(e)")
        });

        assert!(matches!(pratt, Combinator::Pratt(_)));
    }

    #[test]
    fn test_ast_mapping() {
        let builder = RuleBuilder::new("test");

        let mapped = builder
            .sequence((builder.token("A"), builder.token("B")))
            .ast("|(a, b)| Node { a, b }");

        assert!(matches!(mapped, Combinator::Mapped { .. }));
    }
}
