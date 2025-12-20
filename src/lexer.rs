//! Lexer for TypeScript source code
//!
//! Converts source text into a stream of tokens.

use std::iter::Peekable;
use std::str::CharIndices;

use crate::string_dict::StringDict;
use crate::value::JsString;

/// Source span information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, column: u32) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        }
    }
}

/// Token types for JavaScript/TypeScript
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    String(JsString),
    BigInt(String),         // BigInt literal value as string (e.g., "123" for 123n)
    RegExp(String, String), // (pattern, flags)
    True,
    False,
    Null,

    // Identifiers & Keywords
    Identifier(JsString),

    // JavaScript Keywords
    Let,
    Const,
    Var,
    Function,
    Return,
    If,
    Else,
    For,
    While,
    Do,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    Try,
    Catch,
    Finally,
    Throw,
    New,
    This,
    Super,
    Class,
    Extends,
    Static,
    Import,
    Export,
    From,
    As,
    Typeof,
    Instanceof,
    In,
    Of,
    Void,
    Delete,
    Yield,
    Await,
    Async,
    Debugger,

    // TypeScript Keywords (parsed but ignored at runtime)
    Type,
    Interface,
    Enum,
    Namespace,
    Module,
    Declare,
    Abstract,
    Readonly,
    Accessor,
    Public,
    Private,
    Protected,
    Implements,
    Keyof,
    Infer,
    Is,
    Any,
    Unknown,
    Never,
    Asserts,

    // Operators
    Plus,             // +
    Minus,            // -
    Star,             // *
    Slash,            // /
    Percent,          // %
    StarStar,         // **
    PlusPlus,         // ++
    MinusMinus,       // --
    Eq,               // =
    EqEq,             // ==
    EqEqEq,           // ===
    BangEq,           // !=
    BangEqEq,         // !==
    Lt,               // <
    LtEq,             // <=
    Gt,               // >
    GtEq,             // >=
    LtLt,             // <<
    GtGt,             // >>
    GtGtGt,           // >>>
    Amp,              // &
    AmpAmp,           // &&
    Pipe,             // |
    PipePipe,         // ||
    Caret,            // ^
    Tilde,            // ~
    Bang,             // !
    Question,         // ?
    QuestionQuestion, // ??
    QuestionDot,      // ?.

    // Assignment Operators
    PlusEq,             // +=
    MinusEq,            // -=
    StarEq,             // *=
    SlashEq,            // /=
    PercentEq,          // %=
    StarStarEq,         // **=
    AmpEq,              // &=
    PipeEq,             // |=
    CaretEq,            // ^=
    LtLtEq,             // <<=
    GtGtEq,             // >>=
    GtGtGtEq,           // >>>=
    AmpAmpEq,           // &&=
    PipePipeEq,         // ||=
    QuestionQuestionEq, // ??=

    // Punctuation
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Dot,       // .
    DotDotDot, // ...
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    Arrow,     // =>
    At,        // @
    Hash,      // #

    // Template literals
    TemplateHead(JsString),   // `...${
    TemplateMiddle(JsString), // }...${
    TemplateTail(JsString),   // }...`
    TemplateNoSub(JsString),  // `...` (no substitutions)

    // Special
    Eof,
    Newline, // For ASI
    Invalid(char),
}

/// A token with its source location
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn eof(pos: usize, line: u32, column: u32) -> Self {
        Self {
            kind: TokenKind::Eof,
            span: Span::new(pos, pos, line, column),
        }
    }
}

/// Lexer state checkpoint for backtracking
#[derive(Clone)]
pub struct LexerCheckpoint {
    current_pos: usize,
    line: u32,
    column: u32,
    start_pos: usize,
    start_line: u32,
    start_column: u32,
    saw_newline: bool,
}

/// Lexer for tokenizing TypeScript source code
pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    /// Base offset added to char_indices positions (needed when resetting chars from middle of source)
    chars_base_offset: usize,
    current_pos: usize,
    line: u32,
    column: u32,
    start_pos: usize,
    start_line: u32,
    start_column: u32,
    /// Tracks if we just saw a newline (for ASI)
    saw_newline: bool,
    /// String dictionary for interning identifiers and strings
    string_dict: &'a mut StringDict,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, string_dict: &'a mut StringDict) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            chars_base_offset: 0,
            current_pos: 0,
            line: 1,
            column: 1,
            start_pos: 0,
            start_line: 1,
            start_column: 1,
            saw_newline: false,
            string_dict,
        }
    }

    /// Get mutable reference to the string dictionary for interning
    pub fn string_dict(&mut self) -> &mut StringDict {
        self.string_dict
    }

    /// Create a checkpoint of the current lexer state for backtracking
    pub fn checkpoint(&self) -> LexerCheckpoint {
        LexerCheckpoint {
            current_pos: self.current_pos,
            line: self.line,
            column: self.column,
            start_pos: self.start_pos,
            start_line: self.start_line,
            start_column: self.start_column,
            saw_newline: self.saw_newline,
        }
    }

    /// Restore the lexer state from a checkpoint
    pub fn restore(&mut self, checkpoint: LexerCheckpoint) {
        self.current_pos = checkpoint.current_pos;
        self.line = checkpoint.line;
        self.column = checkpoint.column;
        self.start_pos = checkpoint.start_pos;
        self.start_line = checkpoint.start_line;
        self.start_column = checkpoint.start_column;
        self.saw_newline = checkpoint.saw_newline;
        // Create iterator directly from the checkpoint position (O(1) instead of O(n))
        // The base offset tracks where in the original source we started
        self.chars_base_offset = checkpoint.current_pos;
        self.chars = self
            .source
            .get(checkpoint.current_pos..)
            .unwrap_or("")
            .char_indices()
            .peekable();
    }

    /// Reset the lexer to a specific position (from a Span) to rescan as regexp.
    /// Used when parser determines that a `/` should start a regexp literal.
    pub fn rescan_as_regexp(&mut self, span: Span) -> Token {
        // Reset lexer position to the start of the span
        self.current_pos = span.start;
        self.line = span.line;
        self.column = span.column;
        self.start_pos = span.start;
        self.start_line = span.line;
        self.start_column = span.column;

        // Create iterator directly from the span position (O(1) instead of O(n))
        self.chars_base_offset = span.start;
        self.chars = self
            .source
            .get(span.start..)
            .unwrap_or("")
            .char_indices()
            .peekable();

        // Now scan as regexp
        self.scan_regexp()
    }

    /// Get the next token from the source
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        self.start_pos = self.current_pos;
        self.start_line = self.line;
        self.start_column = self.column;

        let Some((_pos, ch)) = self.advance() else {
            return Token::eof(self.current_pos, self.line, self.column);
        };

        let kind = match ch {
            // Single character tokens
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '~' => TokenKind::Tilde,
            '@' => TokenKind::At,
            '#' => TokenKind::Hash,

            // Potentially multi-character tokens
            '.' => self.scan_dot(),
            ':' => TokenKind::Colon,
            '+' => self.scan_plus(),
            '-' => self.scan_minus(),
            '*' => self.scan_star(),
            '/' => self.scan_slash(),
            '%' => self.scan_percent(),
            '=' => self.scan_equals(),
            '!' => self.scan_bang(),
            '<' => self.scan_less_than(),
            '>' => self.scan_greater_than(),
            '&' => self.scan_ampersand(),
            '|' => self.scan_pipe(),
            '^' => self.scan_caret(),
            '?' => self.scan_question(),

            // String literals
            '"' | '\'' => self.scan_string(ch),

            // Template literals
            '`' => self.scan_template_literal(),

            // Numbers
            '0'..='9' => self.scan_number(ch),

            // Identifiers and keywords
            c if is_id_start(c) => self.scan_identifier(c),

            // Invalid character
            c => TokenKind::Invalid(c),
        };

        Token::new(kind, self.make_span())
    }

    /// Check if there was a newline before the current position
    pub fn had_newline_before(&self) -> bool {
        self.saw_newline
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let result = self.chars.next();
        if let Some((pos, ch)) = result {
            // Add base offset for absolute position (needed when chars is reset from middle of source)
            self.current_pos = self.chars_base_offset + pos + ch.len_utf8();
            // ECMAScript line terminators: LF, LS (U+2028), PS (U+2029)
            if ch == '\n' || ch == '\u{2028}' || ch == '\u{2029}' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        result
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn peek_next(&self) -> Option<char> {
        let slice = self.source.get(self.current_pos..)?;
        let mut iter = slice.char_indices();
        iter.next();
        iter.next().map(|(_, ch)| ch)
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn make_span(&self) -> Span {
        Span::new(
            self.start_pos,
            self.current_pos,
            self.start_line,
            self.start_column,
        )
    }

    fn skip_whitespace_and_comments(&mut self) {
        self.saw_newline = false;

        loop {
            match self.peek() {
                // ECMAScript whitespace characters:
                // - \u0009 (tab)
                // - \u000B (vertical tab)
                // - \u000C (form feed)
                // - \u0020 (space)
                // - \u00A0 (no-break space)
                // - \uFEFF (BOM / zero-width no-break space)
                Some(' ' | '\t' | '\r' | '\u{000B}' | '\u{000C}' | '\u{00A0}' | '\u{FEFF}') => {
                    self.advance();
                }
                // ECMAScript line terminators:
                // - \u000A (LF - line feed)
                // - \u2028 (LS - line separator)
                // - \u2029 (PS - paragraph separator)
                // Note: \r (CR) is handled above as whitespace since it doesn't trigger ASI on its own
                Some('\n' | '\u{2028}' | '\u{2029}') => {
                    self.saw_newline = true;
                    self.advance();
                }
                Some('/') => {
                    let next = self.peek_next();
                    if next == Some('/') {
                        // Single-line comment
                        self.advance(); // /
                        self.advance(); // /
                        while let Some(ch) = self.peek() {
                            // ECMAScript line terminators end single-line comments
                            if ch == '\n' || ch == '\u{2028}' || ch == '\u{2029}' {
                                break;
                            }
                            self.advance();
                        }
                    } else if next == Some('*') {
                        // Multi-line comment
                        self.advance(); // /
                        self.advance(); // *
                        let mut depth = 1;
                        while depth > 0 {
                            match self.advance() {
                                Some((_, '*')) if self.peek() == Some('/') => {
                                    self.advance();
                                    depth -= 1;
                                }
                                Some((_, '/')) if self.peek() == Some('*') => {
                                    self.advance();
                                    depth += 1;
                                }
                                // ECMAScript line terminators: LF, LS (U+2028), PS (U+2029)
                                Some((_, '\n' | '\u{2028}' | '\u{2029}')) => {
                                    self.saw_newline = true;
                                }
                                Some(_) => {}
                                None => break,
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    fn scan_dot(&mut self) -> TokenKind {
        if self.peek() == Some('.') && self.peek_next() == Some('.') {
            self.advance();
            self.advance();
            TokenKind::DotDotDot
        } else if matches!(self.peek(), Some('0'..='9')) {
            // .123 style number
            self.scan_number('.')
        } else {
            TokenKind::Dot
        }
    }

    fn scan_plus(&mut self) -> TokenKind {
        if self.match_char('+') {
            TokenKind::PlusPlus
        } else if self.match_char('=') {
            TokenKind::PlusEq
        } else {
            TokenKind::Plus
        }
    }

    fn scan_minus(&mut self) -> TokenKind {
        if self.match_char('-') {
            TokenKind::MinusMinus
        } else if self.match_char('=') {
            TokenKind::MinusEq
        } else {
            TokenKind::Minus
        }
    }

    fn scan_star(&mut self) -> TokenKind {
        if self.match_char('*') {
            if self.match_char('=') {
                TokenKind::StarStarEq
            } else {
                TokenKind::StarStar
            }
        } else if self.match_char('=') {
            TokenKind::StarEq
        } else {
            TokenKind::Star
        }
    }

    fn scan_slash(&mut self) -> TokenKind {
        if self.match_char('=') {
            TokenKind::SlashEq
        } else {
            TokenKind::Slash
        }
    }

    /// Scan a regular expression literal.
    /// Called by the parser when a regex is expected (e.g., after `=`, `(`, `,`, etc.)
    /// The leading `/` should be the current position (not yet consumed).
    pub fn scan_regexp(&mut self) -> Token {
        let start_pos = self.current_pos;
        let start_line = self.line;
        let start_column = self.column;

        // Consume the opening /
        self.advance();

        let mut pattern = String::new();
        let mut in_class = false; // inside character class [...]

        loop {
            match self.advance() {
                Some((_, '/')) if !in_class => {
                    // End of pattern (only if not inside character class)
                    break;
                }
                Some((_, '[')) => {
                    in_class = true;
                    pattern.push('[');
                }
                Some((_, ']')) => {
                    in_class = false;
                    pattern.push(']');
                }
                Some((_, '\\')) => {
                    // Escape sequence - include both backslash and next char
                    pattern.push('\\');
                    if let Some((_, c)) = self.advance() {
                        pattern.push(c);
                    }
                }
                Some((_, c)) => {
                    pattern.push(c);
                }
                None => {
                    // Unterminated regex
                    break;
                }
            }
        }

        // Scan flags (g, i, m, s, u, y, d)
        let mut flags = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphabetic() {
                flags.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let span = Span::new(start_pos, self.current_pos, start_line, start_column);
        Token::new(TokenKind::RegExp(pattern, flags), span)
    }

    fn scan_percent(&mut self) -> TokenKind {
        if self.match_char('=') {
            TokenKind::PercentEq
        } else {
            TokenKind::Percent
        }
    }

    fn scan_equals(&mut self) -> TokenKind {
        if self.match_char('=') {
            if self.match_char('=') {
                TokenKind::EqEqEq
            } else {
                TokenKind::EqEq
            }
        } else if self.match_char('>') {
            TokenKind::Arrow
        } else {
            TokenKind::Eq
        }
    }

    fn scan_bang(&mut self) -> TokenKind {
        if self.match_char('=') {
            if self.match_char('=') {
                TokenKind::BangEqEq
            } else {
                TokenKind::BangEq
            }
        } else {
            TokenKind::Bang
        }
    }

    fn scan_less_than(&mut self) -> TokenKind {
        if self.match_char('<') {
            if self.match_char('=') {
                TokenKind::LtLtEq
            } else {
                TokenKind::LtLt
            }
        } else if self.match_char('=') {
            TokenKind::LtEq
        } else {
            TokenKind::Lt
        }
    }

    fn scan_greater_than(&mut self) -> TokenKind {
        if self.match_char('>') {
            if self.match_char('>') {
                if self.match_char('=') {
                    TokenKind::GtGtGtEq
                } else {
                    TokenKind::GtGtGt
                }
            } else if self.match_char('=') {
                TokenKind::GtGtEq
            } else {
                TokenKind::GtGt
            }
        } else if self.match_char('=') {
            TokenKind::GtEq
        } else {
            TokenKind::Gt
        }
    }

    fn scan_ampersand(&mut self) -> TokenKind {
        if self.match_char('&') {
            if self.match_char('=') {
                TokenKind::AmpAmpEq
            } else {
                TokenKind::AmpAmp
            }
        } else if self.match_char('=') {
            TokenKind::AmpEq
        } else {
            TokenKind::Amp
        }
    }

    fn scan_pipe(&mut self) -> TokenKind {
        if self.match_char('|') {
            if self.match_char('=') {
                TokenKind::PipePipeEq
            } else {
                TokenKind::PipePipe
            }
        } else if self.match_char('=') {
            TokenKind::PipeEq
        } else {
            TokenKind::Pipe
        }
    }

    fn scan_caret(&mut self) -> TokenKind {
        if self.match_char('=') {
            TokenKind::CaretEq
        } else {
            TokenKind::Caret
        }
    }

    fn scan_question(&mut self) -> TokenKind {
        if self.match_char('?') {
            if self.match_char('=') {
                TokenKind::QuestionQuestionEq
            } else {
                TokenKind::QuestionQuestion
            }
        } else if self.match_char('.') {
            // Don't consume if next is digit (for ?. vs ?.5)
            if matches!(self.peek(), Some('0'..='9')) {
                // Put back the dot - but we can't, so this is a quirk
                // In practice, ?. followed by digit is rare
                TokenKind::QuestionDot
            } else {
                TokenKind::QuestionDot
            }
        } else {
            TokenKind::Question
        }
    }

    fn scan_string(&mut self, quote: char) -> TokenKind {
        let mut value = String::new();

        loop {
            match self.advance() {
                Some((_, c)) if c == quote => break,
                Some((_, '\\')) => {
                    // Escape sequence
                    match self.advance() {
                        Some((_, 'n')) => value.push('\n'),
                        Some((_, 'r')) => value.push('\r'),
                        Some((_, 't')) => value.push('\t'),
                        Some((_, 'b')) => value.push('\x08'), // backspace
                        Some((_, 'f')) => value.push('\x0C'), // form feed
                        Some((_, 'v')) => value.push('\x0B'), // vertical tab
                        Some((_, '\\')) => value.push('\\'),
                        Some((_, '\'')) => value.push('\''),
                        Some((_, '"')) => value.push('"'),
                        Some((_, '0')) => {
                            // \0 is allowed only if not followed by another digit (strict mode)
                            // \01, \07, \077 etc are octal escapes which are forbidden
                            if matches!(self.peek(), Some('0'..='7')) {
                                // Octal escape - not allowed in strict mode
                                // Consume remaining octal digits and return error
                                while matches!(self.peek(), Some('0'..='7')) {
                                    self.advance();
                                }
                                return TokenKind::Invalid('\\');
                            }
                            value.push('\0');
                        }
                        Some((_, c @ '1'..='7')) => {
                            // Octal escape sequence (e.g., \7, \77, \377)
                            // Not allowed in strict mode
                            let mut octal = String::new();
                            octal.push(c);
                            while matches!(self.peek(), Some('0'..='7')) && octal.len() < 3 {
                                if let Some(ch) = self.peek() {
                                    octal.push(ch);
                                    self.advance();
                                }
                            }
                            return TokenKind::Invalid('\\');
                        }
                        Some((_, 'x')) => {
                            // Hex escape \xNN
                            if let Some(hex) = self.scan_hex_escape(2) {
                                if let Some(ch) = char::from_u32(hex) {
                                    value.push(ch);
                                }
                            }
                        }
                        Some((_, 'u')) => {
                            // Unicode escape \uNNNN or \u{N...}
                            if self.peek() == Some('{') {
                                self.advance();
                                let mut hex_str = String::new();
                                while let Some(ch) = self.peek() {
                                    if ch == '}' {
                                        self.advance();
                                        break;
                                    }
                                    if ch.is_ascii_hexdigit() {
                                        hex_str.push(ch);
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }
                                if let Ok(code) = u32::from_str_radix(&hex_str, 16) {
                                    if let Some(ch) = char::from_u32(code) {
                                        value.push(ch);
                                    }
                                }
                            } else if let Some(hex) = self.scan_hex_escape(4) {
                                if let Some(ch) = char::from_u32(hex) {
                                    value.push(ch);
                                }
                            }
                        }
                        Some((_, '\n')) => {
                            // Line continuation
                        }
                        Some((_, c)) => value.push(c),
                        None => break,
                    }
                }
                Some((_, '\n')) => {
                    // Unterminated string
                    break;
                }
                Some((_, c)) => value.push(c),
                None => break,
            }
        }

        TokenKind::String(self.string_dict.get_or_insert(&value))
    }

    fn scan_hex_escape(&mut self, count: usize) -> Option<u32> {
        let mut hex_str = String::new();
        for _ in 0..count {
            if let Some(ch) = self.peek() {
                if ch.is_ascii_hexdigit() {
                    hex_str.push(ch);
                    self.advance();
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        u32::from_str_radix(&hex_str, 16).ok()
    }

    fn scan_template_literal(&mut self) -> TokenKind {
        let mut value = String::new();

        loop {
            match self.advance() {
                Some((_, '`')) => {
                    // End of template
                    return TokenKind::TemplateNoSub(self.string_dict.get_or_insert(&value));
                }
                Some((_, '$')) if self.peek() == Some('{') => {
                    self.advance(); // consume {
                    return TokenKind::TemplateHead(self.string_dict.get_or_insert(&value));
                }
                Some((_, '\\')) => {
                    // Escape sequence (same as strings)
                    match self.advance() {
                        Some((_, 'n')) => value.push('\n'),
                        Some((_, 'r')) => value.push('\r'),
                        Some((_, 't')) => value.push('\t'),
                        Some((_, '\\')) => value.push('\\'),
                        Some((_, '`')) => value.push('`'),
                        Some((_, '$')) => value.push('$'),
                        Some((_, c)) => value.push(c),
                        None => break,
                    }
                }
                Some((_, c)) => value.push(c),
                None => break,
            }
        }

        // Unterminated template
        TokenKind::TemplateNoSub(self.string_dict.get_or_insert(&value))
    }

    /// Continue scanning a template literal after an expression
    pub fn scan_template_continuation(&mut self) -> TokenKind {
        let mut value = String::new();

        loop {
            match self.advance() {
                Some((_, '`')) => {
                    return TokenKind::TemplateTail(self.string_dict.get_or_insert(&value));
                }
                Some((_, '$')) if self.peek() == Some('{') => {
                    self.advance();
                    return TokenKind::TemplateMiddle(self.string_dict.get_or_insert(&value));
                }
                Some((_, '\\')) => match self.advance() {
                    Some((_, 'n')) => value.push('\n'),
                    Some((_, 'r')) => value.push('\r'),
                    Some((_, 't')) => value.push('\t'),
                    Some((_, '\\')) => value.push('\\'),
                    Some((_, '`')) => value.push('`'),
                    Some((_, '$')) => value.push('$'),
                    Some((_, c)) => value.push(c),
                    None => break,
                },
                Some((_, c)) => value.push(c),
                None => break,
            }
        }

        TokenKind::TemplateTail(self.string_dict.get_or_insert(&value))
    }

    /// Rescan template continuation from a given span position (the } token)
    /// This resets the lexer position to after the } and scans the template continuation
    pub fn rescan_template_continuation(&mut self, rbrace_span: Span) -> TokenKind {
        // Reset position to after the } (rbrace_span.end is already past the })
        let base_offset = rbrace_span.end;
        self.current_pos = base_offset;
        self.chars_base_offset = base_offset;
        self.line = rbrace_span.line;
        self.column = rbrace_span.column + 1;
        // Reinitialize the chars iterator from the new position
        self.chars = self
            .source
            .get(base_offset..)
            .unwrap_or("")
            .char_indices()
            .peekable();

        // Now scan the template continuation
        self.scan_template_continuation()
    }

    fn scan_number(&mut self, first: char) -> TokenKind {
        let mut num_str = String::new();

        if first == '0' {
            match self.peek() {
                Some('x' | 'X') => {
                    // Hexadecimal
                    self.advance();
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_hexdigit() || ch == '_' {
                            if ch != '_' {
                                num_str.push(ch);
                            }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    // Check for BigInt suffix
                    if self.peek() == Some('n') {
                        self.advance();
                        let value = i64::from_str_radix(&num_str, 16).unwrap_or(0);
                        return TokenKind::BigInt(value.to_string());
                    }
                    return TokenKind::Number(i64::from_str_radix(&num_str, 16).unwrap_or(0) as f64);
                }
                Some('o' | 'O') => {
                    // Octal
                    self.advance();
                    while let Some(ch) = self.peek() {
                        if ch.is_digit(8) || ch == '_' {
                            if ch != '_' {
                                num_str.push(ch);
                            }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    // Check for BigInt suffix
                    if self.peek() == Some('n') {
                        self.advance();
                        let value = i64::from_str_radix(&num_str, 8).unwrap_or(0);
                        return TokenKind::BigInt(value.to_string());
                    }
                    return TokenKind::Number(i64::from_str_radix(&num_str, 8).unwrap_or(0) as f64);
                }
                Some('b' | 'B') => {
                    // Binary
                    self.advance();
                    while let Some(ch) = self.peek() {
                        if ch == '0' || ch == '1' || ch == '_' {
                            if ch != '_' {
                                num_str.push(ch);
                            }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    // Check for BigInt suffix
                    if self.peek() == Some('n') {
                        self.advance();
                        let value = i64::from_str_radix(&num_str, 2).unwrap_or(0);
                        return TokenKind::BigInt(value.to_string());
                    }
                    return TokenKind::Number(i64::from_str_radix(&num_str, 2).unwrap_or(0) as f64);
                }
                Some('0'..='7') => {
                    // Legacy octal literal (e.g., 0777) - not allowed in strict mode
                    // Consume all the digits to give a better error
                    while matches!(self.peek(), Some('0'..='9')) {
                        self.advance();
                    }
                    return TokenKind::Invalid('0'); // Signal legacy octal error
                }
                Some('8' | '9') => {
                    // 08 or 09 - invalid in strict mode but we parse as decimal for better error
                    num_str.push(first);
                }
                _ => num_str.push(first),
            }
        } else if first != '.' {
            num_str.push(first);
        }

        // Integer part (skip if starting with decimal point)
        if first != '.' {
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() || ch == '_' {
                    if ch != '_' {
                        num_str.push(ch);
                    }
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Decimal part
        if first == '.' {
            num_str.push('.');
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() || ch == '_' {
                    if ch != '_' {
                        num_str.push(ch);
                    }
                    self.advance();
                } else {
                    break;
                }
            }
        } else if self.peek() == Some('.') {
            // Check if it's really a decimal or could be method call
            // A dot followed by a digit means decimal (e.g., 1.5)
            // A dot followed by e/E means decimal with exponent (e.g., 1.e5)
            if matches!(self.peek_next(), Some('0'..='9' | 'e' | 'E')) {
                self.advance();
                num_str.push('.');
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() || ch == '_' {
                        if ch != '_' {
                            num_str.push(ch);
                        }
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Check for BigInt suffix before decimal/exponent parts
        // BigInt cannot have decimal or exponent parts
        if self.peek() == Some('n') && !num_str.contains('.') {
            self.advance();
            return TokenKind::BigInt(num_str);
        }

        // Exponent part
        if matches!(self.peek(), Some('e' | 'E')) {
            num_str.push('e');
            self.advance();
            if matches!(self.peek(), Some('+' | '-')) {
                if let Some((_, ch)) = self.advance() {
                    num_str.push(ch);
                }
            }
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() || ch == '_' {
                    if ch != '_' {
                        num_str.push(ch);
                    }
                    self.advance();
                } else {
                    break;
                }
            }
        }

        TokenKind::Number(num_str.parse().unwrap_or(f64::NAN))
    }

    fn scan_identifier(&mut self, first: char) -> TokenKind {
        let mut name = String::new();
        name.push(first);

        while let Some(ch) = self.peek() {
            if is_id_continue(ch) {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for keywords
        match name.as_str() {
            // Literals
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,

            // JavaScript keywords
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "var" => TokenKind::Var,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "throw" => TokenKind::Throw,
            "new" => TokenKind::New,
            "this" => TokenKind::This,
            "super" => TokenKind::Super,
            "class" => TokenKind::Class,
            "extends" => TokenKind::Extends,
            "static" => TokenKind::Static,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "typeof" => TokenKind::Typeof,
            "instanceof" => TokenKind::Instanceof,
            "in" => TokenKind::In,
            "of" => TokenKind::Of,
            "void" => TokenKind::Void,
            "delete" => TokenKind::Delete,
            "yield" => TokenKind::Yield,
            "await" => TokenKind::Await,
            "async" => TokenKind::Async,
            "debugger" => TokenKind::Debugger,

            // TypeScript keywords
            "type" => TokenKind::Type,
            "interface" => TokenKind::Interface,
            "enum" => TokenKind::Enum,
            "namespace" => TokenKind::Namespace,
            "module" => TokenKind::Module,
            "declare" => TokenKind::Declare,
            "abstract" => TokenKind::Abstract,
            "readonly" => TokenKind::Readonly,
            "accessor" => TokenKind::Accessor,
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "protected" => TokenKind::Protected,
            "implements" => TokenKind::Implements,
            "keyof" => TokenKind::Keyof,
            "infer" => TokenKind::Infer,
            "is" => TokenKind::Is,
            "any" => TokenKind::Any,
            "unknown" => TokenKind::Unknown,
            "never" => TokenKind::Never,
            "asserts" => TokenKind::Asserts,

            // Identifier
            _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
        }
    }
}

/// Check if a character can start an identifier
fn is_id_start(ch: char) -> bool {
    ch == '_' || ch == '$' || unicode_xid::UnicodeXID::is_xid_start(ch)
}

/// Check if a character can continue an identifier
fn is_id_continue(ch: char) -> bool {
    ch == '_' || ch == '$' || unicode_xid::UnicodeXID::is_xid_continue(ch)
}

// FIXME: move this to separate file in tests dir
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create JsString from &str in tests
    fn s(value: &str) -> JsString {
        JsString::from(value)
    }

    fn lex(source: &str) -> Vec<TokenKind> {
        let mut dict = StringDict::new();
        let mut lexer = Lexer::new(source, &mut dict);
        let mut tokens = vec![];
        loop {
            let token = lexer.next_token();
            if token.kind == TokenKind::Eof {
                break;
            }
            tokens.push(token.kind);
        }
        tokens
    }

    #[test]
    fn test_numbers() {
        assert_eq!(lex("42"), vec![TokenKind::Number(42.0)]);
        assert_eq!(lex("3.14"), vec![TokenKind::Number(3.14)]);
        assert_eq!(lex("1e10"), vec![TokenKind::Number(1e10)]);
        assert_eq!(lex("0xff"), vec![TokenKind::Number(255.0)]);
        assert_eq!(lex("0b1010"), vec![TokenKind::Number(10.0)]);
        assert_eq!(lex("0o17"), vec![TokenKind::Number(15.0)]);
    }

    #[test]
    fn test_strings() {
        assert_eq!(
            lex(r#""hello""#),
            vec![TokenKind::String(JsString::from("hello"))]
        );
        assert_eq!(
            lex(r#"'world'"#),
            vec![TokenKind::String(JsString::from("world"))]
        );
        assert_eq!(
            lex(r#""line\nbreak""#),
            vec![TokenKind::String(JsString::from("line\nbreak"))]
        );
    }

    #[test]
    fn test_operators() {
        assert_eq!(
            lex("+ - * /"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash
            ]
        );
        assert_eq!(
            lex("=== !== "),
            vec![TokenKind::EqEqEq, TokenKind::BangEqEq]
        );
        assert_eq!(lex("&&="), vec![TokenKind::AmpAmpEq]);
        assert_eq!(lex("??="), vec![TokenKind::QuestionQuestionEq]);
    }

    #[test]
    fn test_keywords() {
        assert_eq!(
            lex("let const var"),
            vec![TokenKind::Let, TokenKind::Const, TokenKind::Var]
        );
        assert_eq!(
            lex("function return"),
            vec![TokenKind::Function, TokenKind::Return]
        );
    }

    #[test]
    fn test_identifiers() {
        assert_eq!(
            lex("foo bar_baz $test"),
            vec![
                TokenKind::Identifier(JsString::from("foo")),
                TokenKind::Identifier(JsString::from("bar_baz")),
                TokenKind::Identifier(JsString::from("$test")),
            ]
        );
    }

    #[test]
    fn test_comments() {
        assert_eq!(
            lex("1 // comment\n2"),
            vec![TokenKind::Number(1.0), TokenKind::Number(2.0)]
        );
        assert_eq!(
            lex("1 /* comment */ 2"),
            vec![TokenKind::Number(1.0), TokenKind::Number(2.0)]
        );
    }

    // Additional lexer tests for comprehensive coverage

    #[test]
    fn test_comparison_operators() {
        assert_eq!(
            lex("< > <= >= == !="),
            vec![
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::EqEq,
                TokenKind::BangEq
            ]
        );
    }

    #[test]
    fn test_logical_operators() {
        assert_eq!(
            lex("&& || !"),
            vec![TokenKind::AmpAmp, TokenKind::PipePipe, TokenKind::Bang]
        );
    }

    #[test]
    fn test_bitwise_operators() {
        assert_eq!(
            lex("& | ^ ~ << >>"),
            vec![
                TokenKind::Amp,
                TokenKind::Pipe,
                TokenKind::Caret,
                TokenKind::Tilde,
                TokenKind::LtLt,
                TokenKind::GtGt
            ]
        );
    }

    #[test]
    fn test_assignment_operators() {
        assert_eq!(
            lex("= += -= *= /= %="),
            vec![
                TokenKind::Eq,
                TokenKind::PlusEq,
                TokenKind::MinusEq,
                TokenKind::StarEq,
                TokenKind::SlashEq,
                TokenKind::PercentEq
            ]
        );
    }

    #[test]
    fn test_increment_decrement() {
        assert_eq!(
            lex("++ --"),
            vec![TokenKind::PlusPlus, TokenKind::MinusMinus]
        );
    }

    #[test]
    fn test_delimiters() {
        assert_eq!(
            lex("( ) [ ] { }"),
            vec![
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::LBrace,
                TokenKind::RBrace
            ]
        );
    }

    #[test]
    fn test_punctuation() {
        assert_eq!(
            lex(", ; : ."),
            vec![
                TokenKind::Comma,
                TokenKind::Semicolon,
                TokenKind::Colon,
                TokenKind::Dot
            ]
        );
    }

    #[test]
    fn test_arrow_and_spread() {
        assert_eq!(lex("=> ..."), vec![TokenKind::Arrow, TokenKind::DotDotDot]);
    }

    #[test]
    fn test_optional_and_nullish() {
        assert_eq!(
            lex("?. ?? ?"),
            vec![
                TokenKind::QuestionDot,
                TokenKind::QuestionQuestion,
                TokenKind::Question
            ]
        );
    }

    #[test]
    fn test_control_keywords() {
        assert_eq!(
            lex("if else switch case default"),
            vec![
                TokenKind::If,
                TokenKind::Else,
                TokenKind::Switch,
                TokenKind::Case,
                TokenKind::Default
            ]
        );
    }

    #[test]
    fn test_loop_keywords() {
        assert_eq!(
            lex("for while do break continue"),
            vec![
                TokenKind::For,
                TokenKind::While,
                TokenKind::Do,
                TokenKind::Break,
                TokenKind::Continue
            ]
        );
    }

    #[test]
    fn test_class_keywords() {
        assert_eq!(
            lex("class extends new this super"),
            vec![
                TokenKind::Class,
                TokenKind::Extends,
                TokenKind::New,
                TokenKind::This,
                TokenKind::Super
            ]
        );
    }

    #[test]
    fn test_typescript_keywords() {
        assert_eq!(
            lex("interface type implements"),
            vec![TokenKind::Interface, TokenKind::Type, TokenKind::Implements]
        );
    }

    #[test]
    fn test_error_handling_keywords() {
        assert_eq!(
            lex("try catch finally throw"),
            vec![
                TokenKind::Try,
                TokenKind::Catch,
                TokenKind::Finally,
                TokenKind::Throw
            ]
        );
    }

    #[test]
    fn test_type_keywords() {
        assert_eq!(
            lex("void null undefined"),
            vec![
                TokenKind::Void,
                TokenKind::Null,
                TokenKind::Identifier(s("undefined"))
            ]
        );
    }

    #[test]
    fn test_boolean_literals() {
        assert_eq!(lex("true false"), vec![TokenKind::True, TokenKind::False]);
    }

    #[test]
    fn test_template_literal_basic() {
        // Simple template without interpolation becomes TemplateNoSub
        assert_eq!(lex("`hello`"), vec![TokenKind::TemplateNoSub(s("hello"))]);
    }

    #[test]
    fn test_string_escape_sequences() {
        assert_eq!(
            lex(r#""hello\tworld""#),
            vec![TokenKind::String(s("hello\tworld"))]
        );
        assert_eq!(
            lex(r#""line\\break""#),
            vec![TokenKind::String(s("line\\break"))]
        );
        assert_eq!(
            lex(r#""quote\"test""#),
            vec![TokenKind::String(s("quote\"test"))]
        );
    }

    #[test]
    fn test_exponentiation_operator() {
        assert_eq!(lex("**"), vec![TokenKind::StarStar]);
        assert_eq!(lex("**="), vec![TokenKind::StarStarEq]);
    }

    #[test]
    fn test_in_instanceof_operators() {
        assert_eq!(
            lex("in instanceof"),
            vec![TokenKind::In, TokenKind::Instanceof]
        );
    }

    #[test]
    fn test_typeof_operator() {
        assert_eq!(lex("typeof"), vec![TokenKind::Typeof]);
    }

    #[test]
    fn test_unsigned_right_shift() {
        assert_eq!(lex(">>>"), vec![TokenKind::GtGtGt]);
        assert_eq!(lex(">>>="), vec![TokenKind::GtGtGtEq]);
    }

    #[test]
    fn test_bigint_literal() {
        assert_eq!(lex("123n"), vec![TokenKind::BigInt("123".to_string())]);
        assert_eq!(lex("0n"), vec![TokenKind::BigInt("0".to_string())]);
        assert_eq!(
            lex("9007199254740991n"),
            vec![TokenKind::BigInt("9007199254740991".to_string())]
        );
    }

    #[test]
    fn test_bigint_hex() {
        assert_eq!(lex("0xFFn"), vec![TokenKind::BigInt("255".to_string())]);
    }

    #[test]
    fn test_regexp_literal_scan() {
        // Test the explicit regex scanning method
        let mut dict = StringDict::new();
        let mut lexer = Lexer::new("/abc/", &mut dict);
        let token = lexer.scan_regexp();
        assert_eq!(
            token.kind,
            TokenKind::RegExp("abc".to_string(), "".to_string())
        );
    }

    #[test]
    fn test_regexp_literal_with_flags() {
        let mut dict = StringDict::new();
        let mut lexer = Lexer::new("/pattern/gi", &mut dict);
        let token = lexer.scan_regexp();
        assert_eq!(
            token.kind,
            TokenKind::RegExp("pattern".to_string(), "gi".to_string())
        );
    }

    #[test]
    fn test_regexp_literal_with_escapes() {
        let mut dict = StringDict::new();
        let mut lexer = Lexer::new(r"/\d+\.\d+/", &mut dict);
        let token = lexer.scan_regexp();
        assert_eq!(
            token.kind,
            TokenKind::RegExp(r"\d+\.\d+".to_string(), "".to_string())
        );
    }

    #[test]
    fn test_regexp_literal_with_class() {
        let mut dict = StringDict::new();
        let mut lexer = Lexer::new("/[a-z]/i", &mut dict);
        let token = lexer.scan_regexp();
        assert_eq!(
            token.kind,
            TokenKind::RegExp("[a-z]".to_string(), "i".to_string())
        );
    }

    #[test]
    fn test_regexp_literal_with_slash_in_class() {
        // Forward slash inside character class doesn't end the regex
        let mut dict = StringDict::new();
        let mut lexer = Lexer::new("/[/]/", &mut dict);
        let token = lexer.scan_regexp();
        assert_eq!(
            token.kind,
            TokenKind::RegExp("[/]".to_string(), "".to_string())
        );
    }
}
