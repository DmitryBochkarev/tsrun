//! TypeScript Grammar Definition
//!
//! This module defines the complete TypeScript grammar using the trampoline-parser DSL.
//! The grammar generates a fully trampoline-based lexer and parser.

use trampoline_parser::{Assoc, AstConfigBuilder, Combinator, CombinatorExt, Grammar, RuleBuilder};

/// Build the TypeScript grammar
pub fn typescript_grammar() -> Grammar {
    Grammar::new()
        .ast_config(ast_config)
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
        // Program
        .rule("program", rule_program)
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
        .rule("array_type", rule_array_type)
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
fn to_expr(result: ParseResult) -> Expression {
    match result {
        ParseResult::Ast(boxed) => *boxed.downcast::<Expression>().expect("Expected Expression"),
        _ => panic!("Expected Expression AST, got {:?}", result),
    }
}

/// Create binary expression
fn binary(left: ParseResult, right: ParseResult, op: BinaryOp, span: Span) -> Expression {
    Expression::Binary(BinaryExpression {
        operator: op,
        left: Rc::new(to_expr(left)),
        right: Rc::new(to_expr(right)),
        span,
    })
}

/// Create logical expression
fn logical(left: ParseResult, right: ParseResult, op: LogicalOp, span: Span) -> Expression {
    Expression::Logical(LogicalExpression {
        operator: op,
        left: Rc::new(to_expr(left)),
        right: Rc::new(to_expr(right)),
        span,
    })
}

/// Create unary expression
fn unary(operand: ParseResult, op: UnaryOp, span: Span) -> Expression {
    Expression::Unary(UnaryExpression {
        operator: op,
        argument: Rc::new(to_expr(operand)),
        prefix: true,
        span,
    })
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

// === Rule Functions ===

fn rule_program(r: &RuleBuilder) -> Combinator {
    r.zero_or_more(r.parse("statement"))
}

fn rule_statement(r: &RuleBuilder) -> Combinator {
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
        r.choice((kw(r, "let"), kw(r, "const"), kw(r, "var"))),
        r.separated_by(r.parse("variable_declarator"), op(r, ",")),
        r.parse("semicolon"),
    ))
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
    ))
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
    ))
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
    ))
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
    ))
}

fn rule_return_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "return"),
        r.optional(r.parse("expression")),
        r.parse("semicolon"),
    ))
}

fn rule_break_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "break"),
        r.optional(r.parse("identifier")),
        r.parse("semicolon"),
    ))
}

fn rule_continue_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "continue"),
        r.optional(r.parse("identifier")),
        r.parse("semicolon"),
    ))
}

fn rule_throw_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        kw(r, "throw"),
        r.parse("expression"),
        r.parse("semicolon"),
    ))
}

fn rule_debugger_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((kw(r, "debugger"), r.parse("semicolon")))
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
        .ast("|result: ParseResult, span: Span| {
            let items = result.into_list();
            let expr = items.into_iter().next().unwrap().into_ast::<Expression>();
            Statement::Expression(ExpressionStatement {
                expression: Rc::new(expr),
                span,
            })
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
    ))
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
    r.pratt(r.parse("primary"), |ops| {
        ops
            // Assignment - skip for now (complex, need assignment target conversion)
            // Ternary - skip for now
            // Nullish coalescing
            .infix("??", 4, Assoc::Left, "|l, r, s| logical(l, r, LogicalOp::NullishCoalescing, s)")
            // Logical OR
            .infix("||", 5, Assoc::Left, "|l, r, s| logical(l, r, LogicalOp::Or, s)")
            // Logical AND
            .infix("&&", 6, Assoc::Left, "|l, r, s| logical(l, r, LogicalOp::And, s)")
            // Bitwise OR
            .infix("|", 7, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::BitOr, s)")
            // Bitwise XOR
            .infix("^", 8, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::BitXor, s)")
            // Bitwise AND
            .infix("&", 9, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::BitAnd, s)")
            // Equality
            .infix("==", 10, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Eq, s)")
            .infix("!=", 10, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::NotEq, s)")
            .infix("===", 10, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::StrictEq, s)")
            .infix("!==", 10, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::StrictNotEq, s)")
            // Relational
            .infix("<", 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Lt, s)")
            .infix(">", 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Gt, s)")
            .infix("<=", 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::LtEq, s)")
            .infix(">=", 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::GtEq, s)")
            .infix_kw("in", 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::In, s)")
            .infix_kw("instanceof", 11, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Instanceof, s)")
            // Shift
            .infix("<<", 12, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::LShift, s)")
            .infix(">>", 12, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::RShift, s)")
            .infix(">>>", 12, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::URShift, s)")
            // Additive
            .infix("+", 13, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Add, s)")
            .infix("-", 13, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Sub, s)")
            // Multiplicative
            .infix("*", 14, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Mul, s)")
            .infix("/", 14, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Div, s)")
            .infix("%", 14, Assoc::Left, "|l, r, s| binary(l, r, BinaryOp::Mod, s)")
            // Exponentiation (right-to-left)
            .infix("**", 15, Assoc::Right, "|l, r, s| binary(l, r, BinaryOp::Exp, s)")
            // Prefix operators (unary)
            .prefix("-", 16, "|e, s| unary(e, UnaryOp::Minus, s)")
            .prefix("+", 16, "|e, s| unary(e, UnaryOp::Plus, s)")
            .prefix("!", 16, "|e, s| unary(e, UnaryOp::Not, s)")
            .prefix("~", 16, "|e, s| unary(e, UnaryOp::BitNot, s)")
            .prefix_kw("typeof", 16, "|e, s| unary(e, UnaryOp::Typeof, s)")
            .prefix_kw("void", 16, "|e, s| unary(e, UnaryOp::Void, s)")
            .prefix_kw("delete", 16, "|e, s| unary(e, UnaryOp::Delete, s)")
            // Postfix - skip for now
            // Call and member access - skip for now
    })
}

fn rule_primary(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("literal"),
        r.parse("identifier").ast("|result: ParseResult, span: Span| {
            let ident = result.into_ast::<Identifier>();
            Expression::Identifier(ident)
        }"),
        r.parse("this_expression"),
        r.parse("super_expression"),
        r.parse("array_expression"),
        r.parse("object_expression"),
        r.parse("function_expression"),
        r.parse("arrow_function"),
        r.parse("class_expression"),
        r.parse("template_literal"),
        r.choice((
            r.parse("new_expression"),
            r.parse("yield_expression"),
            r.parse("parenthesized"),
        )),
    ))
}

fn rule_literal(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((r.parse("number_literal"), r.parse("ws"))).ast("|result: ParseResult, span: Span| {
            let text = result.into_text();
            let value = parse_number(&text);
            Expression::Literal(Box::new(Literal { value: LiteralValue::Number(value), span }))
        }"),
        r.sequence((r.parse("string_literal"), r.parse("ws"))).ast("|result: ParseResult, span: Span| {
            let text = result.into_text();
            let value = parse_string_literal(&text);
            Expression::Literal(Box::new(Literal { value: LiteralValue::String(value), span }))
        }"),
        kw(r, "true").ast("|_: ParseResult, span: Span| Expression::Literal(Box::new(Literal { value: LiteralValue::Boolean(true), span }))"),
        kw(r, "false").ast("|_: ParseResult, span: Span| Expression::Literal(Box::new(Literal { value: LiteralValue::Boolean(false), span }))"),
        kw(r, "null").ast("|_: ParseResult, span: Span| Expression::Literal(Box::new(Literal { value: LiteralValue::Null, span }))"),
        kw(r, "undefined").ast("|_: ParseResult, span: Span| Expression::Literal(Box::new(Literal { value: LiteralValue::Undefined, span }))"),
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
    .ast("|result: ParseResult, span: Span| Identifier { name: result.into_text(), span }")
}

fn rule_this_expression(r: &RuleBuilder) -> Combinator {
    kw(r, "this")
}

fn rule_super_expression(r: &RuleBuilder) -> Combinator {
    kw(r, "super")
}

fn rule_array_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        op(r, "["),
        r.optional(r.separated_by_trailing(r.parse("array_element"), op(r, ","))),
        op(r, "]"),
    ))
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
        r.sequence((
            r.optional(kw(r, "async")),
            r.optional(r.parse("type_parameters")),
            op(r, "("),
            r.optional(r.parse("parameter_list")),
            op(r, ")"),
            r.optional(r.parse("type_annotation")),
            op(r, "=>"),
            r.parse("arrow_body"),
        )),
        r.sequence((
            r.optional(kw(r, "async")),
            r.parse("identifier"),
            op(r, "=>"),
            r.parse("arrow_body"),
        )),
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
    r.choice((
        r.parse("keyword_type"),
        r.parse("type_reference"),
        r.parse("literal_type"),
        r.parse("object_type"),
        r.parse("array_type"),
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

fn rule_array_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("primary_type"),
        op(r, "["),
        op(r, "]"),
    ))
}

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
