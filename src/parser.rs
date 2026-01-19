//! Trampoline-style parser for TypeScript
//!
//! Uses an explicit work stack instead of recursion to avoid stack overflow.

use crate::ast::{
    Argument, ArrayElement, ArrayExpression, ArrayType, ArrowFunctionBody, ArrowFunctionExpression,
    AssignmentExpression, AssignmentOp, AssignmentTarget, AwaitExpression, BinaryExpression, BinaryOp, BlockStatement,
    BreakStatement, CallExpression, CatchClause, ClassBody, ClassConstructor, ClassDeclaration, ClassExpression,
    ClassMember, ClassMethod, ClassProperty, ContinueStatement, ConditionalExpression,
    Decorator, DoWhileStatement, EnumDeclaration, EnumMember, Expression, ExportDeclaration,
    ExportSpecifier, ExpressionStatement, ForInOfLeft, ForInStatement, ForInit, ForOfStatement,
    ForStatement, FunctionDeclaration, FunctionExpression, FunctionParam, Identifier, IfStatement,
    IndexedAccessType, IndexSignature, InterfaceDeclaration, IntersectionType, KeyofType, Literal, LiteralValue,
    LogicalExpression, LogicalOp, MemberExpression, MemberProperty, MethodKind, MethodSignature,
    NewExpression, NonNullExpression, ObjectExpression, ObjectProperty, ObjectPropertyKey, ObjectType,
    OptionalChainExpression, Pattern, Program, Property, PropertyKind, PropertySignature, RestElement,
    ReturnStatement, SourceType, SequenceExpression, SpreadElement, Statement, StringLiteral, SwitchCase,
    SwitchStatement, TaggedTemplateExpression, TemplateElement, TemplateLiteral, ThrowStatement, TryStatement,
    TypeAliasDeclaration, TypeAnnotation, TypeMember, TypeAssertionExpression, TypeParameter, TypeParameters,
    UnaryExpression, UnaryOp, UpdateExpression, UpdateOp, VariableDeclaration, VariableDeclarator,
    VariableKind, WhileStatement, YieldExpression,
};
use crate::error::JsError;
use crate::lexer::{Lexer, Span, Token, TokenKind};
use crate::prelude::*;
use crate::string_dict::StringDict;
use crate::JsString;
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
        type_annotation: Option<Box<TypeAnnotation>>,
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
/// Maximum nesting depth for expressions
/// Keep this low enough to prevent stack overflow in recursive parsing
const MAX_NESTING_DEPTH: usize = 50;

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    current: Token,
    work: Vec<Work>,
    results: Vec<ParseResult>,
    /// The final program, set when parsing completes
    program: Option<Program>,
    /// Current expression nesting depth
    depth: usize,
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
            depth: 0,
        }
    }

    /// Increment nesting depth, return error if exceeded
    fn enter_nesting(&mut self) -> Result<(), JsError> {
        self.depth += 1;
        if self.depth > MAX_NESTING_DEPTH {
            Err(JsError::syntax_error(
                "Maximum nesting depth exceeded",
                self.current.span.line,
                self.current.span.column,
            ))
        } else {
            Ok(())
        }
    }

    /// Decrement nesting depth
    fn exit_nesting(&mut self) {
        self.depth = self.depth.saturating_sub(1);
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
                type_annotation,
                id_span,
                start_span,
                statements,
            } => self.step_parse_variable_init(kind, id, type_annotation, id_span, start_span, statements),
        }
    }

    // ========================================================================
    // Token helpers
    // ========================================================================

    /// Advance to the next token
    fn advance(&mut self) {
        self.current = self.lexer.next_token();
    }

    /// Peek at the next token without consuming it
    fn peek_token(&mut self) -> Token {
        let checkpoint = self.lexer.checkpoint();
        let current_saved = self.current.clone();
        self.advance();
        let peeked = self.current.clone();
        self.lexer.restore(checkpoint);
        self.current = current_saved;
        peeked
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

    /// Get identifier name from current token (handles contextual keywords)
    /// Returns the name and span, or None if current token is not an identifier-like token
    fn try_get_identifier_name(&mut self) -> Option<(JsString, Span)> {
        let span = self.current.span;
        let name = match &self.current.kind {
            TokenKind::Identifier(name) => Some(name.cheap_clone()),
            // Contextual keywords can be used as identifiers
            TokenKind::Module => Some(self.lexer.string_dict().get_or_insert("module")),
            TokenKind::Namespace => Some(self.lexer.string_dict().get_or_insert("namespace")),
            TokenKind::Type => Some(self.lexer.string_dict().get_or_insert("type")),
            TokenKind::Declare => Some(self.lexer.string_dict().get_or_insert("declare")),
            TokenKind::Abstract => Some(self.lexer.string_dict().get_or_insert("abstract")),
            TokenKind::Readonly => Some(self.lexer.string_dict().get_or_insert("readonly")),
            TokenKind::Async => Some(self.lexer.string_dict().get_or_insert("async")),
            TokenKind::From => Some(self.lexer.string_dict().get_or_insert("from")),
            TokenKind::As => Some(self.lexer.string_dict().get_or_insert("as")),
            TokenKind::Of => Some(self.lexer.string_dict().get_or_insert("of")),
            _ => None,
        };
        name.map(|n| {
            self.advance();
            (n, span)
        })
    }

    /// Expect an identifier (including contextual keywords) and consume it
    fn expect_identifier(&mut self) -> Result<(JsString, Span), JsError> {
        self.try_get_identifier_name().ok_or_else(|| {
            JsError::syntax_error(
                format!("Expected identifier, found {:?}", self.current.kind),
                self.current.span.line,
                self.current.span.column,
            )
        })
    }

    /// Check if current token is a keyword that can be used as a property name
    /// Returns the name string if so, None otherwise. Does NOT advance the token.
    fn keyword_as_property_name(&mut self) -> Option<JsString> {
        match &self.current.kind {
            TokenKind::From => Some(self.lexer.string_dict().get_or_insert("from")),
            TokenKind::As => Some(self.lexer.string_dict().get_or_insert("as")),
            TokenKind::Of => Some(self.lexer.string_dict().get_or_insert("of")),
            TokenKind::Type => Some(self.lexer.string_dict().get_or_insert("type")),
            TokenKind::Declare => Some(self.lexer.string_dict().get_or_insert("declare")),
            TokenKind::Readonly => Some(self.lexer.string_dict().get_or_insert("readonly")),
            TokenKind::Abstract => Some(self.lexer.string_dict().get_or_insert("abstract")),
            TokenKind::Module => Some(self.lexer.string_dict().get_or_insert("module")),
            TokenKind::Namespace => Some(self.lexer.string_dict().get_or_insert("namespace")),
            TokenKind::Async => Some(self.lexer.string_dict().get_or_insert("async")),
            TokenKind::Delete => Some(self.lexer.string_dict().get_or_insert("delete")),
            TokenKind::In => Some(self.lexer.string_dict().get_or_insert("in")),
            TokenKind::Instanceof => Some(self.lexer.string_dict().get_or_insert("instanceof")),
            TokenKind::Typeof => Some(self.lexer.string_dict().get_or_insert("typeof")),
            TokenKind::Void => Some(self.lexer.string_dict().get_or_insert("void")),
            TokenKind::New => Some(self.lexer.string_dict().get_or_insert("new")),
            TokenKind::Return => Some(self.lexer.string_dict().get_or_insert("return")),
            TokenKind::This => Some(self.lexer.string_dict().get_or_insert("this")),
            TokenKind::Super => Some(self.lexer.string_dict().get_or_insert("super")),
            TokenKind::Class => Some(self.lexer.string_dict().get_or_insert("class")),
            TokenKind::Function => Some(self.lexer.string_dict().get_or_insert("function")),
            TokenKind::If => Some(self.lexer.string_dict().get_or_insert("if")),
            TokenKind::Else => Some(self.lexer.string_dict().get_or_insert("else")),
            TokenKind::For => Some(self.lexer.string_dict().get_or_insert("for")),
            TokenKind::While => Some(self.lexer.string_dict().get_or_insert("while")),
            TokenKind::Do => Some(self.lexer.string_dict().get_or_insert("do")),
            TokenKind::Switch => Some(self.lexer.string_dict().get_or_insert("switch")),
            TokenKind::Case => Some(self.lexer.string_dict().get_or_insert("case")),
            TokenKind::Default => Some(self.lexer.string_dict().get_or_insert("default")),
            TokenKind::Break => Some(self.lexer.string_dict().get_or_insert("break")),
            TokenKind::Continue => Some(self.lexer.string_dict().get_or_insert("continue")),
            TokenKind::Try => Some(self.lexer.string_dict().get_or_insert("try")),
            TokenKind::Catch => Some(self.lexer.string_dict().get_or_insert("catch")),
            TokenKind::Finally => Some(self.lexer.string_dict().get_or_insert("finally")),
            TokenKind::Throw => Some(self.lexer.string_dict().get_or_insert("throw")),
            TokenKind::True => Some(self.lexer.string_dict().get_or_insert("true")),
            TokenKind::False => Some(self.lexer.string_dict().get_or_insert("false")),
            TokenKind::Null => Some(self.lexer.string_dict().get_or_insert("null")),
            TokenKind::Let => Some(self.lexer.string_dict().get_or_insert("let")),
            TokenKind::Const => Some(self.lexer.string_dict().get_or_insert("const")),
            TokenKind::Var => Some(self.lexer.string_dict().get_or_insert("var")),
            TokenKind::Static => Some(self.lexer.string_dict().get_or_insert("static")),
            TokenKind::Public => Some(self.lexer.string_dict().get_or_insert("public")),
            TokenKind::Private => Some(self.lexer.string_dict().get_or_insert("private")),
            TokenKind::Protected => Some(self.lexer.string_dict().get_or_insert("protected")),
            TokenKind::Extends => Some(self.lexer.string_dict().get_or_insert("extends")),
            TokenKind::Implements => Some(self.lexer.string_dict().get_or_insert("implements")),
            TokenKind::Import => Some(self.lexer.string_dict().get_or_insert("import")),
            TokenKind::Export => Some(self.lexer.string_dict().get_or_insert("export")),
            TokenKind::Yield => Some(self.lexer.string_dict().get_or_insert("yield")),
            TokenKind::Await => Some(self.lexer.string_dict().get_or_insert("await")),
            _ => None,
        }
    }

    /// Skip a type annotation (skips the entire type, including complex types)
    fn skip_type_annotation(&mut self) -> Result<(), JsError> {
        // Skip primary type
        self.skip_primary_type()?;

        // Skip array suffixes (T[]) or indexed access types (T["key"])
        while self.check(&TokenKind::LBracket) {
            self.advance(); // consume '['
            if self.check(&TokenKind::RBracket) {
                // Array type: T[]
                self.advance(); // consume ']'
            } else {
                // Indexed access type: T["key"] or T[K]
                self.skip_type_annotation()?;
                self.expect(&TokenKind::RBracket)?;
            }
        }

        // Skip union types: T | U | V
        while self.check(&TokenKind::Pipe) {
            self.advance(); // consume '|'
            self.skip_primary_type()?;
            // Skip array suffixes for union member
            while self.check(&TokenKind::LBracket) {
                self.advance();
                self.expect(&TokenKind::RBracket)?;
            }
        }

        // Skip intersection types: T & U & V
        while self.check(&TokenKind::Amp) {
            self.advance(); // consume '&'
            self.skip_primary_type()?;
            // Skip array suffixes for intersection member
            while self.check(&TokenKind::LBracket) {
                self.advance();
                self.expect(&TokenKind::RBracket)?;
            }
        }

        Ok(())
    }

    /// Skip a primary type (identifier, keyword, object type, etc.)
    fn skip_primary_type(&mut self) -> Result<(), JsError> {
        match &self.current.kind {
            TokenKind::Identifier(_)
            | TokenKind::Any
            | TokenKind::Unknown
            | TokenKind::Never
            | TokenKind::Void
            | TokenKind::Null
            | TokenKind::String(_)
            | TokenKind::Number(_)
            | TokenKind::True
            | TokenKind::False => {
                self.advance();
                // Skip optional type arguments: T<U>
                if self.check(&TokenKind::Lt) {
                    self.skip_type_arguments()?;
                }
                Ok(())
            }
            // keyof T or typeof x
            TokenKind::Keyof | TokenKind::Typeof => {
                self.advance();
                self.skip_primary_type()?;
                Ok(())
            }
            // Object type: { a: T; b: U }
            TokenKind::LBrace => {
                self.advance(); // consume '{'
                self.skip_type_members()?;
                self.expect(&TokenKind::RBrace)?;
                Ok(())
            }
            // Parenthesized or function type: (T) or () => U
            TokenKind::LParen => {
                self.advance(); // consume '('
                // Skip to matching ')'
                let mut depth = 1;
                while depth > 0 && !self.check(&TokenKind::Eof) {
                    if self.check(&TokenKind::LParen) {
                        depth += 1;
                    } else if self.check(&TokenKind::RParen) {
                        depth -= 1;
                    }
                    self.advance();
                }
                // Skip optional arrow and return type
                if self.check(&TokenKind::Arrow) {
                    self.advance();
                    self.skip_type_annotation()?;
                }
                Ok(())
            }
            // Tuple type: [T1, T2]
            TokenKind::LBracket => {
                self.advance(); // consume '['
                // Skip to matching ']'
                let mut depth = 1;
                while depth > 0 && !self.check(&TokenKind::Eof) {
                    if self.check(&TokenKind::LBracket) {
                        depth += 1;
                    } else if self.check(&TokenKind::RBracket) {
                        depth -= 1;
                    }
                    self.advance();
                }
                Ok(())
            }
            _ => Err(JsError::syntax_error(
                format!("Expected type, found {:?}", self.current.kind),
                self.current.span.line,
                self.current.span.column,
            )),
        }
    }

    /// Skip type arguments: <T, U>
    fn skip_type_arguments(&mut self) -> Result<(), JsError> {
        self.expect(&TokenKind::Lt)?;
        let mut depth = 1;
        while depth > 0 && !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::Lt) {
                depth += 1;
            } else if self.check(&TokenKind::Gt) {
                depth -= 1;
            }
            self.advance();
        }
        Ok(())
    }

    /// Skip type members inside an object type: { a: T; b: U } or index signatures { [key: T]: U }
    fn skip_type_members(&mut self) -> Result<(), JsError> {
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            // Check for index signature: [key: T]: U
            if self.check(&TokenKind::LBracket) {
                self.advance(); // consume '['
                // Skip to matching ']'
                let mut depth = 1;
                while depth > 0 && !self.check(&TokenKind::Eof) {
                    if self.check(&TokenKind::LBracket) {
                        depth += 1;
                    } else if self.check(&TokenKind::RBracket) {
                        depth -= 1;
                    }
                    self.advance();
                }
                // Skip ':'
                if self.check(&TokenKind::Colon) {
                    self.advance();
                }
                // Skip value type
                self.skip_type_annotation()?;
                // Skip semicolon if present
                if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Comma) {
                    self.advance();
                }
                continue;
            }

            // Skip property key (identifier or string)
            match &self.current.kind {
                TokenKind::Identifier(_) | TokenKind::String(_) => {
                    self.advance();
                }
                _ => {
                    return Err(JsError::syntax_error(
                        format!("Expected property name in type, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
            }

            // Skip optional marker '?'
            if self.check(&TokenKind::Question) {
                self.advance();
            }

            // Expect ':'
            self.expect(&TokenKind::Colon)?;

            // Skip type annotation recursively
            self.skip_type_annotation()?;

            // Skip semicolon or comma separator if present
            if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        Ok(())
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
                // Check for const enum
                if self.check(&TokenKind::Enum) {
                    self.advance();
                    let enum_decl = self.parse_enum_declaration(start_span, true)?;
                    let mut statements = statements;
                    statements.push(Statement::EnumDeclaration(Box::new(enum_decl)));
                    self.work.push(Work::ParseStatements { statements });
                } else {
                    self.work.push(Work::ParseVariableDeclaration {
                        kind: VariableKind::Const,
                        start_span,
                        statements,
                    });
                }
            }
            TokenKind::Var => {
                self.advance();
                self.work.push(Work::ParseVariableDeclaration {
                    kind: VariableKind::Var,
                    start_span,
                    statements,
                });
            }
            TokenKind::Enum => {
                self.advance();
                let enum_decl = self.parse_enum_declaration(start_span, false)?;
                let mut statements = statements;
                statements.push(Statement::EnumDeclaration(Box::new(enum_decl)));
                self.work.push(Work::ParseStatements { statements });
            }
            // Interface declaration
            TokenKind::Interface => {
                self.advance();
                let interface_decl = self.parse_interface_declaration(start_span)?;
                let mut statements = statements;
                statements.push(Statement::InterfaceDeclaration(Box::new(interface_decl)));
                self.work.push(Work::ParseStatements { statements });
            }
            // Type alias declaration
            TokenKind::Type => {
                self.advance();
                let type_alias = self.parse_type_alias_declaration(start_span)?;
                let mut statements = statements;
                statements.push(Statement::TypeAlias(Box::new(type_alias)));
                self.work.push(Work::ParseStatements { statements });
            }
            // Empty statement
            TokenKind::Semicolon => {
                self.advance();
                let mut statements = statements;
                statements.push(Statement::Empty);
                self.work.push(Work::ParseStatements { statements });
            }
            // Function declaration
            TokenKind::Function => {
                self.advance();
                let func_decl = self.parse_function_declaration(start_span, false)?;
                let mut statements = statements;
                statements.push(Statement::FunctionDeclaration(Box::new(func_decl)));
                self.work.push(Work::ParseStatements { statements });
            }
            // Return statement
            TokenKind::Return => {
                self.advance();
                let argument = if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::RBrace) || self.check(&TokenKind::Eof) {
                    None
                } else {
                    Some(Rc::new(self.parse_expression()?))
                };
                self.eat(&TokenKind::Semicolon);
                let end_span = self.current.span;
                let stmt = Statement::Return(ReturnStatement {
                    argument,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                });
                let mut statements = statements;
                statements.push(stmt);
                self.work.push(Work::ParseStatements { statements });
            }
            // Default: use the general statement parser
            _ => {
                let stmt = self.parse_statement()?;
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
        let mut declarations = Vec::new();

        loop {
            let id_span = self.current.span;

            // Parse binding pattern (identifier, array destructuring, or object destructuring)
            let id = self.parse_binding_pattern()?;

            // Parse type annotation if present (: type)
            let type_annotation = if self.eat(&TokenKind::Colon) {
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            // Check for initializer
            let init = if self.eat(&TokenKind::Eq) {
                Some(Rc::new(self.parse_expression()?))
            } else {
                None
            };

            declarations.push(VariableDeclarator {
                id,
                type_annotation,
                init,
                span: id_span,
            });

            // Check for more declarators
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        // Handle semicolon and continue
        self.eat(&TokenKind::Semicolon);
        let decl = VariableDeclaration {
            kind,
            declarations: Rc::from(declarations),
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

        Ok(())
    }

    /// Parse the initializer expression and complete variable declaration
    fn step_parse_variable_init(
        &mut self,
        kind: VariableKind,
        id: Pattern,
        type_annotation: Option<Box<TypeAnnotation>>,
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
                type_annotation,
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
        let expr = self.parse_conditional_expression()?;

        // Check for assignment operators
        if let Some(op) = self.get_assignment_op() {
            self.advance();
            let right = self.parse_expression()?; // Right-associative
            let expr_span = expr.span();
            let right_span = right.span();

            // Convert LHS to assignment target
            let target = self.expr_to_assignment_target(&expr)?;

            return Ok(Expression::Assignment(Box::new(AssignmentExpression {
                operator: op,
                left: target,
                right: Rc::new(right),
                span: Span::new(expr_span.start, right_span.end, expr_span.line, expr_span.column),
            })));
        }

        Ok(expr)
    }

    /// Get assignment operator from current token
    fn get_assignment_op(&self) -> Option<AssignmentOp> {
        match &self.current.kind {
            TokenKind::Eq => Some(AssignmentOp::Assign),
            TokenKind::PlusEq => Some(AssignmentOp::AddAssign),
            TokenKind::MinusEq => Some(AssignmentOp::SubAssign),
            TokenKind::StarEq => Some(AssignmentOp::MulAssign),
            TokenKind::SlashEq => Some(AssignmentOp::DivAssign),
            TokenKind::PercentEq => Some(AssignmentOp::ModAssign),
            TokenKind::StarStarEq => Some(AssignmentOp::ExpAssign),
            TokenKind::AmpEq => Some(AssignmentOp::BitAndAssign),
            TokenKind::PipeEq => Some(AssignmentOp::BitOrAssign),
            TokenKind::CaretEq => Some(AssignmentOp::BitXorAssign),
            TokenKind::LtLtEq => Some(AssignmentOp::LShiftAssign),
            TokenKind::GtGtEq => Some(AssignmentOp::RShiftAssign),
            TokenKind::GtGtGtEq => Some(AssignmentOp::URShiftAssign),
            TokenKind::AmpAmpEq => Some(AssignmentOp::AndAssign),
            TokenKind::PipePipeEq => Some(AssignmentOp::OrAssign),
            TokenKind::QuestionQuestionEq => Some(AssignmentOp::NullishAssign),
            _ => None,
        }
    }

    /// Convert an expression to an assignment target
    fn expr_to_assignment_target(&self, expr: &Expression) -> Result<AssignmentTarget, JsError> {
        match expr {
            Expression::Identifier(id) => Ok(AssignmentTarget::Identifier(id.clone())),
            Expression::Member(m) => Ok(AssignmentTarget::Member((**m).clone())),
            // Array destructuring: [a, b] = ...
            Expression::Array(arr) => {
                use crate::ast::ArrayElement;
                let mut elements = Vec::new();
                for elem in &arr.elements {
                    match elem {
                        None => elements.push(None),
                        Some(ArrayElement::Expression(e)) => {
                            let pattern = self.expr_to_pattern(e)?;
                            elements.push(Some(pattern));
                        }
                        Some(ArrayElement::Spread(s)) => {
                            let pattern = self.expr_to_pattern(&s.argument)?;
                            elements.push(Some(Pattern::Rest(RestElement {
                                argument: Box::new(pattern),
                                type_annotation: None,
                                span: s.span,
                            })));
                        }
                    }
                }
                Ok(AssignmentTarget::Pattern(Pattern::Array(crate::ast::ArrayPattern {
                    elements,
                    type_annotation: None,
                    span: arr.span,
                })))
            }
            // Object destructuring: { a, b } = ...
            Expression::Object(obj) => {
                use crate::ast::ObjectProperty;
                let mut properties = Vec::new();
                for prop in &obj.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            if p.method {
                                return Err(JsError::syntax_error(
                                    "Method shorthand in destructuring not supported",
                                    p.span.line,
                                    p.span.column,
                                ));
                            }
                            let value = self.expr_to_pattern(&p.value)?;
                            properties.push(crate::ast::ObjectPatternProperty::KeyValue {
                                key: p.key.clone(),
                                value,
                                shorthand: p.shorthand,
                                span: p.span,
                            });
                        }
                        ObjectProperty::Spread(s) => {
                            let pattern = self.expr_to_pattern(&s.argument)?;
                            properties.push(crate::ast::ObjectPatternProperty::Rest(RestElement {
                                argument: Box::new(pattern),
                                type_annotation: None,
                                span: s.span,
                            }));
                        }
                    }
                }
                Ok(AssignmentTarget::Pattern(Pattern::Object(crate::ast::ObjectPattern {
                    properties,
                    type_annotation: None,
                    span: obj.span,
                })))
            }
            // Parenthesized expression: (expr) = ...
            Expression::Parenthesized(inner, _) => self.expr_to_assignment_target(inner),
            _ => Err(JsError::syntax_error(
                "Invalid assignment target",
                expr.span().line,
                expr.span().column,
            )),
        }
    }

    /// Convert expression to pattern (for destructuring)
    fn expr_to_pattern(&self, expr: &Expression) -> Result<Pattern, JsError> {
        match expr {
            Expression::Identifier(id) => Ok(Pattern::Identifier(id.clone())),
            Expression::Array(arr) => {
                use crate::ast::ArrayElement;
                let mut elements = Vec::new();
                for elem in &arr.elements {
                    match elem {
                        None => elements.push(None),
                        Some(ArrayElement::Expression(e)) => {
                            let pattern = self.expr_to_pattern(e)?;
                            elements.push(Some(pattern));
                        }
                        Some(ArrayElement::Spread(s)) => {
                            let pattern = self.expr_to_pattern(&s.argument)?;
                            elements.push(Some(Pattern::Rest(RestElement {
                                argument: Box::new(pattern),
                                type_annotation: None,
                                span: s.span,
                            })));
                        }
                    }
                }
                Ok(Pattern::Array(crate::ast::ArrayPattern {
                    elements,
                    type_annotation: None,
                    span: arr.span,
                }))
            }
            Expression::Object(obj) => {
                use crate::ast::ObjectProperty;
                let mut properties = Vec::new();
                for prop in &obj.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            if p.method {
                                return Err(JsError::syntax_error(
                                    "Method shorthand in destructuring not supported",
                                    p.span.line,
                                    p.span.column,
                                ));
                            }
                            let value = self.expr_to_pattern(&p.value)?;
                            properties.push(crate::ast::ObjectPatternProperty::KeyValue {
                                key: p.key.clone(),
                                value,
                                shorthand: p.shorthand,
                                span: p.span,
                            });
                        }
                        ObjectProperty::Spread(s) => {
                            let pattern = self.expr_to_pattern(&s.argument)?;
                            properties.push(crate::ast::ObjectPatternProperty::Rest(RestElement {
                                argument: Box::new(pattern),
                                type_annotation: None,
                                span: s.span,
                            }));
                        }
                    }
                }
                Ok(Pattern::Object(crate::ast::ObjectPattern {
                    properties,
                    type_annotation: None,
                    span: obj.span,
                }))
            }
            Expression::Assignment(assign) => {
                // Handle default values: { a = 1 } or [a = 1]
                let left = match &assign.left {
                    AssignmentTarget::Identifier(id) => Pattern::Identifier(id.clone()),
                    AssignmentTarget::Member(m) => {
                        return Err(JsError::syntax_error(
                            "Member expression not allowed in destructuring default",
                            m.span.line,
                            m.span.column,
                        ));
                    }
                    AssignmentTarget::Pattern(p) => p.clone(),
                };
                Ok(Pattern::Assignment(crate::ast::AssignmentPattern {
                    left: Box::new(left),
                    right: assign.right.clone(),
                    span: assign.span,
                }))
            }
            _ => Err(JsError::syntax_error(
                "Invalid destructuring pattern",
                expr.span().line,
                expr.span().column,
            )),
        }
    }

    /// Parse conditional (ternary) expression: test ? consequent : alternate
    fn parse_conditional_expression(&mut self) -> Result<Expression, JsError> {
        let test = self.parse_binary_expression(0)?;

        if !self.check(&TokenKind::Question) {
            return Ok(test);
        }

        self.advance(); // consume ?
        let consequent = self.parse_expression()?; // Allow full expression in consequent
        self.expect(&TokenKind::Colon)?;
        let alternate = self.parse_conditional_expression()?; // Right-associative

        let span = Span::new(
            test.span().start,
            alternate.span().end,
            test.span().line,
            test.span().column,
        );

        Ok(Expression::Conditional(ConditionalExpression {
            test: Rc::new(test),
            consequent: Rc::new(consequent),
            alternate: Rc::new(alternate),
            span,
        }))
    }

    /// Parse binary expression with minimum precedence
    fn parse_binary_expression(&mut self, min_prec: u8) -> Result<Expression, JsError> {
        let mut left = self.parse_unary_expression()?;

        loop {
            // Check for logical operator first (lower precedence)
            if let Some((op, prec)) = self.get_logical_op_and_prec() {
                if prec < min_prec {
                    break;
                }

                self.advance();
                let next_prec = prec + 1; // left-associative
                let right = self.parse_binary_expression(next_prec)?;

                let span = Span::new(
                    left.span().start,
                    right.span().end,
                    left.span().line,
                    left.span().column,
                );

                left = Expression::Logical(LogicalExpression {
                    operator: op,
                    left: Rc::new(left),
                    right: Rc::new(right),
                    span,
                });
                continue;
            }

            // Check for binary operator
            let Some((op, prec)) = self.get_binary_op_and_prec() else {
                break;
            };

            if prec < min_prec {
                break;
            }

            let _op_span = self.current.span;
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

    /// Get logical operator and its precedence from current token
    fn get_logical_op_and_prec(&self) -> Option<(LogicalOp, u8)> {
        match &self.current.kind {
            TokenKind::QuestionQuestion => Some((LogicalOp::NullishCoalescing, 3)),
            TokenKind::PipePipe => Some((LogicalOp::Or, 4)),
            TokenKind::AmpAmp => Some((LogicalOp::And, 5)),
            _ => None,
        }
    }

    /// Parse unary expressions (!, -, +, typeof, void, delete, ++, --)
    fn parse_unary_expression(&mut self) -> Result<Expression, JsError> {
        self.enter_nesting()?;
        let result = self.parse_unary_expression_inner();
        self.exit_nesting();
        result
    }

    fn parse_unary_expression_inner(&mut self) -> Result<Expression, JsError> {
        let span = self.current.span;

        // Check for prefix increment/decrement
        if self.current.kind == TokenKind::PlusPlus || self.current.kind == TokenKind::MinusMinus {
            let update_op = if self.current.kind == TokenKind::PlusPlus {
                UpdateOp::Increment
            } else {
                UpdateOp::Decrement
            };
            self.advance();
            let argument = self.parse_unary_expression()?;
            let full_span = Span::new(
                span.start,
                argument.span().end,
                span.line,
                span.column,
            );
            return Ok(Expression::Update(UpdateExpression {
                operator: update_op,
                argument: Rc::new(argument),
                prefix: true,
                span: full_span,
            }));
        }

        // Check for yield expression
        if self.current.kind == TokenKind::Yield {
            self.advance();
            // Check for yield*
            let delegate = if self.check(&TokenKind::Star) {
                self.advance();
                true
            } else {
                false
            };
            // Check if there's an argument (yield can be standalone in some contexts)
            let argument = if self.check(&TokenKind::Semicolon)
                || self.check(&TokenKind::RBrace)
                || self.check(&TokenKind::RParen)
                || self.check(&TokenKind::RBracket)
                || self.check(&TokenKind::Comma)
                || self.check(&TokenKind::Colon)
                || self.check(&TokenKind::Eof)
            {
                None
            } else {
                Some(Rc::new(self.parse_expression()?))
            };
            let end_pos = argument.as_ref().map_or(span.end, |e| e.span().end);
            return Ok(Expression::Yield(YieldExpression {
                argument,
                delegate,
                span: Span::new(span.start, end_pos, span.line, span.column),
            }));
        }

        // Check for await expression
        if self.current.kind == TokenKind::Await {
            self.advance();
            let argument = self.parse_unary_expression()?;
            let full_span = Span::new(
                span.start,
                argument.span().end,
                span.line,
                span.column,
            );
            return Ok(Expression::Await(AwaitExpression {
                argument: Rc::new(argument),
                span: full_span,
            }));
        }

        // Check for new expression
        if self.current.kind == TokenKind::New {
            self.advance();
            // Parse the callee (can have member access but not call)
            let callee = self.parse_new_callee()?;
            // Parse optional type arguments: new Promise<T>(...)
            let type_arguments = if self.check(&TokenKind::Lt) {
                Some(self.parse_type_arguments()?)
            } else {
                None
            };
            // Parse optional arguments
            let arguments = if self.check(&TokenKind::LParen) {
                self.advance();
                let args = self.parse_call_arguments()?;
                self.expect(&TokenKind::RParen)?;
                args
            } else {
                Vec::new()
            };
            let end_span = self.current.span;
            return Ok(Expression::New(Box::new(NewExpression {
                callee: Rc::new(callee),
                arguments,
                type_arguments,
                span: Span::new(span.start, end_span.start, span.line, span.column),
            })));
        }

        // Check for unary prefix operators
        let op = match &self.current.kind {
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
            TokenKind::Typeof => Some(UnaryOp::Typeof),
            TokenKind::Void => Some(UnaryOp::Void),
            TokenKind::Delete => Some(UnaryOp::Delete),
            _ => None,
        };

        if let Some(op) = op {
            self.advance();
            let argument = self.parse_unary_expression()?; // Recursive for multiple prefixes
            let full_span = Span::new(
                span.start,
                argument.span().end,
                span.line,
                span.column,
            );
            return Ok(Expression::Unary(UnaryExpression {
                operator: op,
                argument: Rc::new(argument),
                prefix: true,
                span: full_span,
            }));
        }

        self.parse_postfix_expression()
    }

    /// Parse postfix expressions (type assertions, member access, calls, etc.)
    fn parse_postfix_expression(&mut self) -> Result<Expression, JsError> {
        let mut expr = self.parse_primary_expression()?;
        let start_span = expr.span();
        let mut has_optional_call = false;

        // Handle postfix operations: member access, calls, type assertions
        loop {
            match &self.current.kind {
                // Member access: expr.prop or expr.#privateProp
                TokenKind::Dot => {
                    self.advance();
                    let prop_span = self.current.span;
                    let property = match &self.current.kind {
                        TokenKind::Identifier(name) => {
                            let name = name.cheap_clone();
                            self.advance();
                            MemberProperty::Identifier(Identifier { name, span: prop_span })
                        }
                        // Private member access: expr.#name
                        TokenKind::Hash => {
                            self.advance(); // consume #
                            let id_span = self.current.span;
                            match &self.current.kind {
                                TokenKind::Identifier(name) => {
                                    // Include the # in the name
                                    let name_with_hash = self.lexer.string_dict().get_or_insert(&format!("#{}", name.as_str()));
                                    self.advance();
                                    let full_span = Span::new(prop_span.start, id_span.end, prop_span.line, prop_span.column);
                                    MemberProperty::PrivateIdentifier(Identifier { name: name_with_hash, span: full_span })
                                }
                                _ => {
                                    return Err(JsError::syntax_error(
                                        format!("Expected identifier after #, found {:?}", self.current.kind),
                                        self.current.span.line,
                                        self.current.span.column,
                                    ));
                                }
                            }
                        }
                        // Keywords that can be used as property names
                        _ => {
                            if let Some(name) = self.keyword_as_property_name() {
                                self.advance();
                                MemberProperty::Identifier(Identifier { name, span: prop_span })
                            } else {
                                return Err(JsError::syntax_error(
                                    format!("Expected property name, found {:?}", self.current.kind),
                                    self.current.span.line,
                                    self.current.span.column,
                                ));
                            }
                        }
                    };
                    let end_span = self.current.span;
                    let expr_span = expr.span();
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property,
                        computed: false,
                        optional: false,
                        span: Span::new(expr_span.start, end_span.start, expr_span.line, expr_span.column),
                    }));
                }
                // Optional chaining: expr?.prop or expr?.[key] or expr?.(args)
                TokenKind::QuestionDot => {
                    self.advance();

                    // Check what follows: identifier, [, or (
                    if self.check(&TokenKind::LBracket) {
                        // Optional computed member: expr?.[key]
                        self.advance();
                        let key = self.parse_expression()?;
                        let end_span = self.expect(&TokenKind::RBracket)?;
                        let expr_span = expr.span();
                        expr = Expression::Member(Box::new(MemberExpression {
                            object: Rc::new(expr),
                            property: MemberProperty::Expression(Rc::new(key)),
                            computed: true,
                            optional: true,
                            span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                        }));
                    } else if self.check(&TokenKind::LParen) {
                        // Optional call: expr?.(args)
                        self.advance();
                        let arguments = self.parse_call_arguments()?;
                        let end_span = self.expect(&TokenKind::RParen)?;
                        let expr_span = expr.span();
                        has_optional_call = true;
                        expr = Expression::Call(Box::new(CallExpression {
                            callee: Rc::new(expr),
                            arguments,
                            type_arguments: None,
                            optional: true,
                            span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                        }));
                    } else {
                        // Optional member: expr?.prop
                        let prop_span = self.current.span;
                        let property = match &self.current.kind {
                            TokenKind::Identifier(name) => {
                                let name = name.cheap_clone();
                                self.advance();
                                MemberProperty::Identifier(Identifier { name, span: prop_span })
                            }
                            _ => {
                                return Err(JsError::syntax_error(
                                    format!("Expected property name, found {:?}", self.current.kind),
                                    self.current.span.line,
                                    self.current.span.column,
                                ));
                            }
                        };
                        let end_span = self.current.span;
                        let expr_span = expr.span();
                        expr = Expression::Member(Box::new(MemberExpression {
                            object: Rc::new(expr),
                            property,
                            computed: false,
                            optional: true,
                            span: Span::new(expr_span.start, end_span.start, expr_span.line, expr_span.column),
                        }));
                    }
                }
                // Computed member access: expr[key]
                TokenKind::LBracket => {
                    self.advance();
                    let key = self.parse_expression()?;
                    let end_span = self.expect(&TokenKind::RBracket)?;
                    let expr_span = expr.span();
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property: MemberProperty::Expression(Rc::new(key)),
                        computed: true,
                        optional: false,
                        span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                    }));
                }
                // Function call: expr(args)
                TokenKind::LParen => {
                    self.advance();
                    let arguments = self.parse_call_arguments()?;
                    let end_span = self.expect(&TokenKind::RParen)?;
                    let expr_span = expr.span();
                    expr = Expression::Call(Box::new(CallExpression {
                        callee: Rc::new(expr),
                        arguments,
                        type_arguments: None,
                        optional: false,
                        span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                    }));
                }
                // Type arguments followed by call: expr<T>(args)
                TokenKind::Lt => {
                    // Check if this looks like type arguments followed by `(`
                    // by using lookahead
                    if self.looks_like_type_arguments_call() {
                        let type_arguments = Some(self.parse_type_arguments()?);
                        self.expect(&TokenKind::LParen)?;
                        let arguments = self.parse_call_arguments()?;
                        let end_span = self.expect(&TokenKind::RParen)?;
                        let expr_span = expr.span();
                        expr = Expression::Call(Box::new(CallExpression {
                            callee: Rc::new(expr),
                            arguments,
                            type_arguments,
                            optional: false,
                            span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                        }));
                    } else {
                        // Not type arguments - let binary expression parsing handle it
                        break;
                    }
                }
                // Type assertion: expr as Type
                TokenKind::As => {
                    self.advance();
                    let type_ann = self.parse_type_annotation()?;
                    let type_end = self.current.span;
                    let expr_span = expr.span();
                    let span = Span::new(
                        expr_span.start,
                        type_end.start,
                        expr_span.line,
                        expr_span.column,
                    );
                    expr = Expression::TypeAssertion(TypeAssertionExpression {
                        expression: Rc::new(expr),
                        type_annotation: Box::new(type_ann),
                        span,
                    });
                }
                // Postfix increment: expr++
                TokenKind::PlusPlus => {
                    let end_span = self.current.span;
                    self.advance();
                    let expr_span = expr.span();
                    expr = Expression::Update(UpdateExpression {
                        operator: UpdateOp::Increment,
                        argument: Rc::new(expr),
                        prefix: false,
                        span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                    });
                }
                // Postfix decrement: expr--
                TokenKind::MinusMinus => {
                    let end_span = self.current.span;
                    self.advance();
                    let expr_span = expr.span();
                    expr = Expression::Update(UpdateExpression {
                        operator: UpdateOp::Decrement,
                        argument: Rc::new(expr),
                        prefix: false,
                        span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                    });
                }
                // Tagged template: expr`template`
                TokenKind::TemplateTail(s) | TokenKind::TemplateNoSub(s) => {
                    let value = s.cheap_clone();
                    let template_span = self.current.span;
                    self.advance();
                    let quasi = TemplateLiteral {
                        quasis: vec![TemplateElement {
                            value,
                            tail: true,
                            span: template_span,
                        }],
                        expressions: vec![],
                        span: template_span,
                    };
                    let expr_span = expr.span();
                    expr = Expression::TaggedTemplate(Box::new(TaggedTemplateExpression {
                        tag: Rc::new(expr),
                        quasi,
                        span: Span::new(expr_span.start, template_span.end, expr_span.line, expr_span.column),
                    }));
                }
                TokenKind::TemplateHead(s) => {
                    let head_value = s.cheap_clone();
                    let start_span = self.current.span;
                    let expr_span = expr.span();
                    // Parse the template with substitutions
                    let quasi_expr = self.parse_template_literal(head_value, start_span)?;
                    if let Expression::Template(template_box) = quasi_expr {
                        let quasi = *template_box;
                        let end_span = quasi.span;
                        expr = Expression::TaggedTemplate(Box::new(TaggedTemplateExpression {
                            tag: Rc::new(expr),
                            quasi,
                            span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                        }));
                    } else {
                        return Err(JsError::syntax_error(
                            "Expected template literal",
                            start_span.line,
                            start_span.column,
                        ));
                    }
                }
                // Non-null assertion: expr!
                TokenKind::Bang => {
                    let end_span = self.current.span;
                    self.advance();
                    let expr_span = expr.span();
                    expr = Expression::NonNull(NonNullExpression {
                        expression: Rc::new(expr),
                        span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                    });
                }
                _ => break,
            }
        }

        // If we had an optional call, wrap in OptionalChain
        if has_optional_call {
            let end_span = expr.span();
            expr = Expression::OptionalChain(OptionalChainExpression {
                base: Rc::new(expr),
                span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
            });
        }

        Ok(expr)
    }

    /// Parse the callee for a new expression (primary with member access, but not calls)
    fn parse_new_callee(&mut self) -> Result<Expression, JsError> {
        let mut expr = self.parse_primary_expression()?;

        // Handle member access only (no calls)
        loop {
            match &self.current.kind {
                TokenKind::Dot => {
                    self.advance();
                    let prop_span = self.current.span;
                    let property = match &self.current.kind {
                        TokenKind::Identifier(name) => {
                            let name = name.cheap_clone();
                            self.advance();
                            MemberProperty::Identifier(Identifier { name, span: prop_span })
                        }
                        _ => {
                            return Err(JsError::syntax_error(
                                format!("Expected property name, found {:?}", self.current.kind),
                                self.current.span.line,
                                self.current.span.column,
                            ));
                        }
                    };
                    let end_span = self.current.span;
                    let expr_span = expr.span();
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property,
                        computed: false,
                        optional: false,
                        span: Span::new(expr_span.start, end_span.start, expr_span.line, expr_span.column),
                    }));
                }
                TokenKind::LBracket => {
                    self.advance();
                    let key = self.parse_expression()?;
                    let end_span = self.expect(&TokenKind::RBracket)?;
                    let expr_span = expr.span();
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property: MemberProperty::Expression(Rc::new(key)),
                        computed: true,
                        optional: false,
                        span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
                    }));
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse call arguments
    fn parse_call_arguments(&mut self) -> Result<Vec<Argument>, JsError> {
        let mut args = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let expr = self.parse_expression()?;
            args.push(Argument::Expression(expr));

            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(args)
    }

    /// Parse a type annotation
    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, JsError> {
        // Track span manually since TypeAnnotation doesn't have span() method
        let start_span = self.current.span;

        // Handle leading pipe for union types: | A | B
        if self.check(&TokenKind::Pipe) {
            self.advance(); // consume leading '|'
            let first_type = self.parse_primary_type()?;
            // Handle array suffix for first type
            let mut first_member = first_type;
            while self.check(&TokenKind::LBracket) {
                self.advance();
                let end_span = self.expect(&TokenKind::RBracket)?;
                first_member = TypeAnnotation::Array(ArrayType {
                    element_type: Box::new(first_member),
                    span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                });
            }

            let mut types = vec![first_member];
            while self.check(&TokenKind::Pipe) {
                self.advance(); // consume '|'
                let next_type = self.parse_primary_type()?;
                let mut member = next_type;
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    let end_span = self.expect(&TokenKind::RBracket)?;
                    member = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(member),
                        span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                    });
                }
                types.push(member);
            }
            let end_span = self.current.span;
            return Ok(TypeAnnotation::Union(crate::ast::UnionType {
                types,
                span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
            }));
        }

        // Parse the primary type first
        let mut type_ann = self.parse_primary_type()?;

        // Check for array suffix (T[]) or indexed access type (T["key"])
        while self.check(&TokenKind::LBracket) {
            self.advance(); // consume '['
            if self.check(&TokenKind::RBracket) {
                // Empty brackets: array type T[]
                let end_span = self.expect(&TokenKind::RBracket)?; // consume ']'
                type_ann = TypeAnnotation::Array(ArrayType {
                    element_type: Box::new(type_ann),
                    span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                });
            } else {
                // Has content: indexed access type T["key"] or T[K]
                let index_type = self.parse_type_annotation()?;
                let end_span = self.expect(&TokenKind::RBracket)?;
                type_ann = TypeAnnotation::Indexed(IndexedAccessType {
                    object_type: Box::new(type_ann),
                    index_type: Box::new(index_type),
                    span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                });
            }
        }

        // Check for union type: T | U | V
        if self.check(&TokenKind::Pipe) {
            let mut types = vec![type_ann];
            while self.check(&TokenKind::Pipe) {
                self.advance(); // consume '|'
                let next_type = self.parse_primary_type()?;
                // Handle array suffix for union member
                let mut member = next_type;
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    let end_span = self.expect(&TokenKind::RBracket)?;
                    member = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(member),
                        span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                    });
                }
                types.push(member);
            }
            let end_span = self.current.span;
            return Ok(TypeAnnotation::Union(crate::ast::UnionType {
                types,
                span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
            }));
        }

        // Check for intersection type: T & U & V
        if self.check(&TokenKind::Amp) {
            let mut types = vec![type_ann];
            while self.check(&TokenKind::Amp) {
                self.advance(); // consume '&'
                let next_type = self.parse_primary_type()?;
                // Handle array suffix for intersection member
                let mut member = next_type;
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    let end_span = self.expect(&TokenKind::RBracket)?;
                    member = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(member),
                        span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                    });
                }
                types.push(member);
            }
            let end_span = self.current.span;
            return Ok(TypeAnnotation::Intersection(IntersectionType {
                types,
                span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
            }));
        }

        Ok(type_ann)
    }

    /// Parse a primary type (identifier, keyword, object type)
    fn parse_primary_type(&mut self) -> Result<TypeAnnotation, JsError> {
        use crate::ast::{TypeKeyword, TypeKeywordKind, TypeReference};

        let span = self.current.span;
        match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name_str = name.as_str();
                // Check for identifier-based type keywords: number, string, boolean
                let keyword_kind = match name_str {
                    "number" => Some(TypeKeywordKind::Number),
                    "string" => Some(TypeKeywordKind::String),
                    "boolean" => Some(TypeKeywordKind::Boolean),
                    "object" => Some(TypeKeywordKind::Object),
                    "symbol" => Some(TypeKeywordKind::Symbol),
                    "bigint" => Some(TypeKeywordKind::BigInt),
                    "undefined" => Some(TypeKeywordKind::Undefined),
                    _ => None,
                };

                if let Some(keyword) = keyword_kind {
                    self.advance();
                    Ok(TypeAnnotation::Keyword(TypeKeyword { keyword, span }))
                } else {
                    let name = name.cheap_clone();
                    self.advance();

                    // Check for type arguments: TypeName<T, U>
                    let type_arguments = if self.check(&TokenKind::Lt) {
                        Some(self.parse_type_arguments()?)
                    } else {
                        None
                    };

                    let end_span = self.current.span;
                    Ok(TypeAnnotation::Reference(TypeReference {
                        name: Identifier { name, span },
                        type_arguments,
                        span: Span::new(span.start, end_span.start, span.line, span.column),
                    }))
                }
            }
            // Object type: { a: T; b: U } or Mapped type: { [P in K]: T }
            TokenKind::LBrace => {
                self.advance(); // consume '{'

                // Check for mapped type: { readonly? [P in K]: T } or { [P in K]: T }
                // But NOT index signature: { [key: string]: T } (has : after identifier, not in)
                let is_mapped = if self.check(&TokenKind::Readonly)
                    || self.check(&TokenKind::Plus)
                    || self.check(&TokenKind::Minus)
                {
                    // These modifiers only appear in mapped types
                    true
                } else if self.check(&TokenKind::LBracket) {
                    // Need to look ahead: [P in K] vs [key: T]
                    // Mapped type has 'in' after identifier, index signature has ':'
                    let checkpoint = self.lexer.checkpoint();
                    let current_saved = self.current.clone();
                    self.advance(); // consume '['
                    self.advance(); // consume identifier
                    let is_in = self.check(&TokenKind::In);
                    self.lexer.restore(checkpoint);
                    self.current = current_saved;
                    is_in
                } else {
                    false
                };

                if is_mapped {
                    self.parse_mapped_type(span)
                } else {
                    let members = self.parse_type_members()?;
                    let end_span = self.expect(&TokenKind::RBrace)?;
                    Ok(TypeAnnotation::Object(ObjectType {
                        members,
                        span: Span::new(span.start, end_span.end, span.line, span.column),
                    }))
                }
            }
            // Keyword types (lexer-recognized)
            TokenKind::Any => {
                self.advance();
                Ok(TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Any,
                    span,
                }))
            }
            TokenKind::Unknown => {
                self.advance();
                Ok(TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Unknown,
                    span,
                }))
            }
            TokenKind::Never => {
                self.advance();
                Ok(TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Never,
                    span,
                }))
            }
            TokenKind::Void => {
                self.advance();
                Ok(TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Void,
                    span,
                }))
            }
            TokenKind::Null => {
                self.advance();
                Ok(TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Null,
                    span,
                }))
            }
            // String literal type: 'active' | 'inactive'
            TokenKind::String(s) => {
                let value = s.cheap_clone();
                self.advance();
                Ok(TypeAnnotation::Literal(crate::ast::TypeLiteral {
                    value: LiteralValue::String(value),
                    span,
                }))
            }
            // Number literal type: 1 | 2 | 3
            TokenKind::Number(n) => {
                let value = *n;
                self.advance();
                Ok(TypeAnnotation::Literal(crate::ast::TypeLiteral {
                    value: LiteralValue::Number(value),
                    span,
                }))
            }
            // Boolean literal types
            TokenKind::True => {
                self.advance();
                Ok(TypeAnnotation::Literal(crate::ast::TypeLiteral {
                    value: LiteralValue::Boolean(true),
                    span,
                }))
            }
            TokenKind::False => {
                self.advance();
                Ok(TypeAnnotation::Literal(crate::ast::TypeLiteral {
                    value: LiteralValue::Boolean(false),
                    span,
                }))
            }
            // Parenthesized type or function type: (T) or () => T or (a: T) => U
            TokenKind::LParen => {
                let paren_span = self.current.span;
                self.advance();

                // Check for empty parens - function type with no params
                if self.check(&TokenKind::RParen) {
                    self.advance();
                    // This should be followed by => for a function type
                    if self.check(&TokenKind::Arrow) {
                        self.advance();
                        let return_type = self.parse_type_annotation()?;
                        let end_span = self.current.span;
                        return Ok(TypeAnnotation::Function(crate::ast::FunctionType {
                            params: Vec::new(),
                            return_type: Box::new(return_type),
                            type_parameters: None,
                            span: Span::new(paren_span.start, end_span.start, paren_span.line, paren_span.column),
                        }));
                    }
                    return Err(JsError::syntax_error(
                        "Expected '=>' after empty parens in type",
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }

                // Check if this looks like function parameters: identifier followed by : or ?
                // vs a parenthesized type
                let is_function_params = {
                    let checkpoint = self.lexer.checkpoint();
                    let current_saved = self.current.clone();
                    // Check if first element is identifier followed by : or ? or ,
                    let looks_like_param = if let TokenKind::Identifier(_) = &self.current.kind {
                        self.advance();
                        self.check(&TokenKind::Colon) || self.check(&TokenKind::Question) || self.check(&TokenKind::Comma)
                    } else if self.check(&TokenKind::DotDotDot) {
                        // Rest parameter
                        true
                    } else {
                        false
                    };
                    self.lexer.restore(checkpoint);
                    self.current = current_saved;
                    looks_like_param
                };

                if is_function_params {
                    // Parse function type parameters
                    let params = self.parse_function_type_params()?;
                    self.expect(&TokenKind::RParen)?;
                    self.expect(&TokenKind::Arrow)?;
                    let return_type = self.parse_type_annotation()?;
                    let end_span = self.current.span;
                    return Ok(TypeAnnotation::Function(crate::ast::FunctionType {
                        params,
                        return_type: Box::new(return_type),
                        type_parameters: None,
                        span: Span::new(paren_span.start, end_span.start, paren_span.line, paren_span.column),
                    }));
                }

                // Parse inner type (parenthesized type)
                let inner = self.parse_type_annotation()?;
                self.expect(&TokenKind::RParen)?;

                // Check for function type: (T) => U
                if self.check(&TokenKind::Arrow) {
                    self.advance();
                    let return_type = self.parse_type_annotation()?;
                    let end_span = self.current.span;
                    return Ok(TypeAnnotation::Function(crate::ast::FunctionType {
                        params: Vec::new(),
                        return_type: Box::new(return_type),
                        type_parameters: None,
                        span: Span::new(paren_span.start, end_span.start, paren_span.line, paren_span.column),
                    }));
                }

                // Just a parenthesized type - return inner
                Ok(inner)
            }
            // keyof T
            TokenKind::Keyof => {
                self.advance();
                let type_ann = self.parse_primary_type()?;
                let end_span = self.current.span;
                Ok(TypeAnnotation::Keyof(KeyofType {
                    type_annotation: Box::new(type_ann),
                    span: Span::new(span.start, end_span.start, span.line, span.column),
                }))
            }
            // typeof x
            TokenKind::Typeof => {
                self.advance();
                // In type context, typeof takes an identifier
                let (name, id_span) = self.expect_identifier()?;
                let end_span = self.current.span;
                Ok(TypeAnnotation::Typeof(crate::ast::TypeofType {
                    expression: Identifier { name, span: id_span },
                    span: Span::new(span.start, end_span.start, span.line, span.column),
                }))
            }
            // Tuple type: [T1, T2, T3]
            TokenKind::LBracket => {
                self.advance(); // consume '['
                let mut element_types = Vec::new();
                while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                    let element_type = self.parse_type_annotation()?;
                    element_types.push(element_type);
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let end_span = self.expect(&TokenKind::RBracket)?;
                Ok(TypeAnnotation::Tuple(crate::ast::TupleType {
                    element_types,
                    span: Span::new(span.start, end_span.end, span.line, span.column),
                }))
            }
            _ => Err(JsError::syntax_error(
                format!("Expected type, found {:?}", self.current.kind),
                self.current.span.line,
                self.current.span.column,
            )),
        }
    }

    /// Parse type arguments: <T, U, V>
    fn parse_type_arguments(&mut self) -> Result<crate::ast::TypeArguments, JsError> {
        let start_span = self.current.span;
        self.expect(&TokenKind::Lt)?; // consume '<'

        let mut params = Vec::new();

        while !self.check(&TokenKind::Gt) && !self.check(&TokenKind::GtGt) && !self.check(&TokenKind::Eof) {
            let type_ann = self.parse_type_annotation()?;
            params.push(type_ann);

            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        // Handle >> as two > tokens (common in nested generics like Promise<Array<T>>)
        let end_span = if self.check(&TokenKind::GtGt) {
            // Split >> into two > by consuming just the first >
            // We update the current token to be Gt at the position of the second >
            let span = self.current.span;
            let new_span = Span::new(span.start + 1, span.end, span.line, span.column + 1);
            self.current = Token { kind: TokenKind::Gt, span: new_span };
            Span::new(span.start, span.start + 1, span.line, span.column)
        } else if self.check(&TokenKind::GtGtGt) {
            // Handle >>> similarly
            let span = self.current.span;
            let new_span = Span::new(span.start + 1, span.end, span.line, span.column + 1);
            self.current = Token { kind: TokenKind::GtGt, span: new_span };
            Span::new(span.start, span.start + 1, span.line, span.column)
        } else {
            self.expect(&TokenKind::Gt)? // consume '>'
        };

        Ok(crate::ast::TypeArguments {
            params,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        })
    }

    /// Parse function type parameters: (a: T, b?: U, ...rest: V[])
    /// Used in function type annotations like (item: T) => string
    fn parse_function_type_params(&mut self) -> Result<Vec<FunctionParam>, JsError> {
        let mut params = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let param_span = self.current.span;

            // Check for rest parameter: ...param
            let is_rest = self.check(&TokenKind::DotDotDot);
            if is_rest {
                self.advance(); // consume ...
            }

            // Parse parameter name
            let inner_pattern = if let Some((name, span)) = self.try_get_identifier_name() {
                Pattern::Identifier(Identifier { name, span })
            } else {
                return Err(JsError::syntax_error(
                    format!("Expected parameter name in function type, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                ));
            };

            // Check for optional marker '?'
            let optional = if self.check(&TokenKind::Question) {
                self.advance();
                true
            } else {
                false
            };

            // Parse type annotation (required for function type params - colon and type)
            let type_annotation = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let end_span = self.current.span;
            let full_span = Span::new(param_span.start, end_span.start, param_span.line, param_span.column);

            // Create pattern (with rest wrapper if needed)
            let pattern = if is_rest {
                Pattern::Rest(RestElement {
                    argument: Box::new(inner_pattern),
                    type_annotation: type_annotation.clone(),
                    span: full_span,
                })
            } else {
                inner_pattern
            };

            params.push(FunctionParam {
                pattern,
                type_annotation,
                optional,
                decorators: Vec::new(),
                accessibility: None,
                readonly: false,
                span: full_span,
            });

            // Check for comma
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }

    /// Parse a mapped type: { [P in K]: T } or { readonly [P in K]?: T }
    fn parse_mapped_type(&mut self, start_span: Span) -> Result<TypeAnnotation, JsError> {
        use crate::ast::{MappedType, MappedTypeModifier};

        // Parse optional readonly modifier: readonly, +readonly, -readonly
        let readonly = if self.check(&TokenKind::Plus) {
            self.advance();
            if self.check(&TokenKind::Readonly) {
                self.advance();
                Some(MappedTypeModifier::Add)
            } else {
                None
            }
        } else if self.check(&TokenKind::Minus) {
            self.advance();
            if self.check(&TokenKind::Readonly) {
                self.advance();
                Some(MappedTypeModifier::Remove)
            } else {
                None
            }
        } else if self.check(&TokenKind::Readonly) {
            self.advance();
            Some(MappedTypeModifier::Add)
        } else {
            None
        };

        // Expect '[' for the type parameter
        self.expect(&TokenKind::LBracket)?;

        // Parse type parameter name: P
        let (param_name, param_span) = self.expect_identifier()?;

        // Expect 'in'
        self.expect(&TokenKind::In)?;

        // Parse constraint type: K or keyof T
        let constraint = self.parse_type_annotation()?;

        // Check for 'as' clause (name remapping): [P in K as NewName]
        let name_type = if self.check(&TokenKind::As) {
            self.advance();
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        // Expect ']'
        self.expect(&TokenKind::RBracket)?;

        // Parse optional modifier: ?, +?, -?
        let optional = if self.check(&TokenKind::Plus) {
            self.advance();
            if self.check(&TokenKind::Question) {
                self.advance();
                Some(MappedTypeModifier::Add)
            } else {
                None
            }
        } else if self.check(&TokenKind::Minus) {
            self.advance();
            if self.check(&TokenKind::Question) {
                self.advance();
                Some(MappedTypeModifier::Remove)
            } else {
                None
            }
        } else if self.check(&TokenKind::Question) {
            self.advance();
            Some(MappedTypeModifier::Add)
        } else {
            None
        };

        // Expect ':'
        self.expect(&TokenKind::Colon)?;

        // Parse the value type
        let type_annotation = Some(Box::new(self.parse_type_annotation()?));

        // Consume optional semicolon
        self.eat(&TokenKind::Semicolon);

        // Expect '}'
        let end_span = self.expect(&TokenKind::RBrace)?;

        Ok(TypeAnnotation::Mapped(MappedType {
            type_parameter: TypeParameter {
                name: Identifier { name: param_name, span: param_span },
                constraint: Some(Box::new(constraint)),
                default: None,
                span: param_span,
            },
            name_type,
            type_annotation,
            readonly,
            optional,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        }))
    }

    /// Parse type members inside an object type: { a: T; b: U }
    fn parse_type_members(&mut self) -> Result<Vec<TypeMember>, JsError> {
        use crate::ast::{PropertySignature, TypeMember};

        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let member_span = self.current.span;

            // Check for index signature: [key: string]: Type
            if self.check(&TokenKind::LBracket) {
                self.advance(); // consume '['
                let (key_name, key_span) = self.expect_identifier()?;
                self.expect(&TokenKind::Colon)?;
                let key_type = Box::new(self.parse_type_annotation()?);
                self.expect(&TokenKind::RBracket)?;
                self.expect(&TokenKind::Colon)?;
                let value_type = Box::new(self.parse_type_annotation()?);
                let end_span = self.current.span;
                members.push(TypeMember::Index(IndexSignature {
                    key: Identifier { name: key_name, span: key_span },
                    key_type,
                    value_type,
                    readonly: false,
                    span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
                }));
                // Consume semicolon if present
                self.eat(&TokenKind::Semicolon);
                continue;
            }

            // Parse the property key (identifier, string, or keyword that can be used as property name)
            let key = match &self.current.kind {
                TokenKind::Identifier(name) => {
                    let name = name.cheap_clone();
                    self.advance();
                    ObjectPropertyKey::Identifier(Identifier { name, span: member_span })
                }
                TokenKind::String(s) => {
                    let value = s.cheap_clone();
                    self.advance();
                    ObjectPropertyKey::String(StringLiteral { value, span: member_span })
                }
                // Allow keywords as property names
                TokenKind::Type => {
                    self.advance();
                    ObjectPropertyKey::Identifier(Identifier {
                        name: self.lexer.string_dict().get_or_insert("type"),
                        span: member_span,
                    })
                }
                _ => {
                    return Err(JsError::syntax_error(
                        format!("Expected property name in type, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
            };

            // Check for optional marker '?'
            let optional = if self.check(&TokenKind::Question) {
                self.advance();
                true
            } else {
                false
            };

            // Expect ':'
            self.expect(&TokenKind::Colon)?;

            // Parse type annotation
            let type_annotation = self.parse_type_annotation()?;
            let end_span = self.current.span;

            members.push(TypeMember::Property(PropertySignature {
                key,
                type_annotation: Some(Box::new(type_annotation)),
                optional,
                readonly: false,
                span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
            }));

            // Consume semicolon or comma separator if present
            if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        Ok(members)
    }

    // ========================================================================
    // Function parsing
    // ========================================================================

    /// Parse a function declaration: function name(params): returnType { body }
    fn parse_function_declaration(
        &mut self,
        start_span: Span,
        is_async: bool,
    ) -> Result<FunctionDeclaration, JsError> {
        // Check for generator
        let generator = if self.check(&TokenKind::Star) {
            self.advance();
            true
        } else {
            false
        };

        // Parse function name
        let id = match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                let span = self.current.span;
                self.advance();
                Some(Identifier { name, span })
            }
            _ => {
                return Err(JsError::syntax_error(
                    format!("Expected function name, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                ));
            }
        };

        // Parse optional type parameters: <T, U>
        let type_parameters = if self.check(&TokenKind::Lt) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };

        // Parse parameters
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_function_params()?;
        self.expect(&TokenKind::RParen)?;

        // Parse optional return type: : Type
        let return_type = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        // Parse function body
        let body = self.parse_block_statement()?;
        let end_span = self.current.span;

        Ok(FunctionDeclaration {
            id,
            params: Rc::from(params),
            return_type,
            type_parameters,
            body: Rc::new(body),
            generator,
            async_: is_async,
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })
    }

    /// Parse a class declaration
    fn parse_class_declaration(&mut self, start_span: Span) -> Result<ClassDeclaration, JsError> {
        // Parse class name
        let id = match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                let span = self.current.span;
                self.advance();
                Some(Identifier { name, span })
            }
            _ => None,
        };

        // Parse optional extends clause
        let super_class = if self.check(&TokenKind::Extends) {
            self.advance();
            Some(Rc::new(self.parse_expression()?))
        } else {
            None
        };

        // Parse optional implements clause (skip for now)
        if let TokenKind::Identifier(name) = &self.current.kind {
            if name.as_str() == "implements" {
                self.advance();
                // Skip the implements list - parse as identifiers separated by commas
                loop {
                    if let TokenKind::Identifier(_) = &self.current.kind {
                        self.advance();
                    }
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Parse class body
        let body_start = self.current.span;
        self.expect(&TokenKind::LBrace)?;
        let members = self.parse_class_members()?;
        let body_end = self.expect(&TokenKind::RBrace)?;

        let body = ClassBody {
            members,
            span: Span::new(body_start.start, body_end.end, body_start.line, body_start.column),
        };

        Ok(ClassDeclaration {
            id,
            type_parameters: None,
            super_class,
            implements: Vec::new(),
            body,
            decorators: Vec::new(),
            abstract_: false,
            span: Span::new(start_span.start, body_end.end, start_span.line, start_span.column),
        })
    }

    /// Parse a class expression (in expression context)
    fn parse_class_expression(
        &mut self,
        start_span: Span,
        decorators: Vec<Decorator>,
    ) -> Result<ClassExpression, JsError> {

        // Parse optional class name
        let id = match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                let span = self.current.span;
                self.advance();
                Some(Identifier { name, span })
            }
            _ => None,
        };

        // Parse optional extends clause
        let super_class = if self.check(&TokenKind::Extends) {
            self.advance();
            Some(Rc::new(self.parse_expression()?))
        } else {
            None
        };

        // Parse optional implements clause
        if let TokenKind::Identifier(name) = &self.current.kind {
            if name.as_str() == "implements" {
                self.advance();
                // Skip the implements list
                loop {
                    if let TokenKind::Identifier(_) = &self.current.kind {
                        self.advance();
                    }
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Parse class body
        let body_start = self.current.span;
        self.expect(&TokenKind::LBrace)?;
        let members = self.parse_class_members()?;
        let body_end = self.expect(&TokenKind::RBrace)?;

        let body = ClassBody {
            members,
            span: Span::new(body_start.start, body_end.end, body_start.line, body_start.column),
        };

        Ok(ClassExpression {
            id,
            type_parameters: None,
            super_class,
            implements: Vec::new(),
            body,
            decorators,
            span: Span::new(start_span.start, body_end.end, start_span.line, start_span.column),
        })
    }

    /// Parse class members
    fn parse_class_members(&mut self) -> Result<Vec<ClassMember>, JsError> {
        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            // Parse any decorators for this member
            let decorators = if self.check(&TokenKind::At) {
                self.parse_decorators()?
            } else {
                Vec::new()
            };

            let member_span = self.current.span;
            let mut is_static = false;
            let mut method_kind = MethodKind::Method;

            // Handle the static keyword (lexed as TokenKind::Static, not Identifier)
            if self.check(&TokenKind::Static) {
                // Consume "static" and see what follows
                self.advance();

                // If followed by {, it's a static block
                if self.check(&TokenKind::LBrace) {
                    let block = self.parse_block_statement()?;
                    members.push(ClassMember::StaticBlock(block));
                    continue;
                }

                // If followed by (, :, =, or ;, then "static" was the member name
                if self.check(&TokenKind::LParen) || self.check(&TokenKind::Colon)
                   || self.check(&TokenKind::Eq) || self.check(&TokenKind::Semicolon) {
                    let key = ObjectPropertyKey::Identifier(Identifier {
                        name: self.lexer.string_dict().get_or_insert("static"),
                        span: member_span,
                    });
                    let member = self.parse_class_member_with_key(member_span, key, false, false, MethodKind::Method, decorators.clone())?;
                    members.push(member);
                    continue;
                }

                // Otherwise "static" is a modifier
                is_static = true;
            }

            // Check for static modifier - save name to check if this is "static" keyword (identifier case)
            let first_name = if let TokenKind::Identifier(name) = &self.current.kind {
                Some(name.cheap_clone())
            } else {
                None
            };

            // If first identifier is "static", we need to determine if it's:
            // 1. A modifier for another member
            // 2. A static block: static { ... }
            // 3. A method or property named "static"
            if let Some(ref name) = first_name {
                if name.as_str() == "static" {
                    // Consume "static" and see what follows
                    self.advance();

                    // If followed by {, it's a static block
                    if self.check(&TokenKind::LBrace) {
                        let block = self.parse_block_statement()?;
                        members.push(ClassMember::StaticBlock(block));
                        continue;
                    }

                    // If followed by (, :, =, or ;, then "static" was the member name
                    if self.check(&TokenKind::LParen) || self.check(&TokenKind::Colon)
                       || self.check(&TokenKind::Eq) || self.check(&TokenKind::Semicolon) {
                        // Rewind: treat "static" as the member name
                        // Since we already consumed it, we'll handle it specially below
                        let key = ObjectPropertyKey::Identifier(Identifier {
                            name: name.cheap_clone(),
                            span: member_span,
                        });
                        let member = self.parse_class_member_with_key(member_span, key, false, false, MethodKind::Method, decorators.clone())?;
                        members.push(member);
                        continue;
                    }

                    // Otherwise "static" is a modifier
                    is_static = true;
                }
            }

            // Check for get/set modifier (only if we haven't already handled static as a member name)
            let current_name = if let TokenKind::Identifier(name) = &self.current.kind {
                Some(name.cheap_clone())
            } else {
                None
            };

            if let Some(ref name) = current_name {
                let name_str = name.as_str();
                if name_str == "get" || name_str == "set" {
                    let saved_span = self.current.span;
                    self.advance();

                    // Check if followed by something that looks like a member key
                    if matches!(self.current.kind, TokenKind::Identifier(_) | TokenKind::String(_) | TokenKind::Number(_) | TokenKind::LBracket) {
                        method_kind = if name_str == "get" {
                            MethodKind::Get
                        } else {
                            MethodKind::Set
                        };
                        // Continue to parse the actual method name
                    } else {
                        // "get" or "set" is the actual member name
                        let key = ObjectPropertyKey::Identifier(Identifier {
                            name: name.cheap_clone(),
                            span: saved_span,
                        });
                        let member = self.parse_class_member_with_key(member_span, key, false, is_static, MethodKind::Method, decorators.clone())?;
                        members.push(member);
                        continue;
                    }
                }
            }

            // Parse the member key
            let (key, computed) = self.parse_class_member_key()?;
            let member = self.parse_class_member_with_key(member_span, key, computed, is_static, method_kind, decorators)?;
            members.push(member);
        }

        Ok(members)
    }

    /// Parse a class member after the key has been parsed
    fn parse_class_member_with_key(
        &mut self,
        member_span: Span,
        key: ObjectPropertyKey,
        computed: bool,
        is_static: bool,
        method_kind: MethodKind,
        decorators: Vec<Decorator>,
    ) -> Result<ClassMember, JsError> {
        // Check if this is a constructor
        let is_constructor = if let ObjectPropertyKey::Identifier(ref id) = key {
            id.name.as_str() == "constructor"
        } else {
            false
        };

        if is_constructor {
            // Parse constructor
            self.expect(&TokenKind::LParen)?;
            let params = self.parse_function_params()?;
            self.expect(&TokenKind::RParen)?;
            let body = self.parse_block_statement()?;
            let end_span = self.current.span;
            Ok(ClassMember::Constructor(Box::new(ClassConstructor {
                params,
                body,
                accessibility: None,
                span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
            })))
        } else if self.check(&TokenKind::LParen) {
            // This is a method
            self.advance();
            let params = self.parse_function_params()?;
            self.expect(&TokenKind::RParen)?;

            // Parse optional return type
            let return_type = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let body = self.parse_block_statement()?;
            let end_span = self.current.span;

            let value = FunctionExpression {
                id: None,
                params: Rc::from(params),
                return_type,
                type_parameters: None,
                body: Rc::new(body),
                generator: false,
                async_: false,
                span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
            };

            Ok(ClassMember::Method(Box::new(ClassMethod {
                key,
                value,
                kind: method_kind,
                computed,
                static_: is_static,
                accessibility: None,
                decorators,
                span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
            })))
        } else {
            // This is a property
            // Parse optional type annotation
            let type_annotation = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            // Parse optional initializer
            let value = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            self.eat(&TokenKind::Semicolon);
            let end_span = self.current.span;

            Ok(ClassMember::Property(Box::new(ClassProperty {
                key,
                value,
                type_annotation,
                computed,
                static_: is_static,
                readonly: false,
                optional: false,
                accessor: false,
                accessibility: None,
                decorators,
                span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
            })))
        }
    }

    /// Parse a class member key (identifier, string, number, computed, or private #name)
    fn parse_class_member_key(&mut self) -> Result<(ObjectPropertyKey, bool), JsError> {
        let span = self.current.span;
        match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                self.advance();
                Ok((ObjectPropertyKey::Identifier(Identifier { name, span }), false))
            }
            // Private identifier: #name
            TokenKind::Hash => {
                self.advance(); // consume #
                let id_span = self.current.span;
                match &self.current.kind {
                    TokenKind::Identifier(name) => {
                        // Include the # in the name
                        let name_with_hash = self.lexer.string_dict().get_or_insert(&format!("#{}", name.as_str()));
                        self.advance();
                        let full_span = Span::new(span.start, id_span.end, span.line, span.column);
                        Ok((ObjectPropertyKey::PrivateIdentifier(Identifier { name: name_with_hash, span: full_span }), false))
                    }
                    _ => Err(JsError::syntax_error(
                        format!("Expected identifier after #, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ))
                }
            }
            TokenKind::String(s) => {
                let value = s.cheap_clone();
                self.advance();
                Ok((ObjectPropertyKey::String(StringLiteral { value, span }), false))
            }
            TokenKind::Number(n) => {
                let value = *n;
                self.advance();
                Ok((ObjectPropertyKey::Number(Literal {
                    value: LiteralValue::Number(value),
                    span,
                }), false))
            }
            TokenKind::LBracket => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RBracket)?;
                Ok((ObjectPropertyKey::Computed(Rc::new(expr)), true))
            }
            _ => Err(JsError::syntax_error(
                format!("Expected class member key, found {:?}", self.current.kind),
                self.current.span.line,
                self.current.span.column,
            )),
        }
    }

    /// Parse an enum declaration
    fn parse_enum_declaration(&mut self, start_span: Span, const_: bool) -> Result<EnumDeclaration, JsError> {
        // Parse enum name
        let id = match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                let span = self.current.span;
                self.advance();
                Identifier { name, span }
            }
            _ => {
                return Err(JsError::syntax_error(
                    format!("Expected enum name, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                ));
            }
        };

        // Parse enum body
        self.expect(&TokenKind::LBrace)?;
        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let member_span = self.current.span;

            // Parse member name
            let member_id = match &self.current.kind {
                TokenKind::Identifier(name) => {
                    let name = name.cheap_clone();
                    let span = self.current.span;
                    self.advance();
                    Identifier { name, span }
                }
                _ => {
                    return Err(JsError::syntax_error(
                        format!("Expected enum member name, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
            };

            // Parse optional initializer
            let initializer = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(self.parse_expression()?)
            } else {
                None
            };

            let member_end = self.current.span;
            members.push(EnumMember {
                id: member_id,
                initializer,
                span: Span::new(member_span.start, member_end.start, member_span.line, member_span.column),
            });

            // Handle comma
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        let end_span = self.expect(&TokenKind::RBrace)?;

        Ok(EnumDeclaration {
            id,
            members,
            const_,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        })
    }

    /// Parse interface declaration: interface Name { members }
    fn parse_interface_declaration(&mut self, start_span: Span) -> Result<InterfaceDeclaration, JsError> {
        // Parse interface name
        let id = match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                let span = self.current.span;
                self.advance();
                Identifier { name, span }
            }
            _ => {
                return Err(JsError::syntax_error(
                    format!("Expected interface name, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                ));
            }
        };

        // Skip type parameters if present
        let type_parameters = if self.check(&TokenKind::Lt) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };

        // Parse extends clause if present
        let extends = if self.check(&TokenKind::Extends) {
            self.advance();
            let mut extends_list = Vec::new();
            loop {
                // Parse type reference
                let type_ref = self.parse_type_reference()?;
                extends_list.push(type_ref);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
            extends_list
        } else {
            Vec::new()
        };

        // Parse interface body
        self.expect(&TokenKind::LBrace)?;
        let body = self.parse_interface_body()?;
        let end_span = self.expect(&TokenKind::RBrace)?;

        Ok(InterfaceDeclaration {
            id,
            type_parameters,
            extends,
            body,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        })
    }

    /// Parse interface body members
    fn parse_interface_body(&mut self) -> Result<Vec<TypeMember>, JsError> {
        let mut members = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let member = self.parse_type_member()?;
            members.push(member);

            // Handle separator (semicolon or comma or newline)
            if self.check(&TokenKind::Semicolon) || self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        Ok(members)
    }

    /// Parse a type member (property signature, method signature, etc.)
    fn parse_type_member(&mut self) -> Result<TypeMember, JsError> {
        let member_span = self.current.span;

        // Check for index signature: [key: string]: value
        if self.check(&TokenKind::LBracket) {
            self.advance(); // consume '['

            // Parse the key name
            let (key_name, key_span) = self.expect_identifier()?;

            // Expect ':'
            self.expect(&TokenKind::Colon)?;

            // Parse the key type
            let key_type = Box::new(self.parse_type_annotation()?);

            // Expect ']'
            self.expect(&TokenKind::RBracket)?;

            // Expect ':'
            self.expect(&TokenKind::Colon)?;

            // Parse the value type
            let value_type = Box::new(self.parse_type_annotation()?);

            // Eat semicolon
            self.eat(&TokenKind::Semicolon);

            let end_span = self.current.span;
            return Ok(TypeMember::Index(IndexSignature {
                key: Identifier { name: key_name, span: key_span },
                key_type,
                value_type,
                readonly: false,
                span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
            }));
        }

        // Parse member name (including contextual keywords that can be property names)
        let key = match &self.current.kind {
            TokenKind::Identifier(s) => {
                let s = s.cheap_clone();
                let span = self.current.span;
                self.advance();
                ObjectPropertyKey::Identifier(Identifier { name: s, span })
            }
            TokenKind::String(s) => {
                let s = s.cheap_clone();
                let span = self.current.span;
                self.advance();
                ObjectPropertyKey::String(StringLiteral { value: s, span })
            }
            // Contextual keywords can be used as property names
            TokenKind::Type => {
                let span = self.current.span;
                self.advance();
                ObjectPropertyKey::Identifier(Identifier {
                    name: self.lexer.string_dict().get_or_insert("type"),
                    span,
                })
            }
            TokenKind::Readonly => {
                let span = self.current.span;
                self.advance();
                ObjectPropertyKey::Identifier(Identifier {
                    name: self.lexer.string_dict().get_or_insert("readonly"),
                    span,
                })
            }
            TokenKind::From => {
                let span = self.current.span;
                self.advance();
                ObjectPropertyKey::Identifier(Identifier {
                    name: self.lexer.string_dict().get_or_insert("from"),
                    span,
                })
            }
            TokenKind::As => {
                let span = self.current.span;
                self.advance();
                ObjectPropertyKey::Identifier(Identifier {
                    name: self.lexer.string_dict().get_or_insert("as"),
                    span,
                })
            }
            _ => {
                return Err(JsError::syntax_error(
                    format!("Expected member name, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                ));
            }
        };

        // Check for optional marker
        let optional = self.eat(&TokenKind::Question);

        // Check if this is a method signature (has parentheses)
        if self.check(&TokenKind::LParen) {
            // Skip params for now - just consume them
            self.advance(); // consume (
            while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                self.advance();
            }
            self.expect(&TokenKind::RParen)?;

            // Parse return type
            let return_type = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let end_span = self.current.span;
            return Ok(TypeMember::Method(MethodSignature {
                key,
                params: Vec::new(), // TODO: parse params properly
                return_type,
                type_parameters: None,
                optional,
                span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
            }));
        }

        // Property signature - expect colon and type
        let type_annotation = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        let end_span = self.current.span;
        Ok(TypeMember::Property(PropertySignature {
            key,
            type_annotation,
            optional,
            readonly: false,
            span: Span::new(member_span.start, end_span.start, member_span.line, member_span.column),
        }))
    }

    /// Parse type reference for extends clause
    fn parse_type_reference(&mut self) -> Result<crate::ast::TypeReference, JsError> {
        let start_span = self.current.span;

        let name = match &self.current.kind {
            TokenKind::Identifier(s) => {
                let s = s.cheap_clone();
                self.advance();
                s
            }
            _ => {
                return Err(JsError::syntax_error(
                    format!("Expected type name, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                ));
            }
        };

        // Parse type arguments if present
        let type_arguments = if self.check(&TokenKind::Lt) {
            Some(self.parse_type_arguments()?)
        } else {
            None
        };

        let end_span = self.current.span;
        Ok(crate::ast::TypeReference {
            name: Identifier { name, span: start_span },
            type_arguments,
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })
    }

    /// Parse type alias declaration: type Name = Type
    fn parse_type_alias_declaration(&mut self, start_span: Span) -> Result<TypeAliasDeclaration, JsError> {
        // Parse type alias name
        let id = match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                let span = self.current.span;
                self.advance();
                Identifier { name, span }
            }
            _ => {
                return Err(JsError::syntax_error(
                    format!("Expected type alias name, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                ));
            }
        };

        // Skip type parameters if present
        let type_parameters = if self.check(&TokenKind::Lt) {
            Some(self.parse_type_parameters()?)
        } else {
            None
        };

        // Expect =
        self.expect(&TokenKind::Eq)?;

        // Parse the type
        let type_annotation = self.parse_type_annotation()?;

        // Handle optional semicolon
        self.eat(&TokenKind::Semicolon);

        let end_span = self.current.span;
        Ok(TypeAliasDeclaration {
            id,
            type_parameters,
            type_annotation: Box::new(type_annotation),
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })
    }

    /// Parse type parameters: <T, U extends V, W = Default>
    fn parse_type_parameters(&mut self) -> Result<TypeParameters, JsError> {
        let start_span = self.current.span;
        self.expect(&TokenKind::Lt)?;

        let mut params = Vec::new();

        while !self.check(&TokenKind::Gt) && !self.check(&TokenKind::Eof) {
            let param_span = self.current.span;

            // Parse type parameter name
            let name = match &self.current.kind {
                TokenKind::Identifier(s) => {
                    let s = s.cheap_clone();
                    let span = self.current.span;
                    self.advance();
                    Identifier { name: s, span }
                }
                _ => {
                    return Err(JsError::syntax_error(
                        format!("Expected type parameter name, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
            };

            // Parse optional constraint: extends Type
            let constraint = if self.check(&TokenKind::Extends) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            // Parse optional default: = Type
            let default = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let end_span = self.current.span;
            params.push(TypeParameter {
                name,
                constraint,
                default,
                span: Span::new(param_span.start, end_span.start, param_span.line, param_span.column),
            });

            // Handle comma separator
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        let end_span = self.expect(&TokenKind::Gt)?;

        Ok(TypeParameters {
            params,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        })
    }

    /// Parse function parameters: (a: T, b: U, c?: V)
    fn parse_function_params(&mut self) -> Result<Vec<FunctionParam>, JsError> {
        let mut params = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let param_span = self.current.span;

            // Check for parameter decorators: @decorator param: Type
            let decorators = if self.check(&TokenKind::At) {
                self.parse_decorators()?
            } else {
                Vec::new()
            };

            // Check for rest parameter: ...param
            let is_rest = self.check(&TokenKind::DotDotDot);
            if is_rest {
                self.advance(); // consume ...
            }

            // Check for accessibility modifier (public, private, protected) for constructor parameter properties
            let accessibility = match &self.current.kind {
                TokenKind::Public => {
                    self.advance();
                    Some(crate::ast::Accessibility::Public)
                }
                TokenKind::Private => {
                    self.advance();
                    Some(crate::ast::Accessibility::Private)
                }
                TokenKind::Protected => {
                    self.advance();
                    Some(crate::ast::Accessibility::Protected)
                }
                _ => None,
            };

            // Check for readonly modifier
            let readonly = if self.check(&TokenKind::Readonly) {
                self.advance();
                true
            } else {
                false
            };

            // Parse parameter name (contextual keywords allowed)
            let inner_pattern = if let Some((name, span)) = self.try_get_identifier_name() {
                Pattern::Identifier(Identifier { name, span })
            } else {
                return Err(JsError::syntax_error(
                    format!("Expected parameter name, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                ));
            };

            // Check for optional marker '?'
            let optional = if self.check(&TokenKind::Question) {
                self.advance();
                true
            } else {
                false
            };

            // Parse optional type annotation
            let type_annotation = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let end_span = self.current.span;
            let full_span = Span::new(param_span.start, end_span.start, param_span.line, param_span.column);

            // Create pattern (with rest wrapper if needed)
            let pattern = if is_rest {
                Pattern::Rest(RestElement {
                    argument: Box::new(inner_pattern),
                    type_annotation: type_annotation.clone(),
                    span: full_span,
                })
            } else {
                inner_pattern
            };

            params.push(FunctionParam {
                pattern,
                type_annotation,
                optional,
                decorators,
                accessibility,
                readonly,
                span: full_span,
            });

            // Check for comma
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }

    /// Parse a block statement: { statements }
    fn parse_block_statement(&mut self) -> Result<BlockStatement, JsError> {
        let start_span = self.current.span;
        self.expect(&TokenKind::LBrace)?;

        let mut statements = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }

        let end_span = self.expect(&TokenKind::RBrace)?;

        Ok(BlockStatement {
            body: Rc::from(statements),
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        })
    }

    /// Parse decorators: @decorator @decorator() @decorator.property
    fn parse_decorators(&mut self) -> Result<Vec<Decorator>, JsError> {
        let mut decorators = Vec::new();
        while self.check(&TokenKind::At) {
            self.enter_nesting()?;
            let start_span = self.current.span;
            self.advance(); // consume '@'
            // Parse the decorator expression (identifier, member access, or call)
            let expr = self.parse_decorator_expression()?;
            let end_span = self.current.span;
            decorators.push(Decorator {
                expression: expr,
                span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
            });
            self.exit_nesting();
        }
        Ok(decorators)
    }

    /// Parse decorator expression: identifier, member access, call, or parenthesized
    fn parse_decorator_expression(&mut self) -> Result<Expression, JsError> {
        // Check for parenthesized decorator expression: @(expr)
        let mut expr = if self.check(&TokenKind::LParen) {
            self.enter_nesting()?;
            let paren_span = self.current.span;
            self.advance();
            let inner = self.parse_expression()?;
            let end_span = self.expect(&TokenKind::RParen)?;
            self.exit_nesting();
            Expression::Parenthesized(
                Rc::new(inner),
                Span::new(paren_span.start, end_span.end, paren_span.line, paren_span.column),
            )
        } else {
            // Parse the base identifier
            let (name, id_span) = self.expect_identifier()?;
            Expression::Identifier(Identifier { name, span: id_span })
        };

        // Handle member access: @decorator.property
        while self.check(&TokenKind::Dot) {
            self.advance();
            let (prop_name, prop_span) = self.expect_identifier()?;
            let expr_span = expr.span();
            expr = Expression::Member(Box::new(MemberExpression {
                object: Rc::new(expr),
                property: MemberProperty::Identifier(Identifier { name: prop_name, span: prop_span }),
                computed: false,
                optional: false,
                span: Span::new(expr_span.start, prop_span.end, expr_span.line, expr_span.column),
            }));
        }

        // Handle call: @decorator() or @decorator.property()
        if self.check(&TokenKind::LParen) {
            self.advance();
            let arguments = self.parse_call_arguments()?;
            let end_span = self.expect(&TokenKind::RParen)?;
            let expr_span = expr.span();
            expr = Expression::Call(Box::new(CallExpression {
                callee: Rc::new(expr),
                arguments,
                type_arguments: None,
                optional: false,
                span: Span::new(expr_span.start, end_span.end, expr_span.line, expr_span.column),
            }));
        }

        Ok(expr)
    }

    /// Parse a single statement (for use inside blocks)
    fn parse_statement(&mut self) -> Result<Statement, JsError> {
        let start_span = self.current.span;

        // Check for decorators
        let decorators = if self.check(&TokenKind::At) {
            self.parse_decorators()?
        } else {
            Vec::new()
        };

        // If we have decorators, the next token must be `class`, `abstract class`, or `export`
        if !decorators.is_empty() {
            return match &self.current.kind {
                TokenKind::Class => {
                    self.advance();
                    let mut class_decl = self.parse_class_declaration(start_span)?;
                    class_decl.decorators = decorators;
                    Ok(Statement::ClassDeclaration(Box::new(class_decl)))
                }
                TokenKind::Abstract => {
                    self.advance();
                    self.expect(&TokenKind::Class)?;
                    let mut class_decl = self.parse_class_declaration(start_span)?;
                    class_decl.decorators = decorators;
                    Ok(Statement::ClassDeclaration(Box::new(class_decl)))
                }
                _ => Err(JsError::syntax_error(
                    format!("Expected class after decorators, found {:?}", self.current.kind),
                    self.current.span.line,
                    self.current.span.column,
                )),
            };
        }

        match &self.current.kind {
            TokenKind::Let => {
                self.advance();
                self.parse_variable_declaration_stmt(VariableKind::Let, start_span)
            }
            TokenKind::Const => {
                self.advance();
                // Check for const enum
                if self.check(&TokenKind::Enum) {
                    self.advance();
                    let enum_decl = self.parse_enum_declaration(start_span, true)?;
                    return Ok(Statement::EnumDeclaration(Box::new(enum_decl)));
                }
                self.parse_variable_declaration_stmt(VariableKind::Const, start_span)
            }
            TokenKind::Enum => {
                self.advance();
                let enum_decl = self.parse_enum_declaration(start_span, false)?;
                Ok(Statement::EnumDeclaration(Box::new(enum_decl)))
            }
            TokenKind::Var => {
                self.advance();
                self.parse_variable_declaration_stmt(VariableKind::Var, start_span)
            }
            TokenKind::Return => {
                self.advance();
                let argument = if self.check(&TokenKind::Semicolon)
                    || self.check(&TokenKind::RBrace)
                    || self.check(&TokenKind::Eof)
                {
                    None
                } else {
                    Some(Rc::new(self.parse_expression()?))
                };
                self.eat(&TokenKind::Semicolon);
                let end_span = self.current.span;
                Ok(Statement::Return(ReturnStatement {
                    argument,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                }))
            }
            TokenKind::Break => {
                self.advance();
                // Optional label
                let label = if let TokenKind::Identifier(name) = &self.current.kind {
                    let n = name.cheap_clone();
                    let label_span = self.current.span;
                    self.advance();
                    Some(Identifier { name: n, span: label_span })
                } else {
                    None
                };
                self.eat(&TokenKind::Semicolon);
                let end_span = self.current.span;
                Ok(Statement::Break(BreakStatement {
                    label,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                }))
            }
            TokenKind::Continue => {
                self.advance();
                // Optional label
                let label = if let TokenKind::Identifier(name) = &self.current.kind {
                    let n = name.cheap_clone();
                    let label_span = self.current.span;
                    self.advance();
                    Some(Identifier { name: n, span: label_span })
                } else {
                    None
                };
                self.eat(&TokenKind::Semicolon);
                let end_span = self.current.span;
                Ok(Statement::Continue(ContinueStatement {
                    label,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                }))
            }
            TokenKind::Throw => {
                self.advance();
                let argument = Rc::new(self.parse_expression()?);
                self.eat(&TokenKind::Semicolon);
                let end_span = self.current.span;
                Ok(Statement::Throw(ThrowStatement {
                    argument,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                }))
            }
            TokenKind::Function => {
                self.advance();
                let func_decl = self.parse_function_declaration(start_span, false)?;
                Ok(Statement::FunctionDeclaration(Box::new(func_decl)))
            }
            TokenKind::Class => {
                self.advance();
                let class_decl = self.parse_class_declaration(start_span)?;
                Ok(Statement::ClassDeclaration(Box::new(class_decl)))
            }
            TokenKind::Semicolon => {
                self.advance();
                Ok(Statement::Empty)
            }
            // Block statement
            TokenKind::LBrace => {
                let block = self.parse_block_statement()?;
                Ok(Statement::Block(block))
            }
            // If statement
            TokenKind::If => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let test = Rc::new(self.parse_expression()?);
                self.expect(&TokenKind::RParen)?;
                let consequent = Rc::new(self.parse_statement()?);
                let alternate = if self.check(&TokenKind::Else) {
                    self.advance();
                    Some(Rc::new(self.parse_statement()?))
                } else {
                    None
                };
                let end_span = self.current.span;
                Ok(Statement::If(IfStatement {
                    test,
                    consequent,
                    alternate,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                }))
            }
            // For loop
            TokenKind::For => {
                self.advance();
                self.expect(&TokenKind::LParen)?;

                // Parse init
                let init = if self.check(&TokenKind::Semicolon) {
                    None
                } else if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) || self.check(&TokenKind::Var) {
                    let kind = match &self.current.kind {
                        TokenKind::Let => VariableKind::Let,
                        TokenKind::Const => VariableKind::Const,
                        TokenKind::Var => VariableKind::Var,
                        _ => VariableKind::Var,
                    };
                    self.advance();
                    let decl = self.parse_variable_declaration_for_init(kind)?;

                    // Check for for-in/for-of
                    if self.check(&TokenKind::In) {
                        self.advance();
                        let right = Rc::new(self.parse_expression()?);
                        self.expect(&TokenKind::RParen)?;
                        let body = Rc::new(self.parse_statement()?);
                        let end_span = self.current.span;
                        return Ok(Statement::ForIn(Box::new(ForInStatement {
                            left: ForInOfLeft::Variable(decl),
                            right,
                            body,
                            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                        })));
                    }
                    if self.check(&TokenKind::Of) {
                        self.advance();
                        let right = Rc::new(self.parse_expression()?);
                        self.expect(&TokenKind::RParen)?;
                        let body = Rc::new(self.parse_statement()?);
                        let end_span = self.current.span;
                        return Ok(Statement::ForOf(Box::new(ForOfStatement {
                            left: ForInOfLeft::Variable(decl),
                            right,
                            body,
                            await_: false,
                            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                        })));
                    }

                    Some(ForInit::Variable(decl))
                } else {
                    Some(ForInit::Expression(Rc::new(self.parse_expression()?)))
                };

                self.expect(&TokenKind::Semicolon)?;

                // Parse test
                let test = if self.check(&TokenKind::Semicolon) {
                    None
                } else {
                    Some(Rc::new(self.parse_expression()?))
                };
                self.expect(&TokenKind::Semicolon)?;

                // Parse update
                let update = if self.check(&TokenKind::RParen) {
                    None
                } else {
                    Some(Rc::new(self.parse_expression()?))
                };
                self.expect(&TokenKind::RParen)?;

                let body = Rc::new(self.parse_statement()?);
                let end_span = self.current.span;
                Ok(Statement::For(Box::new(ForStatement {
                    init,
                    test,
                    update,
                    body,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                })))
            }
            // While loop
            TokenKind::While => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let test = Rc::new(self.parse_expression()?);
                self.expect(&TokenKind::RParen)?;
                let body = Rc::new(self.parse_statement()?);
                let end_span = self.current.span;
                Ok(Statement::While(WhileStatement {
                    test,
                    body,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                }))
            }
            // Do-while loop
            TokenKind::Do => {
                self.advance();
                let body = Rc::new(self.parse_statement()?);
                self.expect(&TokenKind::While)?;
                self.expect(&TokenKind::LParen)?;
                let test = Rc::new(self.parse_expression()?);
                self.expect(&TokenKind::RParen)?;
                self.eat(&TokenKind::Semicolon);
                let end_span = self.current.span;
                Ok(Statement::DoWhile(DoWhileStatement {
                    body,
                    test,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                }))
            }
            // Try-catch-finally
            TokenKind::Try => {
                self.advance();
                let block = self.parse_block_statement()?;

                // Parse optional catch clause
                let handler = if self.check(&TokenKind::Catch) {
                    let catch_start = self.current.span;
                    self.advance();

                    // Parse optional catch parameter
                    let param = if self.check(&TokenKind::LParen) {
                        self.advance();
                        let param_span = self.current.span;
                        let param_name = match &self.current.kind {
                            TokenKind::Identifier(name) => {
                                let n = name.cheap_clone();
                                self.advance();
                                n
                            }
                            _ => {
                                return Err(JsError::syntax_error(
                                    format!("Expected identifier in catch clause, found {:?}", self.current.kind),
                                    self.current.span.line,
                                    self.current.span.column,
                                ));
                            }
                        };
                        // Skip optional type annotation
                        if self.check(&TokenKind::Colon) {
                            self.advance();
                            self.skip_type_annotation()?;
                        }
                        self.expect(&TokenKind::RParen)?;
                        Some(Pattern::Identifier(Identifier { name: param_name, span: param_span }))
                    } else {
                        None
                    };

                    let body = self.parse_block_statement()?;
                    let catch_end = self.current.span;
                    Some(CatchClause {
                        param,
                        body,
                        span: Span::new(catch_start.start, catch_end.start, catch_start.line, catch_start.column),
                    })
                } else {
                    None
                };

                // Parse optional finally clause
                let finalizer = if self.check(&TokenKind::Finally) {
                    self.advance();
                    Some(self.parse_block_statement()?)
                } else {
                    None
                };

                let end_span = self.current.span;
                Ok(Statement::Try(Box::new(TryStatement {
                    block,
                    handler,
                    finalizer,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                })))
            }
            // Switch statement
            TokenKind::Switch => {
                self.advance();
                self.expect(&TokenKind::LParen)?;
                let discriminant = Rc::new(self.parse_expression()?);
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::LBrace)?;

                let mut cases = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                    let case_start = self.current.span;
                    let test = if self.check(&TokenKind::Case) {
                        self.advance();
                        let expr = Some(Rc::new(self.parse_expression()?));
                        self.expect(&TokenKind::Colon)?;
                        expr
                    } else if self.check(&TokenKind::Default) {
                        self.advance();
                        self.expect(&TokenKind::Colon)?;
                        None
                    } else {
                        return Err(JsError::syntax_error(
                            format!("Expected 'case' or 'default', found {:?}", self.current.kind),
                            self.current.span.line,
                            self.current.span.column,
                        ));
                    };

                    // Parse consequent statements until next case/default/}
                    let mut consequent = Vec::new();
                    while !self.check(&TokenKind::Case)
                        && !self.check(&TokenKind::Default)
                        && !self.check(&TokenKind::RBrace)
                        && !self.check(&TokenKind::Eof)
                    {
                        consequent.push(self.parse_statement()?);
                    }

                    let case_end = self.current.span;
                    cases.push(SwitchCase {
                        test,
                        consequent: consequent.into(),
                        span: Span::new(case_start.start, case_end.start, case_start.line, case_start.column),
                    });
                }

                let end_span = self.expect(&TokenKind::RBrace)?;
                Ok(Statement::Switch(SwitchStatement {
                    discriminant,
                    cases: cases.into(),
                    span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
                }))
            }
            // Import statement
            TokenKind::Import => {
                self.advance();
                self.parse_import_declaration(start_span)
            }
            // Export statement
            TokenKind::Export => {
                self.advance();
                self.parse_export_declaration(start_span)
            }
            _ => {
                // Parse as expression statement
                let expr = self.parse_expression()?;
                self.eat(&TokenKind::Semicolon);
                let end_span = self.current.span;
                Ok(Statement::Expression(ExpressionStatement {
                    expression: Rc::new(expr),
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                }))
            }
        }
    }

    /// Parse an import declaration
    fn parse_import_declaration(&mut self, start_span: Span) -> Result<Statement, JsError> {
        use crate::ast::{ImportDeclaration, ImportSpecifier};

        // Check for type-only import: import type ...
        let type_only = if self.check(&TokenKind::Type) {
            let peeked = self.peek_token();
            // import type X from "..." or import type { X } from "..."
            if matches!(peeked.kind, TokenKind::Identifier(_) | TokenKind::LBrace | TokenKind::Star) {
                self.advance(); // consume 'type'
                true
            } else {
                false
            }
        } else {
            false
        };

        let mut specifiers = Vec::new();

        // import "module" (side-effect import)
        if let TokenKind::String(source_value) = &self.current.kind {
            let source = StringLiteral {
                value: source_value.cheap_clone(),
                span: self.current.span,
            };
            self.advance();
            self.eat(&TokenKind::Semicolon);
            let end_span = self.current.span;
            return Ok(Statement::Import(Box::new(ImportDeclaration {
                specifiers,
                source,
                type_only,
                span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
            })));
        }

        // import * as ns from "module"
        if self.check(&TokenKind::Star) {
            self.advance(); // consume '*'
            self.expect(&TokenKind::As)?;
            let (local_name, local_span) = self.expect_identifier()?;
            specifiers.push(ImportSpecifier::Namespace {
                local: Identifier { name: local_name, span: local_span },
                span: local_span,
            });
        }
        // import defaultExport from "module" or import { named } from "module"
        else if let Some((name, span)) = self.try_get_identifier_name() {
            // Default import
            specifiers.push(ImportSpecifier::Default {
                local: Identifier { name, span },
                span,
            });

            // Check for additional named imports: import Default, { named } from "..."
            if self.check(&TokenKind::Comma) {
                self.advance();
                if self.check(&TokenKind::LBrace) {
                    self.parse_named_imports(&mut specifiers)?;
                } else if self.check(&TokenKind::Star) {
                    // import Default, * as ns from "..."
                    self.advance();
                    self.expect(&TokenKind::As)?;
                    let (local_name, local_span) = self.expect_identifier()?;
                    specifiers.push(ImportSpecifier::Namespace {
                        local: Identifier { name: local_name, span: local_span },
                        span: local_span,
                    });
                }
            }
        }
        // import { named } from "module"
        else if self.check(&TokenKind::LBrace) {
            self.parse_named_imports(&mut specifiers)?;
        }

        // Expect 'from'
        self.expect(&TokenKind::From)?;

        // Parse module specifier
        let source = self.parse_string_literal()?;
        self.eat(&TokenKind::Semicolon);
        let end_span = self.current.span;

        Ok(Statement::Import(Box::new(ImportDeclaration {
            specifiers,
            source,
            type_only,
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })))
    }

    /// Parse named imports: { a, b as c, type d }
    fn parse_named_imports(&mut self, specifiers: &mut Vec<crate::ast::ImportSpecifier>) -> Result<(), JsError> {
        use crate::ast::ImportSpecifier;

        self.expect(&TokenKind::LBrace)?;

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let spec_start = self.current.span;

            // Check for type modifier: import { type X } from "..."
            let _is_type = if self.check(&TokenKind::Type) {
                let peeked = self.peek_token();
                if matches!(peeked.kind, TokenKind::Identifier(_) | TokenKind::As) {
                    self.advance();
                    true
                } else {
                    false
                }
            } else {
                false
            };

            let (imported_name, imported_span) = self.expect_identifier()?;

            // Check for 'as local'
            let local = if self.check(&TokenKind::As) {
                self.advance();
                let (local_name, local_span) = self.expect_identifier()?;
                Identifier { name: local_name, span: local_span }
            } else {
                Identifier { name: imported_name.cheap_clone(), span: imported_span }
            };

            let spec_end = self.current.span;
            specifiers.push(ImportSpecifier::Named {
                local,
                imported: Identifier { name: imported_name, span: imported_span },
                span: Span::new(spec_start.start, spec_end.start, spec_start.line, spec_start.column),
            });

            if !self.check(&TokenKind::RBrace) {
                self.expect(&TokenKind::Comma)?;
            }
        }
        self.expect(&TokenKind::RBrace)?;

        Ok(())
    }

    /// Parse an export declaration
    fn parse_export_declaration(&mut self, start_span: Span) -> Result<Statement, JsError> {
        // Check for type-only export: export type ...
        let type_only = if self.check(&TokenKind::Type) {
            // Peek to see if this is `export type * ...` or `export type { ... }`
            let peeked = self.peek_token();
            if matches!(peeked.kind, TokenKind::Star | TokenKind::LBrace) {
                self.advance(); // consume 'type'
                true
            } else {
                false
            }
        } else {
            false
        };

        // Check for decorators (export @decorator class Foo)
        let decorators = if self.check(&TokenKind::At) {
            self.parse_decorators()?
        } else {
            Vec::new()
        };

        // export default ...
        if self.check(&TokenKind::Default) {
            self.advance();
            return self.parse_export_default(start_span, decorators, type_only);
        }

        // export * ...
        if self.check(&TokenKind::Star) {
            self.advance();
            return self.parse_export_star(start_span, type_only);
        }

        // export { ... }
        if self.check(&TokenKind::LBrace) {
            return self.parse_export_named(start_span, type_only);
        }

        // export function, export class, export const/let/var, etc.
        let declaration = self.parse_export_declaration_statement(decorators)?;
        let end_span = self.current.span;

        Ok(Statement::Export(Box::new(ExportDeclaration {
            declaration: Some(Box::new(declaration)),
            specifiers: Vec::new(),
            source: None,
            namespace_export: None,
            default: false,
            type_only,
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })))
    }

    /// Parse export default ...
    fn parse_export_default(&mut self, start_span: Span, decorators: Vec<Decorator>, type_only: bool) -> Result<Statement, JsError> {
        // Check for decorators on default export
        let decorators = if decorators.is_empty() && self.check(&TokenKind::At) {
            self.parse_decorators()?
        } else {
            decorators
        };

        // export default class ...
        if self.check(&TokenKind::Class) {
            self.advance();
            let mut class_decl = self.parse_class_declaration(start_span)?;
            class_decl.decorators = decorators;
            let end_span = self.current.span;
            return Ok(Statement::Export(Box::new(ExportDeclaration {
                declaration: Some(Box::new(Statement::ClassDeclaration(Box::new(class_decl)))),
                specifiers: Vec::new(),
                source: None,
                namespace_export: None,
                default: true,
                type_only,
                span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
            })));
        }

        // export default async function ...
        if self.check(&TokenKind::Async) {
            self.advance();
            if self.check(&TokenKind::Function) {
                self.advance();
                let func_decl = self.parse_function_declaration(start_span, true)?;
                let end_span = self.current.span;
                return Ok(Statement::Export(Box::new(ExportDeclaration {
                    declaration: Some(Box::new(Statement::FunctionDeclaration(Box::new(func_decl)))),
                    specifiers: Vec::new(),
                    source: None,
                    namespace_export: None,
                    default: true,
                    type_only,
                    span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
                })));
            }
        }

        // export default function ...
        if self.check(&TokenKind::Function) {
            self.advance();
            let func_decl = self.parse_function_declaration(start_span, false)?;
            let end_span = self.current.span;
            return Ok(Statement::Export(Box::new(ExportDeclaration {
                declaration: Some(Box::new(Statement::FunctionDeclaration(Box::new(func_decl)))),
                specifiers: Vec::new(),
                source: None,
                namespace_export: None,
                default: true,
                type_only,
                span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
            })));
        }

        // export default <expression>
        let expr = self.parse_expression()?;
        self.eat(&TokenKind::Semicolon);
        let end_span = self.current.span;

        Ok(Statement::Export(Box::new(ExportDeclaration {
            declaration: Some(Box::new(Statement::Expression(ExpressionStatement {
                expression: Rc::new(expr),
                span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
            }))),
            specifiers: Vec::new(),
            source: None,
            namespace_export: None,
            default: true,
            type_only,
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })))
    }

    /// Parse export * from "..." or export * as ns from "..."
    fn parse_export_star(&mut self, start_span: Span, type_only: bool) -> Result<Statement, JsError> {
        // Check for `export * as namespace from "..."`
        let namespace_export = if self.check(&TokenKind::As) {
            self.advance();
            let (name, name_span) = self.expect_identifier()?;
            Some(Identifier { name, span: name_span })
        } else {
            None
        };

        // Expect 'from'
        self.expect(&TokenKind::From)?;

        // Parse module specifier
        let source = self.parse_string_literal()?;
        self.eat(&TokenKind::Semicolon);
        let end_span = self.current.span;

        Ok(Statement::Export(Box::new(ExportDeclaration {
            declaration: None,
            specifiers: Vec::new(),
            source: Some(source),
            namespace_export,
            default: false,
            type_only,
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })))
    }

    /// Parse export { ... } or export { ... } from "..."
    fn parse_export_named(&mut self, start_span: Span, type_only: bool) -> Result<Statement, JsError> {
        self.expect(&TokenKind::LBrace)?;

        let mut specifiers = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let spec_start = self.current.span;
            let (local_name, local_span) = self.expect_identifier()?;
            let local = Identifier { name: local_name.cheap_clone(), span: local_span };

            // Check for `as exported`
            let exported = if self.check(&TokenKind::As) {
                self.advance();
                let (exported_name, exported_span) = self.expect_identifier()?;
                Identifier { name: exported_name, span: exported_span }
            } else {
                Identifier { name: local_name, span: local_span }
            };

            let spec_end = self.current.span;
            specifiers.push(ExportSpecifier {
                local,
                exported,
                span: Span::new(spec_start.start, spec_end.start, spec_start.line, spec_start.column),
            });

            if !self.check(&TokenKind::RBrace) {
                self.expect(&TokenKind::Comma)?;
            }
        }
        self.expect(&TokenKind::RBrace)?;

        // Check for 'from "..."'
        let source = if self.check(&TokenKind::From) {
            self.advance();
            Some(self.parse_string_literal()?)
        } else {
            None
        };

        self.eat(&TokenKind::Semicolon);
        let end_span = self.current.span;

        Ok(Statement::Export(Box::new(ExportDeclaration {
            declaration: None,
            specifiers,
            source,
            namespace_export: None,
            default: false,
            type_only,
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })))
    }

    /// Parse a declaration after export keyword (function, class, const, etc.)
    fn parse_export_declaration_statement(&mut self, decorators: Vec<Decorator>) -> Result<Statement, JsError> {
        let start_span = self.current.span;

        match &self.current.kind {
            TokenKind::Async => {
                self.advance();
                self.expect(&TokenKind::Function)?;
                let func_decl = self.parse_function_declaration(start_span, true)?;
                Ok(Statement::FunctionDeclaration(Box::new(func_decl)))
            }
            TokenKind::Function => {
                self.advance();
                let func_decl = self.parse_function_declaration(start_span, false)?;
                Ok(Statement::FunctionDeclaration(Box::new(func_decl)))
            }
            TokenKind::Class => {
                self.advance();
                let mut class_decl = self.parse_class_declaration(start_span)?;
                class_decl.decorators = decorators;
                Ok(Statement::ClassDeclaration(Box::new(class_decl)))
            }
            TokenKind::Abstract => {
                self.advance();
                self.expect(&TokenKind::Class)?;
                let mut class_decl = self.parse_class_declaration(start_span)?;
                class_decl.decorators = decorators;
                Ok(Statement::ClassDeclaration(Box::new(class_decl)))
            }
            TokenKind::Const => {
                self.advance();
                // Check for const enum
                if self.check(&TokenKind::Enum) {
                    self.advance();
                    let enum_decl = self.parse_enum_declaration(start_span, true)?;
                    return Ok(Statement::EnumDeclaration(Box::new(enum_decl)));
                }
                self.parse_variable_declaration_stmt(VariableKind::Const, start_span)
            }
            TokenKind::Let => {
                self.advance();
                self.parse_variable_declaration_stmt(VariableKind::Let, start_span)
            }
            TokenKind::Var => {
                self.advance();
                self.parse_variable_declaration_stmt(VariableKind::Var, start_span)
            }
            TokenKind::Enum => {
                self.advance();
                let enum_decl = self.parse_enum_declaration(start_span, false)?;
                Ok(Statement::EnumDeclaration(Box::new(enum_decl)))
            }
            TokenKind::Interface => {
                self.advance();
                let iface = self.parse_interface_declaration(start_span)?;
                Ok(Statement::InterfaceDeclaration(Box::new(iface)))
            }
            TokenKind::Type => {
                self.advance();
                let type_alias = self.parse_type_alias_declaration(start_span)?;
                Ok(Statement::TypeAlias(Box::new(type_alias)))
            }
            _ => Err(JsError::syntax_error(
                format!("Expected declaration after export, found {:?}", self.current.kind),
                self.current.span.line,
                self.current.span.column,
            )),
        }
    }

    /// Parse a string literal for import/export source
    fn parse_string_literal(&mut self) -> Result<StringLiteral, JsError> {
        let span = self.current.span;
        match &self.current.kind {
            TokenKind::String(value) => {
                let v = value.cheap_clone();
                self.advance();
                Ok(StringLiteral { value: v, span })
            }
            _ => Err(JsError::syntax_error(
                format!("Expected string literal, found {:?}", self.current.kind),
                span.line,
                span.column,
            )),
        }
    }

    /// Parse a variable declaration for for-loop init (without semicolon)
    fn parse_variable_declaration_for_init(&mut self, kind: VariableKind) -> Result<VariableDeclaration, JsError> {
        let start_span = self.current.span;

        // Parse binding pattern (identifier, array, or object destructuring)
        let pattern = self.parse_binding_pattern()?;
        let pattern_span = pattern.span();

        // Check for type annotation
        let type_annotation = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        // Check for initializer (only for regular for loops, not for-in/for-of)
        let init = if self.check(&TokenKind::Eq) {
            self.advance();
            Some(Rc::new(self.parse_expression()?))
        } else {
            None
        };

        let end_span = self.current.span;

        let declarator = VariableDeclarator {
            id: pattern,
            init,
            type_annotation,
            span: Span::new(pattern_span.start, end_span.start, pattern_span.line, pattern_span.column),
        };

        Ok(VariableDeclaration {
            kind,
            declarations: Rc::from(vec![declarator]),
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        })
    }

    /// Parse a variable declaration statement (helper for parse_statement)
    fn parse_variable_declaration_stmt(
        &mut self,
        kind: VariableKind,
        start_span: Span,
    ) -> Result<Statement, JsError> {
        let mut declarations = Vec::new();

        loop {
            let pattern_span = self.current.span;

            // Parse binding pattern (identifier, array destructuring, or object destructuring)
            let pattern = self.parse_binding_pattern()?;

            // Check for type annotation
            let type_annotation = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            // Check for initializer
            let init = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(Rc::new(self.parse_expression()?))
            } else {
                None
            };

            let end_span = self.current.span;
            declarations.push(VariableDeclarator {
                id: pattern,
                init,
                type_annotation,
                span: Span::new(pattern_span.start, end_span.start, pattern_span.line, pattern_span.column),
            });

            // Check for multiple declarations: const a = 1, b = 2
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.eat(&TokenKind::Semicolon);
        let end_span = self.current.span;

        Ok(Statement::VariableDeclaration(VariableDeclaration {
            kind,
            declarations: Rc::from(declarations),
            span: Span::new(start_span.start, end_span.start, start_span.line, start_span.column),
        }))
    }

    /// Parse a binding pattern (identifier, array pattern, or object pattern)
    fn parse_binding_pattern(&mut self) -> Result<Pattern, JsError> {
        match &self.current.kind {
            // Array destructuring: [a, b, c]
            TokenKind::LBracket => self.parse_array_pattern(),
            // Object destructuring: { a, b: c }
            TokenKind::LBrace => self.parse_object_pattern(),
            // Simple identifier
            _ => {
                if let Some((name, span)) = self.try_get_identifier_name() {
                    Ok(Pattern::Identifier(Identifier { name, span }))
                } else {
                    Err(JsError::syntax_error(
                        format!("Expected binding pattern, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ))
                }
            }
        }
    }

    /// Parse array destructuring pattern: [a, b, ...rest]
    fn parse_array_pattern(&mut self) -> Result<Pattern, JsError> {
        use crate::ast::ArrayPattern;

        let start_span = self.current.span;
        self.expect(&TokenKind::LBracket)?;

        let mut elements = Vec::new();

        while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
            // Check for hole (elision): [a, , b]
            if self.check(&TokenKind::Comma) {
                elements.push(None);
                self.advance();
                continue;
            }

            // Check for rest element: [...rest]
            if self.check(&TokenKind::DotDotDot) {
                let rest_span = self.current.span;
                self.advance();
                let argument = Box::new(self.parse_binding_pattern()?);
                let end_span = self.current.span;
                elements.push(Some(Pattern::Rest(RestElement {
                    argument,
                    type_annotation: None,
                    span: Span::new(rest_span.start, end_span.start, rest_span.line, rest_span.column),
                })));
                break; // Rest must be last
            }

            // Parse binding element
            let elem_pattern = self.parse_binding_pattern()?;

            // Check for default value: [a = 1]
            let pattern = if self.check(&TokenKind::Eq) {
                self.advance();
                let default_value = self.parse_expression()?;
                let elem_span = elem_pattern.span();
                Pattern::Assignment(crate::ast::AssignmentPattern {
                    left: Box::new(elem_pattern),
                    right: Rc::new(default_value),
                    span: Span::new(elem_span.start, self.current.span.start, elem_span.line, elem_span.column),
                })
            } else {
                elem_pattern
            };

            elements.push(Some(pattern));

            if !self.check(&TokenKind::RBracket) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        let end_span = self.expect(&TokenKind::RBracket)?;

        Ok(Pattern::Array(ArrayPattern {
            elements,
            type_annotation: None,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        }))
    }

    /// Parse object destructuring pattern: { a, b: c, ...rest }
    fn parse_object_pattern(&mut self) -> Result<Pattern, JsError> {
        use crate::ast::{ObjectPattern, ObjectPatternProperty};

        let start_span = self.current.span;
        self.expect(&TokenKind::LBrace)?;

        let mut properties = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let prop_span = self.current.span;

            // Check for rest element: { ...rest }
            if self.check(&TokenKind::DotDotDot) {
                self.advance();
                let argument = Box::new(self.parse_binding_pattern()?);
                let end_span = self.current.span;
                properties.push(ObjectPatternProperty::Rest(RestElement {
                    argument,
                    type_annotation: None,
                    span: Span::new(prop_span.start, end_span.start, prop_span.line, prop_span.column),
                }));
                break; // Rest must be last
            }

            // Parse property key
            let (key_name, key_span) = self.expect_identifier()?;
            let key = Identifier { name: key_name.cheap_clone(), span: key_span };

            // Check for : value (rename)
            let value = if self.check(&TokenKind::Colon) {
                self.advance();
                self.parse_binding_pattern()?
            } else {
                // Shorthand: { a } means { a: a }
                Pattern::Identifier(key.clone())
            };

            // Check for default value: { a = 1 } or { a: b = 1 }
            let final_value = if self.check(&TokenKind::Eq) {
                self.advance();
                let default_value = self.parse_expression()?;
                let val_span = value.span();
                Pattern::Assignment(crate::ast::AssignmentPattern {
                    left: Box::new(value),
                    right: Rc::new(default_value),
                    span: Span::new(val_span.start, self.current.span.start, val_span.line, val_span.column),
                })
            } else {
                value
            };

            let end_span = self.current.span;
            properties.push(ObjectPatternProperty::KeyValue {
                key: ObjectPropertyKey::Identifier(key),
                value: final_value,
                shorthand: false, // Simplified for now
                span: Span::new(prop_span.start, end_span.start, prop_span.line, prop_span.column),
            });

            if !self.check(&TokenKind::RBrace) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        let end_span = self.expect(&TokenKind::RBrace)?;

        Ok(Pattern::Object(ObjectPattern {
            properties,
            type_annotation: None,
            span: Span::new(start_span.start, end_span.end, start_span.line, start_span.column),
        }))
    }

    // ========================================================================
    // Arrow function parsing
    // ========================================================================

    /// Parse arrow function body: either a block or an expression
    fn parse_arrow_function_body(&mut self) -> Result<ArrowFunctionBody, JsError> {
        if self.check(&TokenKind::LBrace) {
            let block = self.parse_block_statement()?;
            Ok(ArrowFunctionBody::Block(Rc::new(block)))
        } else {
            let expr = self.parse_expression()?;
            Ok(ArrowFunctionBody::Expression(Rc::new(expr)))
        }
    }

    /// Parse arrow function parameters (or what looks like them)
    /// This is similar to parse_function_params but also handles expressions
    /// Check if the current position looks like arrow function parameters
    /// This is used to distinguish between (a, b) => ... and (1 + 2)
    fn looks_like_arrow_params(&mut self) -> bool {
        // Empty parens: () =>
        if self.check(&TokenKind::RParen) {
            return true;
        }

        // Rest parameter: (...x) =>
        if self.check(&TokenKind::DotDotDot) {
            return true;
        }

        // Must start with an identifier for arrow params
        if !matches!(self.current.kind, TokenKind::Identifier(_)) {
            return false;
        }

        // Check what follows the identifier to distinguish arrow params from expressions
        // Arrow params: identifier followed by `:`, `?`, `,`, `)`
        // Expressions: identifier followed by `=`, `+`, `-`, `*`, `/`, `[`, `.`, `(`, etc.
        let next = self.peek_token();
        matches!(
            next.kind,
            TokenKind::Colon      // type annotation: (x: number) =>
            | TokenKind::Question // optional param: (x?) =>
            | TokenKind::Comma    // multiple params: (x, y) =>
            | TokenKind::RParen   // single param: (x) => or end of params
            | TokenKind::Eq       // default value: (x = 1) => - but NOT assignment expression!
        ) && !self.is_assignment_in_parens()
    }

    /// Check if we're looking at an assignment expression inside parentheses
    /// like (val = false, x) rather than arrow param with default like (x = 1) =>
    fn is_assignment_in_parens(&mut self) -> bool {
        // Save state
        let checkpoint = self.lexer.checkpoint();
        let current_saved = self.current.clone();

        // Skip the identifier
        self.advance();

        // If next is `=`, we need to check if this is an arrow param default or an assignment
        if self.check(&TokenKind::Eq) {
            self.advance(); // skip `=`
            // Skip the assigned expression to see what follows
            // For simplicity, scan until we hit `,` or `)` at depth 0
            let mut depth = 0;
            loop {
                match &self.current.kind {
                    TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                    TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                        if depth == 0 {
                            // Hit `)` at depth 0 - check if followed by `=>`
                            // Restore and check for arrow
                            self.lexer.restore(checkpoint);
                            self.current = current_saved;
                            // We need to scan all params to see if there's an arrow at the end
                            // This is complex, so use a simpler heuristic:
                            // If after the close paren there's `=>`, it's arrow params
                            // Otherwise it's an expression
                            return !self.scan_for_arrow_after_parens();
                        }
                        depth -= 1;
                    }
                    TokenKind::Comma if depth == 0 => {
                        // Found comma at depth 0 - check next token
                        self.advance();
                        // If next is an identifier followed by `=` again, continue scanning
                        // If next is `)`, we need to check for `=>`
                        // For now, assume it's an expression (assignment with comma)
                        self.lexer.restore(checkpoint);
                        self.current = current_saved;
                        return !self.scan_for_arrow_after_parens();
                    }
                    TokenKind::Eof => {
                        self.lexer.restore(checkpoint);
                        self.current = current_saved;
                        return false;
                    }
                    _ => {}
                }
                self.advance();
            }
        }

        // No `=` after identifier, so not an assignment
        self.lexer.restore(checkpoint);
        self.current = current_saved;
        false
    }

    /// Scan forward to see if there's an arrow after the closing paren
    fn scan_for_arrow_after_parens(&mut self) -> bool {
        // Save state
        let checkpoint = self.lexer.checkpoint();
        let current_saved = self.current.clone();

        // Skip to matching close paren
        let mut depth = 1; // We're already inside the opening paren
        loop {
            match &self.current.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        self.advance(); // consume `)`
                        break;
                    }
                }
                TokenKind::Eof => {
                    self.lexer.restore(checkpoint);
                    self.current = current_saved;
                    return false;
                }
                _ => {}
            }
            self.advance();
        }

        // Skip optional type annotation
        if self.check(&TokenKind::Colon) {
            self.advance();
            // Skip the type annotation - scan until we hit `=>` or something else
            let mut type_depth = 0;
            loop {
                match &self.current.kind {
                    TokenKind::Lt => type_depth += 1,
                    TokenKind::Gt => {
                        if type_depth > 0 {
                            type_depth -= 1;
                        }
                    }
                    TokenKind::Arrow | TokenKind::LBrace | TokenKind::Eof => break,
                    _ => {}
                }
                self.advance();
            }
        }

        // Check for arrow
        let is_arrow = self.check(&TokenKind::Arrow);

        self.lexer.restore(checkpoint);
        self.current = current_saved;
        is_arrow
    }

    fn parse_arrow_params_or_expression(&mut self) -> Result<Vec<FunctionParam>, JsError> {
        let mut params = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let param_span = self.current.span;

            // Parse parameter name
            let pattern = match &self.current.kind {
                TokenKind::Identifier(name) => {
                    let name = name.cheap_clone();
                    let span = self.current.span;
                    self.advance();
                    Pattern::Identifier(Identifier { name, span })
                }
                _ => {
                    return Err(JsError::syntax_error(
                        format!("Expected parameter name, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
            };

            // Check for optional marker '?'
            let optional = if self.check(&TokenKind::Question) {
                self.advance();
                true
            } else {
                false
            };

            // Parse optional type annotation
            let type_annotation = if self.check(&TokenKind::Colon) {
                self.advance();
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let end_span = self.current.span;
            params.push(FunctionParam {
                pattern,
                type_annotation,
                optional,
                decorators: Vec::new(),
                accessibility: None,
                readonly: false,
                span: Span::new(param_span.start, end_span.start, param_span.line, param_span.column),
            });

            // Check for comma
            if self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }

    /// Check if the current `<` token starts type arguments followed by `(`
    /// This distinguishes `fn<T>(...)` from `a < b`
    fn looks_like_type_arguments_call(&mut self) -> bool {
        // Must start with `<`
        if !self.check(&TokenKind::Lt) {
            return false;
        }

        // Save state
        let checkpoint = self.lexer.checkpoint();
        let current_saved = self.current.clone();

        // Try to skip balanced <...>
        self.advance(); // consume '<'
        let mut depth = 1;
        let mut found_close = false;

        while depth > 0 && !self.check(&TokenKind::Eof) {
            match &self.current.kind {
                TokenKind::Lt => depth += 1,
                TokenKind::Gt => {
                    depth -= 1;
                    if depth == 0 {
                        found_close = true;
                    }
                }
                TokenKind::GtGt => {
                    // >> can close two levels
                    if depth >= 2 {
                        depth -= 2;
                        if depth == 0 {
                            found_close = true;
                        }
                    } else {
                        depth -= 1;
                        if depth == 0 {
                            found_close = true;
                        }
                    }
                }
                TokenKind::GtGtGt => {
                    // >>> can close three levels
                    if depth >= 3 {
                        depth -= 3;
                    } else {
                        depth = 0;
                    }
                    found_close = true;
                }
                // These tokens wouldn't appear in type arguments
                TokenKind::Semicolon | TokenKind::LBrace | TokenKind::RBrace => {
                    break;
                }
                _ => {}
            }
            self.advance();
        }

        // Check if followed by `(`
        let result = found_close && self.check(&TokenKind::LParen);

        // Restore state
        self.lexer.restore(checkpoint);
        self.current = current_saved;

        result
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
            // BigInt literal
            TokenKind::BigInt(s) => {
                let value = s.clone();
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::BigInt(value),
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
            // Regex literal - rescan from / as regexp
            TokenKind::Slash | TokenKind::SlashEq => {
                let regexp_token = self.lexer.rescan_as_regexp(span);
                if let TokenKind::RegExp(pattern, flags) = regexp_token.kind {
                    let regexp_span = regexp_token.span;
                    self.advance(); // advance past the regex token
                    Ok(Expression::Literal(Box::new(Literal {
                        value: LiteralValue::RegExp { pattern, flags },
                        span: regexp_span,
                    })))
                } else {
                    Err(JsError::syntax_error(
                        "Expected regular expression",
                        span.line,
                        span.column,
                    ))
                }
            }
            // Template literal (no substitution) - `hello`
            // TemplateTail is also used for templates that start with `} (continuation after substitution)
            // TemplateNoSub is for templates like `hello` with no ${} at all
            TokenKind::TemplateTail(s) | TokenKind::TemplateNoSub(s) => {
                let value = s.cheap_clone();
                self.advance();
                let template = TemplateLiteral {
                    quasis: vec![TemplateElement {
                        value,
                        tail: true,
                        span,
                    }],
                    expressions: vec![],
                    span,
                };
                Ok(Expression::Template(Box::new(template)))
            }
            // Template literal with substitutions - `hello ${name}!`
            TokenKind::TemplateHead(s) => {
                self.parse_template_literal(s.cheap_clone(), span)
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
            // This expression
            TokenKind::This => {
                self.advance();
                Ok(Expression::This(span))
            }
            // Contextual keywords that can be used as identifiers
            TokenKind::Module => {
                let name = self.lexer.string_dict().get_or_insert("module");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            TokenKind::Namespace => {
                let name = self.lexer.string_dict().get_or_insert("namespace");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            // Identifier (might be start of arrow function: x => ...)
            TokenKind::Identifier(name) => {
                let name = name.cheap_clone();
                self.advance();

                // Check for single-param arrow function: x => expr
                if self.check(&TokenKind::Arrow) {
                    self.advance(); // consume '=>'
                    let body = self.parse_arrow_function_body()?;
                    let end_span = self.current.span;
                    return Ok(Expression::ArrowFunction(Box::new(ArrowFunctionExpression {
                        params: Rc::from(vec![FunctionParam {
                            pattern: Pattern::Identifier(Identifier { name: name.cheap_clone(), span }),
                            type_annotation: None,
                            optional: false,
                            decorators: Vec::new(),
                            accessibility: None,
                            readonly: false,
                            span,
                        }]),
                        return_type: None,
                        type_parameters: None,
                        body: Box::new(body),
                        async_: false,
                        span: Span::new(span.start, end_span.start, span.line, span.column),
                    })));
                }

                Ok(Expression::Identifier(Identifier { name, span }))
            }
            // Parenthesized expression or arrow function
            TokenKind::LParen => {
                self.enter_nesting()?;
                self.advance();

                // Check for empty params: () => ...
                if self.check(&TokenKind::RParen) {
                    self.advance(); // consume ')'

                    // Check for return type
                    let return_type = if self.check(&TokenKind::Colon) {
                        self.advance();
                        Some(Box::new(self.parse_type_annotation()?))
                    } else {
                        None
                    };

                    if self.check(&TokenKind::Arrow) {
                        self.advance(); // consume '=>'
                        let body = self.parse_arrow_function_body()?;
                        let end_span = self.current.span;
                        return Ok(Expression::ArrowFunction(Box::new(ArrowFunctionExpression {
                            params: Rc::from(Vec::new()),
                            return_type,
                            type_parameters: None,
                            body: Box::new(body),
                            async_: false,
                            span: Span::new(span.start, end_span.start, span.line, span.column),
                        })));
                    }
                }

                // Determine if this is arrow function params or parenthesized expression
                // by looking at first token after '('
                let looks_like_arrow_params = self.looks_like_arrow_params();

                if looks_like_arrow_params {
                    let params = self.parse_arrow_params_or_expression()?;
                    let end_span = self.expect(&TokenKind::RParen)?;

                    // Check for return type after params
                    let return_type = if self.check(&TokenKind::Colon) {
                        self.advance();
                        Some(Box::new(self.parse_type_annotation()?))
                    } else {
                        None
                    };

                    // Check for arrow
                    if self.check(&TokenKind::Arrow) {
                        self.advance(); // consume '=>'
                        let body = self.parse_arrow_function_body()?;
                        let body_end_span = self.current.span;
                        return Ok(Expression::ArrowFunction(Box::new(ArrowFunctionExpression {
                            params: Rc::from(params),
                            return_type,
                            type_parameters: None,
                            body: Box::new(body),
                            async_: false,
                            span: Span::new(span.start, body_end_span.start, span.line, span.column),
                        })));
                    }

                    // Not an arrow function - convert params back to identifier expression if possible
                    if params.len() == 1 && return_type.is_none() {
                        let param = params.into_iter().next().ok_or_else(|| {
                            JsError::syntax_error("Unexpected empty params", span.line, span.column)
                        })?;
                        if let Pattern::Identifier(id) = param.pattern {
                            if param.type_annotation.is_none() && !param.optional {
                                let full_span = Span::new(span.start, end_span.end, span.line, span.column);
                                return Ok(Expression::Parenthesized(
                                    Rc::new(Expression::Identifier(id)),
                                    full_span,
                                ));
                            }
                        }
                    }

                    return Err(JsError::syntax_error(
                        "Expected '=>' for arrow function",
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }

                // Parse as parenthesized expression (may contain comma sequence)
                let first_expr = self.parse_expression()?;

                // Check for comma sequence (comma operator)
                if self.check(&TokenKind::Comma) {
                    let mut expressions = vec![first_expr];
                    while self.check(&TokenKind::Comma) {
                        self.advance(); // consume ','
                        expressions.push(self.parse_expression()?);
                    }
                    let end_span = self.expect(&TokenKind::RParen)?;
                    let full_span = Span::new(span.start, end_span.end, span.line, span.column);
                    let seq = Expression::Sequence(SequenceExpression {
                        expressions,
                        span: full_span,
                    });
                    Ok(Expression::Parenthesized(Rc::new(seq), full_span))
                } else {
                    let end_span = self.expect(&TokenKind::RParen)?;
                    let full_span = Span::new(span.start, end_span.end, span.line, span.column);
                    Ok(Expression::Parenthesized(Rc::new(first_expr), full_span))
                }
            }
            // Array literal
            TokenKind::LBracket => {
                self.advance();
                let elements = self.parse_array_elements()?;
                let end_span = self.expect(&TokenKind::RBracket)?;
                let full_span = Span::new(span.start, end_span.end, span.line, span.column);
                Ok(Expression::Array(ArrayExpression {
                    elements,
                    span: full_span,
                }))
            }
            // Object literal
            TokenKind::LBrace => {
                self.advance();
                let properties = self.parse_object_properties()?;
                let end_span = self.expect(&TokenKind::RBrace)?;
                let full_span = Span::new(span.start, end_span.end, span.line, span.column);
                Ok(Expression::Object(ObjectExpression {
                    properties,
                    span: full_span,
                }))
            }
            // More contextual keywords that can be used as identifiers
            TokenKind::From => {
                let name = self.lexer.string_dict().get_or_insert("from");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            TokenKind::As => {
                let name = self.lexer.string_dict().get_or_insert("as");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            TokenKind::Of => {
                let name = self.lexer.string_dict().get_or_insert("of");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            TokenKind::Type => {
                let name = self.lexer.string_dict().get_or_insert("type");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            TokenKind::Declare => {
                let name = self.lexer.string_dict().get_or_insert("declare");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            TokenKind::Readonly => {
                let name = self.lexer.string_dict().get_or_insert("readonly");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            TokenKind::Abstract => {
                let name = self.lexer.string_dict().get_or_insert("abstract");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }
            // Class expression: class { } or class Foo { }
            TokenKind::Class => {
                self.advance();
                let class_expr = self.parse_class_expression(span, Vec::new())?;
                Ok(Expression::Class(Box::new(class_expr)))
            }
            // Decorated class expression: @decorator class { }
            TokenKind::At => {
                let decorators = self.parse_decorators()?;
                // Must be followed by class
                if !self.check(&TokenKind::Class) {
                    return Err(JsError::syntax_error(
                        format!("Expected class after decorators, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
                self.advance(); // consume 'class'
                let class_expr = self.parse_class_expression(span, decorators)?;
                Ok(Expression::Class(Box::new(class_expr)))
            }
            _ => Err(JsError::syntax_error(
                format!("Expected expression, found {:?}", self.current.kind),
                self.current.span.line,
                self.current.span.column,
            )),
        }
    }

    /// Parse a template literal with substitutions
    /// Called after we've seen a TemplateHead token
    fn parse_template_literal(
        &mut self,
        head_value: JsString,
        start_span: Span,
    ) -> Result<Expression, JsError> {
        let head_span = self.current.span;
        self.advance(); // consume TemplateHead

        let mut quasis = vec![TemplateElement {
            value: head_value,
            tail: false,
            span: head_span,
        }];
        let mut expressions = Vec::new();

        loop {
            // Parse the substitution expression
            let expr = self.parse_expression()?;
            expressions.push(expr);

            // Expect } and rescan as template continuation
            if !self.check(&TokenKind::RBrace) {
                return Err(JsError::syntax_error(
                    "Expected } in template literal",
                    self.current.span.line,
                    self.current.span.column,
                ));
            }

            let rbrace_span = self.current.span;
            let continuation = self.lexer.rescan_template_continuation(rbrace_span);

            match continuation {
                TokenKind::TemplateTail(s) => {
                    quasis.push(TemplateElement {
                        value: s,
                        tail: true,
                        span: rbrace_span, // Use rbrace_span as approximate span
                    });
                    // Advance past the synthesized tail token
                    self.advance();
                    break;
                }
                TokenKind::TemplateMiddle(s) => {
                    quasis.push(TemplateElement {
                        value: s,
                        tail: false,
                        span: rbrace_span, // Use rbrace_span as approximate span
                    });
                    // Re-advance the lexer to get next token after the template middle
                    self.advance();
                    // Continue to parse next expression
                }
                _ => {
                    return Err(JsError::syntax_error(
                        "Expected template continuation",
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
            }
        }

        let end_span = self.current.span;
        let full_span = Span::new(start_span.start, end_span.start, start_span.line, start_span.column);

        Ok(Expression::Template(Box::new(TemplateLiteral {
            quasis,
            expressions,
            span: full_span,
        })))
    }

    /// Parse array elements (comma-separated, with holes)
    fn parse_array_elements(&mut self) -> Result<Vec<Option<ArrayElement>>, JsError> {
        let mut elements = Vec::new();

        while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
            // Check for hole (elision)
            if self.check(&TokenKind::Comma) {
                elements.push(None);
                self.advance();
                continue;
            }

            // Check for spread element: ...expr
            if self.check(&TokenKind::DotDotDot) {
                let spread_span = self.current.span;
                self.advance(); // consume ...
                let argument = self.parse_expression()?;
                let end_span = argument.span();
                let full_span = Span::new(spread_span.start, end_span.end, spread_span.line, spread_span.column);
                elements.push(Some(ArrayElement::Spread(SpreadElement {
                    argument: Rc::new(argument),
                    span: full_span,
                })));
            } else {
                // Parse element expression
                let expr = self.parse_expression()?;
                elements.push(Some(ArrayElement::Expression(expr)));
            }

            // Check for comma or end
            if !self.check(&TokenKind::RBracket) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        Ok(elements)
    }

    /// Parse object literal properties
    fn parse_object_properties(&mut self) -> Result<Vec<ObjectProperty>, JsError> {
        let mut properties = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let prop_span = self.current.span;

            // Parse property key
            let (key, key_name, computed) = match &self.current.kind {
                TokenKind::Identifier(name) => {
                    let name = name.cheap_clone();
                    let id = Identifier {
                        name: name.cheap_clone(),
                        span: self.current.span,
                    };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::String(s) => {
                    let value = s.cheap_clone();
                    let lit = crate::ast::StringLiteral {
                        value: value.cheap_clone(),
                        span: self.current.span,
                    };
                    self.advance();
                    (ObjectPropertyKey::String(lit), None, false)
                }
                TokenKind::Number(n) => {
                    let value = *n;
                    let lit = Literal {
                        value: LiteralValue::Number(value),
                        span: self.current.span,
                    };
                    self.advance();
                    (ObjectPropertyKey::Number(lit), None, false)
                }
                TokenKind::LBracket => {
                    // Computed property: [expr]
                    self.advance();
                    let key_expr = self.parse_expression()?;
                    self.expect(&TokenKind::RBracket)?;
                    (ObjectPropertyKey::Computed(Rc::new(key_expr)), None, true)
                }
                // Contextual keywords can be used as property names
                TokenKind::Module => {
                    let name = self.lexer.string_dict().get_or_insert("module");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::Namespace => {
                    let name = self.lexer.string_dict().get_or_insert("namespace");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::Type => {
                    let name = self.lexer.string_dict().get_or_insert("type");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::Async => {
                    let name = self.lexer.string_dict().get_or_insert("async");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::From => {
                    let name = self.lexer.string_dict().get_or_insert("from");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::Of => {
                    let name = self.lexer.string_dict().get_or_insert("of");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::Readonly => {
                    let name = self.lexer.string_dict().get_or_insert("readonly");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::Declare => {
                    let name = self.lexer.string_dict().get_or_insert("declare");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::Abstract => {
                    let name = self.lexer.string_dict().get_or_insert("abstract");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                TokenKind::As => {
                    let name = self.lexer.string_dict().get_or_insert("as");
                    let id = Identifier { name: name.cheap_clone(), span: self.current.span };
                    self.advance();
                    (ObjectPropertyKey::Identifier(id), Some(name), false)
                }
                _ => {
                    return Err(JsError::syntax_error(
                        format!("Expected property key, found {:?}", self.current.kind),
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
            };

            // Check for method shorthand: { method() { ... } } or { method(): Type { ... } }
            if self.check(&TokenKind::LParen) {
                // This is a method shorthand
                self.advance(); // consume (
                let params = self.parse_function_params()?;
                self.expect(&TokenKind::RParen)?;

                // Optional return type
                let return_type = if self.check(&TokenKind::Colon) {
                    self.advance();
                    Some(Box::new(self.parse_type_annotation()?))
                } else {
                    None
                };

                // Parse function body
                let body = self.parse_block_statement()?;
                let body_end_span = self.current.span;

                // Create a function expression as the value
                let func_expr = Expression::Function(Box::new(FunctionExpression {
                    id: None,
                    params: Rc::from(params),
                    return_type,
                    type_parameters: None,
                    body: Rc::new(body),
                    generator: false,
                    async_: false,
                    span: Span::new(prop_span.start, body_end_span.start, prop_span.line, prop_span.column),
                }));

                let prop = Property {
                    key,
                    value: func_expr,
                    kind: PropertyKind::Init,
                    computed,
                    shorthand: false,
                    method: true,
                    span: Span::new(prop_span.start, body_end_span.start, prop_span.line, prop_span.column),
                };
                properties.push(ObjectProperty::Property(Box::new(prop)));

                // Check for comma or end
                if !self.check(&TokenKind::RBrace) {
                    if self.check(&TokenKind::Comma) {
                        self.advance();
                    }
                }
                continue;
            }

            // Check for shorthand property: { x } means { x: x }
            let (value, shorthand) = if self.check(&TokenKind::Colon) {
                self.advance();
                (self.parse_expression()?, false)
            } else if let Some(name) = key_name {
                // Shorthand property
                let id = match &key {
                    ObjectPropertyKey::Identifier(id) => id.clone(),
                    _ => {
                        return Err(JsError::syntax_error(
                            "Shorthand property must be identifier",
                            prop_span.line,
                            prop_span.column,
                        ));
                    }
                };
                (Expression::Identifier(id), true)
            } else {
                return Err(JsError::syntax_error(
                    "Expected ':' after property key",
                    self.current.span.line,
                    self.current.span.column,
                ));
            };

            let prop_end_span = self.current.span;
            let prop = Property {
                key,
                value,
                kind: PropertyKind::Init,
                computed,
                shorthand,
                method: false,
                span: Span::new(prop_span.start, prop_end_span.start, prop_span.line, prop_span.column),
            };
            properties.push(ObjectProperty::Property(Box::new(prop)));

            // Check for comma or end
            if !self.check(&TokenKind::RBrace) {
                if self.check(&TokenKind::Comma) {
                    self.advance();
                }
            }
        }

        Ok(properties)
    }
}
