//! Trampoline-style parser for TypeScript
//!
//! Uses an explicit work stack instead of recursion to avoid stack overflow.

use crate::ast::{
    BinaryExpression, BinaryOp, Expression, ExpressionStatement, Identifier, Literal,
    LiteralValue, Pattern, Program, SourceType, Statement, VariableDeclaration,
    VariableDeclarator, VariableKind,
};
use crate::error::JsError;
use crate::lexer::{Lexer, Span, Token, TokenKind};
use crate::prelude::*;
use crate::string_dict::StringDict;
use crate::value::CheapClone;

// ============================================================================
// Work items - what the parser needs to do next
// ============================================================================

/// Work items for the trampoline loop
enum Work {
    /// Parse statements until EOF, then build Program
    ParseStatements { statements: Vec<Statement> },

    /// Parse a variable declaration (let/const/var already consumed)
    ParseVariableDeclaration {
        kind: VariableKind,
        start_span: Span,
        statements: Vec<Statement>,
    },

    /// Parse the initializer expression for a variable
    ParseVariableInit {
        kind: VariableKind,
        id: Pattern,
        id_span: Span,
        start_span: Span,
        statements: Vec<Statement>,
    },
}

// ============================================================================
// Parse results - intermediate values on the result stack
// ============================================================================

/// Intermediate results during parsing
enum ParseResult {
    /// A parsed expression
    Expression(Rc<Expression>),
}

// ============================================================================
// Parser
// ============================================================================

/// Trampoline-style parser that avoids recursion
pub struct Parser<'src> {
    lexer: Lexer<'src>,
    current: Token,
    work: Vec<Work>,
    results: Vec<ParseResult>,
    /// The final program, set when parsing completes
    program: Option<Program>,
}

impl<'src> Parser<'src> {
    /// Create a new parser for the given source code
    pub fn new(source: &'src str, dict: &'src mut StringDict) -> Self {
        let mut lexer = Lexer::new(source, dict);
        let current = lexer.next_token();
        Self {
            lexer,
            current,
            work: Vec::new(),
            results: Vec::new(),
            program: None,
        }
    }

    /// Parse the source code into a Program AST
    pub fn parse_program(&mut self) -> Result<Program, JsError> {
        // Push initial work
        self.work.push(Work::ParseStatements {
            statements: Vec::new(),
        });

        // Main trampoline loop
        while let Some(work) = self.work.pop() {
            self.step(work)?;
        }

        // Return the completed program
        self.program.take().ok_or_else(|| {
            JsError::syntax_error_simple("Internal error: program not set after parsing")
        })
    }

    /// Execute one step of parsing
    fn step(&mut self, work: Work) -> Result<(), JsError> {
        match work {
            Work::ParseStatements { statements } => self.step_parse_statements(statements),
            Work::ParseVariableDeclaration {
                kind,
                start_span,
                statements,
            } => self.step_parse_variable_declaration(kind, start_span, statements),
            Work::ParseVariableInit {
                kind,
                id,
                id_span,
                start_span,
                statements,
            } => self.step_parse_variable_init(kind, id, id_span, start_span, statements),
        }
    }

    // ========================================================================
    // Token helpers
    // ========================================================================

    /// Advance to the next token
    fn advance(&mut self) {
        self.current = self.lexer.next_token();
    }

    /// Check if current token matches the given kind
    fn check(&self, kind: &TokenKind) -> bool {
        core::mem::discriminant(&self.current.kind) == core::mem::discriminant(kind)
    }

    /// Consume current token if it matches, return true if consumed
    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Require the current token to match, consume it, or return error
    fn expect(&mut self, kind: &TokenKind) -> Result<Span, JsError> {
        if self.check(kind) {
            let span = self.current.span;
            self.advance();
            Ok(span)
        } else {
            Err(JsError::syntax_error(
                format!("Expected {:?}, found {:?}", kind, self.current.kind),
                self.current.span.line,
                self.current.span.column,
            ))
        }
    }

    /// Skip a type annotation (minimal implementation - just skips identifier)
    fn skip_type_annotation(&mut self) -> Result<(), JsError> {
        // For now, just skip a single identifier like "number", "string", etc.
        match &self.current.kind {
            TokenKind::Identifier(_)
            | TokenKind::Any
            | TokenKind::Unknown
            | TokenKind::Never
            | TokenKind::Void
            | TokenKind::Null => {
                self.advance();
                Ok(())
            }
            _ => Err(JsError::syntax_error(
                format!("Expected type, found {:?}", self.current.kind),
                self.current.span.line,
                self.current.span.column,
            )),
        }
    }

    // ========================================================================
    // Step implementations
    // ========================================================================

    /// Parse statements until EOF
    fn step_parse_statements(&mut self, statements: Vec<Statement>) -> Result<(), JsError> {
        // Check for EOF
        if self.check(&TokenKind::Eof) {
            // Done parsing - create the program
            self.program = Some(Program {
                body: Rc::from(statements),
                source_type: SourceType::Script,
            });
            return Ok(());
        }

        // Parse one statement based on current token
        let start_span = self.current.span;
        match &self.current.kind {
            TokenKind::Let => {
                self.advance();
                self.work.push(Work::ParseVariableDeclaration {
                    kind: VariableKind::Let,
                    start_span,
                    statements,
                });
            }
            TokenKind::Const => {
                self.advance();
                self.work.push(Work::ParseVariableDeclaration {
                    kind: VariableKind::Const,
                    start_span,
                    statements,
                });
            }
            TokenKind::Var => {
                self.advance();
                self.work.push(Work::ParseVariableDeclaration {
                    kind: VariableKind::Var,
                    start_span,
                    statements,
                });
            }
            // Empty statement
            TokenKind::Semicolon => {
                self.advance();
                let mut statements = statements;
                statements.push(Statement::Empty);
                self.work.push(Work::ParseStatements { statements });
            }
            // Default: try to parse as expression statement
            _ => {
                // Parse as expression statement
                let expr = self.parse_expression()?;
                self.eat(&TokenKind::Semicolon);
                let end_span = self.current.span;
                let stmt = Statement::Expression(ExpressionStatement {
                    expression: Rc::new(expr),
                    span: Span::new(
                        start_span.start,
                        end_span.start,
                        start_span.line,
                        start_span.column,
                    ),
                });
                let mut statements = statements;
                statements.push(stmt);
                self.work.push(Work::ParseStatements { statements });
            }
        }
        Ok(())
    }

    /// Parse variable declaration after let/const/var
    fn step_parse_variable_declaration(
        &mut self,
        kind: VariableKind,
        start_span: Span,
        statements: Vec<Statement>,
    ) -> Result<(), JsError> {
        // Expect identifier
        let (name, id_span) = match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                let span = self.current.span;
                self.advance();
                (name, span)
            }
            _ => {
                return Err(JsError::syntax_error(
                    "Expected identifier in variable declaration",
                    self.current.span.line,
                    self.current.span.column,
                ));
            }
        };

        let id = Pattern::Identifier(Identifier { name, span: id_span });

        // Skip type annotation if present (: type)
        if self.eat(&TokenKind::Colon) {
            // For now, just skip the type - consume identifier
            self.skip_type_annotation()?;
        }

        // Check for initializer
        if self.eat(&TokenKind::Eq) {
            // Need to parse expression
            self.work.push(Work::ParseVariableInit {
                kind,
                id,
                id_span,
                start_span,
                statements,
            });
        } else {
            // No initializer - handle semicolon and continue
            self.eat(&TokenKind::Semicolon);
            let decl = VariableDeclaration {
                kind,
                declarations: Rc::from([VariableDeclarator {
                    id,
                    type_annotation: None,
                    init: None,
                    span: id_span,
                }]),
                span: Span::new(
                    start_span.start,
                    self.current.span.start,
                    start_span.line,
                    start_span.column,
                ),
            };
            let mut statements = statements;
            statements.push(Statement::VariableDeclaration(decl));
            self.work.push(Work::ParseStatements { statements });
        }
        Ok(())
    }

    /// Parse the initializer expression and complete variable declaration
    fn step_parse_variable_init(
        &mut self,
        kind: VariableKind,
        id: Pattern,
        id_span: Span,
        start_span: Span,
        statements: Vec<Statement>,
    ) -> Result<(), JsError> {
        // Parse the initializer expression
        let init = self.parse_expression()?;

        // Handle semicolon
        self.eat(&TokenKind::Semicolon);

        // Create the variable declaration
        let end_span = self.current.span;
        let decl = VariableDeclaration {
            kind,
            declarations: Rc::from([VariableDeclarator {
                id,
                type_annotation: None,
                init: Some(Rc::new(init)),
                span: Span::new(id_span.start, end_span.start, id_span.line, id_span.column),
            }]),
            span: Span::new(
                start_span.start,
                end_span.start,
                start_span.line,
                start_span.column,
            ),
        };

        let mut statements = statements;
        statements.push(Statement::VariableDeclaration(decl));
        self.work.push(Work::ParseStatements { statements });
        Ok(())
    }

    // ========================================================================
    // Expression parsing (synchronous for simple expressions)
    // ========================================================================

    /// Parse an expression with operator precedence (Pratt parsing)
    fn parse_expression(&mut self) -> Result<Expression, JsError> {
        self.parse_binary_expression(0)
    }

    /// Parse binary expression with minimum precedence
    fn parse_binary_expression(&mut self, min_prec: u8) -> Result<Expression, JsError> {
        let mut left = self.parse_primary_expression()?;

        loop {
            // Check for binary operator
            let Some((op, prec)) = self.get_binary_op_and_prec() else {
                break;
            };

            if prec < min_prec {
                break;
            }

            let op_span = self.current.span;
            self.advance(); // consume operator

            // Parse right-hand side with higher precedence (for left-associativity)
            // Exponentiation is right-associative, so use same precedence
            let next_prec = if op == BinaryOp::Exp { prec } else { prec + 1 };
            let right = self.parse_binary_expression(next_prec)?;

            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );

            left = Expression::Binary(BinaryExpression {
                operator: op,
                left: Rc::new(left),
                right: Rc::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// Get binary operator and its precedence from current token
    fn get_binary_op_and_prec(&self) -> Option<(BinaryOp, u8)> {
        match &self.current.kind {
            // Exponentiation (highest, right-associative)
            TokenKind::StarStar => Some((BinaryOp::Exp, 14)),

            // Multiplicative
            TokenKind::Star => Some((BinaryOp::Mul, 13)),
            TokenKind::Slash => Some((BinaryOp::Div, 13)),
            TokenKind::Percent => Some((BinaryOp::Mod, 13)),

            // Additive
            TokenKind::Plus => Some((BinaryOp::Add, 12)),
            TokenKind::Minus => Some((BinaryOp::Sub, 12)),

            // Shift
            TokenKind::LtLt => Some((BinaryOp::LShift, 11)),
            TokenKind::GtGt => Some((BinaryOp::RShift, 11)),
            TokenKind::GtGtGt => Some((BinaryOp::URShift, 11)),

            // Relational
            TokenKind::Lt => Some((BinaryOp::Lt, 10)),
            TokenKind::LtEq => Some((BinaryOp::LtEq, 10)),
            TokenKind::Gt => Some((BinaryOp::Gt, 10)),
            TokenKind::GtEq => Some((BinaryOp::GtEq, 10)),
            TokenKind::In => Some((BinaryOp::In, 10)),
            TokenKind::Instanceof => Some((BinaryOp::Instanceof, 10)),

            // Equality
            TokenKind::EqEq => Some((BinaryOp::Eq, 9)),
            TokenKind::BangEq => Some((BinaryOp::NotEq, 9)),
            TokenKind::EqEqEq => Some((BinaryOp::StrictEq, 9)),
            TokenKind::BangEqEq => Some((BinaryOp::StrictNotEq, 9)),

            // Bitwise AND
            TokenKind::Amp => Some((BinaryOp::BitAnd, 8)),

            // Bitwise XOR
            TokenKind::Caret => Some((BinaryOp::BitXor, 7)),

            // Bitwise OR
            TokenKind::Pipe => Some((BinaryOp::BitOr, 6)),

            _ => None,
        }
    }

    /// Parse a primary expression (literals, identifiers)
    fn parse_primary_expression(&mut self) -> Result<Expression, JsError> {
        let span = self.current.span;
        match &self.current.kind {
            // Number literal
            TokenKind::Number(n) => {
                let value = *n;
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::Number(value),
                    span,
                })))
            }
            // String literal
            TokenKind::String(s) => {
                let value = s.cheap_clone();
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::String(value),
                    span,
                })))
            }
            // Boolean literals
            TokenKind::True => {
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::Boolean(true),
                    span,
                })))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::Boolean(false),
                    span,
                })))
            }
            // Null literal
            TokenKind::Null => {
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::Null,
                    span,
                })))
            }
            // Identifier
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            // Parenthesized expression
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expression()?;
                let end_span = self.expect(&TokenKind::RParen)?;
                let full_span = Span::new(span.start, end_span.end, span.line, span.column);
                Ok(Expression::Parenthesized(Rc::new(inner), full_span))
            }
            _ => Err(JsError::syntax_error(
                format!("Expected expression, found {:?}", self.current.kind),
                self.current.span.line,
                self.current.span.column,
            )),
        }
    }
}
