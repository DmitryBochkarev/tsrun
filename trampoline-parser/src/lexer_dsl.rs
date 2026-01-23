//! Lexer DSL for defining tokens
//!
//! # Example
//!
//! ```rust
//! use trampoline_parser::Grammar;
//!
//! // Lexer is defined as part of a Grammar
//! let grammar = Grammar::new()
//!     .lexer(|l| {
//!         l.token("PLUS", "+")
//!          .token("NUMBER", "")
//!              .start_with(|c| c.match_class("digit"))
//!              .continue_with(|c| c.match_class("digit"))
//!              .build()
//!          .keyword("IF", "if")
//!          .skip("whitespace")
//!     })
//!     .rule("expr", |r| r.token("NUMBER"))
//!     .build();
//! ```

use crate::ir::{
    CharCondition, KeywordDef, LexerDef, SpecialScan, TokenDef, TokenKind, TokenPattern,
};

/// Builder for lexer definition
#[derive(Debug, Default)]
pub struct LexerBuilder {
    tokens: Vec<TokenDef>,
    keywords: Vec<KeywordDef>,
    skip_patterns: Vec<String>,
    /// Current token being built (for pattern-based tokens)
    current_token: Option<TokenPatternBuilder>,
}

impl LexerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a simple literal token
    pub fn token(mut self, name: &str, literal: &str) -> Self {
        // Finalize any pending pattern token
        self.finalize_current();

        if literal.is_empty() {
            // Start a pattern-based token
            self.current_token = Some(TokenPatternBuilder::new(name));
        } else {
            // Simple literal token
            self.tokens.push(TokenDef {
                name: name.to_string(),
                kind: TokenKind::Literal(literal.to_string()),
            });
        }
        self
    }

    /// Start defining a pattern-based token (returns pattern builder)
    pub fn pattern<F>(mut self, f: F) -> Self
    where
        F: FnOnce(CharMatcherBuilder) -> CharMatcherBuilder,
    {
        if let Some(ref mut current) = self.current_token {
            let matcher = CharMatcherBuilder::new();
            let matcher = f(matcher);
            current.start = matcher.condition;
        }
        self
    }

    /// Define start condition for current token
    pub fn start_with<F>(mut self, f: F) -> Self
    where
        F: FnOnce(CharMatcherBuilder) -> CharMatcherBuilder,
    {
        if let Some(ref mut current) = self.current_token {
            let matcher = CharMatcherBuilder::new();
            let matcher = f(matcher);
            current.start = matcher.condition;
        }
        self
    }

    /// Define continuation condition for current token
    pub fn continue_with<F>(mut self, f: F) -> Self
    where
        F: FnOnce(CharMatcherBuilder) -> CharMatcherBuilder,
    {
        if let Some(ref mut current) = self.current_token {
            let matcher = CharMatcherBuilder::new();
            let matcher = f(matcher);
            current.continuation = matcher.condition;
        }
        self
    }

    /// Define until condition for current token
    pub fn until<F>(mut self, f: F) -> Self
    where
        F: FnOnce(CharMatcherBuilder) -> CharMatcherBuilder,
    {
        if let Some(ref mut current) = self.current_token {
            let matcher = CharMatcherBuilder::new();
            let matcher = f(matcher);
            current.until = matcher.condition;
        }
        self
    }

    /// Mark current token as scannable until matching quote
    pub fn scan_until_matching_quote(mut self) -> Self {
        if let Some(ref mut current) = self.current_token {
            current.special = Some(SpecialScan::UntilMatchingQuote);
        }
        self
    }

    /// Mark current token as template head scanner
    pub fn scan_template_head(mut self) -> Self {
        if let Some(ref mut current) = self.current_token {
            current.special = Some(SpecialScan::TemplateHead);
        }
        self
    }

    /// Mark current token as template middle scanner
    pub fn scan_template_middle(mut self) -> Self {
        if let Some(ref mut current) = self.current_token {
            current.special = Some(SpecialScan::TemplateMiddle);
        }
        self
    }

    /// Mark current token as template tail scanner
    pub fn scan_template_tail(mut self) -> Self {
        if let Some(ref mut current) = self.current_token {
            current.special = Some(SpecialScan::TemplateTail);
        }
        self
    }

    /// Mark current token as significant (not skipped)
    pub fn significant(mut self) -> Self {
        if let Some(ref mut current) = self.current_token {
            current.significant = true;
        }
        self
    }

    /// Mark current token as rescannable to another token type
    pub fn can_rescan_as(mut self, token_name: &str) -> Self {
        if let Some(ref mut current) = self.current_token {
            current.rescan_as = Some(token_name.to_string());
        }
        self
    }

    /// Finalize the current pattern-based token
    #[allow(clippy::should_implement_trait)]
    pub fn build(mut self) -> Self {
        self.finalize_current();
        self
    }

    /// Define a keyword (higher priority than identifiers)
    pub fn keyword(mut self, name: &str, literal: &str) -> Self {
        self.finalize_current();
        self.keywords.push(KeywordDef {
            name: name.to_string(),
            literal: literal.to_string(),
        });
        self
    }

    /// Add a skip pattern
    pub fn skip(mut self, pattern: &str) -> Self {
        self.finalize_current();
        self.skip_patterns.push(pattern.to_string());
        self
    }

    /// Build the lexer definition (internal, called by Grammar)
    pub(crate) fn build_def(mut self) -> LexerDef {
        self.finalize_current();
        LexerDef {
            tokens: self.tokens,
            keywords: self.keywords,
            skip_patterns: self.skip_patterns,
        }
    }

    fn finalize_current(&mut self) {
        if let Some(current) = self.current_token.take() {
            self.tokens.push(TokenDef {
                name: current.name,
                kind: TokenKind::Pattern(TokenPattern {
                    start: current.start,
                    continuation: current.continuation,
                    until: current.until,
                    special: current.special,
                    rescan_as: current.rescan_as,
                    significant: current.significant,
                }),
            });
        }
    }
}

/// Builder for pattern-based token
#[derive(Debug)]
struct TokenPatternBuilder {
    name: String,
    start: Option<CharCondition>,
    continuation: Option<CharCondition>,
    until: Option<CharCondition>,
    special: Option<SpecialScan>,
    rescan_as: Option<String>,
    significant: bool,
}

impl TokenPatternBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: None,
            continuation: None,
            until: None,
            special: None,
            rescan_as: None,
            significant: false,
        }
    }
}

/// Builder for character matching conditions
#[derive(Debug, Default)]
pub struct CharMatcherBuilder {
    condition: Option<CharCondition>,
}

impl CharMatcherBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Match a predefined character class ("digit", "alpha", "alphanumeric", "whitespace", "hex")
    pub fn match_class(mut self, class: &str) -> Self {
        let cond = CharCondition::Class(class.to_string());
        self.condition = Some(self.combine_or(cond));
        self
    }

    /// Match a single character
    pub fn char(mut self, c: char) -> Self {
        let cond = CharCondition::Char(c);
        self.condition = Some(self.combine_or(cond));
        self
    }

    /// Match a character range
    pub fn range(mut self, from: char, to: char) -> Self {
        let cond = CharCondition::Range(from, to);
        self.condition = Some(self.combine_or(cond));
        self
    }

    /// Match any character
    pub fn any(mut self) -> Self {
        let cond = CharCondition::Any;
        self.condition = Some(self.combine_or(cond));
        self
    }

    /// Match end of input
    pub fn eof(mut self) -> Self {
        let cond = CharCondition::Eof;
        self.condition = Some(self.combine_or(cond));
        self
    }

    /// Combine with OR (for chaining)
    pub fn or(self) -> Self {
        // Just return self, the next match_class/char/etc will combine
        self
    }

    /// Negate the current condition
    pub fn negated(mut self) -> Self {
        if let Some(cond) = self.condition.take() {
            self.condition = Some(CharCondition::Not(Box::new(cond)));
        }
        self
    }

    /// Combine with AND
    pub fn and(mut self, other: CharMatcherBuilder) -> Self {
        if let (Some(left), Some(right)) = (self.condition.take(), other.condition) {
            self.condition = Some(CharCondition::And(Box::new(left), Box::new(right)));
        }
        self
    }

    /// One or more of this pattern (for pattern builder)
    pub fn one_or_more(self) -> Self {
        // This is a hint for the lexer generator
        // The condition stays the same, generator handles repetition
        self
    }

    /// Zero or more of this pattern (for pattern builder)
    pub fn zero_or_more(self) -> Self {
        self
    }

    fn combine_or(&self, new_cond: CharCondition) -> CharCondition {
        match &self.condition {
            Some(existing) => CharCondition::Or(Box::new(existing.clone()), Box::new(new_cond)),
            None => new_cond,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_tokens() {
        let lexer = LexerBuilder::new()
            .token("PLUS", "+")
            .token("MINUS", "-")
            .build_def();

        assert_eq!(lexer.tokens.len(), 2);
        assert!(matches!(&lexer.tokens[0].kind, TokenKind::Literal(s) if s == "+"));
    }

    #[test]
    fn test_pattern_token() {
        let lexer = LexerBuilder::new()
            .token("NUMBER", "")
            .start_with(|c| c.match_class("digit"))
            .continue_with(|c| c.match_class("digit").or().char('.'))
            .build()
            .build_def();

        assert_eq!(lexer.tokens.len(), 1);
        assert!(matches!(&lexer.tokens[0].kind, TokenKind::Pattern(_)));
    }

    #[test]
    fn test_keywords() {
        let lexer = LexerBuilder::new()
            .keyword("IF", "if")
            .keyword("ELSE", "else")
            .build_def();

        assert_eq!(lexer.keywords.len(), 2);
        assert_eq!(lexer.keywords[0].name, "IF");
        assert_eq!(lexer.keywords[0].literal, "if");
    }
}
