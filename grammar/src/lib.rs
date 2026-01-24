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
        .rule("regexp_literal", rule_regexp_literal)
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
        .rule("declare_statement", rule_declare_statement)
        .rule("declare_function", rule_declare_function)
        .rule("declare_module", rule_declare_module)
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
        .rule("assignment_expression", rule_assignment_expression)
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
        .rule("private_name", rule_private_name)
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
        .result_variant("ClassBody", "ClassBody")
        .result_variant("ClassMember", "ClassMember")
        .result_variant("SwitchCase", "SwitchCase")
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

/// Parse a regexp literal: /pattern/flags
fn parse_regexp_literal(text: &JsString) -> (String, String) {
    let s = text.as_ref();
    // Format: /pattern/flags
    if !s.starts_with('/') {
        return (String::new(), String::new());
    }
    // Find the closing / (account for escapes and character classes)
    let mut i = 1;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 2; // Skip escaped char
        } else if bytes[i] == b'/' {
            break;
        } else {
            i += 1;
        }
    }
    if i >= bytes.len() {
        return (String::new(), String::new());
    }
    let pattern = s[1..i].to_string();
    let flags = s[i + 1..].to_string();
    (pattern, flags)
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

/// Strip quotes from a string literal value
fn strip_string_quotes(s: &str) -> &str {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s.get(1..s.len().saturating_sub(1)).unwrap_or("")
    } else {
        s
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

/// Create sequence expression (comma operator), flattening nested sequences
fn sequence_expr(left: ParseResult, right: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    let left_expr = to_expr(left)?;
    let right_expr = to_expr(right)?;

    // Flatten: if left is already a sequence, extend it
    let mut expressions = match left_expr {
        Expression::Sequence(seq) => seq.expressions,
        other => vec![other],
    };
    expressions.push(right_expr);

    Ok(ParseResult::Expr(Expression::Sequence(SequenceExpression {
        expressions,
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
    // Check if this is a private name (starts with #)
    let property = if prop.as_str().starts_with('#') {
        MemberProperty::PrivateIdentifier(Identifier { name: prop, span: span.clone() })
    } else {
        MemberProperty::Identifier(Identifier { name: prop, span: span.clone() })
    };
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

/// Create non-null assertion expression (TypeScript)
fn non_null(expr: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    Ok(ParseResult::Expr(Expression::NonNull(NonNullExpression {
        expression: Rc::new(to_expr(expr)?),
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

/// Convert an expression to a pattern for destructuring assignment
fn expr_to_pattern(expr: Expression) -> Result<Pattern, ParseError> {
    match expr {
        Expression::Identifier(id) => Ok(Pattern::Identifier(id)),
        Expression::Array(arr) => {
            // Convert array expression to array pattern
            let elements: Vec<Option<Pattern>> = arr.elements.into_iter().map(|elem| {
                match elem {
                    Some(ArrayElement::Expression(e)) => expr_to_pattern(e).ok(),
                    Some(ArrayElement::Spread(s)) => {
                        // Convert spread to rest pattern
                        expr_to_pattern((*s.argument).clone()).ok().map(|p| {
                            Pattern::Rest(RestElement {
                                argument: Box::new(p),
                                type_annotation: None,
                                span: s.span,
                            })
                        })
                    }
                    None => None,
                }
            }).collect();
            Ok(Pattern::Array(ArrayPattern {
                elements,
                type_annotation: None,
                span: arr.span,
            }))
        }
        Expression::Object(obj) => {
            // Convert object expression to object pattern
            let properties: Vec<ObjectPatternProperty> = obj.properties.into_iter().filter_map(|prop| {
                match prop {
                    ObjectProperty::Property(p) => {
                        let pattern = expr_to_pattern(p.value.clone()).ok()?;
                        Some(ObjectPatternProperty::KeyValue {
                            key: p.key,
                            value: pattern,
                            shorthand: p.shorthand,
                            span: p.span,
                        })
                    }
                    ObjectProperty::Spread(s) => {
                        expr_to_pattern((*s.argument).clone()).ok().map(|p| {
                            ObjectPatternProperty::Rest(RestElement {
                                argument: Box::new(p),
                                type_annotation: None,
                                span: s.span,
                            })
                        })
                    }
                }
            }).collect();
            Ok(Pattern::Object(ObjectPattern {
                properties,
                type_annotation: None,
                span: obj.span,
            }))
        }
        Expression::Assignment(assign) => {
            // Assignment expression in pattern context: `[a = 1]` or `{x = 1}`
            let left_pattern = match assign.left {
                AssignmentTarget::Identifier(id) => Pattern::Identifier(id),
                AssignmentTarget::Pattern(p) => p,
                AssignmentTarget::Member(_) => return Err(ParseError::new("Invalid pattern".to_string(), 0, 0)),
            };
            Ok(Pattern::Assignment(AssignmentPattern {
                left: Box::new(left_pattern),
                right: assign.right,
                span: assign.span,
            }))
        }
        _ => Err(ParseError::new("Cannot convert expression to pattern".to_string(), 0, 0)),
    }
}

/// Create assignment expression
fn assign(left: ParseResult, right: ParseResult, op: AssignmentOp, span: Span) -> Result<ParseResult, ParseError> {
    let target = match left {
        ParseResult::Ident(id) => AssignmentTarget::Identifier(id),
        ParseResult::Expr(Expression::Identifier(id)) => AssignmentTarget::Identifier(id),
        ParseResult::Expr(Expression::Member(m)) => AssignmentTarget::Member(*m),
        ParseResult::Expr(Expression::Array(arr)) => {
            // Destructuring assignment: convert array expression to pattern
            AssignmentTarget::Pattern(expr_to_pattern(Expression::Array(arr))?)
        }
        ParseResult::Expr(Expression::Object(obj)) => {
            // Destructuring assignment: convert object expression to pattern
            AssignmentTarget::Pattern(expr_to_pattern(Expression::Object(obj))?)
        }
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
            let decorators_result = iter.next().unwrap_or(ParseResult::None);
            let _accessibility = iter.next(); // Skip accessibility
            let _readonly = iter.next(); // Skip readonly
            let pattern_result = iter.next()?; // Get pattern

            // Try to convert pattern_result to a Pattern
            let pattern = match pattern_result {
                ParseResult::Pat(p) => p,
                ParseResult::Ident(id) => Pattern::Identifier(id),
                _ => return None,
            };

            // Parse decorators
            let decorators = parse_decorators(decorators_result, default_span.clone());

            Some(FunctionParam {
                pattern,
                type_annotation: None,
                optional: false,
                decorators,
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

/// Create object expression from parsed object_expression result
fn create_object_expr(result: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    // result is ["{", optional(separated_by_trailing), "}"]
    let mut properties: Vec<ObjectProperty> = Vec::new();

    if let ParseResult::List(parts) = result {
        // parts = [open_brace, optional_content, close_brace]
        if let Some(content) = parts.into_iter().nth(1) {
            // separated_by_trailing returns a List of items (without separators)
            if let ParseResult::List(items) = content {
                for item in items {
                    if let Some(prop) = parse_result_to_object_property(item, span.clone()) {
                        properties.push(prop);
                    }
                }
            }
        }
    }

    Ok(ParseResult::Expr(Expression::Object(ObjectExpression {
        properties,
        span,
    })))
}

/// Parse export specifier from ParseResult
/// Expected format: List([identifier, Optional(List([as, identifier]))])
fn parse_export_specifier(result: ParseResult, span: Span) -> Option<ExportSpecifier> {
    if let ParseResult::List(parts) = result {
        let mut iter = parts.into_iter();
        let local_part = iter.next()?;
        let as_part = iter.next();

        let local = if let ParseResult::Ident(id) = local_part {
            id
        } else {
            return None;
        };

        // Check for "as exported" part
        let exported = if let Some(ParseResult::List(as_parts)) = as_part {
            // [as, identifier]
            as_parts.into_iter().nth(1).and_then(|id| {
                if let ParseResult::Ident(ident) = id {
                    Some(ident)
                } else {
                    None
                }
            }).unwrap_or_else(|| local.clone())
        } else {
            local.clone()
        };

        Some(ExportSpecifier { local, exported, span })
    } else {
        None
    }
}

/// Parse import clause into Vec<ImportSpecifier>
/// import_clause can be:
/// - List([*, as, Ident]) - namespace import
/// - List([{, List([specifiers]), }]) - named imports
/// - List([Ident, Optional(List([,, rest]))]) - default import with optional rest
fn parse_import_clause(clause: ParseResult, span: Span) -> Vec<ImportSpecifier> {
    if let ParseResult::List(parts) = clause {
        parse_import_clause_parts(parts, span)
    } else {
        vec![]
    }
}

/// Parse import clause parts into Vec<ImportSpecifier>
fn parse_import_clause_parts(parts: Vec<ParseResult>, span: Span) -> Vec<ImportSpecifier> {
    let mut specifiers = vec![];
    let len = parts.len();

    // Check for namespace import: [*, as, Ident]
    // The * is captured as None, then 'as' is None, then Ident
    if len == 3 {
        let mut iter = parts.clone().into_iter();
        let first = iter.next();
        let _second = iter.next();
        let third = iter.next();

        // If third is an Ident and first is None (from *), this is namespace import
        if let Some(ParseResult::Ident(id)) = third {
            if matches!(first, Some(ParseResult::None)) {
                specifiers.push(ImportSpecifier::Namespace {
                    local: id,
                    span: span.clone(),
                });
                return specifiers;
            }
        }

        // Check for named imports: [{, List([specifiers]), }]
        let mut iter = parts.clone().into_iter();
        let _first = iter.next();
        let second = iter.next();

        if let Some(ParseResult::List(spec_list)) = second {
            // Named imports
            for spec in spec_list {
                if let Some(is) = parse_import_specifier(spec, span.clone()) {
                    specifiers.push(is);
                }
            }
            return specifiers;
        }
    }

    // Default import with optional rest: [Ident, Optional(...)]
    if len == 2 {
        let mut iter = parts.into_iter();
        let first = iter.next();
        let second = iter.next();

        if let Some(ParseResult::Ident(id)) = first {
            specifiers.push(ImportSpecifier::Default {
                local: id,
                span: span.clone(),
            });

            // Check for rest part: List([,, rest_clause])
            if let Some(ParseResult::List(rest_parts)) = second {
                // rest_parts is [comma, import_clause_rest]
                if let Some(rest_clause) = rest_parts.into_iter().nth(1) {
                    let rest_specifiers = parse_import_clause(rest_clause, span);
                    specifiers.extend(rest_specifiers);
                }
            }
        }
    }

    specifiers
}

/// Parse a single import specifier: [Ident, Optional(List([as, Ident]))]
fn parse_import_specifier(spec: ParseResult, span: Span) -> Option<ImportSpecifier> {
    if let ParseResult::List(parts) = spec {
        let mut iter = parts.into_iter();
        let imported_part = iter.next()?;
        let as_part = iter.next();

        let imported = if let ParseResult::Ident(id) = imported_part {
            id
        } else {
            return None;
        };

        // Check for "as local" part
        let local = if let Some(ParseResult::List(as_parts)) = as_part {
            // [as, identifier]
            as_parts.into_iter().nth(1).and_then(|id| {
                if let ParseResult::Ident(ident) = id {
                    Some(ident)
                } else {
                    None
                }
            }).unwrap_or_else(|| imported.clone())
        } else {
            imported.clone()
        };

        Some(ImportSpecifier::Named {
            local,
            imported,
            span,
        })
    } else {
        None
    }
}

/// Parse decorators from ParseResult
/// Decorators come as a List of List([@ op, expression])
fn parse_decorators(decorators: ParseResult, span: Span) -> Vec<Decorator> {
    let mut result = vec![];
    if let ParseResult::List(decorator_list) = decorators {
        for dec in decorator_list {
            if let ParseResult::List(parts) = dec {
                // Each decorator is [@, expression]
                if let Some(expr_part) = parts.into_iter().nth(1) {
                    if let Ok(expr) = to_expr(expr_part) {
                        result.push(Decorator {
                            expression: expr,
                            span: span.clone(),
                        });
                    }
                }
            }
        }
    }
    result
}

/// Parse type arguments from ParseResult (e.g., <string, number>)
/// Returns Some(TypeArguments) if present, None otherwise
fn parse_type_arguments(result: ParseResult) -> Option<TypeArguments> {
    match result {
        ParseResult::None => None,
        ParseResult::List(parts) => {
            // Structure: [<, type_list, >]
            let mut iter = parts.into_iter();
            let _open = iter.next(); // <
            let types_result = iter.next().unwrap_or(ParseResult::None);
            let _close = iter.next(); // >

            let params = match types_result {
                ParseResult::List(type_items) => {
                    type_items.into_iter().filter_map(|t| {
                        parse_result_to_type_annotation(t)
                    }).collect()
                }
                other => {
                    if let Some(t) = parse_result_to_type_annotation(other) {
                        vec![t]
                    } else {
                        vec![]
                    }
                }
            };

            if params.is_empty() {
                None
            } else {
                Some(TypeArguments {
                    params,
                    span: Span { start: 0, end: 0, line: 0, column: 0 },
                })
            }
        }
        _ => None,
    }
}

/// Convert ParseResult to TypeAnnotation
fn parse_result_to_type_annotation(result: ParseResult) -> Option<TypeAnnotation> {
    let default_span = Span { start: 0, end: 0, line: 0, column: 0 };
    match result {
        ParseResult::Ident(id) => {
            // Simple type like "string", "number", "void"
            let span = id.span.clone();
            Some(TypeAnnotation::Reference(TypeReference {
                name: id,
                type_arguments: None,
                span,
            }))
        }
        ParseResult::Text(s, sp) => {
            // Captured text like "void"
            Some(TypeAnnotation::Reference(TypeReference {
                name: Identifier { name: s, span: sp.clone() },
                type_arguments: None,
                span: sp,
            }))
        }
        ParseResult::List(parts) => {
            // Could be various structures:
            // - primary_type: [base_type, array_suffixes*]
            // - type_reference: [identifier, type_arguments?]
            // Try to extract the first meaningful part
            let mut iter = parts.into_iter();
            if let Some(first) = iter.next() {
                // Recursively try to convert the first part
                parse_result_to_type_annotation(first)
            } else {
                None
            }
        }
        ParseResult::None => {
            // For empty/None results, create a placeholder
            Some(TypeAnnotation::Reference(TypeReference {
                name: Identifier { name: JsString::from("unknown"), span: default_span.clone() },
                type_arguments: None,
                span: default_span,
            }))
        }
        _ => None,
    }
}

/// Convert ParseResult to ObjectProperty
fn parse_result_to_object_property(result: ParseResult, span: Span) -> Option<ObjectProperty> {
    match result {
        // Identifier = shorthand property
        ParseResult::Ident(id) => {
            Some(ObjectProperty::Property(Box::new(Property {
                key: ObjectPropertyKey::Identifier(id.clone()),
                value: Expression::Identifier(id),
                kind: PropertyKind::Init,
                computed: false,
                shorthand: true,
                method: false,
                span,
            })))
        }
        ParseResult::List(items) => {
            let len = items.len();
            // Check for spread: ["...", expression]
            if len == 2 {
                // Check if first is the spread operator (peek without consuming)
                let is_spread = match items.first() {
                    Some(ParseResult::Text(s, _)) => s.as_ref() == "...",
                    _ => false,
                };
                if is_spread {
                    let mut iter = items.into_iter();
                    let _spread_op = iter.next()?;
                    let second = iter.next()?;
                    if let Ok(expr) = to_expr(second) {
                        return Some(ObjectProperty::Spread(SpreadElement {
                            argument: Rc::new(expr),
                            span,
                        }));
                    }
                    return None;
                }
                // Not a spread, fall through to other checks
            }
            // Check for key-value: [property_key, ":", expression]
            if len == 3 {
                let mut iter = items.into_iter();
                let key_result = iter.next()?;
                let _colon = iter.next()?; // skip ":"
                let value_result = iter.next()?;

                let key = parse_result_to_property_key(key_result)?;
                let value = to_expr(value_result).ok()?;
                let computed = matches!(&key, ObjectPropertyKey::Computed(_));

                return Some(ObjectProperty::Property(Box::new(Property {
                    key,
                    value,
                    kind: PropertyKind::Init,
                    computed,
                    shorthand: false,
                    method: false,
                    span,
                })));
            }
            // Method property: [optional async, optional *, property_key, "(", optional params, ")", block]
            // This has 5+ items
            if len >= 5 {
                return parse_method_property(items, span);
            }
            None
        }
        _ => None,
    }
}

/// Extract BlockStatement from a parsed block result
fn to_block_statement(result: ParseResult) -> Option<Rc<BlockStatement>> {
    match to_stmt(result) {
        Ok(Statement::Block(block)) => Some(Rc::new(block)),
        _ => None,
    }
}

/// Parse method property from list items
fn parse_method_property(items: Vec<ParseResult>, span: Span) -> Option<ObjectProperty> {
    // Method: [async?, *?, key, "(", params?, ")", block]
    // Getter: ["get", key, "(", ")", block] - 5 items
    // Setter: ["set", key, "(", params, ")", block] - 6 items
    let len = items.len();
    let mut iter = items.into_iter().peekable();

    // Check for getter/setter
    if len == 5 {
        let is_getter = match iter.peek() {
            Some(ParseResult::Text(s, _)) => s.as_ref() == "get",
            _ => false,
        };
        if is_getter {
            iter.next(); // consume "get"
            let key_result = iter.next()?;
            let _open = iter.next()?; // "("
            let _close = iter.next()?; // ")"
            let block = iter.next()?;

            let key = parse_result_to_property_key(key_result)?;
            let body = to_block_statement(block)?;

            return Some(ObjectProperty::Property(Box::new(Property {
                key,
                value: Expression::Function(Box::new(FunctionExpression {
                    id: None,
                    params: Rc::from(Vec::<FunctionParam>::new()),
                    body,
                    async_: false,
                    generator: false,
                    type_parameters: None,
                    return_type: None,
                    span: span.clone(),
                })),
                kind: PropertyKind::Get,
                computed: false,
                shorthand: false,
                method: true,
                span,
            })));
        }
    }

    if len == 6 {
        let is_setter = match iter.peek() {
            Some(ParseResult::Text(s, _)) => s.as_ref() == "set",
            _ => false,
        };
        if is_setter {
            iter.next(); // consume "set"
            let key_result = iter.next()?;
            let _open = iter.next()?; // "("
            let params_result = iter.next()?;
            let _close = iter.next()?; // ")"
            let block = iter.next()?;

            let key = parse_result_to_property_key(key_result)?;
            let params: Rc<[FunctionParam]> = parse_function_params(params_result).into();
            let body = to_block_statement(block)?;

            return Some(ObjectProperty::Property(Box::new(Property {
                key,
                value: Expression::Function(Box::new(FunctionExpression {
                    id: None,
                    params,
                    body,
                    async_: false,
                    generator: false,
                    type_parameters: None,
                    return_type: None,
                    span: span.clone(),
                })),
                kind: PropertyKind::Set,
                computed: false,
                shorthand: false,
                method: true,
                span,
            })));
        }
    }

    // Regular method: [async?, *?, key, "(", params?, ")", block]
    let first = iter.next()?;
    let is_async = match &first {
        ParseResult::Text(s, _) => s.as_ref() == "async",
        _ => false,
    };
    let first_after_async = if is_async { iter.next()? } else { first };

    let is_generator = match &first_after_async {
        ParseResult::Text(s, _) => s.as_ref() == "*",
        _ => false,
    };
    let key_result = if is_generator { iter.next()? } else { first_after_async };

    let key = parse_result_to_property_key(key_result)?;
    let _open = iter.next()?; // "("
    let params_result = iter.next()?;
    let _close = iter.next()?; // ")"
    let block = iter.next()?;

    let params: Rc<[FunctionParam]> = parse_function_params(params_result).into();
    let body = to_block_statement(block)?;

    Some(ObjectProperty::Property(Box::new(Property {
        key,
        value: Expression::Function(Box::new(FunctionExpression {
            id: None,
            params,
            body,
            async_: is_async,
            generator: is_generator,
            type_parameters: None,
            return_type: None,
            span: span.clone(),
        })),
        kind: PropertyKind::Init,
        computed: false,
        shorthand: false,
        method: true,
        span,
    })))
}

/// Convert ParseResult to ObjectPropertyKey
fn parse_result_to_property_key(result: ParseResult) -> Option<ObjectPropertyKey> {
    match result {
        ParseResult::Ident(id) => Some(ObjectPropertyKey::Identifier(id)),
        ParseResult::Text(s, span) => {
            let s_str = s.as_ref();
            // String or number literal as key
            if s_str.starts_with('"') || s_str.starts_with('\'') {
                Some(ObjectPropertyKey::String(StringLiteral {
                    value: JsString::from(s_str.trim_matches(|c| c == '"' || c == '\'').to_string()),
                    span,
                }))
            } else {
                // Try as number
                Some(ObjectPropertyKey::Number(Literal {
                    value: LiteralValue::Number(s_str.parse().unwrap_or(0.0)),
                    span,
                }))
            }
        }
        ParseResult::Expr(e) => Some(ObjectPropertyKey::Computed(Rc::new(e))),
        ParseResult::List(items) => {
            // Could be computed: ["[", expression, "]"]
            if items.len() == 3 {
                let expr = items.into_iter().nth(1)?;
                if let Ok(e) = to_expr(expr) {
                    return Some(ObjectPropertyKey::Computed(Rc::new(e)));
                }
            }
            None
        }
        _ => None,
    }
}

/// Parse function params from ParseResult
fn parse_function_params(result: ParseResult) -> Vec<FunctionParam> {
    let default_span = Span { start: 0, end: 0, line: 0, column: 0 };
    match result {
        ParseResult::None => vec![],
        ParseResult::List(items) => {
            items.into_iter().filter_map(|item| {
                extract_param_pattern(item, &default_span)
            }).collect()
        }
        ParseResult::Pat(p) => {
            let span = p.span();
            vec![FunctionParam {
                pattern: p,
                type_annotation: None,
                optional: false,
                decorators: vec![],
                accessibility: None,
                readonly: false,
                span,
            }]
        }
        _ => vec![],
    }
}

/// Create class declaration
fn create_class_decl(
    decorators: ParseResult,
    abstract_kw: ParseResult,
    name: ParseResult,
    extends_clause: ParseResult,
    body: ParseResult,
    span: Span
) -> Result<ParseResult, ParseError> {
    let is_abstract = !matches!(abstract_kw, ParseResult::None);
    let id = match name {
        ParseResult::Ident(id) => Some(id),
        _ => None,
    };

    // Extract super class from extends clause
    let super_class = match extends_clause {
        ParseResult::List(parts) => {
            // [extends, expr]
            parts.into_iter().nth(1).and_then(|e| to_expr(e).ok()).map(Rc::new)
        }
        _ => None,
    };

    // Extract class body
    let class_body = match body {
        ParseResult::ClassBody(b) => b,
        _ => ClassBody { members: vec![], span: span.clone() },
    };

    // Parse decorators
    let parsed_decorators = parse_decorators(decorators, span.clone());

    Ok(ParseResult::Stmt(Statement::ClassDeclaration(Box::new(ClassDeclaration {
        id,
        type_parameters: None,
        super_class,
        implements: vec![],
        body: class_body,
        decorators: parsed_decorators,
        abstract_: is_abstract,
        span,
    }))))
}

/// Create class body from members list
fn create_class_body(members: ParseResult, span: Span) -> Result<ParseResult, ParseError> {
    let members_vec: Vec<ClassMember> = match members {
        ParseResult::List(items) => {
            items.into_iter().filter_map(|item| {
                match item {
                    ParseResult::ClassMember(m) => Some(m),
                    _ => None,
                }
            }).collect()
        }
        ParseResult::None => vec![],
        _ => vec![],
    };

    Ok(ParseResult::ClassBody(ClassBody {
        members: members_vec,
        span,
    }))
}

/// Create class constructor
fn create_constructor(
    accessibility: ParseResult,
    params: ParseResult,
    body: ParseResult,
    span: Span
) -> Result<ParseResult, ParseError> {
    let acc = match accessibility {
        ParseResult::Text(s, _) => {
            match s.as_ref() {
                "public" => Some(Accessibility::Public),
                "private" => Some(Accessibility::Private),
                "protected" => Some(Accessibility::Protected),
                _ => None,
            }
        }
        _ => None,
    };

    let params_vec = params_to_vec(params, &span);
    let body_block = match body {
        ParseResult::Stmt(Statement::Block(b)) => b,
        _ => BlockStatement { body: Rc::from(vec![]), span: span.clone() },
    };

    Ok(ParseResult::ClassMember(ClassMember::Constructor(Box::new(ClassConstructor {
        params: params_vec,
        body: body_block,
        accessibility: acc,
        span,
    }))))
}

/// Create class method
fn create_class_method(
    decorators: ParseResult,
    static_kw: ParseResult,
    async_kw: ParseResult,
    generator: ParseResult,
    accessor: ParseResult,
    key: ParseResult,
    params: ParseResult,
    body: ParseResult,
    span: Span
) -> Result<ParseResult, ParseError> {
    let is_static = !matches!(static_kw, ParseResult::None);
    let is_async = !matches!(async_kw, ParseResult::None);
    let is_generator = !matches!(generator, ParseResult::None);

    let kind = match accessor {
        ParseResult::Text(s, _) => {
            match s.as_ref() {
                "get" => MethodKind::Get,
                "set" => MethodKind::Set,
                _ => MethodKind::Method,
            }
        }
        ParseResult::List(parts) => {
            // captured_kw produces [Text("get"|"set", span), None, None]
            match parts.into_iter().next() {
                Some(ParseResult::Text(s, _)) => {
                    match s.as_ref() {
                        "get" => MethodKind::Get,
                        "set" => MethodKind::Set,
                        _ => MethodKind::Method,
                    }
                }
                _ => MethodKind::Method,
            }
        }
        _ => MethodKind::Method,
    };

    let prop_key = result_to_prop_key(key, &span);
    let params_vec = params_to_vec(params, &span);
    let body_block = match body {
        ParseResult::Stmt(Statement::Block(b)) => Rc::new(b),
        _ => Rc::new(BlockStatement { body: Rc::from(vec![]), span: span.clone() }),
    };

    // Parse decorators
    let parsed_decorators = parse_decorators(decorators, span.clone());

    let func_expr = FunctionExpression {
        id: None,
        params: Rc::from(params_vec),
        return_type: None,
        type_parameters: None,
        body: body_block,
        generator: is_generator,
        async_: is_async,
        span: span.clone(),
    };

    Ok(ParseResult::ClassMember(ClassMember::Method(Box::new(ClassMethod {
        key: prop_key,
        value: func_expr,
        kind,
        computed: false,
        static_: is_static,
        accessibility: None,
        decorators: parsed_decorators,
        span,
    }))))
}

/// Create class property
fn create_class_property(
    decorators: ParseResult,
    static_kw: ParseResult,
    readonly: ParseResult,
    accessor: ParseResult,
    key: ParseResult,
    optional: ParseResult,
    initializer: ParseResult,
    span: Span
) -> Result<ParseResult, ParseError> {
    let is_static = !matches!(static_kw, ParseResult::None);
    let is_readonly = !matches!(readonly, ParseResult::None);
    let is_accessor = !matches!(accessor, ParseResult::None);
    let is_optional = !matches!(optional, ParseResult::None);

    let prop_key = result_to_prop_key(key, &span);

    let value = match initializer {
        ParseResult::List(parts) => {
            // [=, expr]
            parts.into_iter().nth(1).and_then(|e| to_expr(e).ok()).map(Box::new)
        }
        _ => None,
    };

    let parsed_decorators = parse_decorators(decorators, span.clone());

    Ok(ParseResult::ClassMember(ClassMember::Property(Box::new(ClassProperty {
        key: prop_key,
        value,
        type_annotation: None,
        computed: false,
        static_: is_static,
        readonly: is_readonly,
        optional: is_optional,
        accessor: is_accessor,
        accessibility: None,
        decorators: parsed_decorators,
        span,
    }))))
}

/// Convert ParseResult to ObjectPropertyKey
fn result_to_prop_key(result: ParseResult, span: &Span) -> ObjectPropertyKey {
    match result {
        ParseResult::Ident(id) => ObjectPropertyKey::Identifier(id),
        ParseResult::Text(s, sp) => {
            // Check if this is a private name (starts with #)
            if s.as_str().starts_with('#') {
                ObjectPropertyKey::PrivateIdentifier(Identifier { name: s, span: sp })
            } else {
                ObjectPropertyKey::Identifier(Identifier { name: s, span: sp })
            }
        }
        ParseResult::Expr(Expression::Literal(lit)) => {
            match lit.value {
                LiteralValue::String(s) => ObjectPropertyKey::String(StringLiteral { value: s, span: lit.span }),
                _ => ObjectPropertyKey::Number(*lit),
            }
        }
        _ => ObjectPropertyKey::Identifier(Identifier { name: JsString::from(""), span: span.clone() }),
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
        r.zero_or_more(r.sequence((r.not_followed_by(r.char('\n')), r.any_char()))),
    ))
}

fn rule_block_comment(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.lit("/*"),
        r.zero_or_more(r.sequence((r.not_followed_by(r.lit("*/")), r.any_char()))),
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

// Regexp literal: /pattern/flags
fn rule_regexp_literal(r: &RuleBuilder) -> Combinator {
    r.capture(r.sequence((
        r.char('/'),
        // Pattern: any char except unescaped / or newline, or escaped char
        r.one_or_more(r.choice((
            r.sequence((r.char('\\'), r.any_char())), // Escape sequence
            r.sequence((
                r.char('['),
                r.zero_or_more(r.choice((
                    r.sequence((r.char('\\'), r.any_char())), // Escape in char class
                    r.sequence((
                        r.not_followed_by(r.choice((r.char(']'), r.char('\\'), r.char('\n')))),
                        r.any_char(),
                    )),
                ))),
                r.char(']'),
            )), // Character class
            r.sequence((
                r.not_followed_by(r.choice((r.char('/'), r.char('\\'), r.char('\n'), r.char('[')))),
                r.any_char(),
            )),
        ))),
        r.char('/'),
        // Optional flags
        r.zero_or_more(r.alpha()),
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

/// Keyword that captures the matched text (for distinguishing get/set)
fn captured_kw(r: &RuleBuilder, keyword: &str) -> Combinator {
    r.sequence((
        r.capture(r.lit(keyword)),
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

fn ws_prefix(r: &RuleBuilder, operator: &str) -> Combinator {
    r.sequence((r.parse("ws"), r.lit(operator)))
}

fn ws_prefix_kw(r: &RuleBuilder, keyword: &str) -> Combinator {
    r.sequence((
        r.parse("ws"),
        r.lit(keyword),
        r.not_followed_by(r.ident_cont()),
    ))
}

// === Rule Functions ===

fn rule_program(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("ws"), // Consume leading whitespace
        r.zero_or_more(r.parse("statement")),
    ))
    .ast(
        "|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
            // result is [ws, statements]
            let parts = result.into_list();
            let stmts_result = parts.into_iter().nth(1).unwrap_or(ParseResult::None);
            let items = stmts_result.into_list();
            let mut statements = Vec::new();
            for item in items {
                statements.push(to_stmt(item)?);
            }
            Ok(ParseResult::Prog(Program {
                body: Rc::from(statements),
                source_type: SourceType::Module,
            }))
        }",
    )
}

fn rule_statement(r: &RuleBuilder) -> Combinator {
    // Each sub-rule should return ParseResult::Stmt, so we just pass through
    // Use vec![] instead of nested choice() tuples to avoid tuple dimension limits
    r.choice(vec![
        r.parse("declare_statement"),
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
        r.parse("namespace_declaration"),
        r.parse("labeled_statement"),
        r.parse("expression_statement"),
        r.parse("empty_statement"),
    ])
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
        // Use assignment_expression (not expression) to avoid comma being consumed
        r.optional(r.sequence((op(r, "="), r.parse("assignment_expression")))),
    ))
}

fn rule_variable_declaration_no_semi(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.capture(r.choice((kw(r, "let"), kw(r, "const"), kw(r, "var")))),
        r.separated_by(r.parse("variable_declarator"), op(r, ",")),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
                let mut parts_iter = parts.into_iter();
                let pattern = to_pattern(parts_iter.next().unwrap_or(ParseResult::None))?;
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
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
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
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [decorators, abstract?, class, name?, type_params?, extends?, implements?, body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let decorators = iter.next().unwrap_or(ParseResult::None);
            let abstract_kw = iter.next().unwrap_or(ParseResult::None);
            let _class_kw = iter.next();
            let name = iter.next().unwrap_or(ParseResult::None);
            let _type_params = iter.next();
            let extends_clause = iter.next().unwrap_or(ParseResult::None);
            let _implements = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);

            create_class_decl(decorators, abstract_kw, name, extends_clause, body, span)
        } else {
            Err(ParseError::new(\"Expected class declaration parts\".to_string(), 0, 0))
        }
    }",
    )
}

fn rule_class_body(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.zero_or_more(r.parse("class_member")),
        op(r, "}"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [{, members*, }]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _open_brace = iter.next();
            let members = iter.next().unwrap_or(ParseResult::None);
            let _close_brace = iter.next();

            create_class_body(members, span)
        } else {
            Err(ParseError::new(\"Expected class body parts\".to_string(), 0, 0))
        }
    }",
    )
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
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [accessibility?, constructor, (, params?, ), body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let accessibility = iter.next().unwrap_or(ParseResult::None);
            let _constructor_kw = iter.next();
            let _open_paren = iter.next();
            let params = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);

            create_constructor(accessibility, params, body, span)
        } else {
            Err(ParseError::new(\"Expected constructor parts\".to_string(), 0, 0))
        }
    }",
    )
}

fn rule_class_method(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(r.parse("accessibility_modifier")),
        r.optional(kw(r, "static")),
        r.optional(kw(r, "async")),
        r.optional(op(r, "*")),
        r.optional(r.choice((captured_kw(r, "get"), captured_kw(r, "set")))),
        r.parse("property_key"),
        r.optional(r.parse("type_parameters")),
        r.sequence((
            op(r, "("),
            r.optional(r.parse("parameter_list")),
            op(r, ")"),
            r.optional(r.parse("type_annotation")),
            r.parse("block_statement"),
        )),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [decorators, accessibility?, static?, async?, *, get/set?, key, type_params?, method_sig]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let decorators = iter.next().unwrap_or(ParseResult::None);
            let _accessibility = iter.next();
            let static_kw = iter.next().unwrap_or(ParseResult::None);
            let async_kw = iter.next().unwrap_or(ParseResult::None);
            let generator = iter.next().unwrap_or(ParseResult::None);
            let accessor = iter.next().unwrap_or(ParseResult::None);
            let key = iter.next().unwrap_or(ParseResult::None);
            let _type_params = iter.next();
            let method_sig = iter.next().unwrap_or(ParseResult::None);

            // method_sig = [(, params?, ), type_ann?, body]
            let (params, body) = if let ParseResult::List(sig_parts) = method_sig {
                let mut sig_iter = sig_parts.into_iter();
                let _open_paren = sig_iter.next();
                let params = sig_iter.next().unwrap_or(ParseResult::None);
                let _close_paren = sig_iter.next();
                let _type_ann = sig_iter.next();
                let body = sig_iter.next().unwrap_or(ParseResult::None);
                (params, body)
            } else {
                (ParseResult::None, ParseResult::None)
            };

            create_class_method(decorators, static_kw, async_kw, generator, accessor, key, params, body, span)
        } else {
            Err(ParseError::new(\"Expected class method parts\".to_string(), 0, 0))
        }
    }")
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
        // Use assignment_expression to avoid comma being consumed
        r.optional(r.sequence((op(r, "="), r.parse("assignment_expression")))),
        r.parse("semicolon"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [decorators, accessibility?, static?, readonly?, accessor?, key, ?, type_ann?, init?, ;]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let decorators = iter.next().unwrap_or(ParseResult::None);
            let _accessibility = iter.next();
            let static_kw = iter.next().unwrap_or(ParseResult::None);
            let readonly = iter.next().unwrap_or(ParseResult::None);
            let accessor = iter.next().unwrap_or(ParseResult::None);
            let key = iter.next().unwrap_or(ParseResult::None);
            let optional = iter.next().unwrap_or(ParseResult::None);
            let _type_ann = iter.next();
            let initializer = iter.next().unwrap_or(ParseResult::None);
            let _semi = iter.next();

            create_class_property(decorators, static_kw, readonly, accessor, key, optional, initializer, span)
        } else {
            Err(ParseError::new(\"Expected class property parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_static_block(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "static"), r.parse("block_statement")))
        .ast(
            "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [static, block]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _static_kw = iter.next();
            let block = iter.next().unwrap_or(ParseResult::None);

            let block_stmt = match block {
                ParseResult::Stmt(Statement::Block(b)) => b,
                _ => BlockStatement { body: Rc::from(vec![]), span: span.clone() },
            };

            Ok(ParseResult::ClassMember(ClassMember::StaticBlock(block_stmt)))
        } else {
            Err(ParseError::new(\"Expected static block parts\".to_string(), 0, 0))
        }
    }",
        )
}

fn rule_if_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "if"),
        op(r, "("),
        r.parse("expression"),
        op(r, ")"),
        r.parse("statement"),
        r.optional(r.sequence((kw(r, "else"), r.parse("statement")))),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
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
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [for, (, init?, ;, test?, ;, update?, ), body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _for_kw = iter.next();
            let _open_paren = iter.next();
            let init_result = iter.next().unwrap_or(ParseResult::None);
            let _semi1 = iter.next();
            let test = iter.next().unwrap_or(ParseResult::None);
            let _semi2 = iter.next();
            let update = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);

            let init = match init_result {
                ParseResult::None => None,
                ParseResult::Stmt(Statement::VariableDeclaration(decl)) => Some(ForInit::Variable(decl)),
                ParseResult::Expr(e) => Some(ForInit::Expression(Rc::new(e))),
                other => to_expr(other).ok().map(|e| ForInit::Expression(Rc::new(e))),
            };

            let test_expr = match test {
                ParseResult::None => None,
                other => Some(Rc::new(to_expr(other)?)),
            };

            let update_expr = match update {
                ParseResult::None => None,
                other => Some(Rc::new(to_expr(other)?)),
            };

            let body_stmt = match body {
                ParseResult::Stmt(s) => Rc::new(s),
                _ => return Err(ParseError::new(\"Expected statement\".to_string(), 0, 0)),
            };

            Ok(ParseResult::Stmt(Statement::For(Box::new(ForStatement {
                init,
                test: test_expr,
                update: update_expr,
                body: body_stmt,
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected for statement parts\".to_string(), 0, 0))
        }
    }")
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
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [for, (, left, in, right, ), body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _for_kw = iter.next();
            let _open_paren = iter.next();
            let left_result = iter.next().unwrap_or(ParseResult::None);
            let _in_kw = iter.next();
            let right = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);

            let left = match left_result {
                ParseResult::Stmt(Statement::VariableDeclaration(decl)) => ForInOfLeft::Variable(decl),
                ParseResult::Pat(p) => ForInOfLeft::Pattern(p),
                ParseResult::Ident(id) => ForInOfLeft::Pattern(Pattern::Identifier(id)),
                ParseResult::Expr(Expression::Identifier(id)) => ForInOfLeft::Pattern(Pattern::Identifier(id)),
                _ => return Err(ParseError::new(\"Invalid for-in left side\".to_string(), 0, 0)),
            };

            let body_stmt = match body {
                ParseResult::Stmt(s) => Rc::new(s),
                _ => return Err(ParseError::new(\"Expected statement\".to_string(), 0, 0)),
            };

            Ok(ParseResult::Stmt(Statement::ForIn(Box::new(ForInStatement {
                left,
                right: Rc::new(to_expr(right)?),
                body: body_stmt,
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected for-in statement parts\".to_string(), 0, 0))
        }
    }")
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
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [for, await?, (, left, of, right, ), body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _for_kw = iter.next();
            let await_result = iter.next().unwrap_or(ParseResult::None);
            let _open_paren = iter.next();
            let left_result = iter.next().unwrap_or(ParseResult::None);
            let _of_kw = iter.next();
            let right = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);

            let await_ = !matches!(await_result, ParseResult::None);

            let left = match left_result {
                ParseResult::Stmt(Statement::VariableDeclaration(decl)) => ForInOfLeft::Variable(decl),
                ParseResult::Pat(p) => ForInOfLeft::Pattern(p),
                ParseResult::Ident(id) => ForInOfLeft::Pattern(Pattern::Identifier(id)),
                ParseResult::Expr(Expression::Identifier(id)) => ForInOfLeft::Pattern(Pattern::Identifier(id)),
                _ => return Err(ParseError::new(\"Invalid for-of left side\".to_string(), 0, 0)),
            };

            let body_stmt = match body {
                ParseResult::Stmt(s) => Rc::new(s),
                _ => return Err(ParseError::new(\"Expected statement\".to_string(), 0, 0)),
            };

            Ok(ParseResult::Stmt(Statement::ForOf(Box::new(ForOfStatement {
                left,
                right: Rc::new(to_expr(right)?),
                body: body_stmt,
                await_,
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected for-of statement parts\".to_string(), 0, 0))
        }
    }")
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
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
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
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
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
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [switch, (, discriminant, ), {, cases, }]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _switch_kw = iter.next();
            let _open_paren = iter.next();
            let discriminant = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next();
            let _open_brace = iter.next();
            let cases_result = iter.next().unwrap_or(ParseResult::None);
            let _close_brace = iter.next();

            let cases: Vec<SwitchCase> = match cases_result {
                ParseResult::List(items) => {
                    items.into_iter().filter_map(|item| {
                        if let ParseResult::SwitchCase(c) = item {
                            Some(c)
                        } else {
                            None
                        }
                    }).collect()
                }
                ParseResult::None => vec![],
                _ => vec![],
            };

            Ok(ParseResult::Stmt(Statement::Switch(SwitchStatement {
                discriminant: Rc::new(to_expr(discriminant)?),
                cases: Rc::from(cases),
                span,
            })))
        } else {
            Err(ParseError::new(\"Expected switch statement parts\".to_string(), 0, 0))
        }
    }",
    )
}

fn rule_switch_case(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((
            kw(r, "case"),
            r.parse("expression"),
            op(r, ":"),
            r.zero_or_more(r.parse("statement")),
        ))
        .ast(
            "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            // [case, test, :, consequent]
            if let ParseResult::List(parts) = result {
                let mut iter = parts.into_iter();
                let _case_kw = iter.next();
                let test = iter.next().unwrap_or(ParseResult::None);
                let _colon = iter.next();
                let consequent_result = iter.next().unwrap_or(ParseResult::None);

                let consequent: Vec<Statement> = match consequent_result {
                    ParseResult::List(items) => {
                        items.into_iter().filter_map(|item| {
                            if let ParseResult::Stmt(s) = item { Some(s) } else { None }
                        }).collect()
                    }
                    ParseResult::Stmt(s) => vec![s],
                    _ => vec![],
                };

                Ok(ParseResult::SwitchCase(SwitchCase {
                    test: Some(Rc::new(to_expr(test)?)),
                    consequent: Rc::from(consequent),
                    span,
                }))
            } else {
                Err(ParseError::new(\"Expected case parts\".to_string(), 0, 0))
            }
        }",
        ),
        r.sequence((
            kw(r, "default"),
            op(r, ":"),
            r.zero_or_more(r.parse("statement")),
        ))
        .ast(
            "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            // [default, :, consequent]
            if let ParseResult::List(parts) = result {
                let mut iter = parts.into_iter();
                let _default_kw = iter.next();
                let _colon = iter.next();
                let consequent_result = iter.next().unwrap_or(ParseResult::None);

                let consequent: Vec<Statement> = match consequent_result {
                    ParseResult::List(items) => {
                        items.into_iter().filter_map(|item| {
                            if let ParseResult::Stmt(s) = item { Some(s) } else { None }
                        }).collect()
                    }
                    ParseResult::Stmt(s) => vec![s],
                    _ => vec![],
                };

                Ok(ParseResult::SwitchCase(SwitchCase {
                    test: None,
                    consequent: Rc::from(consequent),
                    span,
                }))
            } else {
                Err(ParseError::new(\"Expected default case parts\".to_string(), 0, 0))
            }
        }",
        ),
    ))
}

fn rule_try_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "try"),
        r.parse("block_statement"),
        r.optional(r.parse("catch_clause")),
        r.optional(r.sequence((kw(r, "finally"), r.parse("block_statement")))),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [try, block, catch_clause?, finally_clause?]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _try_kw = iter.next();
            let block_result = iter.next().unwrap_or(ParseResult::None);
            let catch_result = iter.next().unwrap_or(ParseResult::None);
            let finally_result = iter.next().unwrap_or(ParseResult::None);

            let block = match block_result {
                ParseResult::Stmt(Statement::Block(b)) => b,
                _ => return Err(ParseError::new(\"Expected block\".to_string(), 0, 0)),
            };

            // catch_clause returns a CatchClause-like structure
            let handler = match catch_result {
                ParseResult::List(parts) => {
                    // [catch, param_clause?, body]
                    let mut iter = parts.into_iter();
                    let _catch_kw = iter.next();
                    let param_result = iter.next();
                    let body_result = iter.next().unwrap_or(ParseResult::None);

                    // Extract param from optional (paren, pattern, paren)
                    let param = match param_result {
                        Some(ParseResult::List(param_parts)) => {
                            param_parts.into_iter().nth(1).and_then(|p| {
                                match p {
                                    ParseResult::Pat(pat) => Some(pat),
                                    ParseResult::Ident(id) => Some(Pattern::Identifier(id)),
                                    _ => None,
                                }
                            })
                        }
                        _ => None,
                    };

                    let body = match body_result {
                        ParseResult::Stmt(Statement::Block(b)) => b,
                        _ => return Err(ParseError::new(\"Expected catch body\".to_string(), 0, 0)),
                    };

                    Some(CatchClause { param, body, span: span.clone() })
                }
                ParseResult::None => None,
                _ => None,
            };

            // finally clause is [finally, block]
            let finalizer = match finally_result {
                ParseResult::List(finally_parts) => {
                    finally_parts.into_iter().nth(1).and_then(|b| {
                        match b {
                            ParseResult::Stmt(Statement::Block(block)) => Some(block),
                            _ => None,
                        }
                    })
                }
                ParseResult::None => None,
                _ => None,
            };

            Ok(ParseResult::Stmt(Statement::Try(Box::new(TryStatement {
                block,
                handler,
                finalizer,
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected try statement parts\".to_string(), 0, 0))
        }
    }",
    )
}

fn rule_catch_clause(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "catch"),
        r.optional(r.sequence((op(r, "("), r.parse("pattern"), op(r, ")")))),
        r.parse("block_statement"),
    ))
}

fn rule_block_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, "{"), r.zero_or_more(r.parse("statement")), op(r, "}")))
        .ast(
            "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        create_block_stmt(result, span)
    }",
        )
}

fn rule_return_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "return"),
        r.optional(r.parse("expression")),
        r.parse("semicolon"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
}

fn rule_break_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "break"),
        r.optional(r.parse("identifier")),
        r.parse("semicolon"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
}

fn rule_continue_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "continue"),
        r.optional(r.parse("identifier")),
        r.parse("semicolon"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
}

fn rule_throw_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "throw"), r.parse("expression"), r.parse("semicolon")))
        .ast(
            "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
        )
}

fn rule_debugger_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "debugger"), r.parse("semicolon"))).ast(
        "|_result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
            Ok(ParseResult::Stmt(Statement::Debugger))
        }",
    )
}

fn rule_labeled_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((r.parse("identifier"), op(r, ":"), r.parse("statement")))
        .ast(
            "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [label, :, body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let label = iter.next().unwrap_or(ParseResult::None);
            let _colon = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);

            let label_id = to_ident(label)?;
            let body_stmt = match body {
                ParseResult::Stmt(s) => Rc::new(s),
                _ => return Err(ParseError::new(\"Expected statement\".to_string(), 0, 0)),
            };

            Ok(ParseResult::Stmt(Statement::Labeled(LabeledStatement {
                label: label_id,
                body: body_stmt,
                span,
            })))
        } else {
            Err(ParseError::new(\"Expected labeled statement parts\".to_string(), 0, 0))
        }
    }",
        )
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

// Declare statements (TypeScript ambient declarations)

fn rule_declare_statement(r: &RuleBuilder) -> Combinator {
    // TypeScript `declare` keyword for ambient declarations
    // These have no runtime effect - the declaration is parsed and passed through
    r.sequence((kw(r, "declare"), r.choice((
        r.parse("declare_module"),
        r.parse("declare_function"),
        r.parse("variable_declaration"),
        r.parse("class_declaration"),
        r.parse("enum_declaration"),
        r.parse("namespace_declaration"),
        r.parse("interface_declaration"),
        r.parse("type_alias_declaration"),
    )))).ast("|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
        // Just return the inner declaration, stripping the 'declare' keyword
        let items = result.into_list();
        // The second item is the actual declaration
        items.into_iter().nth(1).ok_or_else(|| ParseError::new(\"Expected declaration after declare\".to_string(), 0, 0))
    }")
}

fn rule_declare_function(r: &RuleBuilder) -> Combinator {
    // `declare function foo(x: T): R;` - ambient function declaration (no body)
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
        r.parse("semicolon"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [async?, function, *?, name?, type_params?, (, params?, ), return_type?, ;]
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
            // No body for ambient functions - use empty block
            let body = ParseResult::Stmt(Statement::Block(BlockStatement {
                body: Rc::from(vec![]),
                span,
            }));
            create_function_decl(async_kw, generator, name, params, body, span)
        } else {
            Err(ParseError::new(\"Expected ambient function declaration parts\".to_string(), 0, 0))
        }
    }",
    )
}

fn rule_declare_module(r: &RuleBuilder) -> Combinator {
    // `declare module "name" { ... }` - ambient module declaration
    // Use sequence with explicit whitespace after string_literal since it doesn't consume ws
    r.sequence((
        kw(r, "module"),
        r.sequence((r.parse("string_literal"), r.parse("ws"))),
        op(r, "{"),
        r.zero_or_more(r.parse("statement")),
        op(r, "}"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // Parse as a namespace declaration with the string name as identifier
        let mut items = result.into_list().into_iter();
        let _ = items.next(); // skip 'module' keyword
        let name_result = items.next().unwrap_or(ParseResult::None);
        let name_str = match name_result {
            ParseResult::Expr(Expression::Literal(lit)) => {
                match lit.value {
                    LiteralValue::String(s) => s.to_string(),
                    _ => String::new(),
                }
            }
            _ => String::new(),
        };
        let _ = items.next(); // skip '{'
        let body_list = items.next().unwrap_or(ParseResult::List(vec![]));
        let mut body = Vec::new();
        for item in body_list.into_list() {
            if let ParseResult::Stmt(stmt) = item {
                body.push(stmt);
            }
        }
        Ok(ParseResult::Stmt(Statement::NamespaceDeclaration(Box::new(NamespaceDeclaration {
            id: Identifier { name: JsString::from(name_str), span },
            body: Rc::from(body),
            span,
        }))))
    }",
    )
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
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [import, type?, import_clause, from, string, ws, semicolon]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _import = iter.next();
            let type_part = iter.next();
            let clause_part = iter.next().unwrap_or(ParseResult::None);
            let _from = iter.next();
            let string_part = iter.next().unwrap_or(ParseResult::None);

            let type_only = !matches!(type_part, Some(ParseResult::None) | None);

            // Parse source string
            let source = if let ParseResult::Text(value, sp) = string_part {
                let stripped = strip_string_quotes(value.as_ref());
                StringLiteral { value: stripped.into(), span: sp }
            } else {
                return Err(ParseError::new(\"Expected import source string\".to_string(), 0, 0));
            };

            // Parse import clause into specifiers
            let specifiers = parse_import_clause(clause_part, span.clone());

            Ok(ParseResult::Stmt(Statement::Import(Box::new(ImportDeclaration {
                specifiers,
                source,
                type_only,
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected import declaration parts\".to_string(), 0, 0))
        }
    }",
    )
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
        // Variant 1: export default expression/declaration
        r.sequence((
            kw(r, "export"),
            r.capture(r.lit("default")),  // Capture "default" to distinguish from variant 4
            r.parse("ws"),
            r.parse("export_default_expression"),
        )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            // [export, \"default\", ws, expr/stmt]
            if let ParseResult::List(parts) = result {
                let decl_part = parts.into_iter().nth(3).unwrap_or(ParseResult::None);
                let declaration = match decl_part {
                    ParseResult::Stmt(s) => Some(Box::new(s)),
                    ParseResult::Expr(e) => Some(Box::new(Statement::Expression(ExpressionStatement {
                        expression: Rc::new(e),
                        span: span.clone(),
                    }))),
                    ParseResult::List(inner) => {
                        // expression + semicolon case
                        if let Some(ParseResult::Expr(e)) = inner.into_iter().next() {
                            Some(Box::new(Statement::Expression(ExpressionStatement {
                                expression: Rc::new(e),
                                span: span.clone(),
                            })))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                Ok(ParseResult::Stmt(Statement::Export(Box::new(ExportDeclaration {
                    declaration,
                    specifiers: vec![],
                    source: None,
                    namespace_export: None,
                    default: true,
                    type_only: false,
                    span,
                }))))
            } else {
                Err(ParseError::new(\"Expected export default parts\".to_string(), 0, 0))
            }
        }"),
        // Variant 2: export { named } from "source"
        r.sequence((
            kw(r, "export"),
            r.optional(kw(r, "type")),
            r.parse("named_exports"),
            r.optional(r.sequence((kw(r, "from"), r.parse("string_literal"), r.parse("ws")))),
            r.parse("semicolon"),
        )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            // [export, type?, named_exports, (from, string, ws)?, semicolon]
            if let ParseResult::List(parts) = result {
                let mut iter = parts.into_iter();
                let _export = iter.next();
                let type_part = iter.next();
                let named_part = iter.next().unwrap_or(ParseResult::None);
                let from_part = iter.next().unwrap_or(ParseResult::None);

                let type_only = !matches!(type_part, Some(ParseResult::None) | None);

                // Parse named exports: List([{, List([specifiers]), }])
                let mut specifiers = vec![];
                if let ParseResult::List(named_parts) = named_part {
                    if let Some(ParseResult::List(spec_list)) = named_parts.into_iter().nth(1) {
                        for spec in spec_list {
                            if let Some(es) = parse_export_specifier(spec, span.clone()) {
                                specifiers.push(es);
                            }
                        }
                    }
                }

                // Parse optional source (strip quotes)
                let source = if let ParseResult::List(from_parts) = from_part {
                    // [from, string_literal, ws]
                    from_parts.into_iter().nth(1).and_then(|s| {
                        if let ParseResult::Text(value, sp) = s {
                            let stripped = strip_string_quotes(value.as_ref());
                            Some(StringLiteral { value: stripped.into(), span: sp })
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                Ok(ParseResult::Stmt(Statement::Export(Box::new(ExportDeclaration {
                    declaration: None,
                    specifiers,
                    source,
                    namespace_export: None,
                    default: false,
                    type_only,
                    span,
                }))))
            } else {
                Err(ParseError::new(\"Expected named export parts\".to_string(), 0, 0))
            }
        }"),
        // Variant 3: export [type] * as ns from "source"
        r.sequence((
            kw(r, "export"),
            r.optional(kw(r, "type")),
            op(r, "*"),
            r.optional(r.sequence((kw(r, "as"), r.parse("identifier")))),
            kw(r, "from"),
            r.parse("string_literal"),
            r.parse("ws"),
            r.parse("semicolon"),
        )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            // [export, type?, *, (as, ident)?, from, string, ws, semicolon]
            if let ParseResult::List(parts) = result {
                let mut iter = parts.into_iter();
                let _export = iter.next();
                let type_part = iter.next();
                let _star = iter.next();
                let as_part = iter.next().unwrap_or(ParseResult::None);
                let _from = iter.next();
                let string_part = iter.next().unwrap_or(ParseResult::None);

                let type_only = !matches!(type_part, Some(ParseResult::None) | None);

                // Parse optional namespace identifier
                let namespace_export = if let ParseResult::List(as_parts) = as_part {
                    as_parts.into_iter().nth(1).and_then(|id| {
                        if let ParseResult::Ident(ident) = id {
                            Some(ident)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                // Parse source string (strip quotes)
                let source = if let ParseResult::Text(value, sp) = string_part {
                    let stripped = strip_string_quotes(value.as_ref());
                    Some(StringLiteral { value: stripped.into(), span: sp })
                } else {
                    None
                };

                Ok(ParseResult::Stmt(Statement::Export(Box::new(ExportDeclaration {
                    declaration: None,
                    specifiers: vec![],
                    source,
                    namespace_export,
                    default: false,
                    type_only,
                    span,
                }))))
            } else {
                Err(ParseError::new(\"Expected namespace export parts\".to_string(), 0, 0))
            }
        }"),
        // Variant 4: export declaration
        r.sequence((
            kw(r, "export"),
            r.optional(kw(r, "type")),
            r.parse("exportable_declaration"),
        )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            // [export, type?, declaration]
            if let ParseResult::List(parts) = result {
                let mut iter = parts.into_iter();
                let _export = iter.next();
                let type_part = iter.next();
                let decl_part = iter.next().unwrap_or(ParseResult::None);

                let type_only = !matches!(type_part, Some(ParseResult::None) | None);
                let declaration = match decl_part {
                    ParseResult::Stmt(s) => Some(Box::new(s)),
                    _ => None,
                };

                Ok(ParseResult::Stmt(Statement::Export(Box::new(ExportDeclaration {
                    declaration,
                    specifiers: vec![],
                    source: None,
                    namespace_export: None,
                    default: false,
                    type_only,
                    span,
                }))))
            } else {
                Err(ParseError::new(\"Expected export declaration parts\".to_string(), 0, 0))
            }
        }"),
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
        r.parse("namespace_declaration"),
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
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [type, id, type_params?, =, type, ;]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _type_kw = iter.next();
            let id = iter.next().unwrap_or(ParseResult::None);
            let _type_params = iter.next();
            let _eq = iter.next();
            let _type_annotation = iter.next();
            // TypeAlias is a no-op at runtime, so we create a placeholder type
            let identifier = to_ident(id)?;
            Ok(ParseResult::Stmt(Statement::TypeAlias(Box::new(TypeAliasDeclaration {
                id: identifier,
                type_parameters: None,
                // Placeholder: any type since we don't type-check at runtime
                type_annotation: Box::new(TypeAnnotation::Keyword(TypeKeyword {
                    keyword: TypeKeywordKind::Any,
                    span,
                })),
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected type alias declaration parts\".to_string(), 0, 0))
        }
    }",
    )
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
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [interface, id, type_params?, extends?, body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _interface_kw = iter.next();
            let id = iter.next().unwrap_or(ParseResult::None);
            let _type_params = iter.next();
            let _extends = iter.next();
            let _body = iter.next();

            let identifier = to_ident(id)?;

            // Types are stripped at runtime, so we use empty body
            Ok(ParseResult::Stmt(Statement::InterfaceDeclaration(Box::new(InterfaceDeclaration {
                id: identifier,
                type_parameters: None,
                extends: vec![],
                body: vec![],
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected interface declaration parts\".to_string(), 0, 0))
        }
    }",
    )
}

fn rule_enum_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(kw(r, "const")),
        kw(r, "enum"),
        r.parse("identifier"),
        op(r, "{"),
        r.optional(r.separated_by_trailing(r.parse("enum_member"), op(r, ","))),
        op(r, "}"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
}

fn rule_enum_member(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        // Use assignment_expression to avoid comma being consumed
        r.optional(r.sequence((op(r, "="), r.parse("assignment_expression")))),
    ))
}

fn rule_namespace_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "namespace"),
        r.parse("identifier"),
        r.parse("block_statement"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [namespace, id, body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _namespace_kw = iter.next();
            let id = iter.next().unwrap_or(ParseResult::None);
            let body = iter.next().unwrap_or(ParseResult::None);

            let identifier = to_ident(id)?;
            let body_stmts: Vec<Statement> = match body {
                ParseResult::Stmt(Statement::Block(b)) => b.body.iter().cloned().collect(),
                _ => vec![],
            };

            Ok(ParseResult::Stmt(Statement::NamespaceDeclaration(Box::new(NamespaceDeclaration {
                id: identifier,
                body: Rc::from(body_stmts),
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected namespace declaration parts\".to_string(), 0, 0))
        }
    }",
    )
}

// Expressions

/// Full expression including comma operator (sequence expression)
fn rule_expression(r: &RuleBuilder) -> Combinator {
    // expression = assignment_expression (, assignment_expression)*
    r.sequence((
        r.parse("assignment_expression"),
        r.zero_or_more(r.sequence((op(r, ","), r.parse("assignment_expression")))),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        let items = result.into_list();
        let mut iter = items.into_iter();
        let first = iter.next().unwrap_or(ParseResult::None);
        let rest = iter.next();

        // If there are comma-separated items, create a sequence expression
        if let Some(ParseResult::List(comma_items)) = rest {
            if comma_items.is_empty() {
                return Ok(first);
            }
            let mut expressions = vec![to_expr(first)?];
            for item in comma_items {
                // Each item is [comma, expr]
                if let ParseResult::List(parts) = item {
                    if let Some(expr_result) = parts.into_iter().nth(1) {
                        expressions.push(to_expr(expr_result)?);
                    }
                }
            }
            if expressions.len() == 1 {
                return Ok(ParseResult::Expr(expressions.remove(0)));
            }
            Ok(ParseResult::Expr(Expression::Sequence(SequenceExpression {
                expressions,
                span,
            })))
        } else {
            Ok(first)
        }
    }",
    )
}

/// Assignment expression (everything except comma operator)
fn rule_assignment_expression(r: &RuleBuilder) -> Combinator {
    // Pratt parsing for expressions, followed by optional `as Type` suffixes
    // Infix operators use leading ws pattern: after operand consumes trailing ws,
    // we need to consume any ws before the operator before parsing right operand
    r.sequence((
        r.pratt(r.parse("primary"), |ops| {
            ops
                // === Assignment operators (right-associative) ===
                .infix(
                    ws_infix(r, ">>>="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::URShiftAssign, s)",
                )
                .infix(
                    ws_infix(r, "**="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::ExpAssign, s)",
                )
                .infix(
                    ws_infix(r, "<<="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::LShiftAssign, s)",
                )
                .infix(
                    ws_infix(r, ">>="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::RShiftAssign, s)",
                )
                .infix(
                    ws_infix(r, "&&="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::AndAssign, s)",
                )
                .infix(
                    ws_infix(r, "||="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::OrAssign, s)",
                )
                .infix(
                    ws_infix(r, "??="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::NullishAssign, s)",
                )
                .infix(
                    ws_infix(r, "+="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::AddAssign, s)",
                )
                .infix(
                    ws_infix(r, "-="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::SubAssign, s)",
                )
                .infix(
                    ws_infix(r, "*="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::MulAssign, s)",
                )
                .infix(
                    ws_infix(r, "/="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::DivAssign, s)",
                )
                .infix(
                    ws_infix(r, "%="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::ModAssign, s)",
                )
                .infix(
                    ws_infix(r, "&="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::BitAndAssign, s)",
                )
                .infix(
                    ws_infix(r, "|="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::BitOrAssign, s)",
                )
                .infix(
                    ws_infix(r, "^="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::BitXorAssign, s)",
                )
                .infix(
                    ws_infix(r, "="),
                    2,
                    Assoc::Right,
                    "|l, r, s| assign(l, r, AssignmentOp::Assign, s)",
                )
                // === Ternary operator ===
                .ternary("?", ":", 3, "|c, t, e, s| conditional(c, t, e, s)")
                // === Nullish coalescing ===
                .infix(
                    ws_infix(r, "??"),
                    4,
                    Assoc::Left,
                    "|l, r, s| logical(l, r, LogicalOp::NullishCoalescing, s)",
                )
                // === Logical OR ===
                .infix(
                    ws_infix(r, "||"),
                    5,
                    Assoc::Left,
                    "|l, r, s| logical(l, r, LogicalOp::Or, s)",
                )
                // === Logical AND ===
                .infix(
                    ws_infix(r, "&&"),
                    6,
                    Assoc::Left,
                    "|l, r, s| logical(l, r, LogicalOp::And, s)",
                )
                // === Bitwise OR ===
                .infix(
                    ws_infix(r, "|"),
                    7,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::BitOr, s)",
                )
                // === Bitwise XOR ===
                .infix(
                    ws_infix(r, "^"),
                    8,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::BitXor, s)",
                )
                // === Bitwise AND ===
                .infix(
                    ws_infix(r, "&"),
                    9,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::BitAnd, s)",
                )
                // === Equality ===
                .infix(
                    ws_infix(r, "==="),
                    10,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::StrictEq, s)",
                )
                .infix(
                    ws_infix(r, "!=="),
                    10,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::StrictNotEq, s)",
                )
                .infix(
                    ws_infix(r, "=="),
                    10,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Eq, s)",
                )
                .infix(
                    ws_infix(r, "!="),
                    10,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::NotEq, s)",
                )
                // === Relational ===
                .infix(
                    ws_infix(r, "<="),
                    11,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::LtEq, s)",
                )
                .infix(
                    ws_infix(r, ">="),
                    11,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::GtEq, s)",
                )
                .infix(
                    ws_infix(r, "<"),
                    11,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Lt, s)",
                )
                .infix(
                    ws_infix(r, ">"),
                    11,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Gt, s)",
                )
                // Keyword operators with ws + keyword + not_followed_by(ident_cont)
                .infix(
                    r.sequence((
                        r.parse("ws"),
                        r.lit("in"),
                        r.not_followed_by(r.ident_cont()),
                    )),
                    11,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::In, s)",
                )
                .infix(
                    r.sequence((
                        r.parse("ws"),
                        r.lit("instanceof"),
                        r.not_followed_by(r.ident_cont()),
                    )),
                    11,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Instanceof, s)",
                )
                // === Shift ===
                .infix(
                    ws_infix(r, ">>>"),
                    12,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::URShift, s)",
                )
                .infix(
                    ws_infix(r, "<<"),
                    12,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::LShift, s)",
                )
                .infix(
                    ws_infix(r, ">>"),
                    12,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::RShift, s)",
                )
                // === Additive ===
                .infix(
                    ws_infix(r, "+"),
                    13,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Add, s)",
                )
                .infix(
                    ws_infix(r, "-"),
                    13,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Sub, s)",
                )
                // === Multiplicative (** must come before * so longer operator matches first) ===
                .infix(
                    ws_infix(r, "**"),
                    15,
                    Assoc::Right,
                    "|l, r, s| binary(l, r, BinaryOp::Exp, s)",
                )
                .infix(
                    ws_infix(r, "*"),
                    14,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Mul, s)",
                )
                .infix(
                    ws_infix(r, "/"),
                    14,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Div, s)",
                )
                .infix(
                    ws_infix(r, "%"),
                    14,
                    Assoc::Left,
                    "|l, r, s| binary(l, r, BinaryOp::Mod, s)",
                )
                // === Prefix operators ===
                // Note: prefix operators need leading ws to work as right operand of infix
                .prefix(ws_prefix(r, "++"), 16, "|e, s| update(e, UpdateOp::Increment, true, s)")
                .prefix(ws_prefix(r, "--"), 16, "|e, s| update(e, UpdateOp::Decrement, true, s)")
                .prefix(ws_prefix(r, "-"), 16, "|e, s| unary(e, UnaryOp::Minus, s)")
                .prefix(ws_prefix(r, "+"), 16, "|e, s| unary(e, UnaryOp::Plus, s)")
                .prefix(ws_prefix(r, "!"), 16, "|e, s| unary(e, UnaryOp::Not, s)")
                .prefix(ws_prefix(r, "~"), 16, "|e, s| unary(e, UnaryOp::BitNot, s)")
                .prefix(ws_prefix_kw(r, "typeof"), 16, "|e, s| unary(e, UnaryOp::Typeof, s)")
                .prefix(ws_prefix_kw(r, "void"), 16, "|e, s| unary(e, UnaryOp::Void, s)")
                .prefix(ws_prefix_kw(r, "delete"), 16, "|e, s| unary(e, UnaryOp::Delete, s)")
                .prefix(ws_prefix_kw(r, "await"), 16, "|e, s| await_expr(e, s)")
                // === Postfix operators (highest precedence) ===
                .postfix("++", 17, "|e, s| update(e, UpdateOp::Increment, false, s)")
                .postfix("--", 17, "|e, s| update(e, UpdateOp::Decrement, false, s)")
                // TypeScript non-null assertion: x! (must not be followed by = to avoid conflict with !=)
                .postfix(
                    r.sequence((r.lit("!"), r.not_followed_by(r.lit("=")))),
                    17,
                    "|e, s| non_null(e, s)",
                )
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
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
    }",
    )
}

fn rule_primary(r: &RuleBuilder) -> Combinator {
    // Start with ws to handle whitespace after infix operators
    r.sequence((r.parse("ws"), r.parse("primary_inner"))).ast(
        "|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
        // Return the second element (primary_inner), skip the ws
        if let ParseResult::List(mut items) = result {
            Ok(items.pop().unwrap_or(ParseResult::None))
        } else {
            Ok(result)
        }
    }",
    )
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
        r.parse("identifier").ast(
            "|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
            let ident = to_ident(result)?;
            Ok(ParseResult::Expr(Expression::Identifier(ident)))
        }",
        ),
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
        r.sequence((r.parse("regexp_literal"), r.parse("ws"))).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
            let text = result.into_text();
            let (pattern, flags) = parse_regexp_literal(&text);
            Ok(ParseResult::Expr(Expression::Literal(Box::new(Literal { value: LiteralValue::RegExp { pattern, flags }, span }))))
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
    kw(r, "this").ast(
        "|_result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        Ok(ParseResult::Expr(Expression::This(span)))
    }",
    )
}

fn rule_super_expression(r: &RuleBuilder) -> Combinator {
    kw(r, "super").ast(
        "|_result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        Ok(ParseResult::Expr(Expression::Super(span)))
    }",
    )
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
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        create_array_expr(result, span)
    }",
    )
}

fn rule_array_element(r: &RuleBuilder) -> Combinator {
    // Use assignment_expression to avoid comma being consumed
    r.choice((r.parse("spread_element"), r.parse("assignment_expression")))
}

fn rule_object_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.optional(r.separated_by_trailing(r.parse("object_property"), op(r, ","))),
        op(r, "}"),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        create_object_expr(result, span)
    }",
    )
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
        // Use assignment_expression (not expression) to avoid comma being consumed
        r.parse("assignment_expression"),
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
        r.parse("private_name"),
        r.parse("identifier"),
        r.sequence((r.parse("string_literal"), r.parse("ws"))),
        r.sequence((r.parse("number_literal"), r.parse("ws"))),
        r.sequence((op(r, "["), r.parse("expression"), op(r, "]"))),
    ))
}

fn rule_private_name(r: &RuleBuilder) -> Combinator {
    r.capture(r.sequence((r.lit("#"), r.ident_start(), r.zero_or_more(r.ident_cont()))))
}

fn rule_spread_element(r: &RuleBuilder) -> Combinator {
    // Use assignment_expression to avoid comma being consumed
    r.sequence((op(r, "..."), r.parse("assignment_expression")))
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
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [async?, function, *, name?, type_params?, (, params?, ), type_ann?, body]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let async_kw = iter.next().unwrap_or(ParseResult::None);
            let _function_kw = iter.next();
            let generator = iter.next().unwrap_or(ParseResult::None);
            let name = iter.next().unwrap_or(ParseResult::None);
            let _type_params = iter.next();
            let _open_paren = iter.next();
            let params = iter.next().unwrap_or(ParseResult::None);
            let _close_paren = iter.next();
            let _type_ann = iter.next();
            let body = iter.next().unwrap_or(ParseResult::None);

            let is_async = !matches!(async_kw, ParseResult::None);
            let is_generator = !matches!(generator, ParseResult::None);
            let id = match name {
                ParseResult::Ident(id) => Some(id),
                _ => None,
            };
            let params_vec = params_to_vec(params, &span);
            let body_block = match body {
                ParseResult::Stmt(Statement::Block(b)) => Rc::new(b),
                _ => Rc::new(BlockStatement { body: Rc::from(vec![]), span: span.clone() }),
            };

            Ok(ParseResult::Expr(Expression::Function(Box::new(FunctionExpression {
                id,
                params: Rc::from(params_vec),
                return_type: None,
                type_parameters: None,
                body: body_block,
                generator: is_generator,
                async_: is_async,
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected function expression parts\".to_string(), 0, 0))
        }
    }",
    )
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
        ))
        .ast(
            "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
        }",
        ),
        // Single unparenthesized param: x => body
        r.sequence((
            r.optional(kw(r, "async")),
            r.parse("identifier"),
            op(r, "=>"),
            r.parse("arrow_body"),
        ))
        .ast(
            "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
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
        }",
        ),
    ))
}

fn rule_arrow_body(r: &RuleBuilder) -> Combinator {
    // Arrow body uses assignment_expression: () => a, b parses as ((() => a), b)
    r.choice((r.parse("block_statement"), r.parse("assignment_expression")))
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
            r.zero_or_more(r.sequence((
                r.parse("template_middle"),
                r.parse("ws"),
                r.parse("expression"),
            ))),
            r.parse("template_tail"),
            r.parse("ws"),
        )),
    ))
}

fn rule_new_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "new"),
        r.parse("primary"), // Use primary instead of expression to avoid consuming too much
        r.optional(r.parse("type_arguments")), // Type arguments like <void> or <string, number>
        r.optional(r.sequence((op(r, "("), r.optional(r.parse("argument_list")), op(r, ")")))),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [new, callee, type_args?, args?]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _new_kw = iter.next();
            let callee = iter.next().unwrap_or(ParseResult::None);
            let type_args_result = iter.next().unwrap_or(ParseResult::None);
            let args_result = iter.next().unwrap_or(ParseResult::None);

            // Parse type arguments
            let type_arguments = parse_type_arguments(type_args_result);

            let arguments: Vec<Argument> = match args_result {
                ParseResult::List(arg_parts) => {
                    // [(, args?, )]
                    let mut arg_iter = arg_parts.into_iter();
                    let _open_paren = arg_iter.next();
                    let args_list = arg_iter.next().unwrap_or(ParseResult::None);
                    let _close_paren = arg_iter.next();

                    match args_list {
                        ParseResult::List(items) => {
                            items.into_iter().filter_map(|a| {
                                to_expr(a).ok().map(Argument::Expression)
                            }).collect()
                        }
                        ParseResult::None => vec![],
                        other => {
                            if let Ok(e) = to_expr(other) {
                                vec![Argument::Expression(e)]
                            } else {
                                vec![]
                            }
                        }
                    }
                }
                _ => vec![],
            };

            Ok(ParseResult::Expr(Expression::New(Box::new(NewExpression {
                callee: Rc::new(to_expr(callee)?),
                arguments,
                type_arguments,
                span,
            }))))
        } else {
            Err(ParseError::new(\"Expected new expression parts\".to_string(), 0, 0))
        }
    }",
    )
}

fn rule_yield_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "yield"),
        r.optional(op(r, "*")),
        r.optional(r.parse("expression")),
    ))
    .ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [yield, optional *, optional expression]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _yield_kw = iter.next();
            let delegate_part = iter.next().unwrap_or(ParseResult::None);
            let arg_part = iter.next().unwrap_or(ParseResult::None);

            let delegate = !matches!(delegate_part, ParseResult::None);
            let argument = match arg_part {
                ParseResult::None => None,
                other => to_expr(other).ok().map(Rc::new),
            };

            Ok(ParseResult::Expr(Expression::Yield(YieldExpression {
                argument,
                delegate,
                span,
            })))
        } else {
            Err(ParseError::new(\"Expected yield expression parts\".to_string(), 0, 0))
        }
    }",
    )
}

fn rule_parenthesized(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, "("), r.parse("expression"), op(r, ")")))
        .ast(
            "|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
            // Extract expression from sequence [open_paren, expr, close_paren]
            if let ParseResult::List(mut items) = result {
                Ok(items.remove(1))
            } else {
                Ok(result)
            }
        }",
        )
}

fn rule_argument_list(r: &RuleBuilder) -> Combinator {
    r.separated_by(r.parse("argument"), op(r, ","))
}

fn rule_argument(r: &RuleBuilder) -> Combinator {
    // Use assignment_expression to avoid comma being consumed
    r.choice((r.parse("spread_element"), r.parse("assignment_expression")))
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
        // Use assignment_expression to avoid comma being consumed
        r.optional(r.sequence((op(r, "="), r.parse("assignment_expression")))),
    ))
}

fn rule_accessibility_modifier(r: &RuleBuilder) -> Combinator {
    r.choice((kw(r, "public"), kw(r, "private"), kw(r, "protected")))
}

fn rule_decorator(r: &RuleBuilder) -> Combinator {
    // Use assignment_expression to avoid comma being consumed
    r.sequence((op(r, "@"), r.parse("assignment_expression"), r.parse("ws")))
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
    r.parse("identifier").ast(
        "|result: ParseResult, _span: Span| -> Result<ParseResult, ParseError> {
        let id = to_ident(result)?;
        Ok(ParseResult::Pat(Pattern::Identifier(id)))
    }",
    )
}

fn rule_object_pattern(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "{"),
        r.optional(r.separated_by_trailing(r.parse("object_pattern_property"), op(r, ","))),
        op(r, "}"),
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [{, properties?, }]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _open_brace = iter.next();
            let properties_result = iter.next().unwrap_or(ParseResult::None);
            let _close_brace = iter.next();

            let properties: Vec<ObjectPatternProperty> = match properties_result {
                ParseResult::List(items) => {
                    items.into_iter().filter_map(|item| {
                        match item {
                            ParseResult::Pat(Pattern::Rest(r)) => Some(ObjectPatternProperty::Rest(r)),
                            ParseResult::List(prop_parts) => {
                                // [key, colon_pattern?, default?]
                                let mut prop_iter = prop_parts.into_iter();
                                let key_result = prop_iter.next()?;
                                let colon_pattern = prop_iter.next();
                                let _default = prop_iter.next();

                                // Determine key and value
                                let key = match &key_result {
                                    ParseResult::Ident(id) => ObjectPropertyKey::Identifier(id.clone()),
                                    ParseResult::Expr(Expression::Literal(lit)) => {
                                        match &lit.value {
                                            LiteralValue::String(s) => ObjectPropertyKey::String(StringLiteral {
                                                value: s.clone(),
                                                span: lit.span.clone(),
                                            }),
                                            LiteralValue::Number(_) => ObjectPropertyKey::Number(*lit.clone()),
                                            _ => return None,
                                        }
                                    }
                                    _ => return None,
                                };

                                // Check if shorthand (no : pattern)
                                let (value, shorthand) = match colon_pattern {
                                    Some(ParseResult::List(colon_parts)) => {
                                        // [colon, pattern]
                                        let pattern = colon_parts.into_iter().nth(1).and_then(|p| to_pattern(p).ok())?;
                                        (pattern, false)
                                    }
                                    _ => {
                                        // Shorthand: { x } means { x: x }
                                        if let ParseResult::Ident(id) = key_result {
                                            (Pattern::Identifier(id.clone()), true)
                                        } else {
                                            return None;
                                        }
                                    }
                                };

                                Some(ObjectPatternProperty::KeyValue {
                                    key,
                                    value,
                                    shorthand,
                                    span: span.clone(),
                                })
                            }
                            _ => None,
                        }
                    }).collect()
                }
                ParseResult::None => vec![],
                _ => vec![],
            };

            Ok(ParseResult::Pat(Pattern::Object(ObjectPattern { properties, type_annotation: None, span })))
        } else {
            Err(ParseError::new(\"Expected object pattern parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_object_pattern_property(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("rest_pattern"),
        r.sequence((
            r.parse("property_key"),
            r.optional(r.sequence((op(r, ":"), r.parse("pattern")))),
            // Use assignment_expression to avoid comma being consumed
            r.optional(r.sequence((op(r, "="), r.parse("assignment_expression")))),
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
    )).ast("|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [[, elements?, ]]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _open_bracket = iter.next();
            let elements_result = iter.next().unwrap_or(ParseResult::None);
            let _close_bracket = iter.next();

            let elements: Vec<Option<Pattern>> = match elements_result {
                ParseResult::List(items) => {
                    items.into_iter().map(|item| {
                        match item {
                            ParseResult::Pat(p) => Some(p),
                            ParseResult::List(elem_parts) => {
                                // [pattern, default?] - for now ignore default, just get pattern
                                let mut elem_iter = elem_parts.into_iter();
                                elem_iter.next().and_then(|p| to_pattern(p).ok())
                            }
                            ParseResult::None => None,
                            _ => None,
                        }
                    }).collect()
                }
                ParseResult::None => vec![],
                _ => vec![],
            };

            Ok(ParseResult::Pat(Pattern::Array(ArrayPattern { elements, type_annotation: None, span })))
        } else {
            Err(ParseError::new(\"Expected array pattern parts\".to_string(), 0, 0))
        }
    }")
}

fn rule_array_pattern_element(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("pattern"),
        // Use assignment_expression to avoid comma being consumed
        r.optional(r.sequence((op(r, "="), r.parse("assignment_expression")))),
    ))
}

fn rule_rest_pattern(r: &RuleBuilder) -> Combinator {
    r.sequence((op(r, "..."), r.parse("pattern"))).ast(
        "|result: ParseResult, span: Span| -> Result<ParseResult, ParseError> {
        // [..., pattern]
        if let ParseResult::List(parts) = result {
            let mut iter = parts.into_iter();
            let _spread = iter.next();
            let pattern = iter.next().unwrap_or(ParseResult::None);

            let inner = to_pattern(pattern)?;
            Ok(ParseResult::Pat(Pattern::Rest(RestElement {
                argument: Box::new(inner),
                type_annotation: None,
                span,
            })))
        } else {
            Err(ParseError::new(\"Expected rest pattern parts\".to_string(), 0, 0))
        }
    }",
    )
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
    // base_type followed by optional type suffixes:
    // - [] for array types
    // - [type] for indexed access types (e.g., Order["status"])
    r.sequence((
        r.parse("base_type"),
        r.zero_or_more(r.sequence((
            op(r, "["),
            r.optional(r.parse("type")), // empty for array, has type for indexed access
            op(r, "]"),
        ))),
    ))
}

fn rule_base_type(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("keyword_type"),
        // typeof, keyof, infer must come before type_reference
        // because type_reference matches any identifier
        r.parse("typeof_type"),
        r.parse("keyof_type"),
        r.parse("infer_type"),
        r.parse("type_reference"),
        r.parse("literal_type"),
        r.parse("object_type"),
        r.parse("tuple_type"),
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
        r.optional(r.optional(r.parse("type_member_separator"))),
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
        r.optional(r.parse("type_member_separator")),
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
        r.optional(r.parse("type_member_separator")),
    ))
}

fn rule_call_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.parse("type_parameters")),
        op(r, "("),
        r.optional(r.parse("parameter_list")),
        op(r, ")"),
        r.optional(r.parse("type_annotation")),
        r.optional(r.parse("type_member_separator")),
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
        r.optional(r.parse("type_member_separator")),
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
