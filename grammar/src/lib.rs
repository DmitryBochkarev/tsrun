//! TypeScript Grammar Definition
//!
//! This module defines the complete TypeScript grammar using the trampoline-parser DSL.
//! The grammar generates a fully trampoline-based lexer and parser.

use trampoline_parser::{Assoc, AstConfigBuilder, Combinator, CombinatorExt, Grammar, RuleBuilder};

/// Build the TypeScript grammar
pub fn typescript_grammar() -> Grammar {
    Grammar::new()
        .ast_config(ast_config)
        // Program (entry point - must be first!)
        .rule("program", rule_program)
        // Whitespace and comments
        .rule("ws", rule_ws)
        .rule("line_comment", rule_line_comment)
        .rule("block_comment", rule_block_comment)
        // Literals
        .rule("number_literal", rule_number_literal)
        .rule("string_literal", rule_string_literal)
        .rule("template_string", rule_template_string)
        .rule("template_head", rule_template_head)
        .rule("template_middle", rule_template_middle)
        .rule("template_tail", rule_template_tail)
        // Statements
        .rule("statement", rule_statement)
        .rule("variable_declaration", rule_variable_declaration)
        .rule("variable_declarator", rule_variable_declarator)
        .rule(
            "variable_declaration_no_semi",
            rule_variable_declaration_no_semi,
        )
        .rule("function_declaration", rule_function_declaration)
        .rule("class_declaration", rule_class_declaration)
        .rule("class_body", rule_class_body)
        .rule("class_member", rule_class_member)
        .rule("class_constructor", rule_class_constructor)
        .rule("class_method", rule_class_method)
        .rule("class_property", rule_class_property)
        .rule("static_block", rule_static_block)
        .rule("if_statement", rule_if_statement)
        .rule("for_statement", rule_for_statement)
        .rule("for_init", rule_for_init)
        .rule("for_in_statement", rule_for_in_statement)
        .rule("for_of_statement", rule_for_of_statement)
        .rule("for_in_of_left", rule_for_in_of_left)
        .rule("while_statement", rule_while_statement)
        .rule("do_while_statement", rule_do_while_statement)
        .rule("switch_statement", rule_switch_statement)
        .rule("switch_case", rule_switch_case)
        .rule("try_statement", rule_try_statement)
        .rule("catch_clause", rule_catch_clause)
        .rule("block_statement", rule_block_statement)
        .rule("return_statement", rule_return_statement)
        .rule("break_statement", rule_break_statement)
        .rule("continue_statement", rule_continue_statement)
        .rule("throw_statement", rule_throw_statement)
        .rule("debugger_statement", rule_debugger_statement)
        .rule("labeled_statement", rule_labeled_statement)
        .rule("expression_statement", rule_expression_statement)
        .rule("empty_statement", rule_empty_statement)
        .rule("semicolon", rule_semicolon)
        // Import/Export
        .rule("import_declaration", rule_import_declaration)
        .rule("import_clause", rule_import_clause)
        .rule("import_clause_rest", rule_import_clause_rest)
        .rule("named_imports", rule_named_imports)
        .rule("import_specifier", rule_import_specifier)
        .rule("export_declaration", rule_export_declaration)
        .rule("export_default_expression", rule_export_default_expression)
        .rule("named_exports", rule_named_exports)
        .rule("export_specifier", rule_export_specifier)
        .rule("exportable_declaration", rule_exportable_declaration)
        // TypeScript Declarations
        .rule("type_alias_declaration", rule_type_alias_declaration)
        .rule("interface_declaration", rule_interface_declaration)
        .rule("enum_declaration", rule_enum_declaration)
        .rule("enum_member", rule_enum_member)
        .rule("namespace_declaration", rule_namespace_declaration)
        // Expressions
        .rule("expression", rule_expression)
        .rule("primary", rule_primary)
        .rule("primary_inner", rule_primary_inner)
        .rule("literal", rule_literal)
        .rule("identifier", rule_identifier)
        .rule("this_expression", rule_this_expression)
        .rule("super_expression", rule_super_expression)
        .rule("array_expression", rule_array_expression)
        .rule("array_element", rule_array_element)
        .rule("object_expression", rule_object_expression)
        .rule("object_property", rule_object_property)
        .rule("key_value_property", rule_key_value_property)
        .rule("shorthand_property", rule_shorthand_property)
        .rule("method_property", rule_method_property)
        .rule("getter_property", rule_getter_property)
        .rule("setter_property", rule_setter_property)
        .rule("property_key", rule_property_key)
        .rule("spread_element", rule_spread_element)
        .rule("function_expression", rule_function_expression)
        .rule("arrow_function", rule_arrow_function)
        .rule("arrow_body", rule_arrow_body)
        .rule("class_expression", rule_class_expression)
        .rule("template_literal", rule_template_literal)
        .rule("new_expression", rule_new_expression)
        .rule("yield_expression", rule_yield_expression)
        .rule("parenthesized", rule_parenthesized)
        .rule("argument_list", rule_argument_list)
        .rule("argument", rule_argument)
        // Parameters
        .rule("parameter_list", rule_parameter_list)
        .rule("parameter", rule_parameter)
        .rule("accessibility_modifier", rule_accessibility_modifier)
        .rule("decorator", rule_decorator)
        // Patterns
        .rule("pattern", rule_pattern)
        .rule("identifier_pattern", rule_identifier_pattern)
        .rule("object_pattern", rule_object_pattern)
        .rule("object_pattern_property", rule_object_pattern_property)
        .rule("array_pattern", rule_array_pattern)
        .rule("array_pattern_element", rule_array_pattern_element)
        .rule("rest_pattern", rule_rest_pattern)
        // Type Annotations
        .rule("type_annotation", rule_type_annotation)
        .rule("type", rule_type)
        .rule("primary_type", rule_primary_type)
        .rule("keyword_type", rule_keyword_type)
        .rule("type_reference", rule_type_reference)
        .rule("type_arguments", rule_type_arguments)
        .rule("type_parameters", rule_type_parameters)
        .rule("type_parameter", rule_type_parameter)
        .rule("literal_type", rule_literal_type)
        .rule("object_type", rule_object_type)
        .rule("type_member", rule_type_member)
        .rule("property_signature", rule_property_signature)
        .rule("method_signature", rule_method_signature)
        .rule("index_signature", rule_index_signature)
        .rule("call_signature", rule_call_signature)
        .rule("construct_signature", rule_construct_signature)
        .rule("type_member_separator", rule_type_member_separator)
        .rule("base_type", rule_base_type)
        .rule("tuple_type", rule_tuple_type)
        .rule("union_type", rule_union_type)
        .rule("intersection_type", rule_intersection_type)
        .rule("function_type", rule_function_type)
        .rule("conditional_type", rule_conditional_type)
        .rule("typeof_type", rule_typeof_type)
        .rule("keyof_type", rule_keyof_type)
        .rule("infer_type", rule_infer_type)
        .rule("mapped_type", rule_mapped_type)
        .rule("parenthesized_type", rule_parenthesized_type)
}

// === AST Configuration ===

fn ast_config(c: AstConfigBuilder) -> AstConfigBuilder {
    c.import("crate::value::JsString")
        .import("crate::prelude::Rc")
        .import("crate::string_dict::StringDict")
        .import("crate::ast::*")
        .string_type("JsString")
        .string_dict("StringDict")
        .apply_mappings()
        // Use existing AST types directly as ParseResult variants
        .result_variant("Expr", "Expression")
        .result_variant("Stmt", "Statement")
        .result_variant("Ident", "Identifier")
        .result_variant("Pat", "Pattern")
        .result_variant("Prog", "Program")
        .helper(HELPER_FUNCTIONS)
}

const HELPER_FUNCTIONS: &str = r#"
/// Parse a number literal string to f64
fn parse_number(text: &JsString) -> f64 {
    let s = text.as_ref();
    // Handle hex, binary, octal
    if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).map(|n| n as f64).unwrap_or(f64::NAN)
    } else if s.starts_with("0b") || s.starts_with("0B") {
        i64::from_str_radix(&s[2..], 2).map(|n| n as f64).unwrap_or(f64::NAN)
    } else if s.starts_with("0o") || s.starts_with("0O") {
        i64::from_str_radix(&s[2..], 8).map(|n| n as f64).unwrap_or(f64::NAN)
    } else {
        // Remove underscores and parse
        let cleaned: String = s.chars().filter(|c| *c != '_').collect();
        cleaned.parse().unwrap_or(f64::NAN)
    }
}

/// Parse a string literal, handling escape sequences
fn parse_string_literal(text: &JsString) -> JsString {
    let s = text.as_ref();
    // Remove quotes
    if s.len() < 2 {
        return JsString::from("");
    }
    let inner = &s[1..s.len()-1];
    // TODO: Handle escape sequences properly
    JsString::from(inner)
}

/// Extract Expression from ParseResult
fn to_expr(result: ParseResult) -> Result<Expression, ParseError> {
    match result {
        ParseResult::Expr(e) => Ok(e),
        ParseResult::Ident(id) => Ok(Expression::Identifier(id)),
        other => Err(ParseError::new(format!("Expected Expression, got {:?}", other), 0, 0)),
    }
}

/// Extract Statement from ParseResult
fn to_stmt(result: ParseResult) -> Result<Statement, ParseError> {
    match result {
        ParseResult::Stmt(s) => Ok(s),
        other => Err(ParseError::new(format!("Expected Statement, got {:?}", other), 0, 0)),
    }
}

/// Extract Identifier from ParseResult
fn to_ident(result: ParseResult) -> Result<Identifier, ParseError> {
    match result {
        ParseResult::Ident(id) => Ok(id),
        other => Err(ParseError::new(format!("Expected Identifier, got {:?}", other), 0, 0)),
    }
}

/// Extract Pattern from ParseResult
fn to_pattern(result: ParseResult) -> Result<Pattern, ParseError> {
    match result {
        ParseResult::Pat(p) => Ok(p),
        ParseResult::Ident(id) => Ok(Pattern::Identifier(id)),
        other => Err(ParseError::new(format!("Expected Pattern, got {:?}", other), 0, 0)),
    }
}

/// Create binary expression
fn binary(left: ParseResult, right: ParseResult, op: BinaryOp, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::Binary(BinaryExpression {
        operator: op,
        left: Rc::new(to_expr(left)?),
        right: Rc::new(to_expr(right)?),
        span,
    })))
}

/// Create logical expression
fn logical(left: ParseResult, right: ParseResult, op: LogicalOp, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::Logical(LogicalExpression {
        operator: op,
        left: Rc::new(to_expr(left)?),
        right: Rc::new(to_expr(right)?),
        span,
    })))
}

/// Create unary expression
fn unary(operand: ParseResult, op: UnaryOp, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::Unary(UnaryExpression {
        operator: op,
        argument: Rc::new(to_expr(operand)?),
        prefix: true,
        span,
    })))
}

/// Create conditional (ternary) expression
fn conditional(test: ParseResult, cons: ParseResult, alt: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::Conditional(ConditionalExpression {
        test: Rc::new(to_expr(test)?),
        consequent: Rc::new(to_expr(cons)?),
        alternate: Rc::new(to_expr(alt)?),
        span,
    })))
}

/// Create call expression
fn call(callee: ParseResult, args: Vec<ParseResult>, optional: bool, span: Span) -> Result<ParseResult, ParseError> {
    let arguments = args.into_iter()
        .map(|a| Ok(Argument::Expression(to_expr(a)?)))
        .collect::<Result<Vec<_>, ParseError>>()?;
    Ok(ParseResult::Expr(Expression::Call(Box::new(CallExpression {
        callee: Rc::new(to_expr(callee)?),
        arguments,
        type_arguments: None,
        optional,
        span,
    }))))
}

/// Create member expression (property is JsString from postfix_member)
fn member(obj: ParseResult, prop: JsString, optional: bool, span: Span) -> Result<ParseResult, ParseError> {
    let property = MemberProperty::Identifier(Identifier { name: prop, span: span.clone() });
    Ok(ParseResult::Expr(Expression::Member(Box::new(MemberExpression {
        object: Rc::new(to_expr(obj)?),
        property,
        computed: false,
        optional,
        span,
    }))))
}

/// Create computed member expression
fn member_computed(obj: ParseResult, expr: ParseResult, optional: bool, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::Member(Box::new(MemberExpression {
        object: Rc::new(to_expr(obj)?),
        property: MemberProperty::Expression(Rc::new(to_expr(expr)?)),
        computed: true,
        optional,
        span,
    }))))
}

/// Create update expression (++/--)
fn update(arg: ParseResult, op: UpdateOp, prefix: bool, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::Update(UpdateExpression {
        operator: op,
        argument: Rc::new(to_expr(arg)?),
        prefix,
        span,
    })))
}

/// Create await expression
fn await_expr(arg: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::Await(AwaitExpression {
        argument: Rc::new(to_expr(arg)?),
        span,
    })))
}

/// Create assignment expression
fn assign(left: ParseResult, right: ParseResult, op: AssignmentOp, span: Span) -> Result<ParseResult, ParseError> {
    let target = match left {
        ParseResult::Ident(id) => AssignmentTarget::Identifier(id),
        ParseResult::Expr(Expression::Member(m)) => AssignmentTarget::Member(*m),
        _ => return Err(ParseError::new("Invalid assignment target".to_string(), 0, 0)),
    };
    Ok(ParseResult::Expr(Expression::Assignment(Box::new(AssignmentExpression {
        operator: op,
        left: target,
        right: Rc::new(to_expr(right)?),
        span,
    }))))
}

/// Create type assertion expression (x as T)
/// Types are stripped at runtime, so we use a placeholder TypeAnnotation
fn type_assertion(expr: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::TypeAssertion(TypeAssertionExpression {
        expression: Rc::new(to_expr(expr)?),
        type_annotation: Box::new(TypeAnnotation::Keyword(TypeKeyword {
            keyword: TypeKeywordKind::Any,
            span: span.clone(),
        })),
        span,
    })))
}

/// Convert ParseResult to ArrowFunctionBody
fn to_arrow_body(result: ParseResult) -> Result<Box<ArrowFunctionBody>, ParseError> {
    match result {
        ParseResult::Stmt(Statement::Block(block)) => {
            Ok(Box::new(ArrowFunctionBody::Block(Rc::new(block))))
        }
        ParseResult::Expr(e) => Ok(Box::new(ArrowFunctionBody::Expression(Rc::new(e)))),
        other => {
            // Try to extract expression
            if let Ok(e) = to_expr(other) {
                Ok(Box::new(ArrowFunctionBody::Expression(Rc::new(e))))
            } else {
                Err(ParseError::new("Expected arrow function body".to_string(), 0, 0))
            }
        }
    }
}

/// Extract pattern from parameter (a sequence of [decorators, accessibility?, readonly?, pattern, ...])
fn extract_param_pattern(item: ParseResult, default_span: &Span) -> Option<FunctionParam> {
    match item {
        // Direct identifier (simplified case)
        ParseResult::Ident(id) => Some(FunctionParam {
            pattern: Pattern::Identifier(id.clone()),
            type_annotation: None,
            optional: false,
            decorators: vec![],
            accessibility: None,
            readonly: false,
            span: id.span,
        }),
        // Direct pattern
        ParseResult::Pat(p) => Some(FunctionParam {
            pattern: p.clone(),
            type_annotation: None,
            optional: false,
            decorators: vec![],
            accessibility: None,
            readonly: false,
            span: default_span.clone(),
        }),
        // Parameter sequence: [decorators, accessibility?, readonly?, pattern, optional?, type_annotation?, default?]
        ParseResult::List(parts) => {
            let mut iter = parts.into_iter();
            let _decorators = iter.next(); // Skip decorators
            let _accessibility = iter.next(); // Skip accessibility
            let _readonly = iter.next(); // Skip readonly
            let pattern_result = iter.next()?; // Get pattern

            // Try to convert pattern_result to a Pattern
            let pattern = match pattern_result {
                ParseResult::Pat(p) => p,
                ParseResult::Ident(id) => Pattern::Identifier(id),
                _ => return None,
            };

            Some(FunctionParam {
                pattern,
                type_annotation: None,
                optional: false,
                decorators: vec![],
                accessibility: None,
                readonly: false,
                span: default_span.clone(),
            })
        }
        _ => None,
    }
}

/// Create arrow function with parenthesized params
fn arrow_function_parens(async_kw: ParseResult, params: ParseResult, body: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    let is_async = !matches!(async_kw, ParseResult::None);
    let params_vec: Vec<FunctionParam> = match params {
        ParseResult::List(items) => {
            items.into_iter().filter_map(|item| {
                extract_param_pattern(item, &span)
            }).collect()
        }
        ParseResult::None => vec![],
        _ => vec![],
    };
    Ok(ParseResult::Expr(Expression::ArrowFunction(Box::new(ArrowFunctionExpression {
        params: Rc::from(params_vec),
        return_type: None,
        type_parameters: None,
        body: to_arrow_body(body)?,
        async_: is_async,
        span,
    }))))
}

/// Create arrow function with single identifier param
fn arrow_function_single(async_kw: ParseResult, param: ParseResult, body: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    let is_async = !matches!(async_kw, ParseResult::None);
    let id = to_ident(param)?;
    let params = vec![FunctionParam {
        pattern: Pattern::Identifier(id.clone()),
        type_annotation: None,
        optional: false,
        decorators: vec![],
        accessibility: None,
        readonly: false,
        span: id.span,
    }];
    Ok(ParseResult::Expr(Expression::ArrowFunction(Box::new(ArrowFunctionExpression {
        params: Rc::from(params),
        return_type: None,
        type_parameters: None,
        body: to_arrow_body(body)?,
        async_: is_async,
        span,
    }))))
}

/// Convert params list to FunctionParam vector
fn params_to_vec(params: ParseResult, span: &Span) -> Vec<FunctionParam> {
    match params {
        ParseResult::List(items) => {
            items.into_iter().filter_map(|item| {
                extract_param_pattern(item, span)
            }).collect()
        }
        ParseResult::None => vec![],
        _ => vec![],
    }
}

/// Extract BlockStatement from ParseResult
fn to_block(result: ParseResult) -> Result<Rc<BlockStatement>, ParseError> {
    match result {
        ParseResult::Stmt(Statement::Block(b)) => Ok(Rc::new(b)),
        _ => Err(ParseError::new("Expected block statement".to_string(), 0, 0)),
    }
}

/// Create block statement from list of statements
fn create_block_stmt(result: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    // result is [{, statements*, }]
    if let ParseResult::List(parts) = result {
        let mut iter = parts.into_iter();
        let _open_brace = iter.next(); // skip {
        let stmts_result = iter.next().unwrap_or(ParseResult::None);
        let _close_brace = iter.next(); // skip }

        let statements: Vec<Statement> = match stmts_result {
            ParseResult::List(items) => {
                items.into_iter().filter_map(|item| {
                    match item {
                        ParseResult::Stmt(s) => Some(s),
                        _ => None,
                    }
                }).collect()
            }
            ParseResult::Stmt(s) => vec![s],
            ParseResult::None => vec![],
            _ => vec![],
        };

        Ok(ParseResult::Stmt(Statement::Block(BlockStatement {
            body: Rc::from(statements),
            span,
        })))
    } else {
        Err(ParseError::new("Expected block parts".to_string(), 0, 0))
    }
}

/// Create enum declaration
fn create_enum_decl(const_kw: ParseResult, name: ParseResult, members: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    let is_const = !matches!(const_kw, ParseResult::None);
    let id = to_ident(name)?;

    let members_vec: Vec<EnumMember> = match members {
        ParseResult::List(items) => {
            items.into_iter().filter_map(|item| {
                // Each member is [identifier, optional([=, expr])]
                if let ParseResult::List(parts) = item {
                    let mut iter = parts.into_iter();
                    let id_result = iter.next()?;
                    let id = to_ident(id_result).ok()?;
                    let init_result = iter.next();

                    let initializer = match init_result {
                        Some(ParseResult::List(init_parts)) => {
                            // [=, expr]
                            init_parts.into_iter().nth(1).and_then(|e| to_expr(e).ok())
                        }
                        _ => None,
                    };

                    Some(EnumMember {
                        id: id.clone(),
                        initializer,
                        span: id.span,
                    })
                } else if let ParseResult::Ident(id) = item {
                    // Simple identifier member without initializer
                    Some(EnumMember {
                        id: id.clone(),
                        initializer: None,
                        span: id.span,
                    })
                } else {
                    None
                }
            }).collect()
        }
        ParseResult::None => vec![],
        _ => vec![],
    };

    Ok(ParseResult::Stmt(Statement::EnumDeclaration(Box::new(EnumDeclaration {
        id,
        members: members_vec,
        const_: is_const,
        span,
    }))))
}

/// Create function declaration
fn create_function_decl(async_kw: ParseResult, generator: ParseResult, name: ParseResult, params: ParseResult, body: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    let is_async = !matches!(async_kw, ParseResult::None);
    let is_generator = !matches!(generator, ParseResult::None);
    let id = match name {
        ParseResult::Ident(id) => Some(id),
        ParseResult::None => None,
        _ => None,
    };
    let params_vec = params_to_vec(params, &span);

    Ok(ParseResult::Stmt(Statement::FunctionDeclaration(Box::new(FunctionDeclaration {
        id,
        params: Rc::from(params_vec),
        return_type: None,
        type_parameters: None,
        body: to_block(body)?,
        generator: is_generator,
        async_: is_async,
        span,
    }))))
}

/// Create array expression from parsed elements
fn create_array_expr(elements_result: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    // elements_result is [open_bracket, optional([first_elem?, [(comma, elem?)*]]), close_bracket]
    let mut elements: Vec<Option<ArrayElement>> = Vec::new();

    if let ParseResult::List(parts) = elements_result {
        // parts = [open_bracket, optional_content, close_bracket]
        if let Some(content) = parts.into_iter().nth(1) {
            // content might be ParseResult::None (empty array) or List
            if let ParseResult::List(content_parts) = content {
                // content_parts = [first_elem?, rest_list]
                let mut iter = content_parts.into_iter();

                // First element (optional)
                if let Some(first) = iter.next() {
                    let elem = parse_result_to_array_element(first);
                    elements.push(elem);
                }

                // Rest: [(comma, elem?)*]
                if let Some(ParseResult::List(rest)) = iter.next() {
                    for item in rest {
                        // item = [comma, elem?]
                        if let ParseResult::List(pair) = item {
                            let elem = pair.into_iter().nth(1).and_then(parse_result_to_array_element);
                            elements.push(elem);
                        }
                    }
                }
            }
        }
    }

    Ok(ParseResult::Expr(Expression::Array(ArrayExpression {
        elements,
        span,
    })))
}

/// Convert ParseResult to ArrayElement (None for holes/elisions)
fn parse_result_to_array_element(result: ParseResult) -> Option<ArrayElement> {
    match result {
        ParseResult::None => None,
        ParseResult::Expr(e) => Some(ArrayElement::Expression(e)),
        // For spread, check if it's wrapped
        ParseResult::List(items) if items.len() == 2 => {
            // Could be spread: [spread_op, expr]
            if let Some(e) = items.into_iter().last() {
                if let Ok(expr) = to_expr(e) {
                    return Some(ArrayElement::Spread(SpreadElement {
                        argument: Rc::new(expr),
                        span: Span { start: 0, end: 0, line: 0, column: 0 },
                    }));
                }
            }
            None
        }
        other => {
            // Try to convert to expression
            if let Ok(e) = to_expr(other) {
                Some(ArrayElement::Expression(e))
            } else {
                None
            }
        }
    }
}
"#;

// === Whitespace and Comments ===

fn rule_ws(r: &RuleBuilder) -> Combinator {
    r.skip(r.zero_or_more(r.choice((
        r.one_or_more(r.ws()),
        r.parse("line_comment"),
        r.parse("block_comment"),
    ))))
}

fn rule_line_comment(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.lit("//"),
        r.zero_or_more(r.sequence((
            r.not_followed_by(r.char('\n')),
            r.any_char(),
        ))),
    ))
}

fn rule_block_comment(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.lit("/*"),
        r.zero_or_more(r.sequence((
            r.not_followed_by(r.lit("*/")),
            r.any_char(),
        ))),
        r.lit("*/"),
    ))
}

// === Literals ===

fn rule_number_literal(r: &RuleBuilder) -> Combinator {
    r.capture(r.choice((
        // Hex: 0x...
        r.sequence((
            r.lit("0"),
            r.choice((r.char('x'), r.char('X'))),
            r.one_or_more(r.choice((r.hex_digit(), r.char('_')))),
            r.optional(r.char('n')), // BigInt suffix
        )),
        // Binary: 0b...
        r.sequence((
            r.lit("0"),
            r.choice((r.char('b'), r.char('B'))),
            r.one_or_more(r.choice((r.range('0', '1'), r.char('_')))),
            r.optional(r.char('n')),
        )),
        // Octal: 0o...
        r.sequence((
            r.lit("0"),
            r.choice((r.char('o'), r.char('O'))),
            r.one_or_more(r.choice((r.range('0', '7'), r.char('_')))),
            r.optional(r.char('n')),
        )),
        // Decimal (possibly with exponent)
        r.sequence((
            r.one_or_more(r.choice((r.digit(), r.char('_')))),
            r.optional(r.sequence((
                r.char('.'),
                r.zero_or_more(r.choice((r.digit(), r.char('_')))),
            ))),
            r.optional(r.sequence((
                r.choice((r.char('e'), r.char('E'))),
                r.optional(r.choice((r.char('+'), r.char('-')))),
                r.one_or_more(r.choice((r.digit(), r.char('_')))),
            ))),
            r.optional(r.char('n')),
        )),
        // Decimal starting with dot
        r.sequence((
            r.char('.'),
            r.one_or_more(r.choice((r.digit(), r.char('_')))),
            r.optional(r.sequence((
                r.choice((r.char('e'), r.char('E'))),
                r.optional(r.choice((r.char('+'), r.char('-')))),
                r.one_or_more(r.choice((r.digit(), r.char('_')))),
            ))),
        )),
    )))
}

fn rule_string_literal(r: &RuleBuilder) -> Combinator {
    r.capture(r.choice((
        // Double-quoted
        r.sequence((
            r.char('"'),
            r.zero_or_more(r.choice((
                r.sequence((r.char('\\'), r.any_char())), // Escape sequence
                r.sequence((
                    r.not_followed_by(r.choice((r.char('"'), r.char('\\'), r.char('\n')))),
                    r.any_char(),
                )),
            ))),
            r.char('"'),
        )),
        // Single-quoted
        r.sequence((
            r.char('\''),
            r.zero_or_more(r.choice((
                r.sequence((r.char('\\'), r.any_char())), // Escape sequence
                r.sequence((
                    r.not_followed_by(r.choice((r.char('\''), r.char('\\'), r.char('\n')))),
                    r.any_char(),
                )),
            ))),
            r.char('\''),
        )),
    )))
}

// Template strings
fn rule_template_string(r: &RuleBuilder) -> Combinator {
    r.capture(r.sequence((
        r.char('`'),
        r.zero_or_more(r.choice((
            r.sequence((r.char('\\'), r.any_char())),
            r.sequence((
                r.not_followed_by(r.choice((r.char('`'), r.char('\\'), r.lit("${")))),
                r.any_char(),
            )),
        ))),
        r.char('`'),
    )))
}

fn rule_template_head(r: &RuleBuilder) -> Combinator {
    r.capture(r.sequence((
        r.char('`'),
        r.zero_or_more(r.choice((
            r.sequence((r.char('\\'), r.any_char())),
            r.sequence((
                r.not_followed_by(r.choice((r.char('`'), r.char('\\'), r.lit("${")))),
                r.any_char(),
            )),
        ))),
        r.lit("${"),
    )))
}

fn rule_template_middle(r: &RuleBuilder) -> Combinator {
    r.capture(r.sequence((
        r.char('}'),
        r.zero_or_more(r.choice((
            r.sequence((r.char('\\'), r.any_char())),
            r.sequence((
                r.not_followed_by(r.choice((r.char('`'), r.char('\\'), r.lit("${")))),
                r.any_char(),
            )),
        ))),
        r.lit("${"),
    )))
}

fn rule_template_tail(r: &RuleBuilder) -> Combinator {
    r.capture(r.sequence((
        r.char('}'),
        r.zero_or_more(r.choice((
            r.sequence((r.char('\\'), r.any_char())),
            r.sequence((
                r.not_followed_by(r.choice((r.char('`'), r.char('\\'), r.lit("${")))),
                r.any_char(),
            )),
        ))),
        r.char('`'),
    )))
}

// === Helper: Keyword matching ===
// Keywords must not be followed by identifier continuation characters

fn kw(r: &RuleBuilder, keyword: &str) -> Combinator {
    r.sequence((
        r.lit(keyword),
        r.not_followed_by(r.ident_cont()),
        r.parse("ws"),
    ))
}

fn op(r: &RuleBuilder, operator: &str) -> Combinator {
    r.sequence((r.lit(operator), r.parse("ws")))
}

/// Create an infix operator pattern with leading whitespace
/// Used in Pratt parsing: after operand consumes trailing ws, infix needs leading ws
fn ws_infix(r: &RuleBuilder, operator: &str) -> Combinator {
    r.sequence((r.parse("ws"), r.lit(operator)))
}

// === Rule Functions ===

fn rule_program(r: &RuleBuilder) -> Combinator {
    r.zero_or_more(r.parse("statement"))
        .ast("|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
            let items = result.into_list();
            let mut statements = Vec::new();
            for item in items {
                statements.push(to_stmt(item)?);
            }
            Ok(ParseResult::Prog(Program {
                body: Rc::from(statements),
                source_type: SourceType::Module,
            }))
        }")
}

fn rule_statement(r: &RuleBuilder) -> Combinator {
    // Each sub-rule should return ParseResult::Stmt, so we just pass through
    r.choice((
        r.parse("variable_declaration"),
        r.parse("function_declaration"),
        r.parse("class_declaration"),
        r.parse("if_statement"),
        r.parse("for_statement"),
        r.parse("for_in_statement"),
        r.parse("for_of_statement"),
        r.parse("while_statement"),
        r.parse("do_while_statement"),
        r.parse("switch_statement"),
        r.parse("try_statement"),
        r.choice((
            r.parse("block_statement"),
            r.parse("return_statement"),
            r.parse("break_statement"),
            r.parse("continue_statement"),
            r.parse("throw_statement"),
            r.parse("debugger_statement"),
            r.parse("import_declaration"),
            r.parse("export_declaration"),
            r.parse("type_alias_declaration"),
            r.parse("interface_declaration"),
            r.parse("enum_declaration"),
            r.choice((
                r.parse("namespace_declaration"),
                r.parse("labeled_statement"),
                r.parse("expression_statement"),
                r.parse("empty_statement"),
            )),
        )),
    ))
}

fn rule_variable_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.capture(r.choice((kw(r, "let"), kw(r, "const"), kw(r, "var")))),
        r.separated_by(r.parse("variable_declarator"), op(r, ",")),
        r.parse("semicolon"),
    ))
    .ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        let mut items = result.into_list().into_iter();
        let kind_text = items.next().ok_or_else(|| ParseError::new(\"Expected variable kind\".to_string(), 0, 0))?.into_text();
        let kind = match kind_text.as_ref() {
            \"let\" => VariableKind::Let,
            \"const\" => VariableKind::Const,
            \"var\" => VariableKind::Var,
            _ => VariableKind::Let,
        };
        // Get declarators from the separated_by result
        let decl_list = items.next().ok_or_else(|| ParseError::new(\"Expected declarators\".to_string(), 0, 0))?;
        let decl_items = decl_list.into_list();
        let mut declarations = Vec::new();
        for item in decl_items {
            if let ParseResult::List(parts) = item {
                // Each declarator is [pattern, optional_type, optional_init]
                let mut parts_iter = parts.into_iter();
                let pattern = to_pattern(parts_iter.next().unwrap_or(ParseResult::None))?;
                // Skip type annotation for now (types are stripped at runtime)
                let _type_ann_result = parts_iter.next();
                let init = parts_iter.next().and_then(|r| {
                    match r {
                        ParseResult::List(init_parts) => {
                            init_parts.into_iter().nth(1).and_then(|e| to_expr(e).ok()).map(Rc::new)
                        }
                        _ => None,
                    }
                });
                declarations.push(VariableDeclarator {
                    id: pattern,
                    type_annotation: None,
                    init,
                    span: span.clone(),
                });
            }
        }
        Ok(ParseResult::Stmt(Statement::VariableDeclaration(VariableDeclaration {
            kind,
            declarations: Rc::from(declarations),
            span,
        })))
    }")
}

fn rule_variable_declarator(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("pattern"),
        r.optional(r.parse("type_annotation")),
        r.optional(r.sequence((op(r, "="), r.parse("expression")))),
    ))
}

fn rule_variable_declaration_no_semi(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.choice((kw(r, "let"), kw(r, "const"), kw(r, "var"))),
        r.separated_by(r.parse("variable_declarator"), op(r, ",")),
    ))
}

fn rule_function_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(kw(r, "async")),
        kw(r, "function"),
        r.optional(op(r, "*")),
        r.optional(r.parse("identifier")),
        r.optional(r.parse("type_parameters")),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        r.optional(r.parse("type_annotation")),
        r.parse("block_statement"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [async?, function, *?, name?, type_params?, (, params?, ), return_type?, body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let async_kw = iter.next().unwrap_or(ParseResult::None);
            let _function_kw = iter.next(); // skip 'function'
            let generator = iter.next().unwrap_or(ParseResult::None);
            let name = iter.next().unwrap_or(ParseResult::None);
            let _type_params = iter.next(); // skip type params
            let _open_paren = iter.next(); // skip (
            let params = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next(); // skip )
            let _return_type = iter.next(); // skip return type
            let body = iter.next().unwrap_or(ParseResult::None);
            create_function_decl(async_kw, generator, name, params, body, span)
        } else {
            Err(ParseError::new(\"Expected function declaration parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_class_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(kw(r, "abstract")),
        kw(r, "class"),
        r.optional(r.parse("identifier")),
        r.optional(r.parse("type_parameters")),
        r.optional(r.sequence((kw(r, "extends"), r.parse("expression")))),
        r.optional(r.sequence((
            kw(r, "implements"),
            r.separated_by(r.parse("type_reference"), op(r, ",")),
        ))),
        r.parse("class_body"),
    ))
}

fn rule_class_body(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.zero_or_more(r.parse("class_member")),
        op(r, "}"),
    ))
}

fn rule_class_member(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("class_constructor"),
        r.parse("class_method"),
        r.parse("class_property"),
        r.parse("static_block"),
    ))
}

fn rule_class_constructor(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.parse("accessibility_modifier")),
        kw(r, "constructor"),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        r.parse("block_statement"),
    ))
}

fn rule_class_method(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(r.parse("accessibility_modifier")),
        r.optional(kw(r, "static")),
        r.optional(kw(r, "async")),
        r.optional(op(r, "*")),
        r.optional(r.choice((kw(r, "get"), kw(r, "set")))),
        r.parse("property_key"),
        r.optional(r.parse("type_parameters")),
        r.sequence((
            op(r, "("),
            r.optional(r.parse("parameter_list")),
            op(r, ")"),
            r.optional(r.parse("type_annotation")),
            r.parse("block_statement"),
        )),
    ))
}

fn rule_class_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(r.parse("accessibility_modifier")),
        r.optional(kw(r, "static")),
        r.optional(kw(r, "readonly")),
        r.optional(kw(r, "accessor")),
        r.parse("property_key"),
        r.optional(op(r, "?")),
        r.optional(r.parse("type_annotation")),
        r.optional(r.sequence((op(r, "="), r.parse("expression")))),
        r.parse("semicolon"),
    ))
}

fn rule_static_block(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "static"), r.parse("block_statement")))
}

fn rule_if_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "if"),
        op(r, "("),
        r.parse("expression"),
        op(r, ")"),
        r.parse("statement"),
        r.optional(r.sequence((kw(r, "else"), r.parse("statement")))),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [if, (, test, ), consequent, else_clause?]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _if_kw = iter.next();
            let _open_paren = iter.next();
            let test = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next();
            let consequent = iter.next().unwrap_or(ParseResult::None);
            let else_clause = iter.next();

            let alternate = match else_clause {
                Some(ParseResult::List(else_parts)) => {
                    // [else, statement]
                    else_parts.into_iter().nth(1).map(|s| {
                        match s {
                            ParseResult::Stmt(stmt) => Rc::new(stmt),
                            _ => Rc::new(Statement::Empty),
                        }
                    })
                }
                _ => None,
            };

            let consequent_stmt = match consequent {
                ParseResult::Stmt(s) => Rc::new(s),
                _ => return Err(ParseError::new(\"Expected statement\".to_string(), 0, 0)),
            };

            Ok(ParseResult::Stmt(Statement::If(IfStatement {
                test: Rc::new(to_expr(test)?),
                consequent: consequent_stmt,
                alternate,
                span,
            })))
        } else {
            Err(ParseError::new(\"Expected if statement parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_for_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "for"),
        op(r, "("),
        r.optional(r.parse("for_init")),
        op(r, ";"),
        r.optional(r.parse("expression")),
        op(r, ";"),
        r.optional(r.parse("expression")),
        op(r, ")"),
        r.parse("statement"),
    ))
}

fn rule_for_init(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("variable_declaration_no_semi"),
        r.parse("expression"),
    ))
}

fn rule_for_in_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "for"),
        op(r, "("),
        r.parse("for_in_of_left"),
        kw(r, "in"),
        r.parse("expression"),
        op(r, ")"),
        r.parse("statement"),
    ))
}

fn rule_for_of_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "for"),
        r.optional(kw(r, "await")),
        op(r, "("),
        r.parse("for_in_of_left"),
        kw(r, "of"),
        r.parse("expression"),
        op(r, ")"),
        r.parse("statement"),
    ))
}

fn rule_for_in_of_left(r: &RuleBuilder) -> Combinator {
    r.choice((r.parse("variable_declaration_no_semi"), r.parse("pattern")))
}

fn rule_while_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "while"),
        op(r, "("),
        r.parse("expression"),
        op(r, ")"),
        r.parse("statement"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [while, (, test, ), body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _while_kw = iter.next();
            let _open_paren = iter.next();
            let test = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);

            let body_stmt = match body {
                ParseResult::Stmt(s) => Rc::new(s),
                _ => return Err(ParseError::new(\"Expected statement\".to_string(), 0, 0)),
            };

            Ok(ParseResult::Stmt(Statement::While(WhileStatement {
                test: Rc::new(to_expr(test)?),
                body: body_stmt,
                span,
            })))
        } else {
            Err(ParseError::new(\"Expected while statement parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_do_while_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "do"),
        r.parse("statement"),
        kw(r, "while"),
        op(r, "("),
        r.parse("expression"),
        op(r, ")"),
        r.parse("semicolon"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [do, body, while, (, test, ), ;]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _do_kw = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);
            let _while_kw = iter.next();
            let _open_paren = iter.next();
            let test = iter.next().unwrap_or(ParseResult::None);

            let body_stmt = match body {
                ParseResult::Stmt(s) => Rc::new(s),
                _ => return Err(ParseError::new(\"Expected statement\".to_string(), 0, 0)),
            };

            Ok(ParseResult::Stmt(Statement::DoWhile(DoWhileStatement {
                body: body_stmt,
                test: Rc::new(to_expr(test)?),
                span,
            })))
        } else {
            Err(ParseError::new(\"Expected do-while statement parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_switch_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "switch"),
        op(r, "("),
        r.parse("expression"),
        op(r, ")"),
        op(r, "{"),
        r.zero_or_more(r.parse("switch_case")),
        op(r, "}"),
    ))
}

fn rule_switch_case(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((
            kw(r, "case"),
            r.parse("expression"),
            op(r, ":"),
            r.zero_or_more(r.parse("statement")),
        )),
        r.sequence((
            kw(r, "default"),
            op(r, ":"),
            r.zero_or_more(r.parse("statement")),
        )),
    ))
}

fn rule_try_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "try"),
        r.parse("block_statement"),
        r.optional(r.parse("catch_clause")),
        r.optional(r.sequence((kw(r, "finally"), r.parse("block_statement")))),
    ))
}

fn rule_catch_clause(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "catch"),
        r.optional(r.sequence((op(r, "("), r.parse("pattern"), op(r, ")")))),
        r.parse("block_statement"),
    ))
}

fn rule_block_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.zero_or_more(r.parse("statement")),
        op(r, "}"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        create_block_stmt(result, span)
    }")
}

fn rule_return_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "return"),
        r.optional(r.parse("expression")),
        r.parse("semicolon"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _return_kw = iter.next(); // skip 'return'
            let expr = iter.next().unwrap_or(ParseResult::None);
            let argument = match expr {
                ParseResult::None => None,
                other => Some(Rc::new(to_expr(other)?)),
            };
            Ok(ParseResult::Stmt(Statement::Return(ReturnStatement { argument, span })))
        } else {
            Err(ParseError::new(\"Expected return statement parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_break_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "break"),
        r.optional(r.parse("identifier")),
        r.parse("semicolon"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _break_kw = iter.next();
            let label = iter.next().and_then(|l| match l {
                ParseResult::Ident(id) => Some(id),
                _ => None,
            });
            Ok(ParseResult::Stmt(Statement::Break(BreakStatement { label, span })))
        } else {
            Err(ParseError::new(\"Expected break statement parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_continue_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "continue"),
        r.optional(r.parse("identifier")),
        r.parse("semicolon"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _continue_kw = iter.next();
            let label = iter.next().and_then(|l| match l {
                ParseResult::Ident(id) => Some(id),
                _ => None,
            });
            Ok(ParseResult::Stmt(Statement::Continue(ContinueStatement { label, span })))
        } else {
            Err(ParseError::new(\"Expected continue statement parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_throw_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "throw"),
        r.parse("expression"),
        r.parse("semicolon"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _throw_kw = iter.next();
            let argument = iter.next().unwrap_or(ParseResult::None);
            Ok(ParseResult::Stmt(Statement::Throw(ThrowStatement {
                argument: Rc::new(to_expr(argument)?),
                span,
            })))
        } else {
            Err(ParseError::new(\"Expected throw statement parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_debugger_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "debugger"), r.parse("semicolon")))
        .ast("|_result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
            Ok(ParseResult::Stmt(Statement::Debugger))
        }")
}

fn rule_labeled_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        op(r, ":"),
        r.parse("statement"),
    ))
}

fn rule_expression_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((r.parse("expression"), r.optional(r.parse("semicolon"))))
        .ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            let items = result.into_list();
            let expr = to_expr(items.into_iter().next().ok_or_else(|| ParseError::new(\"Expected expression\".to_string(), 0, 0))?)?;
            Ok(ParseResult::Stmt(Statement::Expression(ExpressionStatement {
                expression: Rc::new(expr),
                span,
            })))
        }")
}

fn rule_empty_statement(r: &RuleBuilder) -> Combinator {
    op(r, ";")
}

fn rule_semicolon(r: &RuleBuilder) -> Combinator {
    op(r, ";")
}

// Import/Export

fn rule_import_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "import"),
        r.optional(kw(r, "type")),
        r.parse("import_clause"),
        kw(r, "from"),
        r.parse("string_literal"),
        r.parse("ws"),
        r.parse("semicolon"),
    ))
}

fn rule_import_clause(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((op(r, "*"), kw(r, "as"), r.parse("identifier"))),
        r.parse("named_imports"),
        r.sequence((
            r.parse("identifier"),
            r.optional(r.sequence((op(r, ","), r.parse("import_clause_rest")))),
        )),
    ))
}

fn rule_import_clause_rest(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((op(r, "*"), kw(r, "as"), r.parse("identifier"))),
        r.parse("named_imports"),
    ))
}

fn rule_named_imports(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.optional(r.separated_by_trailing(r.parse("import_specifier"), op(r, ","))),
        op(r, "}"),
    ))
}

fn rule_import_specifier(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        r.optional(r.sequence((kw(r, "as"), r.parse("identifier")))),
    ))
}

fn rule_export_declaration(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((
            kw(r, "export"),
            kw(r, "default"),
            r.parse("export_default_expression"),
        )),
        r.sequence((
            kw(r, "export"),
            r.optional(kw(r, "type")),
            r.parse("named_exports"),
            r.optional(r.sequence((kw(r, "from"), r.parse("string_literal"), r.parse("ws")))),
            r.parse("semicolon"),
        )),
        r.sequence((
            kw(r, "export"),
            op(r, "*"),
            r.optional(r.sequence((kw(r, "as"), r.parse("identifier")))),
            kw(r, "from"),
            r.parse("string_literal"),
            r.parse("ws"),
            r.parse("semicolon"),
        )),
        r.sequence((
            kw(r, "export"),
            r.optional(kw(r, "type")),
            r.parse("exportable_declaration"),
        )),
    ))
}

fn rule_export_default_expression(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("function_declaration"),
        r.parse("class_declaration"),
        r.sequence((r.parse("expression"), r.parse("semicolon"))),
    ))
}

fn rule_named_exports(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.optional(r.separated_by_trailing(r.parse("export_specifier"), op(r, ","))),
        op(r, "}"),
    ))
}

fn rule_export_specifier(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        r.optional(r.sequence((kw(r, "as"), r.parse("identifier")))),
    ))
}

fn rule_exportable_declaration(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("variable_declaration"),
        r.parse("function_declaration"),
        r.parse("class_declaration"),
        r.parse("type_alias_declaration"),
        r.parse("interface_declaration"),
        r.parse("enum_declaration"),
    ))
}

// TypeScript Declarations

fn rule_type_alias_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "type"),
        r.parse("identifier"),
        r.optional(r.parse("type_parameters")),
        op(r, "="),
        r.parse("type"),
        r.parse("semicolon"),
    ))
}

fn rule_interface_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "interface"),
        r.parse("identifier"),
        r.optional(r.parse("type_parameters")),
        r.optional(r.sequence((
            kw(r, "extends"),
            r.separated_by(r.parse("type_reference"), op(r, ",")),
        ))),
        r.parse("object_type"),
    ))
}

fn rule_enum_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(kw(r, "const")),
        kw(r, "enum"),
        r.parse("identifier"),
        op(r, "{"),
        r.optional(r.separated_by_trailing(r.parse("enum_member"), op(r, ","))),
        op(r, "}"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [const?, enum, name, {, members?, }]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let const_kw = iter.next().unwrap_or(ParseResult::None);
            let _enum_kw = iter.next(); // skip 'enum'
            let name = iter.next().unwrap_or(ParseResult::None);
            let _open_brace = iter.next(); // skip {
            let members = iter.next().unwrap_or(ParseResult::None);
            let _close_brace = iter.next(); // skip }
            create_enum_decl(const_kw, name, members, span)
        } else {
            Err(ParseError::new(\"Expected enum declaration parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_enum_member(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        r.optional(r.sequence((op(r, "="), r.parse("expression")))),
    ))
}

fn rule_namespace_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "namespace"),
        r.parse("identifier"),
        r.parse("block_statement"),
    ))
}

// Expressions

fn rule_expression(r: &RuleBuilder) -> Combinator {
    // Pratt parsing for expressions, followed by optional `as Type` suffixes
    // Infix operators use leading ws pattern: after operand consumes trailing ws,
    // we need to consume any ws before the operator before parsing right operand
    r.sequence((
        r.pratt(r.parse("primary"), |ops| {
            ops
                // === Assignment operators (lowest precedence, right-associative) ===
                .infix(ws_infix(r, ">>>="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::URShiftAssign, s)")
                .infix(ws_infix(r, "**="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::ExpAssign, s)")
                .infix(ws_infix(r, "<<="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::LShiftAssign, s)")
                .infix(ws_infix(r, ">>="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::RShiftAssign, s)")
                .infix(ws_infix(r, "&&="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::AndAssign, s)")
                .infix(ws_infix(r, "||="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::OrAssign, s)")
                .infix(ws_infix(r, "??="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::NullishAssign, s)")
                .infix(ws_infix(r, "+="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::AddAssign, s)")
                .infix(ws_infix(r, "-="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::SubAssign, s)")
                .infix(ws_infix(r, "*="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::MulAssign, s)")
                .infix(ws_infix(r, "/="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::DivAssign, s)")
                .infix(ws_infix(r, "%="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::ModAssign, s)")
                .infix(ws_infix(r, "&="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::BitAndAssign, s)")
                .infix(ws_infix(r, "|="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::BitOrAssign, s)")
                .infix(ws_infix(r, "^="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::BitXorAssign, s)")
                .infix(ws_infix(r, "="), 2, Assoc::Right, "|l, r, s| assign(l, r, AssignmentOp::Assign, s)")
                // === Ternary operator ===
                .ternary("?", ":", 3, "|c, t, e, s| conditional(c, t, e, s)")
                // === Nullish coalescing ===
                .infix(ws_infix(r, "??"), 4, Assoc::Left, "|l, r, s| logical(l, r, LogicalOp::NullishCoalescing, s)")
                // === Logical OR ===
                .infix(ws_infix(r, "||"), 5, Assoc::Left, "|l, r, s| logical(l, r, LogicalOp::Or, s)")
                // === Logical AND ===
                .infix(ws_infix(r, "&&"), 6, Assoc::Left, "|l, r, s| logical(l, r, LogicalOp::And, s)")
                // === Bitwise OR ===
                .infix(ws_infix(r, "|"), 7, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::BitOr, s)")
                // === Bitwise XOR ===
                .infix(ws_infix(r, "^"), 8, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::BitXor, s)")
                // === Bitwise AND ===
                .infix(ws_infix(r, "&"), 9, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::BitAnd, s)")
                // === Equality ===
                .infix(ws_infix(r, "==="), 10, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::StrictEq, s)")
                .infix(ws_infix(r, "!=="), 10, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::StrictNotEq, s)")
                .infix(ws_infix(r, "=="), 10, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Eq, s)")
                .infix(ws_infix(r, "!="), 10, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::NotEq, s)")
                // === Relational ===
                .infix(ws_infix(r, "<="), 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::LtEq, s)")
                .infix(ws_infix(r, ">="), 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::GtEq, s)")
                .infix(ws_infix(r, "<"), 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Lt, s)")
                .infix(ws_infix(r, ">"), 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Gt, s)")
                // Keyword operators with ws + keyword + not_followed_by(ident_cont)
                .infix(r.sequence((r.parse("ws"), r.lit("in"), r.not_followed_by(r.ident_cont()))), 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::In, s)")
                .infix(r.sequence((r.parse("ws"), r.lit("instanceof"), r.not_followed_by(r.ident_cont()))), 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Instanceof, s)")
                // === Shift ===
                .infix(ws_infix(r, ">>>"), 12, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::URShift, s)")
                .infix(ws_infix(r, "<<"), 12, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::LShift, s)")
                .infix(ws_infix(r, ">>"), 12, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::RShift, s)")
                // === Additive ===
                .infix(ws_infix(r, "+"), 13, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Add, s)")
                .infix(ws_infix(r, "-"), 13, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Sub, s)")
                // === Multiplicative ===
                .infix(ws_infix(r, "*"), 14, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Mul, s)")
                .infix(ws_infix(r, "/"), 14, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Div, s)")
                .infix(ws_infix(r, "%"), 14, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Mod, s)")
                // === Exponentiation (right-to-left) ===
                .infix(ws_infix(r, "**"), 15, Assoc::Right, "|l, r, s| binary(l, r, BinaryOp::Exp, s)")
                // === Prefix operators ===
                .prefix("++", 16, "|e, s| update(e, UpdateOp::Increment, true, s)")
                .prefix("--", 16, "|e, s| update(e, UpdateOp::Decrement, true, s)")
                .prefix("-", 16, "|e, s| unary(e, UnaryOp::Minus, s)")
                .prefix("+", 16, "|e, s| unary(e, UnaryOp::Plus, s)")
                .prefix("!", 16, "|e, s| unary(e, UnaryOp::Not, s)")
                .prefix("~", 16, "|e, s| unary(e, UnaryOp::BitNot, s)")
                .prefix_kw("typeof", 16, "|e, s| unary(e, UnaryOp::Typeof, s)")
                .prefix_kw("void", 16, "|e, s| unary(e, UnaryOp::Void, s)")
                .prefix_kw("delete", 16, "|e, s| unary(e, UnaryOp::Delete, s)")
                .prefix_kw("await", 16, "|e, s| await_expr(e, s)")
                // === Postfix operators (highest precedence) ===
                .postfix("++", 17, "|e, s| update(e, UpdateOp::Increment, false, s)")
                .postfix("--", 17, "|e, s| update(e, UpdateOp::Decrement, false, s)")
                // Call expressions (optional chaining first to match longer pattern)
                .postfix_call("?.(", ")", ",", 18, "|c, a, s| call(c, a, true, s)")
                .postfix_call("(", ")", ",", 18, "|c, a, s| call(c, a, false, s)")
                // Member access (optional chaining first to match longer pattern)
                .postfix_member("?.", 18, "|o, p, s| member(o, p, true, s)")
                .postfix_member(".", 18, "|o, p, s| member(o, p, false, s)")
                // Computed member (optional chaining first to match longer pattern)
                .postfix_index("?.[", "]", 18, "|o, e, s| member_computed(o, e, true, s)")
                .postfix_index("[", "]", 18, "|o, e, s| member_computed(o, e, false, s)")
        }),
        // Optional `as Type` suffixes (TypeScript type assertions)
        r.zero_or_more(r.sequence((
            r.parse("ws"),
            r.lit("as"),
            r.not_followed_by(r.ident_cont()),
            r.parse("ws"),
            r.parse("type"),
        ))),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // Extract the expression and as-clause count
        if let ParseResult::List(items) = result {
            let mut iter = items.into_iter();
            let expr = iter.next().unwrap_or(ParseResult::None);
            let as_clauses = iter.next();

            // If there are as clauses, wrap in TypeAssertion(s)
            if let Some(ParseResult::List(clauses)) = as_clauses {
                if clauses.is_empty() {
                    return Ok(expr);
                }
                // Wrap expression in TypeAssertion for each as clause
                let mut current = expr;
                for _ in clauses {
                    current = type_assertion(current, span.clone())?;
                }
                Ok(current)
            } else {
                Ok(expr)
            }
        } else {
            Ok(result)
        }
    }")
}

fn rule_primary(r: &RuleBuilder) -> Combinator {
    // Start with ws to handle whitespace after infix operators
    r.sequence((
        r.parse("ws"),
        r.parse("primary_inner"),
    )).ast("|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
        // Return the second element (primary_inner), skip the ws
        if let ParseResult::List(mut items) = result {
            Ok(items.pop().unwrap_or(ParseResult::None))
        } else {
            Ok(result)
        }
    }")
}

fn rule_primary_inner(r: &RuleBuilder) -> Combinator {
    r.choice(vec![
        r.parse("literal"),
        r.parse("this_expression"),
        r.parse("super_expression"),
        r.parse("array_expression"),
        r.parse("object_expression"),
        r.parse("function_expression"),
        r.parse("class_expression"),
        r.parse("template_literal"),
        r.parse("new_expression"),
        r.parse("yield_expression"),
        // arrow_function before parenthesized - both start with ( but arrow needs => lookahead
        // Arrow function will fail if no => follows, then parenthesized will match
        r.parse("arrow_function"),
        r.parse("parenthesized"),
        // identifier last since it matches most things
        r.parse("identifier").ast("|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
            let ident = to_ident(result)?;
            Ok(ParseResult::Expr(Expression::Identifier(ident)))
        }"),
    ])
}

fn rule_literal(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((r.parse("number_literal"), r.parse("ws"))).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            let text = result.into_text();
            let value = parse_number(&text);
            Ok(ParseResult::Expr(Expression::Literal(Box::new(Literal { value: LiteralValue::Number(value), span }))))
        }"),
        r.sequence((r.parse("string_literal"), r.parse("ws"))).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            let text = result.into_text();
            let value = parse_string_literal(&text);
            Ok(ParseResult::Expr(Expression::Literal(Box::new(Literal { value: LiteralValue::String(value), span }))))
        }"),
        kw(r, "true").ast("|_: ParseResult, span: Span| -> Result<ParseResult, ParseError> { Ok(ParseResult::Expr(Expression::Literal(Box::new(Literal { value: LiteralValue::Boolean(true), span })))) }"),
        kw(r, "false").ast("|_: ParseResult, span: Span| -> Result<ParseResult, ParseError> { Ok(ParseResult::Expr(Expression::Literal(Box::new(Literal { value: LiteralValue::Boolean(false), span })))) }"),
        kw(r, "null").ast("|_: ParseResult, span: Span| -> Result<ParseResult, ParseError> { Ok(ParseResult::Expr(Expression::Literal(Box::new(Literal { value: LiteralValue::Null, span })))) }"),
        kw(r, "undefined").ast("|_: ParseResult, span: Span| -> Result<ParseResult, ParseError> { Ok(ParseResult::Expr(Expression::Literal(Box::new(Literal { value: LiteralValue::Undefined, span })))) }"),
    ))
}

fn rule_identifier(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.capture(r.sequence((
            r.ident_start(),
            r.zero_or_more(r.ident_cont()),
        ))),
        r.parse("ws"),
    ))
    .ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> { Ok(ParseResult::Ident(Identifier { name: result.into_text(), span })) }")
}

fn rule_this_expression(r: &RuleBuilder) -> Combinator {
    kw(r, "this")
}

fn rule_super_expression(r: &RuleBuilder) -> Combinator {
    kw(r, "super")
}

fn rule_array_expression(r: &RuleBuilder) -> Combinator {
    // Sparse arrays like [1, , 3] have holes (elisions) where commas appear consecutively
    // We handle this by making elements optional between commas
    r.sequence((
        op(r, "["),
        r.optional(r.sequence((
            r.optional(r.parse("array_element")), // First element (may be elided)
            r.zero_or_more(r.sequence((
                op(r, ","),
                r.optional(r.parse("array_element")), // Subsequent elements (may be elided)
            ))),
        ))),
        op(r, "]"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        create_array_expr(result, span)
    }")
}

fn rule_array_element(r: &RuleBuilder) -> Combinator {
    r.choice((r.parse("spread_element"), r.parse("expression")))
}

fn rule_object_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.optional(r.separated_by_trailing(r.parse("object_property"), op(r, ","))),
        op(r, "}"),
    ))
}

fn rule_object_property(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("spread_element"),
        r.parse("method_property"),
        r.parse("getter_property"),
        r.parse("setter_property"),
        r.parse("key_value_property"),
        r.parse("shorthand_property"),
    ))
}

fn rule_key_value_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("property_key"),
        op(r, ":"),
        r.parse("expression"),
    ))
}

fn rule_shorthand_property(r: &RuleBuilder) -> Combinator {
    r.parse("identifier")
}

fn rule_method_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(kw(r, "async")),
        r.optional(op(r, "*")),
        r.parse("property_key"),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        r.parse("block_statement"),
    ))
}

fn rule_getter_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "get"),
        r.parse("property_key"),
        op(r, "("),
        op(r, ")"),
        r.parse("block_statement"),
    ))
}

fn rule_setter_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "set"),
        r.parse("property_key"),
        op(r, "("),
        r.parse("pattern"),
        op(r, ")"),
        r.parse("block_statement"),
    ))
}

fn rule_property_key(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("identifier"),
        r.sequence((r.parse("string_literal"), r.parse("ws"))),
        r.sequence((r.parse("number_literal"), r.parse("ws"))),
        r.sequence((
            op(r, "["),
            r.parse("expression"),
            op(r, "]"),
        )),
    ))
}

fn rule_spread_element(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, "..."), r.parse("expression")))
}

fn rule_function_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(kw(r, "async")),
        kw(r, "function"),
        r.optional(op(r, "*")),
        r.optional(r.parse("identifier")),
        r.optional(r.parse("type_parameters")),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        r.optional(r.parse("type_annotation")),
        r.parse("block_statement"),
    ))
}

fn rule_arrow_function(r: &RuleBuilder) -> Combinator {
    r.choice((
        // Parenthesized params: (a, b) => body
        r.sequence((
            r.optional(kw(r, "async")),
            r.optional(r.parse("type_parameters")),
            op(r, "("),
            r.optional(r.parse("parameter_list")),
            op(r, ")"),
            r.optional(r.parse("type_annotation")),
            op(r, "=>"),
            r.parse("arrow_body"),
        )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            // [async?, type_params?, (, params?, ), return_type?, =>, body]
            if let ParseResult::List(items) = result {
                let mut iter = items.into_iter();
                let async_kw = iter.next().unwrap_or(ParseResult::None);
                let _type_params = iter.next(); // skip type params
                let _open_paren = iter.next();  // skip (
                let params = iter.next().unwrap_or(ParseResult::None);
                let _close_paren = iter.next(); // skip )
                let _return_type = iter.next(); // skip return type
                let _arrow = iter.next();       // skip =>
                let body = iter.next().unwrap_or(ParseResult::None);
                arrow_function_parens(async_kw, params, body, span)
            } else {
                Err(ParseError::new(\"Expected arrow function parts\".to_string(), 0, 0))
            }
        }"),
        // Single unparenthesized param: x => body
        r.sequence((
            r.optional(kw(r, "async")),
            r.parse("identifier"),
            op(r, "=>"),
            r.parse("arrow_body"),
        )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            // [async?, identifier, =>, body]
            if let ParseResult::List(items) = result {
                let mut iter = items.into_iter();
                let async_kw = iter.next().unwrap_or(ParseResult::None);
                let param = iter.next().unwrap_or(ParseResult::None);
                let _arrow = iter.next(); // skip =>
                let body = iter.next().unwrap_or(ParseResult::None);
                arrow_function_single(async_kw, param, body, span)
            } else {
                Err(ParseError::new(\"Expected arrow function parts\".to_string(), 0, 0))
            }
        }"),
    ))
}

fn rule_arrow_body(r: &RuleBuilder) -> Combinator {
    r.choice((r.parse("block_statement"), r.parse("expression")))
}

fn rule_class_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "class"),
        r.optional(r.parse("identifier")),
        r.optional(r.parse("type_parameters")),
        r.optional(r.sequence((kw(r, "extends"), r.parse("expression")))),
        r.optional(r.sequence((
            kw(r, "implements"),
            r.separated_by(r.parse("type_reference"), op(r, ",")),
        ))),
        r.parse("class_body"),
    ))
}

fn rule_template_literal(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((r.parse("template_string"), r.parse("ws"))),
        r.sequence((
            r.parse("template_head"),
            r.parse("ws"),
            r.parse("expression"),
            r.zero_or_more(r.sequence((r.parse("template_middle"), r.parse("ws"), r.parse("expression")))),
            r.parse("template_tail"),
            r.parse("ws"),
        )),
    ))
}

fn rule_new_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "new"),
        r.parse("expression"),
        r.optional(r.sequence((
            op(r, "("),
            r.optional(r.parse("argument_list")),
            op(r, ")"),
        ))),
    ))
}

fn rule_yield_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "yield"),
        r.optional(op(r, "*")),
        r.optional(r.parse("expression")),
    ))
}

fn rule_parenthesized(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, "("), r.parse("expression"), op(r, ")")))
        .ast("|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
            // Extract expression from sequence [open_paren, expr, close_paren]
            if let ParseResult::List(mut items) = result {
                Ok(items.remove(1))
            } else {
                Ok(result)
            }
        }")
}

fn rule_argument_list(r: &RuleBuilder) -> Combinator {
    r.separated_by(r.parse("argument"), op(r, ","))
}

fn rule_argument(r: &RuleBuilder) -> Combinator {
    r.choice((r.parse("spread_element"), r.parse("expression")))
}

// Parameters

fn rule_parameter_list(r: &RuleBuilder) -> Combinator {
    r.separated_by_trailing(r.parse("parameter"), op(r, ","))
}

fn rule_parameter(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(r.parse("accessibility_modifier")),
        r.optional(kw(r, "readonly")),
        r.parse("pattern"),
        r.optional(op(r, "?")),
        r.optional(r.parse("type_annotation")),
        r.optional(r.sequence((op(r, "="), r.parse("expression")))),
    ))
}

fn rule_accessibility_modifier(r: &RuleBuilder) -> Combinator {
    r.choice((kw(r, "public"), kw(r, "private"), kw(r, "protected")))
}

fn rule_decorator(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, "@"), r.parse("expression")))
}

// Patterns

fn rule_pattern(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("identifier_pattern"),
        r.parse("object_pattern"),
        r.parse("array_pattern"),
        r.parse("rest_pattern"),
    ))
}

fn rule_identifier_pattern(r: &RuleBuilder) -> Combinator {
    r.parse("identifier")
}

fn rule_object_pattern(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.optional(r.separated_by_trailing(r.parse("object_pattern_property"), op(r, ","))),
        op(r, "}"),
    ))
}

fn rule_object_pattern_property(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("rest_pattern"),
        r.sequence((
            r.parse("property_key"),
            r.optional(r.sequence((op(r, ":"), r.parse("pattern")))),
            r.optional(r.sequence((op(r, "="), r.parse("expression")))),
        )),
    ))
}

fn rule_array_pattern(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "["),
        r.optional(r.separated_by_trailing(
            r.optional(r.parse("array_pattern_element")),
            op(r, ","),
        )),
        op(r, "]"),
    ))
}

fn rule_array_pattern_element(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("pattern"),
        r.optional(r.sequence((op(r, "="), r.parse("expression")))),
    ))
}

fn rule_rest_pattern(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, "..."), r.parse("pattern")))
}

// Type Annotations

fn rule_type_annotation(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, ":"), r.parse("type")))
}

fn rule_type(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("union_type"),
        r.parse("intersection_type"),
        r.parse("function_type"),
        r.parse("conditional_type"),
        r.parse("primary_type"),
    ))
}

fn rule_primary_type(r: &RuleBuilder) -> Combinator {
    // base_type followed by optional array suffixes []
    // This avoids left recursion (array_type was calling primary_type first)
    r.sequence((
        r.parse("base_type"),
        r.zero_or_more(r.sequence((op(r, "["), op(r, "]")))),
    ))
}

fn rule_base_type(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("keyword_type"),
        r.parse("type_reference"),
        r.parse("literal_type"),
        r.parse("object_type"),
        r.parse("tuple_type"),
        r.parse("typeof_type"),
        r.parse("keyof_type"),
        r.parse("infer_type"),
        r.parse("mapped_type"),
        r.parse("parenthesized_type"),
    ))
}

fn rule_keyword_type(r: &RuleBuilder) -> Combinator {
    r.choice((
        kw(r, "any"),
        kw(r, "unknown"),
        kw(r, "never"),
        kw(r, "void"),
        kw(r, "null"),
        kw(r, "undefined"),
        kw(r, "boolean"),
        kw(r, "number"),
        kw(r, "string"),
        kw(r, "symbol"),
        kw(r, "bigint"),
        kw(r, "object"),
    ))
}

fn rule_type_reference(r: &RuleBuilder) -> Combinator {
    r.sequence((r.parse("identifier"), r.optional(r.parse("type_arguments"))))
}

fn rule_type_arguments(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "<"),
        r.separated_by(r.parse("type"), op(r, ",")),
        op(r, ">"),
    ))
}

fn rule_type_parameters(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "<"),
        r.separated_by(r.parse("type_parameter"), op(r, ",")),
        op(r, ">"),
    ))
}

fn rule_type_parameter(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        r.optional(r.sequence((kw(r, "extends"), r.parse("type")))),
        r.optional(r.sequence((op(r, "="), r.parse("type")))),
    ))
}

fn rule_literal_type(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((r.parse("string_literal"), r.parse("ws"))),
        r.sequence((r.parse("number_literal"), r.parse("ws"))),
        kw(r, "true"),
        kw(r, "false"),
        kw(r, "null"),
    ))
}

fn rule_object_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.zero_or_more(r.parse("type_member")),
        op(r, "}"),
    ))
}

fn rule_type_member(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("property_signature"),
        r.parse("method_signature"),
        r.parse("index_signature"),
        r.parse("call_signature"),
        r.parse("construct_signature"),
    ))
}

fn rule_property_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(kw(r, "readonly")),
        r.parse("property_key"),
        r.optional(op(r, "?")),
        r.optional(r.parse("type_annotation")),
        r.parse("type_member_separator"),
    ))
}

fn rule_method_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("property_key"),
        r.optional(op(r, "?")),
        r.optional(r.parse("type_parameters")),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        r.optional(r.parse("type_annotation")),
        r.parse("type_member_separator"),
    ))
}

fn rule_index_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(kw(r, "readonly")),
        op(r, "["),
        r.parse("identifier"),
        op(r, ":"),
        r.parse("type"),
        op(r, "]"),
        op(r, ":"),
        r.parse("type"),
        r.parse("type_member_separator"),
    ))
}

fn rule_call_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.parse("type_parameters")),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        r.optional(r.parse("type_annotation")),
        r.parse("type_member_separator"),
    ))
}

fn rule_construct_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "new"),
        r.optional(r.parse("type_parameters")),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        r.optional(r.parse("type_annotation")),
        r.parse("type_member_separator"),
    ))
}

fn rule_type_member_separator(r: &RuleBuilder) -> Combinator {
    r.choice((op(r, ";"), op(r, ",")))
}

// Note: array_type is now handled inline in primary_type as postfix []
// This avoids the left recursion bug where array_type called primary_type

fn rule_tuple_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "["),
        r.optional(r.separated_by(r.parse("type"), op(r, ","))),
        op(r, "]"),
    ))
}

fn rule_union_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("primary_type"),
        r.one_or_more(r.sequence((op(r, "|"), r.parse("primary_type")))),
    ))
}

fn rule_intersection_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("primary_type"),
        r.one_or_more(r.sequence((op(r, "&"), r.parse("primary_type")))),
    ))
}

fn rule_function_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.parse("type_parameters")),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        op(r, "=>"),
        r.parse("type"),
    ))
}

fn rule_conditional_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("primary_type"),
        kw(r, "extends"),
        r.parse("type"),
        op(r, "?"),
        r.parse("type"),
        op(r, ":"),
        r.parse("type"),
    ))
}

fn rule_typeof_type(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "typeof"), r.parse("identifier")))
}

fn rule_keyof_type(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "keyof"), r.parse("type")))
}

fn rule_infer_type(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "infer"), r.parse("type_parameter")))
}

fn rule_mapped_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.optional(r.choice((
            r.sequence((op(r, "+"), kw(r, "readonly"))),
            r.sequence((op(r, "-"), kw(r, "readonly"))),
            kw(r, "readonly"),
        ))),
        op(r, "["),
        r.parse("type_parameter"),
        r.optional(r.sequence((kw(r, "as"), r.parse("type")))),
        op(r, "]"),
        r.optional(r.choice((
            r.sequence((op(r, "+"), op(r, "?"))),
            r.sequence((op(r, "-"), op(r, "?"))),
            op(r, "?"),
        ))),
        r.optional(r.parse("type_annotation")),
        op(r, "}"),
    ))
}

fn rule_parenthesized_type(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, "("), r.parse("type"), op(r, ")")))
}
