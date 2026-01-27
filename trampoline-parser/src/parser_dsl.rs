//! Parser DSL for defining grammar rules (scannerless parsing)
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
//!     });
//! ```

use crate::ir::{CharClass, Combinator, InfixOp, PostfixOp, PrattDef, PrefixOp, TernaryOp};
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

    /// Reference another rule by name
    pub fn parse(&self, rule_name: &str) -> Combinator {
        Combinator::Rule(rule_name.to_string())
    }

    // === Character-level primitives (scannerless parsing) ===

    /// Match a literal string exactly (e.g., "if", "===", "+")
    pub fn lit(&self, s: &str) -> Combinator {
        Combinator::Literal(s.to_string())
    }

    /// Match a single specific character
    pub fn char(&self, c: char) -> Combinator {
        Combinator::Char(c)
    }

    /// Match any decimal digit (0-9)
    pub fn digit(&self) -> Combinator {
        Combinator::CharClass(CharClass::Digit)
    }

    /// Match any hexadecimal digit (0-9, a-f, A-F)
    pub fn hex_digit(&self) -> Combinator {
        Combinator::CharClass(CharClass::HexDigit)
    }

    /// Match any alphabetic character (a-z, A-Z)
    pub fn alpha(&self) -> Combinator {
        Combinator::CharClass(CharClass::Alpha)
    }

    /// Match any alphanumeric character (a-z, A-Z, 0-9)
    pub fn alpha_num(&self) -> Combinator {
        Combinator::CharClass(CharClass::AlphaNumeric)
    }

    /// Match any whitespace character (space, tab, newline, etc.)
    pub fn ws(&self) -> Combinator {
        Combinator::CharClass(CharClass::Whitespace)
    }

    /// Match identifier start character (a-z, A-Z, _, $)
    pub fn ident_start(&self) -> Combinator {
        Combinator::CharClass(CharClass::IdentStart)
    }

    /// Match identifier continue character (a-z, A-Z, 0-9, _, $)
    pub fn ident_cont(&self) -> Combinator {
        Combinator::CharClass(CharClass::IdentCont)
    }

    /// Match any single character
    pub fn any_char(&self) -> Combinator {
        Combinator::AnyChar
    }

    /// Match a character in the given range (inclusive)
    pub fn range(&self, from: char, to: char) -> Combinator {
        Combinator::CharRange(from, to)
    }

    /// Negative lookahead: succeed if inner does NOT match, consume nothing
    pub fn not_followed_by(&self, inner: Combinator) -> Combinator {
        Combinator::NotFollowedBy(Box::new(inner))
    }

    /// Positive lookahead: succeed if inner matches, consume nothing
    pub fn followed_by(&self, inner: Combinator) -> Combinator {
        Combinator::FollowedBy(Box::new(inner))
    }

    /// Capture the matched text as a string token
    pub fn capture(&self, inner: Combinator) -> Combinator {
        Combinator::Capture(Box::new(inner))
    }

    /// Memoize the result of parsing to avoid exponential backtracking.
    ///
    /// When a memoized combinator is tried at a position, the result is cached.
    /// If the same combinator is tried again at the same position (due to
    /// backtracking), the cached result is returned instead of re-parsing.
    ///
    /// Use this for rules that:
    /// 1. Appear in multiple Choice alternatives
    /// 2. Contain recursion
    /// 3. Are frequently backtracked
    ///
    /// The `id` parameter must be unique across all memoization points.
    pub fn memoize(&self, id: usize, inner: Combinator) -> Combinator {
        Combinator::Memoize {
            id,
            inner: Box::new(inner),
        }
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

    /// Define a prefix operator with a pattern
    /// Example: `ops.prefix("-", 16, "|e| unary(e, Neg)")`
    /// Example: `ops.prefix(r.sequence((r.lit("-"), r.not_followed_by(r.lit("-")))), 16, "...")`
    pub fn prefix(mut self, pattern: impl Into<Combinator>, precedence: u8, mapping: &str) -> Self {
        self.prefix_ops.push(PrefixOp {
            pattern: Box::new(pattern.into()),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a prefix operator for a keyword (ensures not followed by identifier char)
    /// Example: `ops.prefix_kw("typeof", 16, "|e| unary(e, Typeof)")`
    pub fn prefix_kw(mut self, keyword: &str, precedence: u8, mapping: &str) -> Self {
        self.prefix_ops.push(PrefixOp {
            pattern: Box::new(Combinator::Sequence(vec![
                Combinator::Literal(keyword.to_string()),
                Combinator::NotFollowedBy(Box::new(Combinator::CharClass(CharClass::IdentCont))),
            ])),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define an infix operator with a pattern
    /// Example: `ops.infix("+", 13, Assoc::Left, "|l, r| binary(l, r, Add)")`
    /// Example: `ops.infix(r.sequence((r.lit("-"), r.not_followed_by(r.lit("-")))), 9, Left, "...")`
    pub fn infix(
        mut self,
        pattern: impl Into<Combinator>,
        precedence: u8,
        assoc: Assoc,
        mapping: &str,
    ) -> Self {
        self.infix_ops.push(InfixOp {
            pattern: Box::new(pattern.into()),
            precedence,
            assoc,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define an infix operator for a keyword (ensures not followed by identifier char)
    /// Example: `ops.infix_kw("in", 11, Assoc::Left, "|l, r| binary(l, r, In)")`
    pub fn infix_kw(mut self, keyword: &str, precedence: u8, assoc: Assoc, mapping: &str) -> Self {
        self.infix_ops.push(InfixOp {
            pattern: Box::new(Combinator::Sequence(vec![
                Combinator::Literal(keyword.to_string()),
                Combinator::NotFollowedBy(Box::new(Combinator::CharClass(CharClass::IdentCont))),
            ])),
            precedence,
            assoc,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a simple postfix operator with a pattern (++, --)
    /// Example: `ops.postfix("++", 17, "|e| update(e, Increment, false)")`
    pub fn postfix(
        mut self,
        pattern: impl Into<Combinator>,
        precedence: u8,
        mapping: &str,
    ) -> Self {
        self.postfix_ops.push(PostfixOp::Simple {
            pattern: Box::new(pattern.into()),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a call expression postfix: callee(args)
    /// Example: `ops.postfix_call("(", ")", ",", 18, "|callee, args| call(callee, args)")`
    pub fn postfix_call(
        mut self,
        open: &str,
        close: &str,
        separator: &str,
        precedence: u8,
        mapping: &str,
    ) -> Self {
        self.postfix_ops.push(PostfixOp::Call {
            open: Box::new(Combinator::Literal(open.to_string())),
            close: Box::new(Combinator::Literal(close.to_string())),
            separator: Box::new(Combinator::Literal(separator.to_string())),
            arg_rule: None,
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a call expression postfix with a custom argument rule: callee(args)
    /// The arg_rule is used to parse each argument (e.g., to support spread)
    /// Example: `ops.postfix_call_with_arg_rule("(", ")", ",", "call_argument", 18, "|callee, args| call(callee, args)")`
    pub fn postfix_call_with_arg_rule(
        mut self,
        open: &str,
        close: &str,
        separator: &str,
        arg_rule: &str,
        precedence: u8,
        mapping: &str,
    ) -> Self {
        self.postfix_ops.push(PostfixOp::Call {
            open: Box::new(Combinator::Literal(open.to_string())),
            close: Box::new(Combinator::Literal(close.to_string())),
            separator: Box::new(Combinator::Literal(separator.to_string())),
            arg_rule: Some(arg_rule.to_string()),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define an index expression postfix: obj[index]
    /// Example: `ops.postfix_index("[", "]", 18, "|obj, prop| member_computed(obj, prop)")`
    pub fn postfix_index(mut self, open: &str, close: &str, precedence: u8, mapping: &str) -> Self {
        self.postfix_ops.push(PostfixOp::Index {
            open: Box::new(Combinator::Literal(open.to_string())),
            close: Box::new(Combinator::Literal(close.to_string())),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a member access postfix: obj.prop
    /// Example: `ops.postfix_member(".", 18, "|obj, prop| member(obj, prop)")`
    pub fn postfix_member(mut self, literal: &str, precedence: u8, mapping: &str) -> Self {
        self.postfix_ops.push(PostfixOp::Member {
            pattern: Box::new(Combinator::Literal(literal.to_string())),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a member access postfix with a custom pattern
    /// Use this when you need not_followed_by constraints
    /// Example: `ops.postfix_member_pattern(r.sequence((r.lit("."), r.not_followed_by(r.char('.')))), 18, "|obj, prop| member(obj, prop)")`
    pub fn postfix_member_pattern(
        mut self,
        pattern: Combinator,
        precedence: u8,
        mapping: &str,
    ) -> Self {
        self.postfix_ops.push(PostfixOp::Member {
            pattern: Box::new(pattern),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a rule-based postfix: parses another rule as the suffix
    /// Used for tagged template literals: tag`template`
    /// Example: `ops.postfix_rule("template_literal", 18, "|tag, template| tagged_template(tag, template)")`
    pub fn postfix_rule(mut self, rule_name: &str, precedence: u8, mapping: &str) -> Self {
        self.postfix_ops.push(PostfixOp::Rule {
            rule_name: rule_name.to_string(),
            precedence,
            mapping: mapping.to_string(),
        });
        self
    }

    /// Define a ternary operator: cond ? then : else
    /// Example: `ops.ternary("?", ":", 3, "|c, t, f| conditional(c, t, f)")`
    pub fn ternary(mut self, first: &str, second: &str, precedence: u8, mapping: &str) -> Self {
        self.ternary = Some(TernaryOp {
            first: Box::new(Combinator::Literal(first.to_string())),
            second: Box::new(Combinator::Literal(second.to_string())),
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

        let seq = builder.sequence((builder.lit("a"), builder.lit("b")));
        assert!(matches!(seq, Combinator::Sequence(_)));

        let choice = builder.choice((builder.lit("a"), builder.lit("b")));
        assert!(matches!(choice, Combinator::Choice(_)));
    }

    #[test]
    fn test_char_level_primitives() {
        let builder = RuleBuilder::new("test");

        // Test literal
        assert!(matches!(builder.lit("hello"), Combinator::Literal(_)));

        // Test char
        assert!(matches!(builder.char('x'), Combinator::Char('x')));

        // Test character classes
        assert!(matches!(
            builder.digit(),
            Combinator::CharClass(CharClass::Digit)
        ));
        assert!(matches!(
            builder.alpha(),
            Combinator::CharClass(CharClass::Alpha)
        ));
        assert!(matches!(
            builder.ident_start(),
            Combinator::CharClass(CharClass::IdentStart)
        ));

        // Test range
        assert!(matches!(
            builder.range('a', 'z'),
            Combinator::CharRange('a', 'z')
        ));

        // Test any_char
        assert!(matches!(builder.any_char(), Combinator::AnyChar));

        // Test capture
        assert!(matches!(
            builder.capture(builder.digit()),
            Combinator::Capture(_)
        ));

        // Test lookahead
        assert!(matches!(
            builder.not_followed_by(builder.digit()),
            Combinator::NotFollowedBy(_)
        ));
    }

    #[test]
    fn test_pratt_builder() {
        let builder = RuleBuilder::new("expr");

        let pratt = builder.pratt(builder.parse("primary"), |ops| {
            ops.prefix("-", 10, "|e| Expr::Neg(e)")
                .infix("+", 5, Assoc::Left, "|l, r| Expr::Add(l, r)")
                .postfix("++", 15, "|e| Expr::PostInc(e)")
        });

        assert!(matches!(pratt, Combinator::Pratt(_)));
    }

    #[test]
    fn test_ast_mapping() {
        let builder = RuleBuilder::new("test");

        let mapped = builder
            .sequence((builder.lit("a"), builder.lit("b")))
            .ast("|(a, b)| Node { a, b }");

        assert!(matches!(mapped, Combinator::Mapped { .. }));
    }

    #[test]
    fn test_memoize() {
        let builder = RuleBuilder::new("test");

        let memoized = builder.memoize(0, builder.parse("expensive_rule"));

        assert!(matches!(memoized, Combinator::Memoize { id: 0, .. }));
    }
}
