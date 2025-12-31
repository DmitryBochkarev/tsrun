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
                            if let Some(hex) = self.scan_hex_escape(2)
                                && let Some(ch) = char::from_u32(hex)
                            {
                                value.push(ch);
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
                                if let Ok(code) = u32::from_str_radix(&hex_str, 16)
                                    && let Some(ch) = char::from_u32(code)
                                {
                                    value.push(ch);
                                }
                            } else if let Some(hex) = self.scan_hex_escape(4)
                                && let Some(ch) = char::from_u32(hex)
                            {
                                value.push(ch);
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

    /// Scan a unicode escape sequence in an identifier.
    /// Expects to be called after the backslash has been consumed.
    /// Returns the decoded character if valid, None otherwise.
    fn scan_unicode_escape_in_identifier(&mut self) -> Option<char> {
        // Must start with 'u'
        if self.peek() != Some('u') {
            return None;
        }
        self.advance(); // consume 'u'

        // Check for \u{...} form
        if self.peek() == Some('{') {
            self.advance(); // consume '{'
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
                    return None;
                }
            }
            if hex_str.is_empty() {
                return None;
            }
            let code = u32::from_str_radix(&hex_str, 16).ok()?;
            return char::from_u32(code);
        }

        // \uNNNN form - exactly 4 hex digits
        let code = self.scan_hex_escape(4)?;
        char::from_u32(code)
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
                        Some((_, 'b')) => value.push('\x08'), // backspace
                        Some((_, 'f')) => value.push('\x0C'), // form feed
                        Some((_, 'v')) => value.push('\x0B'), // vertical tab
                        Some((_, '\\')) => value.push('\\'),
                        Some((_, '`')) => value.push('`'),
                        Some((_, '$')) => value.push('$'),
                        Some((_, '0')) => {
                            // \0 is null character (only if not followed by another digit)
                            if !matches!(self.peek(), Some('0'..='9')) {
                                value.push('\0');
                            } else {
                                // Octal escapes not allowed in template literals
                                return TokenKind::Invalid('0');
                            }
                        }
                        Some((_, 'x')) => {
                            // Hex escape \xNN - must be exactly 2 hex digits
                            if let Some(hex) = self.scan_hex_escape(2) {
                                if let Some(ch) = char::from_u32(hex) {
                                    value.push(ch);
                                } else {
                                    return TokenKind::Invalid('x');
                                }
                            } else {
                                return TokenKind::Invalid('x');
                            }
                        }
                        Some((_, 'u')) => {
                            // Unicode escape \uNNNN or \u{N...}
                            if self.peek() == Some('{') {
                                self.advance();
                                let mut hex_str = String::new();
                                let mut found_close = false;
                                while let Some(ch) = self.peek() {
                                    if ch == '}' {
                                        self.advance();
                                        found_close = true;
                                        break;
                                    }
                                    if ch.is_ascii_hexdigit() {
                                        hex_str.push(ch);
                                        self.advance();
                                    } else {
                                        return TokenKind::Invalid('u');
                                    }
                                }
                                if !found_close || hex_str.is_empty() {
                                    return TokenKind::Invalid('u');
                                }
                                if let Ok(code) = u32::from_str_radix(&hex_str, 16) {
                                    if let Some(ch) = char::from_u32(code) {
                                        value.push(ch);
                                    } else {
                                        return TokenKind::Invalid('u');
                                    }
                                } else {
                                    return TokenKind::Invalid('u');
                                }
                            } else if let Some(hex) = self.scan_hex_escape(4) {
                                if let Some(ch) = char::from_u32(hex) {
                                    value.push(ch);
                                } else {
                                    return TokenKind::Invalid('u');
                                }
                            } else {
                                return TokenKind::Invalid('u');
                            }
                        }
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
                    Some((_, 'b')) => value.push('\x08'), // backspace
                    Some((_, 'f')) => value.push('\x0C'), // form feed
                    Some((_, 'v')) => value.push('\x0B'), // vertical tab
                    Some((_, '\\')) => value.push('\\'),
                    Some((_, '`')) => value.push('`'),
                    Some((_, '$')) => value.push('$'),
                    Some((_, '0')) => {
                        // \0 is null character (only if not followed by another digit)
                        if !matches!(self.peek(), Some('0'..='9')) {
                            value.push('\0');
                        } else {
                            // Octal escapes not allowed in template literals
                            return TokenKind::Invalid('0');
                        }
                    }
                    Some((_, 'x')) => {
                        // Hex escape \xNN - must be exactly 2 hex digits
                        if let Some(hex) = self.scan_hex_escape(2) {
                            if let Some(ch) = char::from_u32(hex) {
                                value.push(ch);
                            } else {
                                return TokenKind::Invalid('x');
                            }
                        } else {
                            return TokenKind::Invalid('x');
                        }
                    }
                    Some((_, 'u')) => {
                        // Unicode escape \uNNNN or \u{N...}
                        if self.peek() == Some('{') {
                            self.advance();
                            let mut hex_str = String::new();
                            let mut found_close = false;
                            while let Some(ch) = self.peek() {
                                if ch == '}' {
                                    self.advance();
                                    found_close = true;
                                    break;
                                }
                                if ch.is_ascii_hexdigit() {
                                    hex_str.push(ch);
                                    self.advance();
                                } else {
                                    return TokenKind::Invalid('u');
                                }
                            }
                            if !found_close || hex_str.is_empty() {
                                return TokenKind::Invalid('u');
                            }
                            if let Ok(code) = u32::from_str_radix(&hex_str, 16) {
                                if let Some(ch) = char::from_u32(code) {
                                    value.push(ch);
                                } else {
                                    return TokenKind::Invalid('u');
                                }
                            } else {
                                return TokenKind::Invalid('u');
                            }
                        } else if let Some(hex) = self.scan_hex_escape(4) {
                            if let Some(ch) = char::from_u32(hex) {
                                value.push(ch);
                            } else {
                                return TokenKind::Invalid('u');
                            }
                        } else {
                            return TokenKind::Invalid('u');
                        }
                    }
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
            // Check what follows the dot to determine if it's part of the number.
            // In JavaScript:
            // - 1.5 = decimal number
            // - 1.e5 = decimal with exponent (1.0 × 10^5)
            // - 1. = valid number literal (trailing dot, no fractional digits)
            // - 1.. = 1. (number) followed by . (member access)
            // - 1.[ = 1. (number) followed by [ (computed member access)
            // - 1.toString() = syntax error (dot is consumed as decimal point, but no digits follow)
            //
            // The rule: consume the dot as part of the number UNLESS it would start an identifier
            // (which would be a syntax error in parsing anyway).
            let next = self.peek_next();
            let should_consume_dot = match next {
                // Digit after dot = decimal number
                Some('0'..='9') => true,
                // e/E after dot = decimal with exponent (1.e5)
                Some('e' | 'E') => true,
                // Another dot = this dot is part of number, next dot is member access (1..toString())
                Some('.') => true,
                // Left bracket = this dot is part of number, bracket is computed access (1.["foo"])
                Some('[') => true,
                // Underscore can be numeric separator (1._5 is not valid but 1.5_0 is)
                // But 1._ by itself is not valid, so don't consume
                Some('_') => false,
                // Identifier start = don't consume dot (1.foo would be error anyway)
                Some(c) if is_id_start(c) => false,
                // Anything else (operators, whitespace, EOF, etc) = consume dot as trailing decimal
                _ => true,
            };

            if should_consume_dot {
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
            if matches!(self.peek(), Some('+' | '-'))
                && let Some((_, ch)) = self.advance()
            {
                num_str.push(ch);
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
        let mut had_escape = false;

        // Handle first character - might be a unicode escape
        if first == '\\' {
            if let Some(decoded) = self.scan_unicode_escape_in_identifier() {
                if is_id_start_char(decoded) {
                    name.push(decoded);
                    had_escape = true;
                } else {
                    return TokenKind::Invalid('\\');
                }
            } else {
                return TokenKind::Invalid('\\');
            }
        } else {
            name.push(first);
        }

        // Continue scanning identifier characters
        while let Some(ch) = self.peek() {
            if ch == '\\' {
                self.advance(); // consume the backslash
                if let Some(decoded) = self.scan_unicode_escape_in_identifier() {
                    if is_id_continue_char(decoded) {
                        name.push(decoded);
                        had_escape = true;
                    } else {
                        return TokenKind::Invalid('\\');
                    }
                } else {
                    return TokenKind::Invalid('\\');
                }
            } else if is_id_continue_char(ch) {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // If the identifier contained escapes, it cannot be a keyword
        // (ECMAScript spec: escaped identifiers that spell keywords are still identifiers)
        if had_escape {
            return TokenKind::Identifier(self.string_dict.get_or_insert(&name));
        }

        // Length-prefixed keyword dispatch for faster matching
        // First dispatch on length, then compare only keywords of that length
        match name.len() {
            2 => match name.as_str() {
                "if" => TokenKind::If,
                "in" => TokenKind::In,
                "do" => TokenKind::Do,
                "as" => TokenKind::As,
                "of" => TokenKind::Of,
                "is" => TokenKind::Is,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            3 => match name.as_str() {
                "let" => TokenKind::Let,
                "var" => TokenKind::Var,
                "for" => TokenKind::For,
                "new" => TokenKind::New,
                "try" => TokenKind::Try,
                "any" => TokenKind::Any,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            4 => match name.as_str() {
                "true" => TokenKind::True,
                "null" => TokenKind::Null,
                "else" => TokenKind::Else,
                "case" => TokenKind::Case,
                "this" => TokenKind::This,
                "void" => TokenKind::Void,
                "enum" => TokenKind::Enum,
                "type" => TokenKind::Type,
                "from" => TokenKind::From,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            5 => match name.as_str() {
                "false" => TokenKind::False,
                "const" => TokenKind::Const,
                "while" => TokenKind::While,
                "break" => TokenKind::Break,
                "class" => TokenKind::Class,
                "super" => TokenKind::Super,
                "throw" => TokenKind::Throw,
                "await" => TokenKind::Await,
                "async" => TokenKind::Async,
                "yield" => TokenKind::Yield,
                "infer" => TokenKind::Infer,
                "never" => TokenKind::Never,
                "catch" => TokenKind::Catch,
                "keyof" => TokenKind::Keyof,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            6 => match name.as_str() {
                "return" => TokenKind::Return,
                "switch" => TokenKind::Switch,
                "static" => TokenKind::Static,
                "import" => TokenKind::Import,
                "export" => TokenKind::Export,
                "typeof" => TokenKind::Typeof,
                "delete" => TokenKind::Delete,
                "public" => TokenKind::Public,
                "module" => TokenKind::Module,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            7 => match name.as_str() {
                "default" => TokenKind::Default,
                "finally" => TokenKind::Finally,
                "extends" => TokenKind::Extends,
                "declare" => TokenKind::Declare,
                "private" => TokenKind::Private,
                "unknown" => TokenKind::Unknown,
                "asserts" => TokenKind::Asserts,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            8 => match name.as_str() {
                "function" => TokenKind::Function,
                "continue" => TokenKind::Continue,
                "debugger" => TokenKind::Debugger,
                "readonly" => TokenKind::Readonly,
                "accessor" => TokenKind::Accessor,
                "abstract" => TokenKind::Abstract,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            9 => match name.as_str() {
                "protected" => TokenKind::Protected,
                "namespace" => TokenKind::Namespace,
                "interface" => TokenKind::Interface,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            10 => match name.as_str() {
                "instanceof" => TokenKind::Instanceof,
                "implements" => TokenKind::Implements,
                _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
            },
            _ => TokenKind::Identifier(self.string_dict.get_or_insert(&name)),
        }
    }
}

/// Check if a character can start an identifier (including unicode escape sequence)
fn is_id_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch == '\\' || ch.is_ascii_alphabetic()
}

/// Check if a decoded character is valid as identifier start (without escape check)
fn is_id_start_char(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic()
}

/// Check if a decoded character is valid as identifier continue (without escape check)
fn is_id_continue_char(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphanumeric()
}
