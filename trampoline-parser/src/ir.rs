//! Intermediate Representation for the grammar
//!
//! These types represent the grammar after DSL processing but before code generation.

/// Complete lexer definition
#[derive(Debug, Default, Clone)]
pub struct LexerDef {
    pub tokens: Vec<TokenDef>,
    pub keywords: Vec<KeywordDef>,
    pub skip_patterns: Vec<String>,
}

/// A token definition
#[derive(Debug, Clone)]
pub struct TokenDef {
    pub name: String,
    pub kind: TokenKind,
}

/// How a token is recognized
#[derive(Debug, Clone)]
pub enum TokenKind {
    /// Fixed string literal ("+", "=>", "===")
    Literal(String),
    /// Pattern-based (NUMBER, IDENTIFIER, STRING)
    Pattern(TokenPattern),
}

/// Pattern for complex tokens (state machine)
#[derive(Debug, Clone, Default)]
pub struct TokenPattern {
    pub start: Option<CharCondition>,
    pub continuation: Option<CharCondition>,
    pub until: Option<CharCondition>,
    /// Special scanning modes
    pub special: Option<SpecialScan>,
    /// Can this token be rescanned as another?
    pub rescan_as: Option<String>,
    /// Is this token significant (not skipped even if matches skip pattern)?
    pub significant: bool,
}

/// Special scanning modes for complex tokens
#[derive(Debug, Clone)]
pub enum SpecialScan {
    /// Scan string until matching quote (handles escapes)
    UntilMatchingQuote,
    /// Scan template literal head (`...${)
    TemplateHead,
    /// Scan template literal middle (}...${)
    TemplateMiddle,
    /// Scan template literal tail (}...`)
    TemplateTail,
}

/// Condition for matching characters
#[derive(Debug, Clone)]
pub enum CharCondition {
    /// Match a character class by name ("digit", "alpha", etc.)
    Class(String),
    /// Match a single character
    Char(char),
    /// Match a character range
    Range(char, char),
    /// Match any character
    Any,
    /// Match end of input
    Eof,
    /// Logical OR of conditions
    Or(Box<CharCondition>, Box<CharCondition>),
    /// Logical AND of conditions
    And(Box<CharCondition>, Box<CharCondition>),
    /// Logical NOT of condition
    Not(Box<CharCondition>),
}

impl CharCondition {
    pub fn or(self, other: CharCondition) -> CharCondition {
        CharCondition::Or(Box::new(self), Box::new(other))
    }

    pub fn and(self, other: CharCondition) -> CharCondition {
        CharCondition::And(Box::new(self), Box::new(other))
    }

    pub fn not(self) -> CharCondition {
        CharCondition::Not(Box::new(self))
    }
}

/// Keyword definition (higher priority than identifiers)
#[derive(Debug, Clone)]
pub struct KeywordDef {
    pub name: String,
    pub literal: String,
}

/// A parser rule definition
#[derive(Debug, Clone)]
pub struct RuleDef {
    pub name: String,
    pub combinator: Combinator,
}

/// Parser combinators
#[derive(Debug, Clone)]
pub enum Combinator {
    /// Match a token by name
    Token(String),
    /// Reference another rule by name
    Rule(String),
    /// Sequence of combinators
    Sequence(Vec<Combinator>),
    /// Ordered choice (first match wins, auto-backtrack)
    Choice(Vec<Combinator>),
    /// Zero or more
    ZeroOrMore(Box<Combinator>),
    /// One or more
    OneOrMore(Box<Combinator>),
    /// Optional (zero or one)
    Optional(Box<Combinator>),
    /// Parse but discard result
    Skip(Box<Combinator>),
    /// Positive lookahead (peek without consuming)
    Lookahead(Box<Combinator>),
    /// Negative lookahead (fail if matches)
    NegativeLookahead(Box<Combinator>),
    /// Separated list: item (sep item)*
    SeparatedBy {
        item: Box<Combinator>,
        separator: Box<Combinator>,
        trailing: bool,
    },
    /// Pratt expression parsing
    Pratt(PrattDef),
    /// AST mapping applied to inner combinator
    Mapped {
        inner: Box<Combinator>,
        mapping: String,
    },
}

/// Pratt parsing definition for expression parsing
#[derive(Debug, Clone, Default)]
pub struct PrattDef {
    /// The operand parser (primary expressions)
    pub operand: Box<Option<Combinator>>,
    /// Prefix operators
    pub prefix_ops: Vec<PrefixOp>,
    /// Infix operators
    pub infix_ops: Vec<InfixOp>,
    /// Postfix operators
    pub postfix_ops: Vec<PostfixOp>,
    /// Ternary operator (if any)
    pub ternary: Option<TernaryOp>,
}

/// Prefix operator definition
#[derive(Debug, Clone)]
pub struct PrefixOp {
    pub token: String,
    pub precedence: u8,
    pub mapping: String,
}

/// Infix operator definition
#[derive(Debug, Clone)]
pub struct InfixOp {
    pub token: String,
    pub precedence: u8,
    pub assoc: Assoc,
    pub mapping: String,
}

/// Postfix operator definition
#[derive(Debug, Clone)]
pub enum PostfixOp {
    /// Simple postfix (++, --)
    Simple {
        token: String,
        precedence: u8,
        mapping: String,
    },
    /// Call expression: callee(args)
    Call {
        open: String,
        close: String,
        precedence: u8,
        mapping: String,
    },
    /// Index expression: obj[index]
    Index {
        open: String,
        close: String,
        precedence: u8,
        mapping: String,
    },
    /// Member access: obj.prop
    Member {
        token: String,
        precedence: u8,
        mapping: String,
    },
}

/// Ternary operator definition
#[derive(Debug, Clone)]
pub struct TernaryOp {
    pub first_token: String,
    pub second_token: String,
    pub precedence: u8,
    pub mapping: String,
}

use crate::Assoc;

/// Configuration for AST integration
///
/// Controls how the generated parser integrates with external AST types,
/// imports, and type mappings.
#[derive(Debug, Clone)]
pub struct AstConfig {
    /// External modules to import (e.g., "crate::ast::*", "crate::lexer::Span")
    pub imports: Vec<String>,
    /// Return type of the parse() function (e.g., "Result<Program, JsError>")
    pub result_type: Option<String>,
    /// External span type to use instead of generated Span
    pub span_type: Option<String>,
    /// External string type to use instead of String (e.g., "JsString")
    pub string_type: Option<String>,
    /// External error type to use instead of generated ParseError
    pub error_type: Option<String>,
    /// Whether to generate the internal ParseResult enum (default: true)
    pub generate_parse_result: bool,
    /// Whether to generate the internal Span struct (default: true)
    pub generate_span: bool,
    /// Whether to generate the internal ParseError struct (default: true)
    pub generate_parse_error: bool,
    /// Whether to apply AST mappings during parsing (default: false)
    /// When true, mapping closures are embedded in generated code
    pub apply_mappings: bool,
    /// String dictionary type for string interning (e.g., "StringDict")
    /// When set, Parser accepts a mutable reference to this type
    pub string_dict_type: Option<String>,
    /// Method to call on string dict to intern a string (default: "get_or_insert")
    pub string_dict_method: Option<String>,
}

impl Default for AstConfig {
    fn default() -> Self {
        Self {
            imports: Vec::new(),
            result_type: None,
            span_type: None,
            string_type: None,
            error_type: None,
            generate_parse_result: true,
            generate_span: true,
            generate_parse_error: true,
            apply_mappings: false,
            string_dict_type: None,
            string_dict_method: None,
        }
    }
}

impl AstConfig {
    /// Create a new AstConfig with default values
    pub fn new() -> Self {
        Self::default()
    }
}
