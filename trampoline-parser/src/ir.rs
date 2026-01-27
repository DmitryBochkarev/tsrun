//! Intermediate Representation for the grammar
//!
//! These types represent the grammar after DSL processing but before code generation.
//! This is a scannerless (lexerless) parser - no separate tokenization phase.

use crate::Assoc;

/// A parser rule definition
#[derive(Debug, Clone)]
pub struct RuleDef {
    pub name: String,
    pub combinator: Combinator,
}

/// Parser combinators for scannerless parsing
#[derive(Debug, Clone)]
pub enum Combinator {
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

    // === Character-level primitives ===
    /// Match a literal string exactly (e.g., "if", "===", "+")
    Literal(String),
    /// Match a single character
    Char(char),
    /// Match a character class (digit, alpha, etc.)
    CharClass(CharClass),
    /// Match a character range (e.g., 'a'..='z')
    CharRange(char, char),
    /// Match any single character
    AnyChar,
    /// Negative lookahead (match if inner does NOT match, consume nothing)
    NotFollowedBy(Box<Combinator>),
    /// Positive lookahead (match if inner matches, consume nothing)
    FollowedBy(Box<Combinator>),
    /// Capture the matched text as a string
    Capture(Box<Combinator>),
    /// Memoize the result of parsing at each position to avoid exponential backtracking
    Memoize {
        /// Unique identifier for this memoization point
        id: usize,
        /// The inner combinator to memoize
        inner: Box<Combinator>,
    },
}

/// Allow string literals to be used as Combinator::Literal
impl From<&str> for Combinator {
    fn from(s: &str) -> Self {
        Combinator::Literal(s.to_string())
    }
}

/// Built-in character classes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    /// Decimal digit: 0-9
    Digit,
    /// Hexadecimal digit: 0-9, a-f, A-F
    HexDigit,
    /// Alphabetic: a-z, A-Z
    Alpha,
    /// Alphanumeric: a-z, A-Z, 0-9
    AlphaNumeric,
    /// Whitespace: space, tab, newline, carriage return
    Whitespace,
    /// Identifier start: a-z, A-Z, _, $
    IdentStart,
    /// Identifier continue: a-z, A-Z, 0-9, _, $
    IdentCont,
}

impl CharClass {
    /// Check if a character matches this class
    pub fn matches(self, c: char) -> bool {
        match self {
            CharClass::Digit => c.is_ascii_digit(),
            CharClass::HexDigit => c.is_ascii_hexdigit(),
            CharClass::Alpha => c.is_ascii_alphabetic(),
            CharClass::AlphaNumeric => c.is_ascii_alphanumeric(),
            // ECMAScript whitespace: space, tab, vertical tab, form feed, NBSP, BOM, line terminators
            CharClass::Whitespace => {
                matches!(
                    c,
                    ' ' | '\t'
                        | '\x0B'
                        | '\x0C'
                        | '\r'
                        | '\n'
                        | '\u{00A0}'
                        | '\u{FEFF}'
                        | '\u{2028}'
                        | '\u{2029}'
                ) || c.is_whitespace()
            }
            CharClass::IdentStart => c.is_ascii_alphabetic() || c == '_' || c == '$',
            CharClass::IdentCont => c.is_ascii_alphanumeric() || c == '_' || c == '$',
        }
    }
}

/// Pratt parsing definition for expression parsing
#[derive(Debug, Clone, Default)]
/// Pratt expression parser definition.
///
/// IMPORTANT: The parser generator does NOT handle whitespace automatically.
/// All whitespace handling must be done explicitly in the grammar DSL.
/// This includes whitespace between operators and operands.
///
/// For expressions with postfix operators followed by infix operators (e.g., "a.x * b"),
/// the grammar must ensure whitespace is consumed. Common patterns:
/// 1. Have the operand rule consume surrounding whitespace
/// 2. Use pattern-based operators that include whitespace in their patterns
/// 3. Structure the grammar so postfix chains are parsed as complete units
///
/// DO NOT add automatic/hardcoded whitespace handling to the parser generator.
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
    /// The operator pattern (e.g., Literal("!"), Literal("++"))
    pub pattern: Box<Combinator>,
    pub precedence: u8,
    pub mapping: String,
}

/// Infix operator definition
#[derive(Debug, Clone)]
pub struct InfixOp {
    /// The operator pattern (e.g., Literal("+"), Literal("==="))
    pub pattern: Box<Combinator>,
    pub precedence: u8,
    pub assoc: Assoc,
    pub mapping: String,
}

/// Postfix operator definition
#[derive(Debug, Clone)]
pub enum PostfixOp {
    /// Simple postfix (++, --)
    Simple {
        /// The operator pattern (e.g., Literal("++"))
        pattern: Box<Combinator>,
        precedence: u8,
        mapping: String,
    },
    /// Call expression: callee(args)
    Call {
        /// Open delimiter (e.g., Literal("("))
        open: Box<Combinator>,
        /// Close delimiter (e.g., Literal(")"))
        close: Box<Combinator>,
        /// Argument separator (e.g., Literal(","))
        separator: Box<Combinator>,
        /// Optional rule name for parsing arguments (if None, uses ParseOperand)
        arg_rule: Option<String>,
        precedence: u8,
        mapping: String,
    },
    /// Index expression: obj[index]
    Index {
        /// Open delimiter (e.g., Literal("["))
        open: Box<Combinator>,
        /// Close delimiter (e.g., Literal("]"))
        close: Box<Combinator>,
        precedence: u8,
        mapping: String,
    },
    /// Member access: obj.prop
    Member {
        /// The dot/accessor pattern (e.g., Literal("."), Literal("?."))
        pattern: Box<Combinator>,
        precedence: u8,
        mapping: String,
    },
    /// Rule-based postfix: parses another rule as the suffix
    /// Used for tagged template literals: tag`template`
    Rule {
        /// The name of the rule to parse
        rule_name: String,
        precedence: u8,
        mapping: String,
    },
}

/// Ternary operator definition
#[derive(Debug, Clone)]
pub struct TernaryOp {
    /// First operator (e.g., Literal("?"))
    pub first: Box<Combinator>,
    /// Second operator (e.g., Literal(":"))
    pub second: Box<Combinator>,
    pub precedence: u8,
    pub mapping: String,
}

/// Configuration for AST integration
#[derive(Debug, Clone)]
pub struct AstConfig {
    /// External modules to import (e.g., "crate::ast::*")
    pub imports: Vec<String>,
    /// Return type of the parse() function
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
    pub apply_mappings: bool,
    /// String dictionary type for string interning (e.g., "StringDict")
    pub string_dict_type: Option<String>,
    /// Method to call on string dict to intern a string (default: "get_or_insert")
    pub string_dict_method: Option<String>,
    /// Helper functions to include in generated code
    pub helper_code: Vec<String>,
    /// Custom ParseResult variants for typed AST nodes
    pub result_variants: Vec<ResultVariant>,
}

/// A custom ParseResult variant for typed AST nodes
#[derive(Debug, Clone)]
pub struct ResultVariant {
    /// Name of the variant (e.g., "Expr")
    pub name: String,
    /// Rust type it holds (e.g., "Expression")
    pub rust_type: String,
    /// Expression to get span, where _ is the value (e.g., "_.span")
    pub span_expr: Option<String>,
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
            helper_code: Vec::new(),
            result_variants: Vec::new(),
        }
    }
}

impl AstConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Indexed IR types for state-indexed code generation
// ============================================================================

/// ID types for indexing combinators in generated code.
/// Using u16 to keep Work enum variants compact while supporting up to 65535 combinators.
pub type RuleId = u16;
pub type SeqId = u16;
pub type ChoiceId = u16;
pub type LoopId = u16;
pub type OptId = u16;
pub type CapId = u16;
pub type LookId = u16;
pub type SkipId = u16;
pub type SepById = u16;
pub type PrattId = u16;
pub type MapId = u16;
pub type MemoId = u16;
pub type LitId = u16;

/// Reference to a combinator by its type and index.
/// Used in generated static tables to reference child combinators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombRef {
    /// Reference to a rule
    Rule(RuleId),
    /// Reference to a sequence
    Seq(SeqId),
    /// Reference to a choice
    Choice(ChoiceId),
    /// Reference to zero-or-more loop
    ZeroOrMore(LoopId),
    /// Reference to one-or-more loop
    OneOrMore(LoopId),
    /// Reference to optional
    Optional(OptId),
    /// Reference to a literal
    Literal(LitId),
    /// Match a character class
    CharClass(CharClass),
    /// Match a character range
    CharRange(char, char),
    /// Match a single specific character
    Char(char),
    /// Match any character
    AnyChar,
    /// Reference to a capture
    Capture(CapId),
    /// Reference to not-followed-by lookahead
    NotFollowedBy(LookId),
    /// Reference to followed-by lookahead
    FollowedBy(LookId),
    /// Reference to a skip
    Skip(SkipId),
    /// Reference to separated-by
    SeparatedBy(SepById),
    /// Reference to Pratt expression parser
    Pratt(PrattId),
    /// Reference to a mapped combinator
    Mapped(MapId),
    /// Reference to a memoized combinator
    Memoize(MemoId),
}

/// Compiled sequence definition for generated code
#[derive(Debug, Clone)]
pub struct CompiledSeqDef {
    /// References to child combinators
    pub items: Vec<CombRef>,
}

/// Compiled choice definition for generated code
#[derive(Debug, Clone)]
pub struct CompiledChoiceDef {
    /// References to alternative combinators
    pub alts: Vec<CombRef>,
}

/// Compiled loop definition for generated code (used by ZeroOrMore and OneOrMore)
#[derive(Debug, Clone)]
pub struct CompiledLoopDef {
    /// Reference to the item combinator
    pub item: CombRef,
}

/// Compiled optional definition for generated code
#[derive(Debug, Clone)]
pub struct CompiledOptDef {
    /// Reference to the inner combinator
    pub inner: CombRef,
}

/// Compiled capture definition for generated code
#[derive(Debug, Clone)]
pub struct CompiledCapDef {
    /// Reference to the inner combinator
    pub inner: CombRef,
}

/// Compiled lookahead definition (for NotFollowedBy and FollowedBy)
#[derive(Debug, Clone)]
pub struct CompiledLookDef {
    /// Reference to the inner combinator
    pub inner: CombRef,
}

/// Compiled skip definition
#[derive(Debug, Clone)]
pub struct CompiledSkipDef {
    /// Reference to the inner combinator
    pub inner: CombRef,
}

/// Compiled separated-by definition
#[derive(Debug, Clone)]
pub struct CompiledSepByDef {
    /// Reference to the item combinator
    pub item: CombRef,
    /// Reference to the separator combinator
    pub separator: CombRef,
    /// Whether trailing separator is allowed
    pub trailing: bool,
}

/// Compiled mapped combinator definition
#[derive(Debug, Clone)]
pub struct CompiledMapDef {
    /// Reference to the inner combinator
    pub inner: CombRef,
    /// Index into the mapping function array
    pub mapping_idx: usize,
}

/// Compiled memoize definition
#[derive(Debug, Clone)]
pub struct CompiledMemoDef {
    /// The memoization ID (same as Combinator::Memoize::id)
    pub memo_id: usize,
    /// Reference to the inner combinator
    pub inner: CombRef,
}

/// Compiled rule definition
#[derive(Debug, Clone)]
pub struct CompiledRuleDef {
    /// Rule name
    pub name: String,
    /// Reference to the rule's top-level combinator
    pub entry: CombRef,
}

/// Information extracted from an operator pattern
#[derive(Debug, Clone)]
pub struct PatternInfo {
    /// The literal operator string (e.g., "+", "===")
    pub literal: String,
    /// Whether this is a keyword (needs word boundary check)
    pub is_keyword: bool,
    /// Strings that must NOT follow the operator (e.g., "=" for "=" to not match "==")
    pub not_followed_by: Vec<String>,
    /// Optional leading rule to parse (e.g., whitespace)
    pub leading_rule: Option<String>,
}

/// Compiled prefix operator
#[derive(Debug, Clone)]
pub struct CompiledPrefixOp {
    /// Pattern information
    pub pattern: PatternInfo,
    /// Precedence level
    pub precedence: u8,
    /// Index into prefix mapping function array
    pub mapping_idx: usize,
}

/// Compiled infix operator
#[derive(Debug, Clone)]
pub struct CompiledInfixOp {
    /// Pattern information
    pub pattern: PatternInfo,
    /// Precedence level
    pub precedence: u8,
    /// Associativity
    pub assoc: Assoc,
    /// Index into infix mapping function array
    pub mapping_idx: usize,
}

/// Compiled postfix operator
#[derive(Debug, Clone)]
pub enum CompiledPostfixOp {
    /// Simple postfix (++, --)
    Simple {
        pattern: PatternInfo,
        precedence: u8,
        mapping_idx: usize,
    },
    /// Call expression: callee(args)
    Call {
        open_lit: String,
        close_lit: String,
        sep_lit: String,
        /// Optional rule to parse for arguments
        arg_rule: Option<RuleId>,
        precedence: u8,
        mapping_idx: usize,
    },
    /// Index expression: obj[index]
    Index {
        open_lit: String,
        close_lit: String,
        precedence: u8,
        mapping_idx: usize,
    },
    /// Member access: obj.prop
    Member {
        pattern: PatternInfo,
        precedence: u8,
        mapping_idx: usize,
    },
    /// Rule-based postfix
    Rule {
        rule_id: RuleId,
        precedence: u8,
        mapping_idx: usize,
    },
}

/// Compiled ternary operator
#[derive(Debug, Clone)]
pub struct CompiledTernaryOp {
    /// First operator literal (e.g., "?")
    pub first_lit: String,
    /// Second operator literal (e.g., ":")
    pub second_lit: String,
    /// Precedence level
    pub precedence: u8,
    /// Index into ternary mapping function array
    pub mapping_idx: usize,
}

/// Compiled Pratt expression parser definition
#[derive(Debug, Clone)]
pub struct CompiledPrattDef {
    /// Reference to the operand combinator
    pub operand: Option<CombRef>,
    /// Compiled prefix operators
    pub prefix_ops: Vec<CompiledPrefixOp>,
    /// Compiled infix operators
    pub infix_ops: Vec<CompiledInfixOp>,
    /// Compiled postfix operators
    pub postfix_ops: Vec<CompiledPostfixOp>,
    /// Compiled ternary operator (if any)
    pub ternary: Option<CompiledTernaryOp>,
    /// Whether any infix operator has a leading rule
    pub has_infix_with_leading: bool,
    /// Whether any prefix operator has a leading rule
    pub has_prefix_with_leading: bool,
}

/// Compact position info for Pratt parsing work items
#[derive(Debug, Clone, Copy, Default)]
pub struct PosInfo {
    pub start_pos: usize,
    pub start_line: u32,
    pub start_column: u32,
}

/// Postfix variant tag for work items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostfixVariant {
    Simple,
    Call,
    Index,
    Member,
    Rule,
}

/// Ternary stage for work items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TernaryStage {
    First,
    Second,
}

/// Leading rule context for Pratt work items
#[derive(Debug, Clone, Copy)]
pub enum LeadingRuleContext {
    Prefix,
    Infix {
        op_idx: u8,
        next_prec: u8,
        checkpoint: usize,
    },
}

/// Index of all combinators in the grammar
#[derive(Debug, Clone, Default)]
pub struct CombinatorIndex {
    /// Compiled rules
    pub rules: Vec<CompiledRuleDef>,
    /// Map from rule name to rule ID
    pub rule_map: std::collections::HashMap<String, RuleId>,
    /// Compiled sequences
    pub sequences: Vec<CompiledSeqDef>,
    /// Compiled choices
    pub choices: Vec<CompiledChoiceDef>,
    /// Compiled zero-or-more loops
    pub zero_or_more: Vec<CompiledLoopDef>,
    /// Compiled one-or-more loops
    pub one_or_more: Vec<CompiledLoopDef>,
    /// Compiled optionals
    pub optionals: Vec<CompiledOptDef>,
    /// Compiled captures
    pub captures: Vec<CompiledCapDef>,
    /// Compiled not-followed-by lookaheads
    pub not_followed_by: Vec<CompiledLookDef>,
    /// Compiled followed-by lookaheads
    pub followed_by: Vec<CompiledLookDef>,
    /// Compiled skips
    pub skips: Vec<CompiledSkipDef>,
    /// Compiled separated-by
    pub separated_by: Vec<CompiledSepByDef>,
    /// Compiled Pratt parsers
    pub pratts: Vec<CompiledPrattDef>,
    /// Compiled mapped combinators
    pub mapped: Vec<CompiledMapDef>,
    /// Compiled memoized combinators
    pub memoized: Vec<CompiledMemoDef>,
    /// Unique literals
    pub literals: Vec<String>,
    /// Map from literal string to literal ID
    pub literal_map: std::collections::HashMap<String, LitId>,
    /// Mapping function strings (code snippets)
    pub mappings: Vec<String>,
}
