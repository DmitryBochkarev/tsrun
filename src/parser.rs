//! Parser for TypeScript source code
//!
//! Uses recursive descent with Pratt parsing for expressions.

use std::rc::Rc;

use crate::ast::*;
use crate::error::JsError;
use crate::lexer::{Lexer, Span, Token, TokenKind};
use crate::string_dict::StringDict;
use crate::value::JsString;

/// Parser for TypeScript source code
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    previous: Token,
    /// When true, 'in' is not treated as a binary operator.
    /// This is used in for-loop init expressions where 'in' separates
    /// the variable from the iterable (for x in obj).
    no_in: bool,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, string_dict: &'a mut StringDict) -> Self {
        let mut lexer = Lexer::new(source, string_dict);
        let current = lexer.next_token();
        Self {
            lexer,
            current,
            previous: Token::eof(0, 1, 1),
            no_in: false,
        }
    }

    /// Helper to intern a string in the dictionary
    #[inline]
    fn intern(&mut self, s: &str) -> JsString {
        self.lexer.string_dict().get_or_insert(s)
    }

    /// Parse a complete program
    pub fn parse_program(&mut self) -> Result<Program, JsError> {
        let mut body = Vec::new();

        while !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        Ok(Program {
            body: body.into(),
            source_type: SourceType::Script,
        })
    }

    // ============ DECORATORS ============

    /// Parse a single decorator: @expression
    /// The expression can be:
    /// - An identifier: @decorator
    /// - A member expression: @Reflect.metadata
    /// - A call expression: @decorator() or @decorator("arg")
    fn parse_decorator(&mut self) -> Result<Decorator, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::At)?;

        // Parse the decorator expression - this can be an identifier, member access, or call
        // We use left-hand-side expression parsing which handles member access and calls
        let expression = self.parse_left_hand_side_expression()?;

        let span = self.span_from(start);
        Ok(Decorator { expression, span })
    }

    /// Parse zero or more decorators: @dec1 @dec2 ...
    fn parse_decorators(&mut self) -> Result<Vec<Decorator>, JsError> {
        let mut decorators = vec![];
        while self.check(&TokenKind::At) {
            decorators.push(self.parse_decorator()?);
        }
        Ok(decorators)
    }

    // ============ STATEMENTS ============

    fn parse_statement(&mut self) -> Result<Statement, JsError> {
        // Check for decorators first - they can precede class declarations
        if self.check(&TokenKind::At) {
            let decorators = self.parse_decorators()?;
            // After decorators, we expect a class declaration (or export)
            // Also handle: @decorator abstract class Foo {}
            if self.check(&TokenKind::Abstract) {
                self.advance(); // consume 'abstract'
                let mut class_decl = self.parse_class_declaration()?;
                class_decl.abstract_ = true;
                class_decl.decorators = decorators;
                return Ok(Statement::ClassDeclaration(Box::new(class_decl)));
            } else if self.check(&TokenKind::Class) {
                let mut class_decl = self.parse_class_declaration()?;
                class_decl.decorators = decorators;
                return Ok(Statement::ClassDeclaration(Box::new(class_decl)));
            } else if self.check(&TokenKind::Export) {
                // Handle: @decorator export class Foo {}
                // or: @decorator export default class Foo {}
                let mut export_decl = self.parse_export()?;
                // If the export contains a class declaration, attach decorators to it
                if let Some(ref mut decl) = export_decl.declaration {
                    if let Statement::ClassDeclaration(ref mut class_decl) = **decl {
                        class_decl.decorators = decorators;
                    }
                }
                return Ok(Statement::Export(Box::new(export_decl)));
            } else {
                return Err(JsError::syntax_error(
                    "Decorators can only be applied to class declarations".to_string(),
                    self.current.span.line,
                    self.current.span.column,
                ));
            }
        }

        // Handle abstract class without decorators
        if self.check(&TokenKind::Abstract) {
            self.advance(); // consume 'abstract'
            let mut class_decl = self.parse_class_declaration()?;
            class_decl.abstract_ = true;
            return Ok(Statement::ClassDeclaration(Box::new(class_decl)));
        }

        // Check for labeled statement first (identifier followed by colon)
        // Must be done before match due to borrow checker
        if self.check_identifier() && self.peek_is(&TokenKind::Colon) {
            return self.parse_labeled_statement();
        }

        // Check for const enum before variable declaration
        if self.check(&TokenKind::Const) && self.peek_is(&TokenKind::Enum) {
            return Ok(Statement::EnumDeclaration(Box::new(self.parse_enum()?)));
        }

        match &self.current.kind {
            TokenKind::Let | TokenKind::Const | TokenKind::Var => Ok(
                Statement::VariableDeclaration(self.parse_variable_declaration()?),
            ),
            TokenKind::Function => Ok(Statement::FunctionDeclaration(Box::new(
                self.parse_function_declaration(false)?,
            ))),
            TokenKind::Async => {
                // async function declaration
                self.advance(); // consume 'async'
                self.require_token(&TokenKind::Function)?;
                let mut func = self.parse_function_declaration_inner()?;
                func.async_ = true;
                Ok(Statement::FunctionDeclaration(Box::new(func)))
            }
            TokenKind::Class => Ok(Statement::ClassDeclaration(Box::new(
                self.parse_class_declaration()?,
            ))),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Do => self.parse_do_while_statement(),
            TokenKind::Switch => self.parse_switch_statement(),
            TokenKind::Try => self.parse_try_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Break => self.parse_break_statement(),
            TokenKind::Continue => self.parse_continue_statement(),
            TokenKind::Throw => self.parse_throw_statement(),
            TokenKind::LBrace => Ok(Statement::Block(self.parse_block_statement()?)),
            TokenKind::Semicolon => {
                self.advance();
                Ok(Statement::Empty)
            }
            TokenKind::Debugger => {
                self.advance();
                self.expect_semicolon()?;
                Ok(Statement::Debugger)
            }
            // TypeScript declarations (no-op at runtime)
            TokenKind::Type => Ok(Statement::TypeAlias(Box::new(self.parse_type_alias()?))),
            TokenKind::Interface => Ok(Statement::InterfaceDeclaration(Box::new(
                self.parse_interface()?,
            ))),
            TokenKind::Enum => Ok(Statement::EnumDeclaration(Box::new(self.parse_enum()?))),
            // Module declarations
            TokenKind::Import => Ok(Statement::Import(Box::new(self.parse_import()?))),
            TokenKind::Export => Ok(Statement::Export(Box::new(self.parse_export()?))),
            TokenKind::Namespace | TokenKind::Module => {
                // Check if this is a namespace declaration or an expression
                // namespace Foo { ... } vs module(arg) or namespace.prop
                let checkpoint = self.lexer.checkpoint();
                let saved = self.current.clone();
                self.advance(); // consume namespace/module

                let is_declaration = if self.check_identifier() {
                    // Check if next is { (namespace declaration) or something else
                    let saved2 = self.current.clone();
                    self.advance();
                    let result = self.check(&TokenKind::LBrace);
                    self.current = saved2;
                    result
                } else {
                    false
                };

                // Restore position
                self.lexer.restore(checkpoint);
                self.current = saved;

                if is_declaration {
                    Ok(Statement::NamespaceDeclaration(
                        Box::new(self.parse_namespace()?),
                    ))
                } else {
                    // Parse as expression statement (module() or namespace.something)
                    let expr = self.parse_expression()?;
                    self.expect_semicolon()?;
                    let span = expr.span();
                    Ok(Statement::Expression(ExpressionStatement {
                        expression: Rc::new(expr),
                        span,
                    }))
                }
            }
            _ => {
                // Expression statement
                let expr = self.parse_expression()?;
                self.expect_semicolon()?;
                let span = expr.span();
                Ok(Statement::Expression(ExpressionStatement {
                    expression: Rc::new(expr),
                    span,
                }))
            }
        }
    }

    fn parse_variable_declaration(&mut self) -> Result<VariableDeclaration, JsError> {
        let start = self.current.span;
        let kind = match &self.current.kind {
            TokenKind::Let => VariableKind::Let,
            TokenKind::Const => VariableKind::Const,
            TokenKind::Var => VariableKind::Var,
            _ => return Err(self.unexpected_token("variable declaration")),
        };
        self.advance();

        let mut declarations = vec![self.parse_variable_declarator()?];

        while self.match_token(&TokenKind::Comma) {
            declarations.push(self.parse_variable_declarator()?);
        }

        self.expect_semicolon()?;

        let span = self.span_from(start);
        Ok(VariableDeclaration {
            kind,
            declarations: declarations.into(),
            span,
        })
    }

    fn parse_variable_declarator(&mut self) -> Result<VariableDeclarator, JsError> {
        let start = self.current.span;
        let id = self.parse_binding_pattern()?;

        // Optional type annotation
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        // Optional initializer
        let init = if self.match_token(&TokenKind::Eq) {
            Some(Rc::new(self.parse_assignment_expression()?))
        } else {
            None
        };

        let span = self.span_from(start);
        Ok(VariableDeclarator {
            id,
            type_annotation,
            init,
            span,
        })
    }

    fn parse_binding_pattern(&mut self) -> Result<Pattern, JsError> {
        match &self.current.kind {
            TokenKind::Identifier(_) => {
                let id = self.parse_identifier()?;
                self.validate_binding_identifier(&id)?;
                Ok(Pattern::Identifier(id))
            }
            // Allow contextual keywords as binding identifiers (e.g., function param names)
            TokenKind::Type
            | TokenKind::From
            | TokenKind::As
            | TokenKind::Of
            | TokenKind::Namespace
            | TokenKind::Module
            | TokenKind::Any
            | TokenKind::Unknown
            | TokenKind::Never
            | TokenKind::Keyof
            | TokenKind::Infer
            | TokenKind::Is
            | TokenKind::Asserts
            | TokenKind::Readonly => {
                let id = self.parse_identifier()?;
                self.validate_binding_identifier(&id)?;
                Ok(Pattern::Identifier(id))
            }
            TokenKind::LBrace => self.parse_object_pattern(),
            TokenKind::LBracket => self.parse_array_pattern(),
            _ => Err(self.unexpected_token("binding pattern")),
        }
    }

    /// Validate that an identifier can be used as a binding name in strict mode.
    /// In strict mode, 'eval' and 'arguments' cannot be used as binding identifiers.
    fn validate_binding_identifier(&self, id: &Identifier) -> Result<(), JsError> {
        let name = id.name.as_ref();
        if name == "eval" || name == "arguments" {
            return Err(JsError::syntax_error(
                format!(
                    "'{}' cannot be used as a binding identifier in strict mode",
                    name
                ),
                id.span.line,
                id.span.column,
            ));
        }
        Ok(())
    }

    fn parse_object_pattern(&mut self) -> Result<Pattern, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::LBrace)?;

        let mut properties = vec![];

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.match_token(&TokenKind::DotDotDot) {
                // Rest element
                let arg_start = self.current.span;
                let argument = Box::new(self.parse_binding_pattern()?);
                let span = self.span_from(arg_start);
                properties.push(ObjectPatternProperty::Rest(RestElement {
                    argument,
                    type_annotation: None,
                    span,
                }));
                break; // Rest must be last
            }

            let prop_start = self.current.span;
            let key = self.parse_property_name()?;

            let (value, shorthand) = if self.match_token(&TokenKind::Colon) {
                (self.parse_binding_pattern()?, false)
            } else {
                // Shorthand: { a } is { a: a }
                match &key {
                    ObjectPropertyKey::Identifier(id) => (Pattern::Identifier(id.clone()), true),
                    _ => return Err(self.error("Shorthand property must be an identifier")),
                }
            };

            // Optional default value
            let value = if self.match_token(&TokenKind::Eq) {
                let right = Rc::new(self.parse_assignment_expression()?);
                let span = self.span_from(prop_start);
                Pattern::Assignment(AssignmentPattern {
                    left: Box::new(value),
                    right,
                    span,
                })
            } else {
                value
            };

            let span = self.span_from(prop_start);
            properties.push(ObjectPatternProperty::KeyValue {
                key,
                value,
                shorthand,
                span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RBrace)?;

        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        let span = self.span_from(start);
        Ok(Pattern::Object(ObjectPattern {
            properties,
            type_annotation,
            span,
        }))
    }

    fn parse_array_pattern(&mut self) -> Result<Pattern, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::LBracket)?;

        let mut elements = vec![];

        while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
            if self.match_token(&TokenKind::Comma) {
                // Hole
                elements.push(None);
                continue;
            }

            if self.match_token(&TokenKind::DotDotDot) {
                let arg_start = self.current.span;
                let argument = Box::new(self.parse_binding_pattern()?);
                let span = self.span_from(arg_start);
                elements.push(Some(Pattern::Rest(RestElement {
                    argument,
                    type_annotation: None,
                    span,
                })));
                break; // Rest must be last
            }

            let elem = self.parse_binding_pattern()?;

            // Optional default value
            let elem = if self.match_token(&TokenKind::Eq) {
                let elem_span = elem.span();
                let right = Rc::new(self.parse_assignment_expression()?);
                let span = self.span_from(elem_span);
                Pattern::Assignment(AssignmentPattern {
                    left: Box::new(elem),
                    right,
                    span,
                })
            } else {
                elem
            };

            elements.push(Some(elem));

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RBracket)?;

        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        let span = self.span_from(start);
        Ok(Pattern::Array(ArrayPattern {
            elements,
            type_annotation,
            span,
        }))
    }

    fn parse_function_declaration(
        &mut self,
        is_async: bool,
    ) -> Result<FunctionDeclaration, JsError> {
        self.require_token(&TokenKind::Function)?;
        let mut func = self.parse_function_declaration_inner()?;
        func.async_ = is_async;
        Ok(func)
    }

    fn parse_function_declaration_inner(&mut self) -> Result<FunctionDeclaration, JsError> {
        let start = self.current.span;

        let generator = self.match_token(&TokenKind::Star);
        let id = if self.check_identifier() {
            Some(self.parse_identifier()?)
        } else {
            None
        };

        let type_parameters = self.parse_optional_type_parameters()?;
        let params: Rc<[_]> = self.parse_function_params()?.into();
        let return_type = self.parse_optional_return_type()?;
        let body = Rc::new(self.parse_block_statement()?);

        let span = self.span_from(start);
        Ok(FunctionDeclaration {
            id,
            params,
            return_type,
            type_parameters,
            body,
            generator,
            async_: false,
            span,
        })
    }

    fn parse_function_params(&mut self) -> Result<Vec<FunctionParam>, JsError> {
        self.require_token(&TokenKind::LParen)?;

        let mut params = vec![];
        let mut seen_names: std::collections::HashSet<JsString> = std::collections::HashSet::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            let param_start = self.current.span;

            // Parse parameter decorators (e.g., @inject param)
            let decorators = self.parse_decorators()?;

            // TypeScript parameter properties: public/private/protected/readonly
            let accessibility = self.parse_accessibility();
            let readonly = self.match_token(&TokenKind::Readonly);

            // Check for rest parameter
            let pattern = if self.match_token(&TokenKind::DotDotDot) {
                let arg = self.parse_binding_pattern()?;
                let span = self.span_from(param_start);
                Pattern::Rest(RestElement {
                    argument: Box::new(arg),
                    type_annotation: None,
                    span,
                })
            } else {
                self.parse_binding_pattern()?
            };

            // Check for duplicate parameter names in strict mode
            Self::check_duplicate_params(&pattern, &mut seen_names)?;

            let optional = self.match_token(&TokenKind::Question);

            let type_annotation = if self.match_token(&TokenKind::Colon) {
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            // Default value becomes AssignmentPattern
            let pattern = if self.match_token(&TokenKind::Eq) {
                let right = Rc::new(self.parse_assignment_expression()?);
                let span = self.span_from(param_start);
                Pattern::Assignment(AssignmentPattern {
                    left: Box::new(pattern),
                    right,
                    span,
                })
            } else {
                pattern
            };

            let span = self.span_from(param_start);
            params.push(FunctionParam {
                pattern,
                type_annotation,
                optional,
                decorators,
                accessibility,
                readonly,
                span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RParen)?;
        Ok(params)
    }

    /// Check for duplicate parameter names in strict mode.
    /// Collects all binding names from a pattern and checks against seen names.
    fn check_duplicate_params(
        pattern: &Pattern,
        seen: &mut std::collections::HashSet<JsString>,
    ) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                if !seen.insert(id.name.clone()) {
                    return Err(JsError::syntax_error(
                        format!("Duplicate parameter name '{}' not allowed", id.name),
                        id.span.line,
                        id.span.column,
                    ));
                }
            }
            Pattern::Object(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { value, .. } => {
                            Self::check_duplicate_params(value, seen)?;
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            Self::check_duplicate_params(&rest.argument, seen)?;
                        }
                    }
                }
            }
            Pattern::Array(arr) => {
                for elem in arr.elements.iter().flatten() {
                    Self::check_duplicate_params(elem, seen)?;
                }
            }
            Pattern::Rest(rest) => {
                Self::check_duplicate_params(&rest.argument, seen)?;
            }
            Pattern::Assignment(assign) => {
                Self::check_duplicate_params(&assign.left, seen)?;
            }
        }
        Ok(())
    }

    fn parse_class_declaration(&mut self) -> Result<ClassDeclaration, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Class)?;

        let id = if self.check_identifier() {
            Some(self.parse_identifier()?)
        } else {
            None
        };

        let type_parameters = self.parse_optional_type_parameters()?;

        let super_class = if self.match_token(&TokenKind::Extends) {
            Some(Rc::new(self.parse_left_hand_side_expression()?))
        } else {
            None
        };

        let implements = if self.match_token(&TokenKind::Implements) {
            let mut impls = vec![self.parse_type_reference()?];
            while self.match_token(&TokenKind::Comma) {
                impls.push(self.parse_type_reference()?);
            }
            impls
        } else {
            vec![]
        };

        let body = self.parse_class_body()?;

        let span = self.span_from(start);
        Ok(ClassDeclaration {
            id,
            type_parameters,
            super_class,
            implements,
            body,
            decorators: vec![],
            abstract_: false,
            span,
        })
    }

    fn parse_class_body(&mut self) -> Result<ClassBody, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::LBrace)?;

        let mut members = vec![];

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            // Skip empty statements
            if self.match_token(&TokenKind::Semicolon) {
                continue;
            }

            members.push(self.parse_class_member()?);
        }

        self.require_token(&TokenKind::RBrace)?;

        let span = self.span_from(start);
        Ok(ClassBody { members, span })
    }

    fn parse_class_member(&mut self) -> Result<ClassMember, JsError> {
        let start = self.current.span;

        // Parse decorators first
        let decorators = self.parse_decorators()?;

        let static_ = self.match_token(&TokenKind::Static);

        // Check for static initialization block: static { ... }
        if static_ && self.check(&TokenKind::LBrace) {
            let block = self.parse_block_statement()?;
            return Ok(ClassMember::StaticBlock(block));
        }

        // Parse abstract modifier (TypeScript)
        let is_abstract = self.match_token(&TokenKind::Abstract);

        let accessibility = self.parse_accessibility();
        let readonly = self.match_token(&TokenKind::Readonly);
        let accessor = self.match_token(&TokenKind::Accessor);

        // Check for async method
        let is_async = self.match_token(&TokenKind::Async);

        // Check for generator method (either *method() or async *method())
        let is_generator = self.match_token(&TokenKind::Star);

        // Check for constructor
        if !static_ && self.check_keyword("constructor") {
            self.advance();
            let params = self.parse_function_params()?;
            let body = self.parse_block_statement()?;
            let span = self.span_from(start);
            return Ok(ClassMember::Constructor(Box::new(ClassConstructor {
                params,
                body,
                accessibility,
                span,
            })));
        }

        // Check for getter/setter
        let method_kind = if self.check_keyword("get") {
            self.advance();
            MethodKind::Get
        } else if self.check_keyword("set") {
            self.advance();
            MethodKind::Set
        } else {
            MethodKind::Method
        };

        let (key, computed) = self.parse_class_element_name()?;

        // Method or property?
        if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
            // Method
            let type_params = self.parse_optional_type_parameters()?;
            let params: Rc<[_]> = self.parse_function_params()?.into();
            let return_type = self.parse_optional_return_type()?;

            // Abstract methods have no body - just a semicolon
            let body = if is_abstract {
                self.expect_semicolon()?;
                // Create empty body for abstract methods (they're never called at runtime)
                Rc::new(BlockStatement {
                    body: Rc::from([]),
                    span: self.span_from(start),
                })
            } else {
                Rc::new(self.parse_block_statement()?)
            };

            let value = FunctionExpression {
                id: None,
                params,
                return_type,
                type_parameters: type_params,
                body,
                generator: is_generator,
                async_: is_async,
                span: self.span_from(start),
            };

            let span = self.span_from(start);
            Ok(ClassMember::Method(Box::new(ClassMethod {
                key,
                value,
                kind: method_kind,
                computed,
                static_,
                accessibility,
                decorators,
                span,
            })))
        } else {
            // Property
            let optional = self.match_token(&TokenKind::Question);
            let type_annotation = if self.match_token(&TokenKind::Colon) {
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let value = if self.match_token(&TokenKind::Eq) {
                Some(Box::new(self.parse_assignment_expression()?))
            } else {
                None
            };

            self.expect_semicolon()?;

            let span = self.span_from(start);
            Ok(ClassMember::Property(Box::new(ClassProperty {
                key,
                value,
                type_annotation,
                computed,
                static_,
                readonly,
                optional,
                accessor,
                accessibility,
                decorators,
                span,
            })))
        }
    }

    fn parse_accessibility(&mut self) -> Option<Accessibility> {
        match &self.current.kind {
            TokenKind::Public => {
                self.advance();
                Some(Accessibility::Public)
            }
            TokenKind::Private => {
                self.advance();
                Some(Accessibility::Private)
            }
            TokenKind::Protected => {
                self.advance();
                Some(Accessibility::Protected)
            }
            _ => None,
        }
    }

    fn parse_class_element_name(&mut self) -> Result<(ObjectPropertyKey, bool), JsError> {
        if self.match_token(&TokenKind::LBracket) {
            // Inside [...] brackets, 'in' is allowed as binary operator even in for-loop context
            let saved_no_in = self.no_in;
            self.no_in = false;
            let expr = self.parse_assignment_expression();
            self.no_in = saved_no_in;
            let expr = expr?;
            self.require_token(&TokenKind::RBracket)?;
            Ok((ObjectPropertyKey::Computed(Rc::new(expr)), true))
        } else if self.match_token(&TokenKind::Hash) {
            // Private identifier: #name (name includes # prefix)
            let name = self.parse_private_identifier()?;
            Ok((ObjectPropertyKey::PrivateIdentifier(name), false))
        } else {
            Ok((self.parse_property_name()?, false))
        }
    }

    fn parse_block_statement(&mut self) -> Result<BlockStatement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::LBrace)?;

        let mut body = vec![];

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        self.require_token(&TokenKind::RBrace)?;

        let span = self.span_from(start);
        Ok(BlockStatement {
            body: body.into(),
            span,
        })
    }

    fn parse_if_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::If)?;
        self.require_token(&TokenKind::LParen)?;
        let test = Rc::new(self.parse_expression()?);
        self.require_token(&TokenKind::RParen)?;

        let consequent = Rc::new(self.parse_statement()?);

        let alternate = if self.match_token(&TokenKind::Else) {
            Some(Rc::new(self.parse_statement()?))
        } else {
            None
        };

        let span = self.span_from(start);
        Ok(Statement::If(IfStatement {
            test,
            consequent,
            alternate,
            span,
        }))
    }

    fn parse_for_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::For)?;

        // Check for 'for await' (async iteration)
        let is_await = self.match_token(&TokenKind::Await);

        self.require_token(&TokenKind::LParen)?;

        // Check for for-in or for-of
        let init = if self.check(&TokenKind::Semicolon) {
            None
        } else if self.check(&TokenKind::Let)
            || self.check(&TokenKind::Const)
            || self.check(&TokenKind::Var)
        {
            let kind = match &self.current.kind {
                TokenKind::Let => VariableKind::Let,
                TokenKind::Const => VariableKind::Const,
                TokenKind::Var => VariableKind::Var,
                _ => return Err(self.error("expected let, const, or var")),
            };
            self.advance();

            let decl_start = self.current.span;
            let id = self.parse_binding_pattern()?;

            // Parse optional type annotation (for TypeScript)
            let type_ann = if self.match_token(&TokenKind::Colon) {
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            // Check for for-in or for-of
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_of = self.check(&TokenKind::Of);
                self.advance();

                let right = self.parse_expression()?;
                self.require_token(&TokenKind::RParen)?;
                let body = Rc::new(self.parse_statement()?);

                let span = self.span_from(start);
                let left = ForInOfLeft::Variable(VariableDeclaration {
                    kind,
                    declarations: Rc::from([VariableDeclarator {
                        id,
                        type_annotation: type_ann,
                        init: None,
                        span: self.span_from(decl_start),
                    }]),
                    span: self.span_from(decl_start),
                });

                return if is_of {
                    Ok(Statement::ForOf(Box::new(ForOfStatement {
                        left,
                        right: Rc::new(right),
                        body,
                        await_: is_await,
                        span,
                    })))
                } else {
                    if is_await {
                        return Err(self.error("for await is only valid with for-of loops"));
                    }
                    Ok(Statement::ForIn(Box::new(ForInStatement {
                        left,
                        right: Rc::new(right),
                        body,
                        span,
                    })))
                };
            }

            // Regular for loop - 'await' is not valid here
            if is_await {
                return Err(self.error("for await is only valid with for-of loops"));
            }

            // Regular for loop - type_ann already parsed above

            let init_val = if self.match_token(&TokenKind::Eq) {
                Some(Rc::new(self.parse_assignment_expression()?))
            } else {
                None
            };

            let mut declarations = vec![VariableDeclarator {
                id,
                type_annotation: type_ann,
                init: init_val,
                span: self.span_from(decl_start),
            }];

            while self.match_token(&TokenKind::Comma) {
                declarations.push(self.parse_variable_declarator()?);
            }

            Some(ForInit::Variable(VariableDeclaration {
                kind,
                declarations: declarations.into(),
                span: self.span_from(decl_start),
            }))
        } else {
            // Parse expression with no_in=true to allow 'in' as for-in separator
            self.no_in = true;
            let expr = self.parse_expression();
            self.no_in = false;
            let expr = expr?;

            // Check for for-in or for-of
            if self.check(&TokenKind::In) || self.check(&TokenKind::Of) {
                let is_of = self.check(&TokenKind::Of);
                self.advance();

                let right = self.parse_expression()?;
                self.require_token(&TokenKind::RParen)?;
                let body = Rc::new(self.parse_statement()?);

                let span = self.span_from(start);
                let left = ForInOfLeft::Pattern(self.expression_to_pattern(&expr)?);

                return if is_of {
                    Ok(Statement::ForOf(Box::new(ForOfStatement {
                        left,
                        right: Rc::new(right),
                        body,
                        await_: is_await,
                        span,
                    })))
                } else {
                    if is_await {
                        return Err(self.error("for await is only valid with for-of loops"));
                    }
                    Ok(Statement::ForIn(Box::new(ForInStatement {
                        left,
                        right: Rc::new(right),
                        body,
                        span,
                    })))
                };
            }

            // Regular for loop - 'await' is not valid here
            if is_await {
                return Err(self.error("for await is only valid with for-of loops"));
            }

            Some(ForInit::Expression(Rc::new(expr)))
        };

        self.require_token(&TokenKind::Semicolon)?;

        let test = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(Rc::new(self.parse_expression()?))
        };

        self.require_token(&TokenKind::Semicolon)?;

        let update = if self.check(&TokenKind::RParen) {
            None
        } else {
            Some(Rc::new(self.parse_expression()?))
        };

        self.require_token(&TokenKind::RParen)?;
        let body = Rc::new(self.parse_statement()?);

        let span = self.span_from(start);
        Ok(Statement::For(Box::new(ForStatement {
            init,
            test,
            update,
            body,
            span,
        })))
    }

    fn parse_while_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::While)?;
        self.require_token(&TokenKind::LParen)?;
        let test = Rc::new(self.parse_expression()?);
        self.require_token(&TokenKind::RParen)?;
        let body = Rc::new(self.parse_statement()?);

        let span = self.span_from(start);
        Ok(Statement::While(WhileStatement { test, body, span }))
    }

    fn parse_do_while_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Do)?;
        let body = Rc::new(self.parse_statement()?);
        self.require_token(&TokenKind::While)?;
        self.require_token(&TokenKind::LParen)?;
        let test = Rc::new(self.parse_expression()?);
        self.require_token(&TokenKind::RParen)?;
        self.expect_semicolon()?;

        let span = self.span_from(start);
        Ok(Statement::DoWhile(DoWhileStatement { body, test, span }))
    }

    fn parse_switch_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Switch)?;
        self.require_token(&TokenKind::LParen)?;
        let discriminant = Rc::new(self.parse_expression()?);
        self.require_token(&TokenKind::RParen)?;
        self.require_token(&TokenKind::LBrace)?;

        let mut cases = vec![];

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let case_start = self.current.span;
            let test = if self.match_token(&TokenKind::Case) {
                Some(Rc::new(self.parse_expression()?))
            } else {
                self.require_token(&TokenKind::Default)?;
                None
            };

            self.require_token(&TokenKind::Colon)?;

            let mut consequent = vec![];
            while !self.check(&TokenKind::Case)
                && !self.check(&TokenKind::Default)
                && !self.check(&TokenKind::RBrace)
                && !self.is_at_end()
            {
                consequent.push(self.parse_statement()?);
            }

            let span = self.span_from(case_start);
            cases.push(SwitchCase {
                test,
                consequent: consequent.into(),
                span,
            });
        }

        self.require_token(&TokenKind::RBrace)?;

        let span = self.span_from(start);
        Ok(Statement::Switch(SwitchStatement {
            discriminant,
            cases: cases.into(),
            span,
        }))
    }

    fn parse_try_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Try)?;
        let block = self.parse_block_statement()?;

        let handler = if self.match_token(&TokenKind::Catch) {
            let catch_start = self.current.span;
            let param = if self.match_token(&TokenKind::LParen) {
                let p = self.parse_binding_pattern()?;
                // Parse optional type annotation (TypeScript) - discarded at runtime
                if self.match_token(&TokenKind::Colon) {
                    let _ = self.parse_type_annotation()?;
                }
                self.require_token(&TokenKind::RParen)?;
                Some(p)
            } else {
                None
            };
            let body = self.parse_block_statement()?;
            let span = self.span_from(catch_start);
            Some(CatchClause { param, body, span })
        } else {
            None
        };

        let finalizer = if self.match_token(&TokenKind::Finally) {
            Some(self.parse_block_statement()?)
        } else {
            None
        };

        if handler.is_none() && finalizer.is_none() {
            return Err(self.error("Try statement must have catch or finally"));
        }

        let span = self.span_from(start);
        Ok(Statement::Try(Box::new(TryStatement {
            block,
            handler,
            finalizer,
            span,
        })))
    }

    fn parse_return_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Return)?;

        let argument = if self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::RBrace)
            || self.lexer.had_newline_before()
        {
            None
        } else {
            Some(Rc::new(self.parse_expression()?))
        };

        self.expect_semicolon()?;

        let span = self.span_from(start);
        Ok(Statement::Return(ReturnStatement { argument, span }))
    }

    fn parse_break_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Break)?;

        let label = if !self.check(&TokenKind::Semicolon)
            && !self.lexer.had_newline_before()
            && self.check_identifier()
        {
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.expect_semicolon()?;

        let span = self.span_from(start);
        Ok(Statement::Break(BreakStatement { label, span }))
    }

    fn parse_continue_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Continue)?;

        let label = if !self.check(&TokenKind::Semicolon)
            && !self.lexer.had_newline_before()
            && self.check_identifier()
        {
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.expect_semicolon()?;

        let span = self.span_from(start);
        Ok(Statement::Continue(ContinueStatement { label, span }))
    }

    fn parse_throw_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Throw)?;

        if self.lexer.had_newline_before() {
            return Err(self.error("Illegal newline after throw"));
        }

        let argument = Rc::new(self.parse_expression()?);
        self.expect_semicolon()?;

        let span = self.span_from(start);
        Ok(Statement::Throw(ThrowStatement { argument, span }))
    }

    fn parse_labeled_statement(&mut self) -> Result<Statement, JsError> {
        let start = self.current.span;
        let label = self.parse_identifier()?;
        self.require_token(&TokenKind::Colon)?;
        let body = Rc::new(self.parse_statement()?);

        let span = self.span_from(start);
        Ok(Statement::Labeled(LabeledStatement { label, body, span }))
    }

    // TypeScript declarations (stubs for now)

    fn parse_type_alias(&mut self) -> Result<TypeAliasDeclaration, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Type)?;
        self.parse_type_alias_inner(start)
    }

    /// Parse type alias after 'type' keyword has been consumed.
    /// Used by `export type ID = ...` syntax.
    fn parse_type_alias_after_type_keyword(&mut self) -> Result<TypeAliasDeclaration, JsError> {
        // Use previous token span as start since we already consumed 'type'
        let start = self.previous.span;
        self.parse_type_alias_inner(start)
    }

    fn parse_type_alias_inner(&mut self, start: Span) -> Result<TypeAliasDeclaration, JsError> {
        let id = self.parse_identifier()?;
        let type_parameters = self.parse_optional_type_parameters()?;
        self.require_token(&TokenKind::Eq)?;
        let type_annotation = Box::new(self.parse_type_annotation()?);
        self.expect_semicolon()?;

        let span = self.span_from(start);
        Ok(TypeAliasDeclaration {
            id,
            type_parameters,
            type_annotation,
            span,
        })
    }

    fn parse_interface(&mut self) -> Result<InterfaceDeclaration, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Interface)?;
        let id = self.parse_identifier()?;
        let type_parameters = self.parse_optional_type_parameters()?;

        let extends = if self.match_token(&TokenKind::Extends) {
            let mut refs = vec![self.parse_type_reference()?];
            while self.match_token(&TokenKind::Comma) {
                refs.push(self.parse_type_reference()?);
            }
            refs
        } else {
            vec![]
        };

        self.require_token(&TokenKind::LBrace)?;
        let body = self.parse_type_members()?;
        self.require_token(&TokenKind::RBrace)?;

        let span = self.span_from(start);
        Ok(InterfaceDeclaration {
            id,
            type_parameters,
            extends,
            body,
            span,
        })
    }

    fn parse_enum(&mut self) -> Result<EnumDeclaration, JsError> {
        let start = self.current.span;
        let const_ = self.match_token(&TokenKind::Const);
        self.require_token(&TokenKind::Enum)?;
        let id = self.parse_identifier()?;
        self.require_token(&TokenKind::LBrace)?;

        let mut members = vec![];
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let member_start = self.current.span;
            let member_id = self.parse_identifier()?;
            let initializer = if self.match_token(&TokenKind::Eq) {
                Some(self.parse_assignment_expression()?)
            } else {
                None
            };
            let span = self.span_from(member_start);
            members.push(EnumMember {
                id: member_id,
                initializer,
                span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RBrace)?;

        let span = self.span_from(start);
        Ok(EnumDeclaration {
            id,
            members,
            const_,
            span,
        })
    }

    fn parse_namespace(&mut self) -> Result<NamespaceDeclaration, JsError> {
        let start = self.current.span;
        // Skip 'namespace' or 'module' keyword
        self.advance();

        let id = self.parse_identifier()?;
        self.require_token(&TokenKind::LBrace)?;

        let mut body = vec![];
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_statement()?);
        }

        self.require_token(&TokenKind::RBrace)?;

        let span = self.span_from(start);
        Ok(NamespaceDeclaration {
            id,
            body: body.into(),
            span,
        })
    }

    // Module declarations (stubs)

    fn parse_import(&mut self) -> Result<ImportDeclaration, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Import)?;

        let type_only = self.match_token(&TokenKind::Type);

        let mut specifiers = vec![];

        // import "module"
        if let TokenKind::String(s) = &self.current.kind {
            let source = StringLiteral {
                value: s.clone(),
                span: self.current.span,
            };
            self.advance();
            self.expect_semicolon()?;
            let span = self.span_from(start);
            return Ok(ImportDeclaration {
                specifiers,
                source,
                type_only,
                span,
            });
        }

        // Default import
        if self.check_identifier() {
            let local = self.parse_identifier()?;
            specifiers.push(ImportSpecifier::Default {
                local: local.clone(),
                span: local.span,
            });

            if self.match_token(&TokenKind::Comma) {
                // Continue with named imports
            } else {
                self.require_token(&TokenKind::From)?;
                let source = self.parse_string_literal()?;
                self.expect_semicolon()?;
                let span = self.span_from(start);
                return Ok(ImportDeclaration {
                    specifiers,
                    source,
                    type_only,
                    span,
                });
            }
        }

        // Namespace or named imports
        if self.match_token(&TokenKind::Star) {
            self.require_token(&TokenKind::As)?;
            let local = self.parse_identifier()?;
            specifiers.push(ImportSpecifier::Namespace {
                local: local.clone(),
                span: local.span,
            });
        } else if self.match_token(&TokenKind::LBrace) {
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                let spec_start = self.current.span;
                let imported = self.parse_identifier()?;
                let local = if self.match_token(&TokenKind::As) {
                    self.parse_identifier()?
                } else {
                    imported.clone()
                };
                let span = self.span_from(spec_start);
                specifiers.push(ImportSpecifier::Named {
                    local,
                    imported,
                    span,
                });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.require_token(&TokenKind::RBrace)?;
        }

        self.require_token(&TokenKind::From)?;
        let source = self.parse_string_literal()?;
        self.expect_semicolon()?;

        let span = self.span_from(start);
        Ok(ImportDeclaration {
            specifiers,
            source,
            type_only,
            span,
        })
    }

    fn parse_export(&mut self) -> Result<ExportDeclaration, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Export)?;

        // Check for `export type` which can be:
        // 1. `export type { ... }` - type-only re-export (type_only flag)
        // 2. `export type ID = ...` - type alias export (NOT type_only, fall through to declaration parsing)
        //
        // We consume 'type' and then check what follows. If it's '{' or '*', it's a type-only re-export.
        // Otherwise, we restore and let the normal declaration parsing handle `type ID = ...`
        let type_only = if self.check(&TokenKind::Type) {
            self.advance(); // consume 'type'

            if self.check(&TokenKind::LBrace) || self.check(&TokenKind::Star) {
                // type-only re-export: `export type { x }` or `export type * from`
                true
            } else {
                // This is `export type ID = ...` - restore and let it be parsed as type alias
                // We already consumed 'type', but check() for Type below won't match
                // So we need to insert the type alias parsing here
                let type_alias = self.parse_type_alias_after_type_keyword()?;
                let span = self.span_from(start);
                return Ok(ExportDeclaration {
                    declaration: Some(Box::new(Statement::TypeAlias(Box::new(type_alias)))),
                    specifiers: vec![],
                    source: None,
                    namespace_export: None,
                    default: false,
                    type_only: false,
                    span,
                });
            }
        } else {
            false
        };

        // Check for decorators after export: export @decorator class Foo {}
        let decorators = self.parse_decorators()?;

        // export default
        if self.match_token(&TokenKind::Default) {
            let declaration = if self.check(&TokenKind::At) {
                // export default @decorator class Foo {}
                let more_decorators = self.parse_decorators()?;
                let all_decorators = [decorators, more_decorators].concat();
                if self.check(&TokenKind::Class) {
                    let mut class_decl = self.parse_class_declaration()?;
                    class_decl.decorators = all_decorators;
                    Some(Box::new(Statement::ClassDeclaration(Box::new(class_decl))))
                } else {
                    return Err(JsError::syntax_error(
                        "Decorators can only be applied to class declarations".to_string(),
                        self.current.span.line,
                        self.current.span.column,
                    ));
                }
            } else if self.check(&TokenKind::Async) {
                // export default async function
                self.advance(); // consume 'async'
                self.require_token(&TokenKind::Function)?;
                let mut func = self.parse_function_declaration_inner()?;
                func.async_ = true;
                Some(Box::new(Statement::FunctionDeclaration(Box::new(func))))
            } else if self.check(&TokenKind::Function) {
                Some(Box::new(Statement::FunctionDeclaration(Box::new(
                    self.parse_function_declaration(false)?,
                ))))
            } else if self.check(&TokenKind::Class) {
                let mut class_decl = self.parse_class_declaration()?;
                class_decl.decorators = decorators;
                Some(Box::new(Statement::ClassDeclaration(Box::new(class_decl))))
            } else {
                let expr = self.parse_assignment_expression()?;
                self.expect_semicolon()?;
                let span = expr.span();
                Some(Box::new(Statement::Expression(ExpressionStatement {
                    expression: Rc::new(expr),
                    span,
                })))
            };

            let span = self.span_from(start);
            return Ok(ExportDeclaration {
                declaration,
                specifiers: vec![],
                source: None,
                namespace_export: None,
                default: true,
                type_only,
                span,
            });
        }

        // export { ... }
        if self.check(&TokenKind::LBrace) {
            self.advance();
            let mut specifiers = vec![];

            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                let spec_start = self.current.span;
                // In export specifiers, 'default' is allowed as a name
                let local = self.parse_module_export_name()?;
                let exported = if self.match_token(&TokenKind::As) {
                    self.parse_module_export_name()?
                } else {
                    local.clone()
                };
                let span = self.span_from(spec_start);
                specifiers.push(ExportSpecifier {
                    local,
                    exported,
                    span,
                });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }

            self.require_token(&TokenKind::RBrace)?;

            let source = if self.match_token(&TokenKind::From) {
                Some(self.parse_string_literal()?)
            } else {
                None
            };

            self.expect_semicolon()?;

            let span = self.span_from(start);
            return Ok(ExportDeclaration {
                declaration: None,
                specifiers,
                source,
                namespace_export: None,
                default: false,
                type_only,
                span,
            });
        }

        // export * from OR export * as ns from
        if self.match_token(&TokenKind::Star) {
            // Check for "as <identifier>" for namespace exports
            let namespace_export = if self.match_token(&TokenKind::As) {
                Some(self.parse_identifier()?)
            } else {
                None
            };

            self.require_token(&TokenKind::From)?;
            let source = Some(self.parse_string_literal()?);
            self.expect_semicolon()?;

            let span = self.span_from(start);
            return Ok(ExportDeclaration {
                declaration: None,
                specifiers: vec![],
                source,
                namespace_export,
                default: false,
                type_only,
                span,
            });
        }

        // export declaration
        let declaration = match &self.current.kind {
            TokenKind::Let | TokenKind::Const | TokenKind::Var => Some(Box::new(
                Statement::VariableDeclaration(self.parse_variable_declaration()?),
            )),
            TokenKind::Async => {
                // export async function
                self.advance(); // consume 'async'
                self.require_token(&TokenKind::Function)?;
                let mut func = self.parse_function_declaration_inner()?;
                func.async_ = true;
                Some(Box::new(Statement::FunctionDeclaration(Box::new(func))))
            }
            TokenKind::Function => Some(Box::new(Statement::FunctionDeclaration(Box::new(
                self.parse_function_declaration(false)?,
            )))),
            TokenKind::Class => {
                let mut class_decl = self.parse_class_declaration()?;
                class_decl.decorators = decorators;
                Some(Box::new(Statement::ClassDeclaration(Box::new(class_decl))))
            }
            TokenKind::Interface => Some(Box::new(Statement::InterfaceDeclaration(Box::new(
                self.parse_interface()?,
            )))),
            TokenKind::Type => Some(Box::new(Statement::TypeAlias(Box::new(
                self.parse_type_alias()?,
            )))),
            TokenKind::Enum => Some(Box::new(Statement::EnumDeclaration(Box::new(
                self.parse_enum()?,
            )))),
            TokenKind::Namespace | TokenKind::Module => Some(Box::new(
                Statement::NamespaceDeclaration(Box::new(self.parse_namespace()?)),
            )),
            _ => return Err(self.unexpected_token("export declaration")),
        };

        let span = self.span_from(start);
        Ok(ExportDeclaration {
            declaration,
            specifiers: vec![],
            source: None,
            namespace_export: None,
            default: false,
            type_only,
            span,
        })
    }

    // ============ EXPRESSIONS ============

    fn parse_expression(&mut self) -> Result<Expression, JsError> {
        self.parse_sequence_expression()
    }

    fn parse_sequence_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        let mut expr = self.parse_assignment_expression()?;

        if self.check(&TokenKind::Comma) {
            let mut expressions = vec![expr];
            while self.match_token(&TokenKind::Comma) {
                expressions.push(self.parse_assignment_expression()?);
            }
            let span = self.span_from(start);
            expr = Expression::Sequence(SequenceExpression { expressions, span });
        }

        Ok(expr)
    }

    fn parse_assignment_expression(&mut self) -> Result<Expression, JsError> {
        // Check for yield expression
        if self.check(&TokenKind::Yield) {
            return self.parse_yield_expression();
        }

        // Check for await expression
        if self.check(&TokenKind::Await) {
            return self.parse_await_expression();
        }

        let start = self.current.span;
        let expr = self.parse_conditional_expression()?;

        if let Some(op) = self.current_assignment_op() {
            self.advance();
            let right = Rc::new(self.parse_assignment_expression()?);
            let left = self.expression_to_assignment_target(&expr)?;
            let span = self.span_from(start);
            return Ok(Expression::Assignment(Box::new(AssignmentExpression {
                operator: op,
                left,
                right,
                span,
            })));
        }

        Ok(expr)
    }

    fn parse_yield_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Yield)?;

        // Check for yield* (delegation)
        let delegate = self.match_token(&TokenKind::Star);

        // Check if there's an argument
        // yield without argument is valid, but we need to check if the next token
        // could be the start of an expression (not a statement terminator)
        let argument = if !self.check(&TokenKind::Semicolon)
            && !self.check(&TokenKind::RBrace)
            && !self.check(&TokenKind::RParen)
            && !self.check(&TokenKind::RBracket)
            && !self.check(&TokenKind::Comma)
            && !self.check(&TokenKind::Colon)
            && !self.is_at_end()
            && !self.lexer.had_newline_before()
        {
            Some(Rc::new(self.parse_assignment_expression()?))
        } else {
            None
        };

        let span = self.span_from(start);
        Ok(Expression::Yield(YieldExpression {
            argument,
            delegate,
            span,
        }))
    }

    fn parse_await_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Await)?;

        // await always requires an argument
        let argument = Rc::new(self.parse_unary_expression()?);

        let span = self.span_from(start);
        Ok(Expression::Await(AwaitExpression { argument, span }))
    }

    fn parse_conditional_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        let test = self.parse_binary_expression(0)?;

        if self.match_token(&TokenKind::Question) {
            let consequent = Rc::new(self.parse_assignment_expression()?);
            self.require_token(&TokenKind::Colon)?;
            let alternate = Rc::new(self.parse_assignment_expression()?);
            let span = self.span_from(start);
            return Ok(Expression::Conditional(ConditionalExpression {
                test: Rc::new(test),
                consequent,
                alternate,
                span,
            }));
        }

        Ok(test)
    }

    /// Pratt parser for binary expressions
    fn parse_binary_expression(&mut self, min_prec: u8) -> Result<Expression, JsError> {
        let start = self.current.span;
        let mut left = self.parse_unary_expression()?;

        while let Some((op, prec, is_logical)) = self.current_binary_op() {
            if prec < min_prec {
                break;
            }

            // Save the operator token kind before advancing (needed for logical op detection)
            let op_token_kind = self.current.kind.clone();
            self.advance();

            // Right associativity for ** operator
            let next_prec = if op == BinaryOp::Exp { prec } else { prec + 1 };
            let right = self.parse_binary_expression(next_prec)?;

            let span = self.span_from(start);
            left = if is_logical {
                let logical_op = match op {
                    BinaryOp::BitAnd if op_token_kind == TokenKind::AmpAmp => LogicalOp::And,
                    BinaryOp::BitOr if op_token_kind == TokenKind::PipePipe => LogicalOp::Or,
                    _ => {
                        // Nullish coalescing
                        LogicalOp::NullishCoalescing
                    }
                };
                Expression::Logical(LogicalExpression {
                    operator: logical_op,
                    left: Rc::new(left),
                    right: Rc::new(right),
                    span,
                })
            } else {
                Expression::Binary(BinaryExpression {
                    operator: op,
                    left: Rc::new(left),
                    right: Rc::new(right),
                    span,
                })
            };
        }

        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;

        if let Some(op) = self.current_unary_op() {
            self.advance();
            let argument = Rc::new(self.parse_unary_expression()?);

            // In strict mode, delete on unqualified identifier is a SyntaxError
            if op == UnaryOp::Delete {
                if let Expression::Identifier(id) = argument.as_ref() {
                    return Err(JsError::syntax_error(
                        format!(
                            "Delete of an unqualified identifier '{}' in strict mode",
                            id.name
                        ),
                        id.span.line,
                        id.span.column,
                    ));
                }
            }

            let span = self.span_from(start);
            return Ok(Expression::Unary(UnaryExpression {
                operator: op,
                argument,
                prefix: true,
                span,
            }));
        }

        // Update expressions (prefix)
        if let Some(op) = self.current_update_op() {
            self.advance();
            let argument = Rc::new(self.parse_unary_expression()?);
            let span = self.span_from(start);
            return Ok(Expression::Update(UpdateExpression {
                operator: op,
                argument,
                prefix: true,
                span,
            }));
        }

        // TypeScript angle bracket type assertion: <Type>value
        // Must check for < and distinguish from comparison or generics
        if self.check(&TokenKind::Lt) {
            // Try to parse as type assertion <Type>expr
            if let Some(expr) = self.try_parse_angle_bracket_assertion()? {
                return Ok(expr);
            }
        }

        self.parse_postfix_expression()
    }

    /// Try to parse a TypeScript angle-bracket type assertion: <Type>expression
    /// Returns None if this is not a type assertion (e.g., comparison or JSX)
    fn try_parse_angle_bracket_assertion(&mut self) -> Result<Option<Expression>, JsError> {
        let start = self.current.span;

        // Save position for backtracking
        let saved_current = self.current.clone();
        let checkpoint = self.lexer.checkpoint();

        // Consume <
        self.advance();

        // Try to parse a type
        match self.parse_type_annotation() {
            Ok(type_ann) => {
                // Must be followed by >
                if self.match_token(&TokenKind::Gt) {
                    // Parse the expression being asserted
                    let expr = self.parse_unary_expression()?;
                    let span = self.span_from(start);
                    return Ok(Some(Expression::TypeAssertion(TypeAssertionExpression {
                        expression: Rc::new(expr),
                        type_annotation: Box::new(type_ann),
                        span,
                    })));
                }
                // Not a valid type assertion, restore position
                self.current = saved_current;
                self.lexer.restore(checkpoint);
                Ok(None)
            }
            Err(_) => {
                // Not a type, restore position
                self.current = saved_current;
                self.lexer.restore(checkpoint);
                Ok(None)
            }
        }
    }

    fn parse_postfix_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        let mut expr = self.parse_left_hand_side_expression()?;

        // Postfix update
        if !self.lexer.had_newline_before() {
            if let Some(op) = self.current_update_op() {
                self.advance();
                let span = self.span_from(start);
                expr = Expression::Update(UpdateExpression {
                    operator: op,
                    argument: Rc::new(expr),
                    prefix: false,
                    span,
                });
            }
        }

        Ok(expr)
    }

    fn parse_left_hand_side_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;

        let mut expr = if self.match_token(&TokenKind::New) {
            let callee = Rc::new(self.parse_member_expression()?);
            // Check for type arguments (<T>) or arguments (()
            let (arguments, type_arguments) =
                if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
                    self.parse_call_arguments()?
                } else {
                    (vec![], None)
                };
            let span = self.span_from(start);
            Expression::New(Box::new(NewExpression {
                callee,
                arguments,
                type_arguments,
                span,
            }))
        } else {
            self.parse_member_expression()?
        };

        // Call expressions and member access chain
        // Track if we've seen any optional chaining (?.) in this expression
        let mut in_optional_chain = false;
        let optional_chain_start = start;

        loop {
            // Check for call with either ( or < (type arguments)
            if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
                // Try to parse as call with type arguments
                // Use checkpoint in case < is actually a comparison
                if self.check(&TokenKind::Lt) {
                    if let Some(call_expr) =
                        self.try_parse_call_with_type_args(expr.clone(), start)?
                    {
                        expr = call_expr;
                        continue;
                    }
                    // Not a call with type args, fall through to break
                    break;
                }
                let (arguments, type_arguments) = self.parse_call_arguments()?;
                let span = self.span_from(start);
                expr = Expression::Call(Box::new(CallExpression {
                    callee: Rc::new(expr),
                    arguments,
                    type_arguments,
                    optional: false,
                    span,
                }));
            } else if self.match_token(&TokenKind::Dot) {
                // Check for private identifier (#name)
                if self.match_token(&TokenKind::Hash) {
                    let name = self.parse_private_identifier()?;
                    let span = self.span_from(start);
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property: MemberProperty::PrivateIdentifier(name),
                        computed: false,
                        optional: false,
                        span,
                    }));
                } else {
                    // After a dot, any identifier or keyword can be used as a property name
                    let property = self.parse_identifier_name()?;
                    let span = self.span_from(start);
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property: MemberProperty::Identifier(property),
                        computed: false,
                        optional: false,
                        span,
                    }));
                }
            } else if self.match_token(&TokenKind::LBracket) {
                // Inside [...] brackets, 'in' is allowed as binary operator
                let saved_no_in = self.no_in;
                self.no_in = false;
                let property = self.parse_expression();
                self.no_in = saved_no_in;
                let property = property?;
                self.require_token(&TokenKind::RBracket)?;
                let span = self.span_from(start);
                expr = Expression::Member(Box::new(MemberExpression {
                    object: Rc::new(expr),
                    property: MemberProperty::Expression(Rc::new(property)),
                    computed: true,
                    optional: false,
                    span,
                }));
            } else if let TokenKind::TemplateHead(s) = self.current.kind.clone() {
                // Tagged template literal with substitutions: tag`...${...}...`
                let template_start = self.current.span;
                self.advance(); // consume TemplateHead
                let template = self.parse_template_literal(s, template_start)?;
                if let Expression::Template(quasi) = template {
                    let span = self.span_from(start);
                    expr = Expression::TaggedTemplate(Box::new(TaggedTemplateExpression {
                        tag: Rc::new(expr),
                        quasi: *quasi,
                        span,
                    }));
                }
            } else if let TokenKind::TemplateNoSub(s) = self.current.kind.clone() {
                // Tagged template literal without substitutions: tag`...`
                let template_start = self.current.span;
                self.advance(); // consume TemplateNoSub
                let span = self.span_from(start);
                expr = Expression::TaggedTemplate(Box::new(TaggedTemplateExpression {
                    tag: Rc::new(expr),
                    quasi: TemplateLiteral {
                        quasis: vec![TemplateElement {
                            value: s,
                            tail: true,
                            span: template_start,
                        }],
                        expressions: vec![],
                        span: template_start,
                    },
                    span,
                }));
            } else if self.match_token(&TokenKind::QuestionDot) {
                // Optional chaining - mark that we're in an optional chain
                in_optional_chain = true;

                if self.check(&TokenKind::LParen) {
                    let (arguments, type_arguments) = self.parse_call_arguments()?;
                    let span = self.span_from(start);
                    expr = Expression::Call(Box::new(CallExpression {
                        callee: Rc::new(expr),
                        arguments,
                        type_arguments,
                        optional: true,
                        span,
                    }));
                } else if self.match_token(&TokenKind::LBracket) {
                    // Inside [...] brackets, 'in' is allowed as binary operator
                    let saved_no_in = self.no_in;
                    self.no_in = false;
                    let property = self.parse_expression();
                    self.no_in = saved_no_in;
                    let property = property?;
                    self.require_token(&TokenKind::RBracket)?;
                    let span = self.span_from(start);
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property: MemberProperty::Expression(Rc::new(property)),
                        computed: true,
                        optional: true,
                        span,
                    }));
                } else {
                    // After ?. any identifier or keyword can be used as a property name
                    let property = self.parse_identifier_name()?;
                    let span = self.span_from(start);
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property: MemberProperty::Identifier(property),
                        computed: false,
                        optional: true,
                        span,
                    }));
                }
            } else if self.check(&TokenKind::Bang) && !self.lexer.had_newline_before() {
                // TypeScript non-null assertion (!)
                // The lexer tokenizes != and !== as single tokens (BangEq, BangEqEq)
                // so if we see Bang, it's a standalone !
                self.advance(); // consume !
                let span = self.span_from(start);
                expr = Expression::NonNull(NonNullExpression {
                    expression: Rc::new(expr),
                    span,
                });
            } else {
                break;
            }
        }

        // If we saw any optional chaining, wrap the expression in OptionalChainExpression
        // This allows the interpreter to short-circuit the entire chain if the base is nullish
        if in_optional_chain {
            let span = self.span_from(optional_chain_start);
            expr = Expression::OptionalChain(OptionalChainExpression {
                base: Rc::new(expr),
                span,
            });
        }

        // TypeScript type assertion (as)
        if self.match_token(&TokenKind::As) {
            // Handle "as const" - const assertion (TypeScript 3.4+)
            // This is a compile-time feature; at runtime we just return the value unchanged
            if self.match_token(&TokenKind::Const) {
                // "as const" is a no-op at runtime - the value stays the same
                // Just continue without wrapping in TypeAssertion
            } else {
                let type_annotation = Box::new(self.parse_type_annotation()?);
                let span = self.span_from(start);
                expr = Expression::TypeAssertion(TypeAssertionExpression {
                    expression: Rc::new(expr),
                    type_annotation,
                    span,
                });
            }
        }

        Ok(expr)
    }

    fn parse_member_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        let mut expr = self.parse_primary_expression()?;

        // Handle member access chain (.prop, [expr])
        loop {
            if self.match_token(&TokenKind::Dot) {
                if self.match_token(&TokenKind::Hash) {
                    let name = self.parse_private_identifier()?;
                    let span = self.span_from(start);
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property: MemberProperty::PrivateIdentifier(name),
                        computed: false,
                        optional: false,
                        span,
                    }));
                } else {
                    // After a dot, any identifier or keyword can be used as a property name
                    let property = self.parse_identifier_name()?;
                    let span = self.span_from(start);
                    expr = Expression::Member(Box::new(MemberExpression {
                        object: Rc::new(expr),
                        property: MemberProperty::Identifier(property),
                        computed: false,
                        optional: false,
                        span,
                    }));
                }
            } else if self.match_token(&TokenKind::LBracket) {
                // Inside [...] brackets, 'in' is allowed as binary operator
                let saved_no_in = self.no_in;
                self.no_in = false;
                let property = self.parse_expression();
                self.no_in = saved_no_in;
                let property = property?;
                self.require_token(&TokenKind::RBracket)?;
                let span = self.span_from(start);
                expr = Expression::Member(Box::new(MemberExpression {
                    object: Rc::new(expr),
                    property: MemberProperty::Expression(Rc::new(property)),
                    computed: true,
                    optional: false,
                    span,
                }));
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;

        match &self.current.kind {
            // Literals
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::Number(n),
                    span: self.span_from(start),
                })))
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::String(s),
                    span: self.span_from(start),
                })))
            }
            TokenKind::BigInt(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::BigInt(s),
                    span: self.span_from(start),
                })))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::Boolean(true),
                    span: self.span_from(start),
                })))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::Boolean(false),
                    span: self.span_from(start),
                })))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expression::Literal(Box::new(Literal {
                    value: LiteralValue::Null,
                    span: self.span_from(start),
                })))
            }
            TokenKind::Identifier(_)
            // Contextual keywords can also be used as identifiers in expressions
            | TokenKind::Type
            | TokenKind::From
            | TokenKind::As
            | TokenKind::Of
            | TokenKind::Namespace
            | TokenKind::Module
            | TokenKind::Any
            | TokenKind::Unknown
            | TokenKind::Never
            | TokenKind::Keyof
            | TokenKind::Infer
            | TokenKind::Is
            | TokenKind::Asserts
            | TokenKind::Readonly => {
                // Could be identifier or arrow function
                let id = self.parse_identifier()?;

                // Check for arrow function: id =>
                if self.check(&TokenKind::Arrow) {
                    return self.parse_arrow_function_from_params(
                        vec![FunctionParam {
                            pattern: Pattern::Identifier(id),
                            type_annotation: None,
                            optional: false,
                            decorators: vec![],
                            accessibility: None,
                            readonly: false,
                            span: self.span_from(start),
                        }],
                        start,
                    );
                }

                Ok(Expression::Identifier(id))
            }
            TokenKind::This => {
                self.advance();
                Ok(Expression::This(self.span_from(start)))
            }
            TokenKind::Super => {
                self.advance();
                Ok(Expression::Super(self.span_from(start)))
            }

            // Array literal
            TokenKind::LBracket => self.parse_array_literal(),

            // Object literal
            TokenKind::LBrace => self.parse_object_literal(),

            // Parenthesized expression or arrow function
            TokenKind::LParen => self.parse_parenthesized_or_arrow(),

            // Async function or async arrow
            TokenKind::Async => self.parse_async_expression(),

            // Dynamic import() - treat as identifier "import" so it becomes a function call
            // This allows `import("./module")` to parse as a call expression
            TokenKind::Import => {
                let span = self.current.span;
                let name = self.intern("import");
                self.advance();
                Ok(Expression::Identifier(Identifier { name, span }))
            }

            // Function expression
            TokenKind::Function => self.parse_function_expression(false),

            // Decorated class expression: @decorator class {}
            TokenKind::At => {
                let decorators = self.parse_decorators()?;
                if self.check(&TokenKind::Class) {
                    let decl = self.parse_class_declaration()?;
                    Ok(Expression::Class(Box::new(ClassExpression {
                        id: decl.id,
                        type_parameters: decl.type_parameters,
                        super_class: decl.super_class,
                        implements: decl.implements,
                        body: decl.body,
                        decorators,
                        span: decl.span,
                    })))
                } else {
                    Err(JsError::syntax_error(
                        "Decorators can only be applied to class expressions in expression position"
                            .to_string(),
                        self.current.span.line,
                        self.current.span.column,
                    ))
                }
            }

            // Class expression
            TokenKind::Class => self.parse_class_expression(),

            // Template literal
            TokenKind::TemplateNoSub(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Template(Box::new(TemplateLiteral {
                    quasis: vec![TemplateElement {
                        value: s,
                        tail: true,
                        span: self.span_from(start),
                    }],
                    expressions: vec![],
                    span: self.span_from(start),
                })))
            }
            TokenKind::TemplateHead(s) => {
                let s = s.clone();
                self.advance();
                self.parse_template_literal(s, start)
            }

            // RegExp literal - when we see `/` where an expression is expected
            TokenKind::Slash | TokenKind::SlashEq => {
                // The lexer scanned this as Slash or SlashEq, but we need a regexp.
                // Rescan from the current token's position as a regexp.
                let token = self.lexer.rescan_as_regexp(self.current.span);
                if let TokenKind::RegExp(pattern, flags) = token.kind {
                    self.current = self.lexer.next_token();
                    Ok(Expression::Literal(Box::new(Literal {
                        value: LiteralValue::RegExp { pattern, flags },
                        span: self.span_from(start),
                    })))
                } else {
                    Err(self.unexpected_token("regexp literal"))
                }
            }

            _ => Err(self.unexpected_token("expression")),
        }
    }

    fn parse_array_literal(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::LBracket)?;

        // Inside [...] brackets, 'in' is allowed as binary operator even in for-loop context
        let saved_no_in = self.no_in;
        self.no_in = false;

        let mut elements = vec![];

        while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
            if self.match_token(&TokenKind::Comma) {
                elements.push(None);
                continue;
            }

            if self.match_token(&TokenKind::DotDotDot) {
                let arg_start = self.current.span;
                let argument = Rc::new(self.parse_assignment_expression()?);
                let span = self.span_from(arg_start);
                elements.push(Some(ArrayElement::Spread(SpreadElement { argument, span })));
            } else {
                let expr = self.parse_assignment_expression()?;
                elements.push(Some(ArrayElement::Expression(expr)));
            }

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.no_in = saved_no_in;
        self.require_token(&TokenKind::RBracket)?;

        let span = self.span_from(start);
        Ok(Expression::Array(ArrayExpression { elements, span }))
    }

    fn parse_object_literal(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::LBrace)?;

        let mut properties = vec![];

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.match_token(&TokenKind::DotDotDot) {
                let arg_start = self.current.span;
                let argument = Rc::new(self.parse_assignment_expression()?);
                let span = self.span_from(arg_start);
                properties.push(ObjectProperty::Spread(SpreadElement { argument, span }));
            } else {
                properties.push(ObjectProperty::Property(Box::new(self.parse_property()?)));
            }

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RBrace)?;

        let span = self.span_from(start);
        Ok(Expression::Object(ObjectExpression { properties, span }))
    }

    fn parse_property(&mut self) -> Result<Property, JsError> {
        let start = self.current.span;

        // Check for generator method (*)
        let is_generator = self.match_token(&TokenKind::Star);

        // Check for async method
        let is_async = self.check(&TokenKind::Async) && self.peek_is_property_name();
        if is_async {
            self.advance(); // consume 'async'
                            // async* is also allowed (async generator)
                            // If we already parsed *, we have *async which is invalid - but that's handled above
                            // Check if this is async *gen() { }
        }

        // Check for async generator (async *)
        let is_async_generator = is_async && self.match_token(&TokenKind::Star);
        let is_generator = is_generator || is_async_generator;

        // Check for getter/setter (not valid with generator)
        let kind = if !is_generator && self.check_keyword("get") && self.peek_is_property_name() {
            self.advance();
            PropertyKind::Get
        } else if !is_generator && self.check_keyword("set") && self.peek_is_property_name() {
            self.advance();
            PropertyKind::Set
        } else {
            PropertyKind::Init
        };

        let computed = self.check(&TokenKind::LBracket);
        let key = if computed {
            self.advance();
            // Inside [...] brackets, 'in' is allowed as binary operator even in for-loop context
            let saved_no_in = self.no_in;
            self.no_in = false;
            let expr = self.parse_assignment_expression();
            self.no_in = saved_no_in;
            let expr = expr?;
            self.require_token(&TokenKind::RBracket)?;
            ObjectPropertyKey::Computed(Rc::new(expr))
        } else {
            self.parse_property_name()?
        };

        // Method shorthand (or generator method which must be a method)
        if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) || is_generator {
            let type_params = self.parse_optional_type_parameters()?;
            let params: Rc<[_]> = self.parse_function_params()?.into();
            let return_type = self.parse_optional_return_type()?;
            let body = Rc::new(self.parse_block_statement()?);

            let func_span = self.span_from(start);
            let value = Expression::Function(Box::new(FunctionExpression {
                id: None,
                params,
                return_type,
                type_parameters: type_params,
                body,
                generator: is_generator,
                async_: is_async,
                span: func_span,
            }));

            let span = self.span_from(start);
            return Ok(Property {
                key,
                value,
                kind,
                computed,
                shorthand: false,
                method: true,
                span,
            });
        }

        // Regular property
        let (value, shorthand) = if self.match_token(&TokenKind::Colon) {
            (self.parse_assignment_expression()?, false)
        } else {
            // Shorthand: { a } is { a: a }
            match &key {
                ObjectPropertyKey::Identifier(id) => (Expression::Identifier(id.clone()), true),
                _ => return Err(self.error("Shorthand property must be an identifier")),
            }
        };

        let span = self.span_from(start);
        Ok(Property {
            key,
            value,
            kind: PropertyKind::Init,
            computed,
            shorthand,
            method: false,
            span,
        })
    }

    fn parse_parenthesized_or_arrow(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;

        // Save state for potential rollback
        let lexer_checkpoint = self.lexer.checkpoint();
        let saved_current = self.current.clone();
        let saved_previous = self.previous.clone();

        self.require_token(&TokenKind::LParen)?;

        // Empty parens -> arrow function
        if self.match_token(&TokenKind::RParen) {
            return self.parse_arrow_function_from_params(vec![], start);
        }

        // Try to parse as arrow function params (with type annotations)
        if let Ok(params) = self.try_parse_arrow_params() {
            // Arrow immediately after ) -> definitely arrow function
            if self.check(&TokenKind::Arrow) {
                return self.parse_arrow_function_from_params(params, start);
            }

            // If we have type annotations in params, it must be an arrow function
            let has_type_annotations = params.iter().any(|p| p.type_annotation.is_some());
            if has_type_annotations {
                // Try to parse return type annotation
                if self.check(&TokenKind::Colon) {
                    // Save position to rollback if this isn't an arrow function
                    let type_checkpoint = self.lexer.checkpoint();
                    let type_saved_current = self.current.clone();

                    self.advance(); // consume ':'
                    if let Ok(_type_ann) = self.parse_type_annotation() {
                        if self.check(&TokenKind::Arrow) {
                            // Restore and let parse_arrow_function_from_params handle it
                            self.lexer.restore(type_checkpoint);
                            self.current = type_saved_current;
                            return self.parse_arrow_function_from_params(params, start);
                        }
                    }
                    // Not an arrow function, rollback
                    self.lexer.restore(type_checkpoint);
                    self.current = type_saved_current;
                }
                return Err(self.unexpected_token("'=>'"));
            }

            // Colon after ) could be return type or ternary operator
            // Only treat as arrow function if we see => after the type
            if self.check(&TokenKind::Colon) {
                let type_checkpoint = self.lexer.checkpoint();
                let type_saved_current = self.current.clone();

                self.advance(); // consume ':'
                if let Ok(_type_ann) = self.parse_type_annotation() {
                    if self.check(&TokenKind::Arrow) {
                        // Restore and let parse_arrow_function_from_params handle it
                        self.lexer.restore(type_checkpoint);
                        self.current = type_saved_current;
                        return self.parse_arrow_function_from_params(params, start);
                    }
                }
                // Not an arrow function (could be ternary), rollback
                self.lexer.restore(type_checkpoint);
                self.current = type_saved_current;
            }

            // No arrow - might be parenthesized expression, rollback and re-parse
            self.lexer.restore(lexer_checkpoint);
            self.current = saved_current;
            self.previous = saved_previous;
        } else {
            // Failed to parse as params, rollback
            self.lexer.restore(lexer_checkpoint);
            self.current = saved_current;
            self.previous = saved_previous;
        }

        // Parse as parenthesized expression
        self.require_token(&TokenKind::LParen)?;

        // Inside (...) parentheses, 'in' is allowed as binary operator even in for-loop context
        let saved_no_in = self.no_in;
        self.no_in = false;
        let first = self.parse_assignment_expression();
        self.no_in = saved_no_in;
        let first = first?;

        if self.match_token(&TokenKind::RParen) {
            // Check for arrow (simple identifier case)
            if self.check(&TokenKind::Arrow) {
                let param = self.expression_to_param(&first)?;
                return self.parse_arrow_function_from_params(vec![param], start);
            }

            // Parenthesized expression
            let span = self.span_from(start);
            return Ok(Expression::Parenthesized(Rc::new(first), span));
        }

        // Comma - either sequence or arrow params
        if self.match_token(&TokenKind::Comma) {
            let mut items = vec![first];

            // Inside (...) parentheses, 'in' is allowed as binary operator
            let saved_no_in = self.no_in;
            self.no_in = false;

            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                if self.match_token(&TokenKind::DotDotDot) {
                    // Rest parameter - definitely arrow function
                    let rest_start = self.current.span;
                    let pattern = self.parse_binding_pattern()?;
                    let type_ann = if self.match_token(&TokenKind::Colon) {
                        Some(Box::new(self.parse_type_annotation()?))
                    } else {
                        None
                    };
                    self.no_in = saved_no_in;
                    self.require_token(&TokenKind::RParen)?;

                    let mut params: Vec<FunctionParam> = items
                        .into_iter()
                        .map(|e| self.expression_to_param(&e))
                        .collect::<Result<_, _>>()?;

                    let rest_span = self.span_from(rest_start);
                    params.push(FunctionParam {
                        pattern: Pattern::Rest(RestElement {
                            argument: Box::new(pattern),
                            type_annotation: type_ann,
                            span: rest_span,
                        }),
                        type_annotation: None,
                        optional: false,
                        decorators: vec![],
                        accessibility: None,
                        readonly: false,
                        span: rest_span,
                    });

                    return self.parse_arrow_function_from_params(params, start);
                }

                items.push(self.parse_assignment_expression()?);

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }

            self.no_in = saved_no_in;

            self.require_token(&TokenKind::RParen)?;

            // Check for arrow
            if self.check(&TokenKind::Arrow) {
                let params: Vec<FunctionParam> = items
                    .into_iter()
                    .map(|e| self.expression_to_param(&e))
                    .collect::<Result<_, _>>()?;
                return self.parse_arrow_function_from_params(params, start);
            }

            // Sequence expression in parentheses
            let span = self.span_from(start);
            let seq = Expression::Sequence(SequenceExpression {
                expressions: items,
                span,
            });
            return Ok(Expression::Parenthesized(Rc::new(seq), span));
        }

        Err(self.unexpected_token("')' or ','"))
    }

    /// Try to parse arrow function parameters (with optional type annotations)
    /// Returns Ok with params if successful, Err otherwise
    fn try_parse_arrow_params(&mut self) -> Result<Vec<FunctionParam>, JsError> {
        let mut params = vec![];

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            let param_start = self.current.span;

            // Parse parameter decorators (e.g., @inject param)
            let decorators = self.parse_decorators()?;

            // Check for rest parameter
            let pattern = if self.match_token(&TokenKind::DotDotDot) {
                let arg = self.parse_binding_pattern()?;
                let span = self.span_from(param_start);
                Pattern::Rest(RestElement {
                    argument: Box::new(arg),
                    type_annotation: None,
                    span,
                })
            } else {
                self.parse_binding_pattern()?
            };

            let optional = self.match_token(&TokenKind::Question);

            let type_annotation = if self.match_token(&TokenKind::Colon) {
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            // Default value becomes AssignmentPattern
            let pattern = if self.match_token(&TokenKind::Eq) {
                let right = Rc::new(self.parse_assignment_expression()?);
                let span = self.span_from(param_start);
                Pattern::Assignment(AssignmentPattern {
                    left: Box::new(pattern),
                    right,
                    span,
                })
            } else {
                pattern
            };

            let span = self.span_from(param_start);
            params.push(FunctionParam {
                pattern,
                type_annotation,
                optional,
                decorators,
                accessibility: None,
                readonly: false,
                span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RParen)?;
        Ok(params)
    }

    fn parse_arrow_function_from_params(
        &mut self,
        params: Vec<FunctionParam>,
        start: Span,
    ) -> Result<Expression, JsError> {
        self.parse_arrow_function_from_params_async(params, start, false)
    }

    fn parse_arrow_function_from_params_async(
        &mut self,
        params: Vec<FunctionParam>,
        start: Span,
        is_async: bool,
    ) -> Result<Expression, JsError> {
        // Check for duplicate parameter names in strict mode
        let mut seen_names: std::collections::HashSet<JsString> = std::collections::HashSet::new();
        for param in &params {
            Self::check_duplicate_params(&param.pattern, &mut seen_names)?;
        }

        let return_type = self.parse_optional_return_type()?;
        self.require_token(&TokenKind::Arrow)?;

        let body = if self.check(&TokenKind::LBrace) {
            ArrowFunctionBody::Block(Rc::new(self.parse_block_statement()?))
        } else {
            ArrowFunctionBody::Expression(Rc::new(self.parse_assignment_expression()?))
        };

        let span = self.span_from(start);
        Ok(Expression::ArrowFunction(Box::new(
            ArrowFunctionExpression {
                params: params.into(),
                return_type,
                type_parameters: None,
                body: Box::new(body),
                async_: is_async,
                span,
            },
        )))
    }

    fn parse_async_expression(&mut self) -> Result<Expression, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Async)?;

        // async function - async function expression
        if self.check(&TokenKind::Function) {
            return self.parse_function_expression(true);
        }

        // async () => or async (params) =>
        if self.check(&TokenKind::LParen) {
            self.advance(); // consume '('
                            // Parse params and then arrow
            let mut params = vec![];
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                if self.match_token(&TokenKind::DotDotDot) {
                    // Rest parameter
                    let rest_start = self.current.span;
                    let arg = self.parse_binding_pattern()?;
                    let rest_span = self.span_from(rest_start);
                    let rest_elem = RestElement {
                        argument: Box::new(arg),
                        type_annotation: None,
                        span: rest_span,
                    };
                    params.push(FunctionParam {
                        pattern: Pattern::Rest(rest_elem),
                        type_annotation: None,
                        optional: false,
                        decorators: vec![],
                        accessibility: None,
                        readonly: false,
                        span: rest_span,
                    });
                    break;
                }
                let param = self.parse_function_param()?;
                params.push(param);
                if !self.check(&TokenKind::RParen) {
                    self.require_token(&TokenKind::Comma)?;
                }
            }
            self.require_token(&TokenKind::RParen)?;
            return self.parse_arrow_function_from_params_async(params, start, true);
        }

        // async id => (single param)
        if self.check_identifier() {
            let id = self.parse_identifier()?;
            let param_span = id.span;
            let params = vec![FunctionParam {
                pattern: Pattern::Identifier(id),
                type_annotation: None,
                optional: false,
                decorators: vec![],
                accessibility: None,
                readonly: false,
                span: param_span,
            }];
            return self.parse_arrow_function_from_params_async(params, start, true);
        }

        Err(self.unexpected_token("function, '(' or identifier after 'async'"))
    }

    fn parse_function_param(&mut self) -> Result<FunctionParam, JsError> {
        let start = self.current.span;
        // Parse parameter decorators (e.g., @inject param)
        let decorators = self.parse_decorators()?;
        let pattern = self.parse_binding_pattern()?;
        let optional = self.match_token(&TokenKind::Question);
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };
        // Handle default value
        if self.match_token(&TokenKind::Eq) {
            // For now, skip the default value expression by parsing and ignoring it
            let _default = self.parse_assignment_expression()?;
        }
        let span = self.span_from(start);
        Ok(FunctionParam {
            pattern,
            type_annotation,
            optional,
            decorators,
            accessibility: None,
            readonly: false,
            span,
        })
    }

    fn parse_function_expression(&mut self, is_async: bool) -> Result<Expression, JsError> {
        let start = self.current.span;
        self.require_token(&TokenKind::Function)?;

        let generator = self.match_token(&TokenKind::Star);
        let id = if self.check_identifier() {
            Some(self.parse_identifier()?)
        } else {
            None
        };

        let type_parameters = self.parse_optional_type_parameters()?;
        let params: Rc<[_]> = self.parse_function_params()?.into();
        let return_type = self.parse_optional_return_type()?;
        let body = Rc::new(self.parse_block_statement()?);

        let span = self.span_from(start);
        Ok(Expression::Function(Box::new(FunctionExpression {
            id,
            params,
            return_type,
            type_parameters,
            body,
            generator,
            async_: is_async,
            span,
        })))
    }

    fn parse_class_expression(&mut self) -> Result<Expression, JsError> {
        let decl = self.parse_class_declaration()?;
        Ok(Expression::Class(Box::new(ClassExpression {
            id: decl.id,
            type_parameters: decl.type_parameters,
            super_class: decl.super_class,
            implements: decl.implements,
            body: decl.body,
            decorators: decl.decorators,
            span: decl.span,
        })))
    }

    fn parse_template_literal(
        &mut self,
        first: JsString,
        start: Span,
    ) -> Result<Expression, JsError> {
        let mut quasis = vec![TemplateElement {
            value: first,
            tail: false,
            span: self.span_from(start),
        }];
        let mut expressions = vec![];

        loop {
            // Parse expression
            expressions.push(self.parse_expression()?);
            // Check for closing brace but don't advance - let scan_template_continuation handle it
            if !self.check(&TokenKind::RBrace) {
                return Err(JsError::syntax_error(
                    format!(
                        "Expected '}}' in template literal, found {:?}",
                        self.current.kind
                    ),
                    self.current.span.line,
                    self.current.span.column,
                ));
            }

            // Continue template - rescan from the RBrace position
            // This resets the lexer position and scans the template continuation including }
            let cont = self.lexer.rescan_template_continuation(self.current.span);
            match cont {
                TokenKind::TemplateTail(s) => {
                    quasis.push(TemplateElement {
                        value: s,
                        tail: true,
                        span: self.current.span,
                    });
                    break;
                }
                TokenKind::TemplateMiddle(s) => {
                    quasis.push(TemplateElement {
                        value: s,
                        tail: false,
                        span: self.current.span,
                    });
                    // After TemplateMiddle, we need to parse another expression
                    // The lexer is now positioned after ${, so get the next token
                    self.current = self.lexer.next_token();
                }
                _ => break,
            }
        }

        // After template literal parsing, advance to get the next token
        self.current = self.lexer.next_token();

        let span = self.span_from(start);
        Ok(Expression::Template(Box::new(TemplateLiteral {
            quasis,
            expressions,
            span,
        })))
    }

    /// Try to parse a call expression with type arguments: fn<T>(args)
    /// Returns None if the < is not actually type arguments (e.g., comparison)
    fn try_parse_call_with_type_args(
        &mut self,
        callee: Expression,
        start: Span,
    ) -> Result<Option<Expression>, JsError> {
        // Save state for backtracking
        let checkpoint = self.lexer.checkpoint();
        let saved_current = self.current.clone();
        let saved_previous = self.previous.clone();

        // Try to parse type arguments
        let type_args = match self.parse_type_arguments() {
            Ok(args) => args,
            Err(_) => {
                // Not valid type arguments, restore and return None
                self.lexer.restore(checkpoint);
                self.current = saved_current;
                self.previous = saved_previous;
                return Ok(None);
            }
        };

        // Must be followed by ( for a call
        if !self.check(&TokenKind::LParen) {
            // Not a call, restore and return None
            self.lexer.restore(checkpoint);
            self.current = saved_current;
            self.previous = saved_previous;
            return Ok(None);
        }

        // Parse the call arguments (without type args since we already parsed them)
        self.advance(); // consume (
        let mut arguments = vec![];

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            if self.match_token(&TokenKind::DotDotDot) {
                let arg_start = self.current.span;
                let argument = Rc::new(self.parse_assignment_expression()?);
                let span = self.span_from(arg_start);
                arguments.push(Argument::Spread(SpreadElement { argument, span }));
            } else {
                arguments.push(Argument::Expression(self.parse_assignment_expression()?));
            }

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RParen)?;

        let span = self.span_from(start);
        Ok(Some(Expression::Call(Box::new(CallExpression {
            callee: Rc::new(callee),
            arguments,
            type_arguments: Some(type_args),
            optional: false,
            span,
        }))))
    }

    /// Parse type arguments: <T, U, V>
    /// This is the non-optional version that returns an error if parsing fails
    fn parse_type_arguments(&mut self) -> Result<TypeArguments, JsError> {
        self.require_token(&TokenKind::Lt)?;
        let mut types = vec![self.parse_type_annotation()?];

        while self.match_token(&TokenKind::Comma) {
            types.push(self.parse_type_annotation()?);
        }

        self.require_token(&TokenKind::Gt)?;
        Ok(TypeArguments {
            params: types,
            span: Span::default(),
        })
    }

    fn parse_call_arguments(&mut self) -> Result<(Vec<Argument>, Option<TypeArguments>), JsError> {
        let type_args = self.parse_optional_type_arguments()?;
        self.require_token(&TokenKind::LParen)?;

        let mut arguments = vec![];

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            if self.match_token(&TokenKind::DotDotDot) {
                let arg_start = self.current.span;
                let argument = Rc::new(self.parse_assignment_expression()?);
                let span = self.span_from(arg_start);
                arguments.push(Argument::Spread(SpreadElement { argument, span }));
            } else {
                arguments.push(Argument::Expression(self.parse_assignment_expression()?));
            }

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RParen)?;
        Ok((arguments, type_args))
    }

    // ============ TYPE ANNOTATIONS ============

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, JsError> {
        self.parse_conditional_type()
    }

    /// Parse conditional type: T extends U ? X : Y
    fn parse_conditional_type(&mut self) -> Result<TypeAnnotation, JsError> {
        let start = self.current.span;
        let check_type = self.parse_union_type()?;

        // Check for extends (conditional type)
        if self.match_token(&TokenKind::Extends) {
            let extends_type = self.parse_union_type()?;
            self.require_token(&TokenKind::Question)?;
            let true_type = self.parse_type_annotation()?;
            self.require_token(&TokenKind::Colon)?;
            let false_type = self.parse_type_annotation()?;
            return Ok(TypeAnnotation::Conditional(ConditionalType {
                check_type: Box::new(check_type),
                extends_type: Box::new(extends_type),
                true_type: Box::new(true_type),
                false_type: Box::new(false_type),
                span: self.span_from(start),
            }));
        }

        Ok(check_type)
    }

    fn parse_union_type(&mut self) -> Result<TypeAnnotation, JsError> {
        let first = self.parse_intersection_type()?;

        if !self.check(&TokenKind::Pipe) {
            return Ok(first);
        }

        let mut types = vec![first];
        while self.match_token(&TokenKind::Pipe) {
            types.push(self.parse_intersection_type()?);
        }

        Ok(TypeAnnotation::Union(UnionType {
            types,
            span: Span::default(),
        }))
    }

    fn parse_intersection_type(&mut self) -> Result<TypeAnnotation, JsError> {
        let first = self.parse_primary_type()?;

        if !self.check(&TokenKind::Amp) {
            return Ok(first);
        }

        let mut types = vec![first];
        while self.match_token(&TokenKind::Amp) {
            types.push(self.parse_primary_type()?);
        }

        Ok(TypeAnnotation::Intersection(IntersectionType {
            types,
            span: Span::default(),
        }))
    }

    fn parse_primary_type(&mut self) -> Result<TypeAnnotation, JsError> {
        let start = self.current.span;

        match &self.current.kind {
            // keyof operator: keyof T
            TokenKind::Keyof => {
                self.advance();
                let operand = self.parse_primary_type()?;
                let mut ty = TypeAnnotation::Keyof(KeyofType {
                    type_annotation: Box::new(operand),
                    span: self.span_from(start),
                });
                // Array shorthand: keyof T[]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    if self.check(&TokenKind::RBracket) {
                        self.advance();
                        ty = TypeAnnotation::Array(ArrayType {
                            element_type: Box::new(ty),
                            span: self.span_from(start),
                        });
                    } else {
                        // Indexed access: keyof T[K]
                        let index_type = self.parse_type_annotation()?;
                        self.require_token(&TokenKind::RBracket)?;
                        ty = TypeAnnotation::Indexed(IndexedAccessType {
                            object_type: Box::new(ty),
                            index_type: Box::new(index_type),
                            span: self.span_from(start),
                        });
                    }
                }
                Ok(ty)
            }

            // infer keyword: infer R
            TokenKind::Infer => {
                self.advance();
                // Parse the type parameter name
                let name = self.parse_identifier()?;
                // Optional constraint: infer R extends SomeType
                let constraint = if self.match_token(&TokenKind::Extends) {
                    Some(Box::new(self.parse_primary_type()?))
                } else {
                    None
                };
                Ok(TypeAnnotation::Infer(InferType {
                    type_parameter: TypeParameter {
                        name,
                        constraint,
                        default: None,
                        span: self.span_from(start),
                    },
                    span: self.span_from(start),
                }))
            }

            // Type keywords
            TokenKind::Any => {
                self.advance();
                let mut ty = TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Any,
                    span: self.span_from(start),
                });
                // Array shorthand: any[]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    self.require_token(&TokenKind::RBracket)?;
                    ty = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(ty),
                        span: self.span_from(start),
                    });
                }
                Ok(ty)
            }
            TokenKind::Unknown => {
                self.advance();
                let mut ty = TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Unknown,
                    span: self.span_from(start),
                });
                // Array shorthand: unknown[]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    self.require_token(&TokenKind::RBracket)?;
                    ty = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(ty),
                        span: self.span_from(start),
                    });
                }
                Ok(ty)
            }
            TokenKind::Never => {
                self.advance();
                let mut ty = TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Never,
                    span: self.span_from(start),
                });
                // Array shorthand: never[]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    self.require_token(&TokenKind::RBracket)?;
                    ty = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(ty),
                        span: self.span_from(start),
                    });
                }
                Ok(ty)
            }
            TokenKind::Void => {
                self.advance();
                let mut ty = TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Void,
                    span: self.span_from(start),
                });
                // Array shorthand: void[]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    self.require_token(&TokenKind::RBracket)?;
                    ty = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(ty),
                        span: self.span_from(start),
                    });
                }
                Ok(ty)
            }
            TokenKind::Null => {
                self.advance();
                let mut ty = TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Null,
                    span: self.span_from(start),
                });
                // Array shorthand: null[]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    self.require_token(&TokenKind::RBracket)?;
                    ty = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(ty),
                        span: self.span_from(start),
                    });
                }
                Ok(ty)
            }

            // Identifier (type reference or built-in type name)
            TokenKind::Identifier(name) => {
                let keyword = match name.as_str() {
                    "string" => Some(TypeKeywordKind::String),
                    "number" => Some(TypeKeywordKind::Number),
                    "boolean" => Some(TypeKeywordKind::Boolean),
                    "symbol" => Some(TypeKeywordKind::Symbol),
                    "bigint" => Some(TypeKeywordKind::BigInt),
                    "object" => Some(TypeKeywordKind::Object),
                    "undefined" => Some(TypeKeywordKind::Undefined),
                    _ => None,
                };

                if let Some(kw) = keyword {
                    self.advance();
                    let mut ty = TypeAnnotation::Keyword(TypeKeyword {
                        keyword: kw,
                        span: self.span_from(start),
                    });

                    // Array shorthand: string[]
                    while self.check(&TokenKind::LBracket) {
                        self.advance();
                        self.require_token(&TokenKind::RBracket)?;
                        ty = TypeAnnotation::Array(ArrayType {
                            element_type: Box::new(ty),
                            span: self.span_from(start),
                        });
                    }

                    Ok(ty)
                } else {
                    let ty = self.parse_type_reference()?;
                    let mut ty = TypeAnnotation::Reference(ty);

                    // Array shorthand or indexed access type
                    while self.check(&TokenKind::LBracket) {
                        self.advance();
                        if self.check(&TokenKind::RBracket) {
                            // Array type: T[]
                            self.advance();
                            ty = TypeAnnotation::Array(ArrayType {
                                element_type: Box::new(ty),
                                span: self.span_from(start),
                            });
                        } else {
                            // Indexed access type: T["key"] or T[K]
                            let index_type = self.parse_type_annotation()?;
                            self.require_token(&TokenKind::RBracket)?;
                            ty = TypeAnnotation::Indexed(IndexedAccessType {
                                object_type: Box::new(ty),
                                index_type: Box::new(index_type),
                                span: self.span_from(start),
                            });
                        }
                    }

                    Ok(ty)
                }
            }

            // Object type or mapped type
            TokenKind::LBrace => {
                self.advance();

                // Check for mapped type: { [P in keyof T]: T[P] }
                // vs index signature: { [key: string]: T }
                // We need to detect: { [ident in ...]
                if self.check(&TokenKind::Readonly) || self.check(&TokenKind::LBracket) {
                    // Try to parse as mapped type
                    if let Some(mapped) = self.try_parse_mapped_type(start)? {
                        return Ok(mapped);
                    }
                }

                let members = self.parse_type_members()?;
                self.require_token(&TokenKind::RBrace)?;
                let mut ty = TypeAnnotation::Object(ObjectType {
                    members,
                    span: self.span_from(start),
                });

                // Array shorthand: { a: number }[]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    self.require_token(&TokenKind::RBracket)?;
                    ty = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(ty),
                        span: self.span_from(start),
                    });
                }

                Ok(ty)
            }

            // Tuple or parenthesized
            TokenKind::LBracket => {
                self.advance();
                let mut types = vec![];
                while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
                    types.push(self.parse_type_annotation()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.require_token(&TokenKind::RBracket)?;
                let mut ty = TypeAnnotation::Tuple(TupleType {
                    element_types: types,
                    span: self.span_from(start),
                });
                // Array shorthand: [string, number][]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    self.require_token(&TokenKind::RBracket)?;
                    ty = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(ty),
                        span: self.span_from(start),
                    });
                }
                Ok(ty)
            }

            // Constructor type: new (...args: any[]) => T
            TokenKind::New => {
                self.advance();
                // Parse as function type, the 'new' prefix just marks it as a constructor
                // For our runtime purposes, we just parse and discard the type annotation
                let func_type = self.try_parse_function_type()?;
                Ok(func_type)
            }

            // Parenthesized type or function type expression
            TokenKind::LParen => {
                // Try to parse as function type expression: (a: T, b: T) => R
                if let Ok(func_type) = self.try_parse_function_type() {
                    return Ok(func_type);
                }
                // Fall back to parenthesized type
                self.advance();
                let inner_ty = self.parse_type_annotation()?;
                self.require_token(&TokenKind::RParen)?;
                let mut ty = TypeAnnotation::Parenthesized(Box::new(inner_ty));

                // Array shorthand: (number | undefined)[]
                while self.check(&TokenKind::LBracket) {
                    self.advance();
                    self.require_token(&TokenKind::RBracket)?;
                    ty = TypeAnnotation::Array(ArrayType {
                        element_type: Box::new(ty),
                        span: self.span_from(start),
                    });
                }

                Ok(ty)
            }

            // Literal types
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(TypeAnnotation::Literal(TypeLiteral {
                    value: LiteralValue::String(s),
                    span: self.span_from(start),
                }))
            }
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(TypeAnnotation::Literal(TypeLiteral {
                    value: LiteralValue::Number(n),
                    span: self.span_from(start),
                }))
            }
            TokenKind::True => {
                self.advance();
                Ok(TypeAnnotation::Literal(TypeLiteral {
                    value: LiteralValue::Boolean(true),
                    span: self.span_from(start),
                }))
            }
            TokenKind::False => {
                self.advance();
                Ok(TypeAnnotation::Literal(TypeLiteral {
                    value: LiteralValue::Boolean(false),
                    span: self.span_from(start),
                }))
            }

            // typeof
            TokenKind::Typeof => {
                self.advance();
                let id = self.parse_identifier()?;
                Ok(TypeAnnotation::Typeof(TypeofType {
                    expression: id,
                    span: self.span_from(start),
                }))
            }

            // Template literal type: `Hello ${string}`
            TokenKind::TemplateHead(head) => {
                let head = head.clone();
                self.advance();
                let mut quasis = vec![head];
                let mut types = vec![];

                loop {
                    // Parse the type inside ${}
                    let ty = self.parse_type_annotation()?;
                    types.push(ty);

                    // After parsing type, we're at RBrace - rescan for template continuation
                    if !self.check(&TokenKind::RBrace) {
                        return Err(self.unexpected_token("closing brace in template literal type"));
                    }

                    // Rescan from the RBrace position to get template continuation
                    let cont = self.lexer.rescan_template_continuation(self.current.span);
                    match cont {
                        TokenKind::TemplateTail(tail) => {
                            quasis.push(tail);
                            // Advance past the tail token
                            self.current = self.lexer.next_token();
                            break;
                        }
                        TokenKind::TemplateMiddle(middle) => {
                            quasis.push(middle);
                            // Advance past the middle token to continue parsing
                            self.current = self.lexer.next_token();
                        }
                        _ => return Err(self.unexpected_token("template middle or tail")),
                    }
                }

                Ok(TypeAnnotation::TemplateLiteral(TemplateLiteralType {
                    quasis,
                    types,
                    span: self.span_from(start),
                }))
            }

            // Simple template string (no interpolation): `Hello`
            TokenKind::TemplateNoSub(s) => {
                let s = s.clone();
                self.advance();
                Ok(TypeAnnotation::TemplateLiteral(TemplateLiteralType {
                    quasis: vec![s],
                    types: vec![],
                    span: self.span_from(start),
                }))
            }

            _ => Err(self.unexpected_token("type")),
        }
    }

    /// Try to parse a function type expression: (a: T, b: T) => R
    /// Returns Err if this doesn't look like a function type.
    fn try_parse_function_type(&mut self) -> Result<TypeAnnotation, JsError> {
        let start = self.current.span;

        // Save state for potential rollback
        let lexer_checkpoint = self.lexer.checkpoint();
        let saved_current = self.current.clone();
        let saved_previous = self.previous.clone();

        // Parse parameters
        let params = match self.parse_function_type_params() {
            Ok(p) => p,
            Err(_) => {
                // Rollback
                self.lexer.restore(lexer_checkpoint);
                self.current = saved_current;
                self.previous = saved_previous;
                return Err(JsError::syntax_error_simple("Not a function type"));
            }
        };

        // Must have => after params for it to be a function type
        if !self.match_token(&TokenKind::Arrow) {
            // Rollback
            self.lexer.restore(lexer_checkpoint);
            self.current = saved_current;
            self.previous = saved_previous;
            return Err(JsError::syntax_error_simple("Not a function type"));
        }

        // Parse return type
        let return_type = Box::new(self.parse_type_annotation()?);

        Ok(TypeAnnotation::Function(FunctionType {
            params,
            return_type,
            type_parameters: None,
            span: self.span_from(start),
        }))
    }

    /// Parse function type parameters: (a: T, b?: T, ...rest: T[])
    fn parse_function_type_params(&mut self) -> Result<Vec<FunctionParam>, JsError> {
        self.require_token(&TokenKind::LParen)?;

        let mut params = vec![];

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            let param_start = self.current.span;

            // Check for rest parameter
            let is_rest = self.match_token(&TokenKind::DotDotDot);

            // Parameter name (must be identifier for function type)
            let name = match &self.current.kind {
                TokenKind::Identifier(n) => n.clone(),
                _ => return Err(self.unexpected_token("parameter name")),
            };
            self.advance();

            let pattern = if is_rest {
                Pattern::Rest(RestElement {
                    argument: Box::new(Pattern::Identifier(Identifier {
                        name: name.clone(),
                        span: self.span_from(param_start),
                    })),
                    type_annotation: None,
                    span: self.span_from(param_start),
                })
            } else {
                Pattern::Identifier(Identifier {
                    name: name.clone(),
                    span: self.span_from(param_start),
                })
            };

            let optional = self.match_token(&TokenKind::Question);

            // Type annotation (required for function type params to distinguish from parenthesized)
            let type_annotation = if self.match_token(&TokenKind::Colon) {
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let span = self.span_from(param_start);
            params.push(FunctionParam {
                pattern,
                type_annotation,
                optional,
                decorators: vec![],
                accessibility: None,
                readonly: false,
                span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::RParen)?;
        Ok(params)
    }

    /// Try to parse a mapped type: { [P in keyof T]: T[P] }
    /// Called after consuming { and seeing [ or readonly
    /// Returns None if this is not a mapped type (falls back to object type)
    fn try_parse_mapped_type(&mut self, start: Span) -> Result<Option<TypeAnnotation>, JsError> {
        // Save state for potential rollback
        let lexer_checkpoint = self.lexer.checkpoint();
        let saved_current = self.current.clone();
        let saved_previous = self.previous.clone();

        // Parse optional readonly modifier: +readonly, -readonly, or readonly
        let readonly = if self.match_token(&TokenKind::Plus) {
            if self.match_token(&TokenKind::Readonly) {
                Some(MappedTypeModifier::Add)
            } else {
                // Not a valid mapped type, rollback
                self.lexer.restore(lexer_checkpoint);
                self.current = saved_current;
                self.previous = saved_previous;
                return Ok(None);
            }
        } else if self.match_token(&TokenKind::Minus) {
            if self.match_token(&TokenKind::Readonly) {
                Some(MappedTypeModifier::Remove)
            } else {
                // Not a valid mapped type, rollback
                self.lexer.restore(lexer_checkpoint);
                self.current = saved_current;
                self.previous = saved_previous;
                return Ok(None);
            }
        } else if self.match_token(&TokenKind::Readonly) {
            Some(MappedTypeModifier::Add)
        } else {
            None
        };

        // Must have [
        if !self.match_token(&TokenKind::LBracket) {
            self.lexer.restore(lexer_checkpoint);
            self.current = saved_current;
            self.previous = saved_previous;
            return Ok(None);
        }

        // Type parameter name
        let param_name = match self.parse_identifier() {
            Ok(id) => id,
            Err(_) => {
                self.lexer.restore(lexer_checkpoint);
                self.current = saved_current;
                self.previous = saved_previous;
                return Ok(None);
            }
        };

        // Must have 'in' keyword
        if !self.match_token(&TokenKind::In) {
            self.lexer.restore(lexer_checkpoint);
            self.current = saved_current;
            self.previous = saved_previous;
            return Ok(None);
        }

        // This is definitely a mapped type now - no more rollback needed
        // Parse the constraint type (e.g., keyof T)
        let constraint = self.parse_type_annotation()?;

        // Optional 'as' clause for key remapping: [P in keyof T as NewKey]
        let name_type = if self.match_token(&TokenKind::As) {
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        self.require_token(&TokenKind::RBracket)?;

        // Optional modifier: +?, -?, or ?
        let optional = if self.match_token(&TokenKind::Plus) {
            if self.match_token(&TokenKind::Question) {
                Some(MappedTypeModifier::Add)
            } else {
                return Err(self.unexpected_token("?"));
            }
        } else if self.match_token(&TokenKind::Minus) {
            if self.match_token(&TokenKind::Question) {
                Some(MappedTypeModifier::Remove)
            } else {
                return Err(self.unexpected_token("?"));
            }
        } else if self.match_token(&TokenKind::Question) {
            Some(MappedTypeModifier::Add)
        } else {
            None
        };

        // : type_annotation
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(Box::new(self.parse_type_annotation()?))
        } else {
            None
        };

        // Optional semicolon
        let _ = self.match_token(&TokenKind::Semicolon);

        self.require_token(&TokenKind::RBrace)?;

        let type_parameter = TypeParameter {
            name: param_name,
            constraint: Some(Box::new(constraint)),
            default: None,
            span: self.span_from(start),
        };

        Ok(Some(TypeAnnotation::Mapped(MappedType {
            type_parameter,
            name_type,
            type_annotation,
            readonly,
            optional,
            span: self.span_from(start),
        })))
    }

    fn parse_type_reference(&mut self) -> Result<TypeReference, JsError> {
        let start = self.current.span;
        let name = self.parse_identifier()?;
        let type_arguments = self.parse_optional_type_arguments()?;
        let span = self.span_from(start);
        Ok(TypeReference {
            name,
            type_arguments,
            span,
        })
    }

    fn parse_type_members(&mut self) -> Result<Vec<TypeMember>, JsError> {
        let mut members = vec![];

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let start = self.current.span;
            let readonly = self.match_token(&TokenKind::Readonly);

            // Check for index signature: [key: type]: valueType
            if self.check(&TokenKind::LBracket) {
                self.advance(); // consume [
                let key = self.parse_identifier()?;
                self.require_token(&TokenKind::Colon)?;
                let key_type = Box::new(self.parse_type_annotation()?);
                self.require_token(&TokenKind::RBracket)?;
                self.require_token(&TokenKind::Colon)?;
                let value_type = Box::new(self.parse_type_annotation()?);

                let span = self.span_from(start);
                members.push(TypeMember::Index(IndexSignature {
                    key,
                    key_type,
                    value_type,
                    readonly,
                    span,
                }));
            } else {
                let key = self.parse_property_name()?;
                let optional = self.match_token(&TokenKind::Question);

                if self.check(&TokenKind::LParen) || self.check(&TokenKind::Lt) {
                    // Method signature
                    let type_parameters = self.parse_optional_type_parameters()?;
                    let params = self.parse_function_params()?;
                    let return_type = self.parse_optional_return_type()?;

                    let span = self.span_from(start);
                    members.push(TypeMember::Method(MethodSignature {
                        key,
                        params,
                        return_type,
                        type_parameters,
                        optional,
                        span,
                    }));
                } else {
                    // Property signature
                    let type_annotation = if self.match_token(&TokenKind::Colon) {
                        Some(Box::new(self.parse_type_annotation()?))
                    } else {
                        None
                    };

                    let span = self.span_from(start);
                    members.push(TypeMember::Property(PropertySignature {
                        key,
                        type_annotation,
                        optional,
                        readonly,
                        span,
                    }));
                }
            }

            // Optional semicolon or comma
            self.match_token(&TokenKind::Semicolon);
            self.match_token(&TokenKind::Comma);
        }

        Ok(members)
    }

    fn parse_optional_type_parameters(&mut self) -> Result<Option<TypeParameters>, JsError> {
        if !self.check(&TokenKind::Lt) {
            return Ok(None);
        }

        let start = self.current.span;
        self.advance();

        let mut params = vec![];

        while !self.check(&TokenKind::Gt) && !self.is_at_end() {
            let param_start = self.current.span;
            let name = self.parse_identifier()?;

            let constraint = if self.match_token(&TokenKind::Extends) {
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let default = if self.match_token(&TokenKind::Eq) {
                Some(Box::new(self.parse_type_annotation()?))
            } else {
                None
            };

            let span = self.span_from(param_start);
            params.push(TypeParameter {
                name,
                constraint,
                default,
                span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        self.require_token(&TokenKind::Gt)?;

        let span = self.span_from(start);
        Ok(Some(TypeParameters { params, span }))
    }

    fn parse_optional_type_arguments(&mut self) -> Result<Option<TypeArguments>, JsError> {
        if !self.check(&TokenKind::Lt) {
            return Ok(None);
        }

        let start = self.current.span;
        self.advance();

        let mut params = vec![];

        // Check for >, >>, or >>> (nested generics may produce >> or >>>)
        while !self.check(&TokenKind::Gt)
            && !self.check(&TokenKind::GtGt)
            && !self.check(&TokenKind::GtGtGt)
            && !self.is_at_end()
        {
            params.push(self.parse_type_annotation()?);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        // Use the special method that can split >> and >>> tokens
        self.consume_gt_in_type_context()?;

        let span = self.span_from(start);
        Ok(Some(TypeArguments { params, span }))
    }

    fn parse_optional_return_type(&mut self) -> Result<Option<Box<TypeAnnotation>>, JsError> {
        if self.match_token(&TokenKind::Colon) {
            // Check for type predicate: param is Type
            // This is an identifier followed by 'is' keyword
            if self.check_identifier() && self.peek_is(&TokenKind::Is) {
                let start = self.current.span;
                let param_name = self.parse_identifier()?;
                self.require_token(&TokenKind::Is)?;
                let type_annotation = Box::new(self.parse_type_annotation()?);
                Ok(Some(Box::new(TypeAnnotation::TypePredicate(
                    TypePredicateType {
                        parameter_name: param_name,
                        type_annotation,
                        span: self.span_from(start),
                    },
                ))))
            } else {
                Ok(Some(Box::new(self.parse_type_annotation()?)))
            }
        } else {
            Ok(None)
        }
    }

    // ============ HELPERS ============

    fn parse_identifier(&mut self) -> Result<Identifier, JsError> {
        match &self.current.kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                let span = self.current.span;
                self.advance();
                Ok(Identifier { name, span })
            }
            // Allow contextual keywords as identifiers
            TokenKind::Type
            | TokenKind::From
            | TokenKind::As
            | TokenKind::Of
            | TokenKind::Namespace
            | TokenKind::Module
            // TypeScript type keywords (valid as property names)
            | TokenKind::Any
            | TokenKind::Unknown
            | TokenKind::Never
            | TokenKind::Keyof
            | TokenKind::Infer
            | TokenKind::Is
            | TokenKind::Asserts
            | TokenKind::Readonly => {
                let name = self.keyword_to_js_string();
                let span = self.current.span;
                self.advance();
                Ok(Identifier { name, span })
            }
            _ => Err(self.unexpected_token("identifier")),
        }
    }

    /// Parse a ModuleExportName - used in import/export specifiers.
    /// This allows 'default' as a valid name (e.g., `export { default as foo }`)
    fn parse_module_export_name(&mut self) -> Result<Identifier, JsError> {
        // 'default' is allowed as a module export name
        if self.check(&TokenKind::Default) {
            let name = self.intern("default");
            let span = self.current.span;
            self.advance();
            return Ok(Identifier { name, span });
        }
        // Otherwise, parse as a regular identifier
        self.parse_identifier()
    }

    /// Parse a private identifier (after `#` token has been consumed).
    /// Returns an Identifier with name including the `#` prefix (e.g., "#foo").
    fn parse_private_identifier(&mut self) -> Result<Identifier, JsError> {
        match &self.current.kind {
            TokenKind::Identifier(name) => {
                // Construct "#name" and intern it
                let private_name = format!("#{}", name);
                let name = self.intern(&private_name);
                let span = self.current.span;
                self.advance();
                Ok(Identifier { name, span })
            }
            _ => Err(self.unexpected_token("identifier after #")),
        }
    }

    /// Parse identifier or keyword as a property name (after a dot).
    /// In JavaScript, reserved words can be used as property names.
    fn parse_identifier_name(&mut self) -> Result<Identifier, JsError> {
        // First try as normal identifier
        if let TokenKind::Identifier(name) = &self.current.kind {
            let name = name.clone();
            let span = self.current.span;
            self.advance();
            return Ok(Identifier { name, span });
        }
        // Check for contextual keywords (same as in parse_identifier)
        if matches!(
            self.current.kind,
            TokenKind::Type
                | TokenKind::From
                | TokenKind::As
                | TokenKind::Of
                | TokenKind::Namespace
                | TokenKind::Module
                | TokenKind::Any
                | TokenKind::Unknown
                | TokenKind::Never
                | TokenKind::Keyof
                | TokenKind::Infer
                | TokenKind::Is
                | TokenKind::Asserts
                | TokenKind::Readonly
        ) {
            let name = self.keyword_to_js_string();
            let span = self.current.span;
            self.advance();
            return Ok(Identifier { name, span });
        }
        // Otherwise try as keyword (reserved words are valid as property names)
        if self.is_keyword() || self.is_reserved_word() {
            let name = self.keyword_to_js_string();
            let span = self.current.span;
            self.advance();
            return Ok(Identifier { name, span });
        }
        Err(self.unexpected_token("identifier"))
    }

    /// Check if current token is a reserved word (stricter keywords that can only be used as property names)
    fn is_reserved_word(&self) -> bool {
        matches!(
            self.current.kind,
            TokenKind::Delete
                | TokenKind::Typeof
                | TokenKind::Void
                | TokenKind::Instanceof
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Yield
                | TokenKind::Await
                | TokenKind::Super
                | TokenKind::Async
                | TokenKind::Namespace
                | TokenKind::Private
                | TokenKind::Public
                | TokenKind::Protected
                | TokenKind::Declare
                | TokenKind::Implements
        )
    }

    fn parse_property_name(&mut self) -> Result<ObjectPropertyKey, JsError> {
        match &self.current.kind {
            TokenKind::Identifier(_) => {
                let id = self.parse_identifier()?;
                Ok(ObjectPropertyKey::Identifier(id))
            }
            TokenKind::String(s) => {
                let value = s.clone();
                let span = self.current.span;
                self.advance();
                Ok(ObjectPropertyKey::String(StringLiteral { value, span }))
            }
            TokenKind::Number(n) => {
                let n = *n;
                let span = self.current.span;
                self.advance();
                Ok(ObjectPropertyKey::Number(Literal {
                    value: LiteralValue::Number(n),
                    span,
                }))
            }
            // Handle keywords as property names
            _ if self.is_keyword() => {
                let name = self.keyword_to_js_string();
                let span = self.current.span;
                self.advance();
                Ok(ObjectPropertyKey::Identifier(Identifier { name, span }))
            }
            _ => Err(self.unexpected_token("property name")),
        }
    }

    fn parse_string_literal(&mut self) -> Result<StringLiteral, JsError> {
        match &self.current.kind {
            TokenKind::String(s) => {
                let value = s.clone();
                let span = self.current.span;
                self.advance();
                Ok(StringLiteral { value, span })
            }
            _ => Err(self.unexpected_token("string")),
        }
    }

    fn advance(&mut self) {
        self.previous = std::mem::replace(&mut self.current, self.lexer.next_token());
    }

    fn require_token(&mut self, kind: &TokenKind) -> Result<(), JsError> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.unexpected_token(&format!("{:?}", kind)))
        }
    }

    /// Consume a `>` token in type context, splitting `>>` and `>>>` if needed.
    /// This handles the ambiguity between right shift operators and nested generic closers.
    fn consume_gt_in_type_context(&mut self) -> Result<(), JsError> {
        match &self.current.kind {
            TokenKind::Gt => {
                self.advance();
                Ok(())
            }
            TokenKind::GtGt => {
                // Split >> into > and >
                // Update previous to be a synthetic > token
                self.previous = Token {
                    kind: TokenKind::Gt,
                    span: Span {
                        start: self.current.span.start,
                        end: self.current.span.start + 1,
                        line: self.current.span.line,
                        column: self.current.span.column,
                    },
                };
                // Update current to be the remaining >
                self.current = Token {
                    kind: TokenKind::Gt,
                    span: Span {
                        start: self.current.span.start + 1,
                        end: self.current.span.end,
                        line: self.current.span.line,
                        column: self.current.span.column + 1,
                    },
                };
                Ok(())
            }
            TokenKind::GtGtGt => {
                // Split >>> into > and >>
                self.previous = Token {
                    kind: TokenKind::Gt,
                    span: Span {
                        start: self.current.span.start,
                        end: self.current.span.start + 1,
                        line: self.current.span.line,
                        column: self.current.span.column,
                    },
                };
                // Update current to be the remaining >>
                self.current = Token {
                    kind: TokenKind::GtGt,
                    span: Span {
                        start: self.current.span.start + 1,
                        end: self.current.span.end,
                        line: self.current.span.line,
                        column: self.current.span.column + 1,
                    },
                };
                Ok(())
            }
            _ => Err(self.unexpected_token("Gt")),
        }
    }

    fn expect_semicolon(&mut self) -> Result<(), JsError> {
        if self.match_token(&TokenKind::Semicolon) {
            return Ok(());
        }

        // ASI: accept if at end, before }, or after newline
        if self.is_at_end() || self.check(&TokenKind::RBrace) || self.lexer.had_newline_before() {
            return Ok(());
        }

        Err(self.unexpected_token("';'"))
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current.kind) == std::mem::discriminant(kind)
    }

    /// Check if the next token (after current) is of the given kind
    fn peek_is(&mut self, kind: &TokenKind) -> bool {
        let checkpoint = self.lexer.checkpoint();
        let next = self.lexer.next_token();
        self.lexer.restore(checkpoint);
        std::mem::discriminant(&next.kind) == std::mem::discriminant(kind)
    }

    fn check_identifier(&self) -> bool {
        matches!(self.current.kind, TokenKind::Identifier(_))
    }

    fn check_keyword(&self, keyword: &str) -> bool {
        matches!(&self.current.kind, TokenKind::Identifier(s) if s == keyword)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn is_at_end(&self) -> bool {
        self.current.kind == TokenKind::Eof
    }

    fn is_keyword(&self) -> bool {
        matches!(
            self.current.kind,
            TokenKind::Let
                | TokenKind::Const
                | TokenKind::Var
                | TokenKind::Function
                | TokenKind::Return
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::For
                | TokenKind::While
                | TokenKind::Do
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Switch
                | TokenKind::Case
                | TokenKind::Default
                | TokenKind::Try
                | TokenKind::Catch
                | TokenKind::Finally
                | TokenKind::Throw
                | TokenKind::New
                | TokenKind::This
                | TokenKind::Class
                | TokenKind::Extends
                | TokenKind::Static
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::From
                | TokenKind::As
                | TokenKind::Type
                | TokenKind::Interface
                | TokenKind::Enum
                | TokenKind::Accessor
                | TokenKind::Namespace
                | TokenKind::Module
                | TokenKind::Private
                | TokenKind::Public
                | TokenKind::Protected
                | TokenKind::Readonly
                | TokenKind::Declare
                | TokenKind::Implements
                | TokenKind::Any
                | TokenKind::Unknown
                | TokenKind::Never
                | TokenKind::Keyof
                | TokenKind::Infer
                | TokenKind::Is
                | TokenKind::Asserts
                // Literals that can be used as property names
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                // Other reserved words that can be used as property names
                | TokenKind::Delete
                | TokenKind::Typeof
                | TokenKind::Void
                | TokenKind::Instanceof
                | TokenKind::Yield
                | TokenKind::Await
                | TokenKind::Super
                | TokenKind::Async
                | TokenKind::In
                | TokenKind::Of
        )
    }

    fn keyword_to_js_string(&mut self) -> JsString {
        self.intern(match &self.current.kind {
            TokenKind::Let => "let",
            TokenKind::Const => "const",
            TokenKind::Var => "var",
            TokenKind::Function => "function",
            TokenKind::Return => "return",
            TokenKind::If => "if",
            TokenKind::Else => "else",
            TokenKind::For => "for",
            TokenKind::While => "while",
            TokenKind::Do => "do",
            TokenKind::Break => "break",
            TokenKind::Continue => "continue",
            TokenKind::Switch => "switch",
            TokenKind::Case => "case",
            TokenKind::Default => "default",
            TokenKind::Try => "try",
            TokenKind::Catch => "catch",
            TokenKind::Finally => "finally",
            TokenKind::Throw => "throw",
            TokenKind::New => "new",
            TokenKind::This => "this",
            TokenKind::Class => "class",
            TokenKind::Extends => "extends",
            TokenKind::Static => "static",
            TokenKind::Import => "import",
            TokenKind::Export => "export",
            TokenKind::From => "from",
            TokenKind::As => "as",
            TokenKind::Type => "type",
            TokenKind::Interface => "interface",
            TokenKind::Enum => "enum",
            TokenKind::Of => "of",
            TokenKind::In => "in",
            TokenKind::Any => "any",
            TokenKind::Unknown => "unknown",
            TokenKind::Never => "never",
            TokenKind::Keyof => "keyof",
            TokenKind::Infer => "infer",
            TokenKind::Is => "is",
            TokenKind::Asserts => "asserts",
            TokenKind::Readonly => "readonly",
            TokenKind::Accessor => "accessor",
            // Reserved words that can be used as property names
            TokenKind::Delete => "delete",
            TokenKind::Typeof => "typeof",
            TokenKind::Void => "void",
            TokenKind::Instanceof => "instanceof",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::Null => "null",
            TokenKind::Yield => "yield",
            TokenKind::Await => "await",
            TokenKind::Super => "super",
            TokenKind::Async => "async",
            TokenKind::Namespace => "namespace",
            TokenKind::Module => "module",
            TokenKind::Private => "private",
            TokenKind::Public => "public",
            TokenKind::Protected => "protected",
            TokenKind::Declare => "declare",
            TokenKind::Implements => "implements",
            _ => "",
        })
    }

    fn peek_is_property_name(&mut self) -> bool {
        // Peek at the next token to determine if it's a property name
        // Used for distinguishing `get: ...` (property named "get") from `get x() {}` (getter)
        let checkpoint = self.lexer.checkpoint();
        let next = self.lexer.next_token();
        self.lexer.restore(checkpoint);

        // If next token is `:` or `,` or `}`, current is the property name itself
        // If next token is identifier, string, number, or `[`, current is get/set keyword
        match next.kind {
            // These indicate the current token is used as a property name
            TokenKind::Colon | TokenKind::Comma | TokenKind::RBrace => false,
            // These indicate the current token is a get/set/async keyword and next is the property name
            TokenKind::Identifier(_)
            | TokenKind::String(_)
            | TokenKind::Number(_)
            | TokenKind::LBracket
            | TokenKind::Star => true, // Star for async *gen() {}
            // Keywords can be property names
            _ if self.is_keyword_kind(&next.kind) => true,
            // Anything else is unclear, assume it's a property name (not get/set keyword)
            _ => false,
        }
    }

    fn is_keyword_kind(&self, kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::If
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Function
                | TokenKind::Return
                | TokenKind::Var
                | TokenKind::Let
                | TokenKind::Const
                | TokenKind::Class
                | TokenKind::Extends
                | TokenKind::New
                | TokenKind::This
                | TokenKind::Delete
                | TokenKind::Typeof
                | TokenKind::Void
                | TokenKind::Instanceof
                | TokenKind::In
                | TokenKind::Switch
                | TokenKind::Case
                | TokenKind::Default
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Try
                | TokenKind::Catch
                | TokenKind::Finally
                | TokenKind::Throw
                | TokenKind::Do
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::From
                | TokenKind::As
                | TokenKind::Of
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Yield
                | TokenKind::Await
                | TokenKind::Super
                | TokenKind::Async
                | TokenKind::Static
                | TokenKind::Type
                | TokenKind::Interface
                | TokenKind::Enum
                | TokenKind::Namespace
                | TokenKind::Private
                | TokenKind::Public
                | TokenKind::Protected
                | TokenKind::Declare
                | TokenKind::Implements
                | TokenKind::Abstract
                | TokenKind::Readonly
                | TokenKind::Accessor
        )
    }

    fn span_from(&self, start: Span) -> Span {
        Span::new(
            start.start,
            self.previous.span.end,
            start.line,
            start.column,
        )
    }

    fn error(&self, message: &str) -> JsError {
        JsError::syntax_error(message, self.current.span.line, self.current.span.column)
    }

    fn unexpected_token(&self, expected: &str) -> JsError {
        JsError::syntax_error(
            format!("Unexpected {:?}, expected {}", self.current.kind, expected),
            self.current.span.line,
            self.current.span.column,
        )
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, bool)> {
        // Returns (operator, precedence, is_logical)
        match &self.current.kind {
            TokenKind::PipePipe => Some((BinaryOp::BitOr, 4, true)),
            TokenKind::AmpAmp => Some((BinaryOp::BitAnd, 5, true)),
            TokenKind::QuestionQuestion => Some((BinaryOp::BitOr, 4, true)), // Reuse, handle in parse
            TokenKind::Pipe => Some((BinaryOp::BitOr, 6, false)),
            TokenKind::Caret => Some((BinaryOp::BitXor, 7, false)),
            TokenKind::Amp => Some((BinaryOp::BitAnd, 8, false)),
            TokenKind::EqEq => Some((BinaryOp::Eq, 9, false)),
            TokenKind::BangEq => Some((BinaryOp::NotEq, 9, false)),
            TokenKind::EqEqEq => Some((BinaryOp::StrictEq, 9, false)),
            TokenKind::BangEqEq => Some((BinaryOp::StrictNotEq, 9, false)),
            TokenKind::Lt => Some((BinaryOp::Lt, 10, false)),
            TokenKind::LtEq => Some((BinaryOp::LtEq, 10, false)),
            TokenKind::Gt => Some((BinaryOp::Gt, 10, false)),
            TokenKind::GtEq => Some((BinaryOp::GtEq, 10, false)),
            TokenKind::In if !self.no_in => Some((BinaryOp::In, 10, false)),
            TokenKind::Instanceof => Some((BinaryOp::Instanceof, 10, false)),
            TokenKind::LtLt => Some((BinaryOp::LShift, 11, false)),
            TokenKind::GtGt => Some((BinaryOp::RShift, 11, false)),
            TokenKind::GtGtGt => Some((BinaryOp::URShift, 11, false)),
            TokenKind::Plus => Some((BinaryOp::Add, 12, false)),
            TokenKind::Minus => Some((BinaryOp::Sub, 12, false)),
            TokenKind::Star => Some((BinaryOp::Mul, 13, false)),
            TokenKind::Slash => Some((BinaryOp::Div, 13, false)),
            TokenKind::Percent => Some((BinaryOp::Mod, 13, false)),
            TokenKind::StarStar => Some((BinaryOp::Exp, 14, false)),
            _ => None,
        }
    }

    fn current_unary_op(&self) -> Option<UnaryOp> {
        match &self.current.kind {
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
            TokenKind::Typeof => Some(UnaryOp::Typeof),
            TokenKind::Void => Some(UnaryOp::Void),
            TokenKind::Delete => Some(UnaryOp::Delete),
            _ => None,
        }
    }

    fn current_update_op(&self) -> Option<UpdateOp> {
        match &self.current.kind {
            TokenKind::PlusPlus => Some(UpdateOp::Increment),
            TokenKind::MinusMinus => Some(UpdateOp::Decrement),
            _ => None,
        }
    }

    fn current_assignment_op(&self) -> Option<AssignmentOp> {
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

    #[allow(clippy::only_used_in_recursion)]
    fn expression_to_pattern(&self, expr: &Expression) -> Result<Pattern, JsError> {
        match expr {
            Expression::Identifier(id) => Ok(Pattern::Identifier(id.clone())),
            Expression::Object(obj) => {
                let properties = obj
                    .properties
                    .iter()
                    .map(|prop| match prop {
                        ObjectProperty::Property(p) => {
                            let value = self.expression_to_pattern(&p.value)?;
                            Ok(ObjectPatternProperty::KeyValue {
                                key: p.key.clone(),
                                value,
                                shorthand: p.shorthand,
                                span: p.span,
                            })
                        }
                        ObjectProperty::Spread(s) => {
                            let arg = self.expression_to_pattern(&s.argument)?;
                            Ok(ObjectPatternProperty::Rest(RestElement {
                                argument: Box::new(arg),
                                type_annotation: None,
                                span: s.span,
                            }))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Pattern::Object(ObjectPattern {
                    properties,
                    type_annotation: None,
                    span: obj.span,
                }))
            }
            Expression::Array(arr) => {
                let elements = arr
                    .elements
                    .iter()
                    .map(|elem| {
                        elem.as_ref()
                            .map(|e| match e {
                                ArrayElement::Expression(expr) => self.expression_to_pattern(expr),
                                ArrayElement::Spread(s) => {
                                    let arg = self.expression_to_pattern(&s.argument)?;
                                    Ok(Pattern::Rest(RestElement {
                                        argument: Box::new(arg),
                                        type_annotation: None,
                                        span: s.span,
                                    }))
                                }
                            })
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Pattern::Array(ArrayPattern {
                    elements,
                    type_annotation: None,
                    span: arr.span,
                }))
            }
            Expression::Assignment(assign) => {
                let left = match &assign.left {
                    AssignmentTarget::Identifier(id) => Pattern::Identifier(id.clone()),
                    AssignmentTarget::Pattern(p) => p.clone(),
                    AssignmentTarget::Member(_) => {
                        return Err(JsError::syntax_error(
                            "Invalid destructuring target",
                            assign.span.line,
                            assign.span.column,
                        ));
                    }
                };

                Ok(Pattern::Assignment(AssignmentPattern {
                    left: Box::new(left),
                    right: assign.right.clone(),
                    span: assign.span,
                }))
            }
            _ => Err(JsError::syntax_error(
                "Invalid destructuring target",
                expr.span().line,
                expr.span().column,
            )),
        }
    }

    fn expression_to_assignment_target(
        &self,
        expr: &Expression,
    ) -> Result<AssignmentTarget, JsError> {
        match expr {
            Expression::Identifier(id) => Ok(AssignmentTarget::Identifier(id.clone())),
            Expression::Member(m) => Ok(AssignmentTarget::Member((**m).clone())),
            Expression::Object(_) | Expression::Array(_) => {
                let pattern = self.expression_to_pattern(expr)?;
                Ok(AssignmentTarget::Pattern(pattern))
            }
            _ => Err(JsError::syntax_error(
                "Invalid assignment target",
                expr.span().line,
                expr.span().column,
            )),
        }
    }

    fn expression_to_param(&self, expr: &Expression) -> Result<FunctionParam, JsError> {
        let span = expr.span();

        // Handle assignment pattern (default value)
        if let Expression::Assignment(assign) = expr {
            if assign.operator == AssignmentOp::Assign {
                let left = match &assign.left {
                    AssignmentTarget::Identifier(id) => Pattern::Identifier(id.clone()),
                    AssignmentTarget::Pattern(p) => p.clone(),
                    _ => {
                        return Err(JsError::syntax_error(
                            "Invalid parameter",
                            span.line,
                            span.column,
                        ));
                    }
                };

                return Ok(FunctionParam {
                    pattern: Pattern::Assignment(AssignmentPattern {
                        left: Box::new(left),
                        right: assign.right.clone(),
                        span,
                    }),
                    type_annotation: None,
                    optional: false,
                    decorators: vec![],
                    accessibility: None,
                    readonly: false,
                    span,
                });
            }
        }

        let pattern = self.expression_to_pattern(expr)?;
        Ok(FunctionParam {
            pattern,
            type_annotation: None,
            optional: false,
            decorators: vec![],
            accessibility: None,
            readonly: false,
            span,
        })
    }
}
