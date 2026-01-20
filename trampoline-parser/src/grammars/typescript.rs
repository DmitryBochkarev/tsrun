//! TypeScript Grammar Definition
//!
//! This module defines the complete TypeScript grammar using the trampoline-parser DSL.
//! The grammar generates a fully trampoline-based lexer and parser.

use crate::{Assoc, AstConfigBuilder, Combinator, Grammar, LexerBuilder, RuleBuilder};

/// Build the TypeScript grammar
pub fn typescript_grammar() -> Grammar {
    Grammar::new()
        .ast_config(ast_config)
        .lexer(typescript_lexer)
        // Program
        .rule("program", rule_program)
        // Statements
        .rule("statement", rule_statement)
        .rule("variable_declaration", rule_variable_declaration)
        .rule("variable_declarator", rule_variable_declarator)
        .rule("variable_declaration_no_semi", rule_variable_declaration_no_semi)
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
    c.import("crate::ast::*")
        .import("crate::value::JsString")
        .import("crate::prelude::Rc")
        .span_type("Span")
        .string_type("JsString")
        .apply_mappings()
}

// === Lexer ===

fn typescript_lexer(l: LexerBuilder) -> LexerBuilder {
    l
        // === Operators (multi-char first for proper matching) ===
        .token("QUESTION_QUESTION_EQ", "??=")
        .token("GT_GT_GT_EQ", ">>>=")
        .token("STAR_STAR_EQ", "**=")
        .token("AMP_AMP_EQ", "&&=")
        .token("PIPE_PIPE_EQ", "||=")
        .token("GT_GT_EQ", ">>=")
        .token("LT_LT_EQ", "<<=")
        .token("GT_GT_GT", ">>>")
        .token("EQ_EQ_EQ", "===")
        .token("BANG_EQ_EQ", "!==")
        .token("QUESTION_QUESTION", "??")
        .token("QUESTION_DOT", "?.")
        .token("DOT_DOT_DOT", "...")
        .token("STAR_STAR", "**")
        .token("PLUS_PLUS", "++")
        .token("MINUS_MINUS", "--")
        .token("AMP_AMP", "&&")
        .token("PIPE_PIPE", "||")
        .token("LT_LT", "<<")
        .token("GT_GT", ">>")
        .token("LT_EQ", "<=")
        .token("GT_EQ", ">=")
        .token("EQ_EQ", "==")
        .token("BANG_EQ", "!=")
        .token("PLUS_EQ", "+=")
        .token("MINUS_EQ", "-=")
        .token("STAR_EQ", "*=")
        .token("SLASH_EQ", "/=")
        .token("PERCENT_EQ", "%=")
        .token("AMP_EQ", "&=")
        .token("PIPE_EQ", "|=")
        .token("CARET_EQ", "^=")
        .token("ARROW", "=>")
        // Single-char operators
        .token("PLUS", "+")
        .token("MINUS", "-")
        .token("STAR", "*")
        .token("SLASH", "/")
        .token("PERCENT", "%")
        .token("AMP", "&")
        .token("PIPE", "|")
        .token("CARET", "^")
        .token("TILDE", "~")
        .token("BANG", "!")
        .token("LT", "<")
        .token("GT", ">")
        .token("EQ", "=")
        .token("QUESTION", "?")
        .token("COLON", ":")
        .token("DOT", ".")
        .token("COMMA", ",")
        .token("SEMICOLON", ";")
        .token("AT", "@")
        // Brackets
        .token("LPAREN", "(")
        .token("RPAREN", ")")
        .token("LBRACKET", "[")
        .token("RBRACKET", "]")
        .token("LBRACE", "{")
        .token("RBRACE", "}")
        // === Keywords ===
        .keyword("ABSTRACT", "abstract")
        .keyword("ACCESSOR", "accessor")
        .keyword("ANY", "any")
        .keyword("AS", "as")
        .keyword("ASSERTS", "asserts")
        .keyword("ASYNC", "async")
        .keyword("AWAIT", "await")
        .keyword("BIGINT", "bigint")
        .keyword("BOOLEAN", "boolean")
        .keyword("BREAK", "break")
        .keyword("CASE", "case")
        .keyword("CATCH", "catch")
        .keyword("CLASS", "class")
        .keyword("CONST", "const")
        .keyword("CONSTRUCTOR", "constructor")
        .keyword("CONTINUE", "continue")
        .keyword("DEBUGGER", "debugger")
        .keyword("DECLARE", "declare")
        .keyword("DEFAULT", "default")
        .keyword("DELETE", "delete")
        .keyword("DO", "do")
        .keyword("ELSE", "else")
        .keyword("ENUM", "enum")
        .keyword("EXPORT", "export")
        .keyword("EXTENDS", "extends")
        .keyword("FALSE", "false")
        .keyword("FINALLY", "finally")
        .keyword("FOR", "for")
        .keyword("FROM", "from")
        .keyword("FUNCTION", "function")
        .keyword("GET", "get")
        .keyword("IF", "if")
        .keyword("IMPLEMENTS", "implements")
        .keyword("IMPORT", "import")
        .keyword("IN", "in")
        .keyword("INFER", "infer")
        .keyword("INSTANCEOF", "instanceof")
        .keyword("INTERFACE", "interface")
        .keyword("IS", "is")
        .keyword("KEYOF", "keyof")
        .keyword("LET", "let")
        .keyword("NAMESPACE", "namespace")
        .keyword("NEVER", "never")
        .keyword("NEW", "new")
        .keyword("NULL", "null")
        .keyword("NUMBER_KW", "number")
        .keyword("OBJECT_KW", "object")
        .keyword("OF", "of")
        .keyword("PRIVATE", "private")
        .keyword("PROTECTED", "protected")
        .keyword("PUBLIC", "public")
        .keyword("READONLY", "readonly")
        .keyword("RETURN", "return")
        .keyword("SET", "set")
        .keyword("STATIC", "static")
        .keyword("STRING_KW", "string")
        .keyword("SUPER", "super")
        .keyword("SWITCH", "switch")
        .keyword("SYMBOL", "symbol")
        .keyword("THIS", "this")
        .keyword("THROW", "throw")
        .keyword("TRUE", "true")
        .keyword("TRY", "try")
        .keyword("TYPE", "type")
        .keyword("TYPEOF", "typeof")
        .keyword("UNDEFINED", "undefined")
        .keyword("UNKNOWN", "unknown")
        .keyword("VAR", "var")
        .keyword("VOID", "void")
        .keyword("WHILE", "while")
        .keyword("WITH", "with")
        .keyword("YIELD", "yield")
        // === Pattern-based tokens ===
        // Identifier: starts with letter/$/_, continues with letter/digit/$/_
        .token("IDENTIFIER", "")
            .start_with(|c| c.match_class("alpha").or().char('_').or().char('$'))
            .continue_with(|c| c.match_class("alphanumeric").or().char('_').or().char('$'))
            .build()
        // Number: digits, optionally with decimal, exponent, hex, binary, octal
        .token("NUMBER", "")
            .start_with(|c| c.match_class("digit"))
            .continue_with(|c| {
                c.match_class("digit")
                    .or().char('.')
                    .or().char('x').or().char('X')
                    .or().char('b').or().char('B')
                    .or().char('o').or().char('O')
                    .or().char('e').or().char('E')
                    .or().char('+').or().char('-')
                    .or().match_class("hex")
                    .or().char('n') // BigInt suffix
                    .or().char('_') // Numeric separators
            })
            .build()
        // String: double or single quoted
        .token("STRING", "")
            .start_with(|c| c.char('"').or().char('\''))
            .scan_until_matching_quote()
            .build()
        // Template strings
        .token("TEMPLATE_STRING", "")
            .start_with(|c| c.char('`'))
            .scan_until_matching_quote()
            .build()
        .token("TEMPLATE_HEAD", "")
            .start_with(|c| c.char('`'))
            .scan_template_head()
            .build()
        .token("TEMPLATE_MIDDLE", "")
            .start_with(|c| c.char('}'))
            .scan_template_middle()
            .build()
        .token("TEMPLATE_TAIL", "")
            .start_with(|c| c.char('}'))
            .scan_template_tail()
            .build()
        // Skip whitespace and comments
        .skip("whitespace")
        .skip("line_comment")
        .skip("block_comment")
}

// === Rule Functions ===

fn rule_program(r: &RuleBuilder) -> Combinator {
    r.zero_or_more(r.parse("statement"))
}

fn rule_statement(r: &RuleBuilder) -> Combinator {
    // Split into groups of <= 12 to satisfy IntoCombinatorsVec trait bounds
    r.choice((
        // Control flow statements
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
        // Group 2: more statements
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
        r.choice((r.token("LET"), r.token("CONST"), r.token("VAR"))),
        r.separated_by(r.parse("variable_declarator"), r.token("COMMA")),
        r.parse("semicolon"),
    ))
}

fn rule_variable_declarator(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("pattern"),
        r.optional(r.parse("type_annotation")),
        r.optional(r.sequence((r.token("EQ"), r.parse("expression")))),
    ))
}

fn rule_variable_declaration_no_semi(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.choice((r.token("LET"), r.token("CONST"), r.token("VAR"))),
        r.separated_by(r.parse("variable_declarator"), r.token("COMMA")),
    ))
}

fn rule_function_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.token("ASYNC")),
        r.token("FUNCTION"),
        r.optional(r.token("STAR")),
        r.optional(r.parse("identifier")),
        r.optional(r.parse("type_parameters")),
        r.token("LPAREN"),
        r.optional(r.parse("parameter_list")),
        r.token("RPAREN"),
        r.optional(r.parse("type_annotation")),
        r.parse("block_statement"),
    ))
}

fn rule_class_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(r.token("ABSTRACT")),
        r.token("CLASS"),
        r.optional(r.parse("identifier")),
        r.optional(r.parse("type_parameters")),
        r.optional(r.sequence((r.token("EXTENDS"), r.parse("expression")))),
        r.optional(r.sequence((
            r.token("IMPLEMENTS"),
            r.separated_by(r.parse("type_reference"), r.token("COMMA")),
        ))),
        r.parse("class_body"),
    ))
}

fn rule_class_body(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACE"),
        r.zero_or_more(r.parse("class_member")),
        r.token("RBRACE"),
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
        r.token("CONSTRUCTOR"),
        r.token("LPAREN"),
        r.optional(r.parse("parameter_list")),
        r.token("RPAREN"),
        r.parse("block_statement"),
    ))
}

fn rule_class_method(r: &RuleBuilder) -> Combinator {
    // Split sequence to stay under 12-element limit
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(r.parse("accessibility_modifier")),
        r.optional(r.token("STATIC")),
        r.optional(r.token("ASYNC")),
        r.optional(r.token("STAR")),
        r.optional(r.choice((r.token("GET"), r.token("SET")))),
        r.parse("property_key"),
        r.optional(r.parse("type_parameters")),
        // Group params and body
        r.sequence((
            r.token("LPAREN"),
            r.optional(r.parse("parameter_list")),
            r.token("RPAREN"),
            r.optional(r.parse("type_annotation")),
            r.parse("block_statement"),
        )),
    ))
}

fn rule_class_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(r.parse("accessibility_modifier")),
        r.optional(r.token("STATIC")),
        r.optional(r.token("READONLY")),
        r.optional(r.token("ACCESSOR")),
        r.parse("property_key"),
        r.optional(r.token("QUESTION")),
        r.optional(r.parse("type_annotation")),
        r.optional(r.sequence((r.token("EQ"), r.parse("expression")))),
        r.parse("semicolon"),
    ))
}

fn rule_static_block(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("STATIC"), r.parse("block_statement")))
}

fn rule_if_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("IF"),
        r.token("LPAREN"),
        r.parse("expression"),
        r.token("RPAREN"),
        r.parse("statement"),
        r.optional(r.sequence((r.token("ELSE"), r.parse("statement")))),
    ))
}

fn rule_for_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("FOR"),
        r.token("LPAREN"),
        r.optional(r.parse("for_init")),
        r.token("SEMICOLON"),
        r.optional(r.parse("expression")),
        r.token("SEMICOLON"),
        r.optional(r.parse("expression")),
        r.token("RPAREN"),
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
        r.token("FOR"),
        r.token("LPAREN"),
        r.parse("for_in_of_left"),
        r.token("IN"),
        r.parse("expression"),
        r.token("RPAREN"),
        r.parse("statement"),
    ))
}

fn rule_for_of_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("FOR"),
        r.optional(r.token("AWAIT")),
        r.token("LPAREN"),
        r.parse("for_in_of_left"),
        r.token("OF"),
        r.parse("expression"),
        r.token("RPAREN"),
        r.parse("statement"),
    ))
}

fn rule_for_in_of_left(r: &RuleBuilder) -> Combinator {
    r.choice((r.parse("variable_declaration_no_semi"), r.parse("pattern")))
}

fn rule_while_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("WHILE"),
        r.token("LPAREN"),
        r.parse("expression"),
        r.token("RPAREN"),
        r.parse("statement"),
    ))
}

fn rule_do_while_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("DO"),
        r.parse("statement"),
        r.token("WHILE"),
        r.token("LPAREN"),
        r.parse("expression"),
        r.token("RPAREN"),
        r.parse("semicolon"),
    ))
}

fn rule_switch_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("SWITCH"),
        r.token("LPAREN"),
        r.parse("expression"),
        r.token("RPAREN"),
        r.token("LBRACE"),
        r.zero_or_more(r.parse("switch_case")),
        r.token("RBRACE"),
    ))
}

fn rule_switch_case(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((
            r.token("CASE"),
            r.parse("expression"),
            r.token("COLON"),
            r.zero_or_more(r.parse("statement")),
        )),
        r.sequence((
            r.token("DEFAULT"),
            r.token("COLON"),
            r.zero_or_more(r.parse("statement")),
        )),
    ))
}

fn rule_try_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("TRY"),
        r.parse("block_statement"),
        r.optional(r.parse("catch_clause")),
        r.optional(r.sequence((r.token("FINALLY"), r.parse("block_statement")))),
    ))
}

fn rule_catch_clause(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("CATCH"),
        r.optional(r.sequence((
            r.token("LPAREN"),
            r.parse("pattern"),
            r.token("RPAREN"),
        ))),
        r.parse("block_statement"),
    ))
}

fn rule_block_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACE"),
        r.zero_or_more(r.parse("statement")),
        r.token("RBRACE"),
    ))
}

fn rule_return_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("RETURN"),
        r.optional(r.parse("expression")),
        r.parse("semicolon"),
    ))
}

fn rule_break_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("BREAK"),
        r.optional(r.parse("identifier")),
        r.parse("semicolon"),
    ))
}

fn rule_continue_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("CONTINUE"),
        r.optional(r.parse("identifier")),
        r.parse("semicolon"),
    ))
}

fn rule_throw_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("THROW"), r.parse("expression"), r.parse("semicolon")))
}

fn rule_debugger_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("DEBUGGER"), r.parse("semicolon")))
}

fn rule_labeled_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((r.parse("identifier"), r.token("COLON"), r.parse("statement")))
}

fn rule_expression_statement(r: &RuleBuilder) -> Combinator {
    r.sequence((r.parse("expression"), r.parse("semicolon")))
}

fn rule_empty_statement(r: &RuleBuilder) -> Combinator {
    r.token("SEMICOLON")
}

fn rule_semicolon(r: &RuleBuilder) -> Combinator {
    // TODO: Add ASI (automatic semicolon insertion) support
    r.token("SEMICOLON")
}

// Import/Export

fn rule_import_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("IMPORT"),
        r.optional(r.token("TYPE")),
        r.parse("import_clause"),
        r.token("FROM"),
        r.token("STRING"),
        r.parse("semicolon"),
    ))
}

fn rule_import_clause(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((r.token("STAR"), r.token("AS"), r.parse("identifier"))),
        r.parse("named_imports"),
        r.sequence((
            r.parse("identifier"),
            r.optional(r.sequence((r.token("COMMA"), r.parse("import_clause_rest")))),
        )),
    ))
}

fn rule_import_clause_rest(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((r.token("STAR"), r.token("AS"), r.parse("identifier"))),
        r.parse("named_imports"),
    ))
}

fn rule_named_imports(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACE"),
        r.optional(r.separated_by_trailing(
            r.parse("import_specifier"),
            r.token("COMMA"),
        )),
        r.token("RBRACE"),
    ))
}

fn rule_import_specifier(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        r.optional(r.sequence((r.token("AS"), r.parse("identifier")))),
    ))
}

fn rule_export_declaration(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((
            r.token("EXPORT"),
            r.token("DEFAULT"),
            r.parse("export_default_expression"),
        )),
        r.sequence((
            r.token("EXPORT"),
            r.optional(r.token("TYPE")),
            r.parse("named_exports"),
            r.optional(r.sequence((r.token("FROM"), r.token("STRING")))),
            r.parse("semicolon"),
        )),
        r.sequence((
            r.token("EXPORT"),
            r.token("STAR"),
            r.optional(r.sequence((r.token("AS"), r.parse("identifier")))),
            r.token("FROM"),
            r.token("STRING"),
            r.parse("semicolon"),
        )),
        r.sequence((
            r.token("EXPORT"),
            r.optional(r.token("TYPE")),
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
        r.token("LBRACE"),
        r.optional(r.separated_by_trailing(
            r.parse("export_specifier"),
            r.token("COMMA"),
        )),
        r.token("RBRACE"),
    ))
}

fn rule_export_specifier(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        r.optional(r.sequence((r.token("AS"), r.parse("identifier")))),
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
        r.token("TYPE"),
        r.parse("identifier"),
        r.optional(r.parse("type_parameters")),
        r.token("EQ"),
        r.parse("type"),
        r.parse("semicolon"),
    ))
}

fn rule_interface_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("INTERFACE"),
        r.parse("identifier"),
        r.optional(r.parse("type_parameters")),
        r.optional(r.sequence((
            r.token("EXTENDS"),
            r.separated_by(r.parse("type_reference"), r.token("COMMA")),
        ))),
        r.parse("object_type"),
    ))
}

fn rule_enum_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.token("CONST")),
        r.token("ENUM"),
        r.parse("identifier"),
        r.token("LBRACE"),
        r.optional(r.separated_by_trailing(r.parse("enum_member"), r.token("COMMA"))),
        r.token("RBRACE"),
    ))
}

fn rule_enum_member(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        r.optional(r.sequence((r.token("EQ"), r.parse("expression")))),
    ))
}

fn rule_namespace_declaration(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("NAMESPACE"),
        r.parse("identifier"),
        r.parse("block_statement"),
    ))
}

// Expressions

fn rule_expression(r: &RuleBuilder) -> Combinator {
    r.pratt(r.parse("primary"), |ops| {
        ops
            // Assignment (right-to-left, lowest precedence)
            .infix("EQ", 2, Assoc::Right, "|l, r| assign(l, r)")
            .infix("PLUS_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, AddAssign)")
            .infix("MINUS_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, SubAssign)")
            .infix("STAR_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, MulAssign)")
            .infix("SLASH_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, DivAssign)")
            .infix("PERCENT_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, ModAssign)")
            .infix("STAR_STAR_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, ExpAssign)")
            .infix("AMP_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, BitAndAssign)")
            .infix("PIPE_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, BitOrAssign)")
            .infix("CARET_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, BitXorAssign)")
            .infix("LT_LT_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, LShiftAssign)")
            .infix("GT_GT_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, RShiftAssign)")
            .infix("GT_GT_GT_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, URShiftAssign)")
            .infix("AMP_AMP_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, AndAssign)")
            .infix("PIPE_PIPE_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, OrAssign)")
            .infix("QUESTION_QUESTION_EQ", 2, Assoc::Right, "|l, r| assign_op(l, r, NullishAssign)")
            // Ternary
            .ternary("QUESTION", "COLON", 3, "|c, t, f| conditional(c, t, f)")
            // Nullish coalescing
            .infix("QUESTION_QUESTION", 4, Assoc::Left, "|l, r| logical(l, r, NullishCoalescing)")
            // Logical OR
            .infix("PIPE_PIPE", 5, Assoc::Left, "|l, r| logical(l, r, Or)")
            // Logical AND
            .infix("AMP_AMP", 6, Assoc::Left, "|l, r| logical(l, r, And)")
            // Bitwise OR
            .infix("PIPE", 7, Assoc::Left, "|l, r| binary(l, r, BitOr)")
            // Bitwise XOR
            .infix("CARET", 8, Assoc::Left, "|l, r| binary(l, r, BitXor)")
            // Bitwise AND
            .infix("AMP", 9, Assoc::Left, "|l, r| binary(l, r, BitAnd)")
            // Equality
            .infix("EQ_EQ", 10, Assoc::Left, "|l, r| binary(l, r, Eq)")
            .infix("BANG_EQ", 10, Assoc::Left, "|l, r| binary(l, r, NotEq)")
            .infix("EQ_EQ_EQ", 10, Assoc::Left, "|l, r| binary(l, r, StrictEq)")
            .infix("BANG_EQ_EQ", 10, Assoc::Left, "|l, r| binary(l, r, StrictNotEq)")
            // Relational
            .infix("LT", 11, Assoc::Left, "|l, r| binary(l, r, Lt)")
            .infix("GT", 11, Assoc::Left, "|l, r| binary(l, r, Gt)")
            .infix("LT_EQ", 11, Assoc::Left, "|l, r| binary(l, r, LtEq)")
            .infix("GT_EQ", 11, Assoc::Left, "|l, r| binary(l, r, GtEq)")
            .infix("IN", 11, Assoc::Left, "|l, r| binary(l, r, In)")
            .infix("INSTANCEOF", 11, Assoc::Left, "|l, r| binary(l, r, Instanceof)")
            // Shift
            .infix("LT_LT", 12, Assoc::Left, "|l, r| binary(l, r, LShift)")
            .infix("GT_GT", 12, Assoc::Left, "|l, r| binary(l, r, RShift)")
            .infix("GT_GT_GT", 12, Assoc::Left, "|l, r| binary(l, r, URShift)")
            // Additive
            .infix("PLUS", 13, Assoc::Left, "|l, r| binary(l, r, Add)")
            .infix("MINUS", 13, Assoc::Left, "|l, r| binary(l, r, Sub)")
            // Multiplicative
            .infix("STAR", 14, Assoc::Left, "|l, r| binary(l, r, Mul)")
            .infix("SLASH", 14, Assoc::Left, "|l, r| binary(l, r, Div)")
            .infix("PERCENT", 14, Assoc::Left, "|l, r| binary(l, r, Mod)")
            // Exponentiation (right-to-left)
            .infix("STAR_STAR", 15, Assoc::Right, "|l, r| binary(l, r, Exp)")
            // Prefix unary
            .prefix("BANG", 16, "|e| unary(e, Not)")
            .prefix("TILDE", 16, "|e| unary(e, BitNot)")
            .prefix("PLUS", 16, "|e| unary(e, Plus)")
            .prefix("MINUS", 16, "|e| unary(e, Minus)")
            .prefix("TYPEOF", 16, "|e| unary(e, Typeof)")
            .prefix("VOID", 16, "|e| unary(e, Void)")
            .prefix("DELETE", 16, "|e| unary(e, Delete)")
            .prefix("AWAIT", 16, "|e| await_expr(e)")
            .prefix("PLUS_PLUS", 16, "|e| update(e, Increment, true)")
            .prefix("MINUS_MINUS", 16, "|e| update(e, Decrement, true)")
            // Postfix
            .postfix("PLUS_PLUS", 17, "|e| update(e, Increment, false)")
            .postfix("MINUS_MINUS", 17, "|e| update(e, Decrement, false)")
            .postfix("BANG", 17, "|e| non_null(e)")
            // Call and member access
            .postfix_call("LPAREN", "RPAREN", 18, "|callee, args| call(callee, args)")
            .postfix_index("LBRACKET", "RBRACKET", 18, "|obj, prop| member_computed(obj, prop)")
            .postfix_member("DOT", 18, "|obj, prop| member(obj, prop)")
            // Optional chaining
            .postfix_member("QUESTION_DOT", 18, "|obj, prop| member_optional(obj, prop)")
    })
}

fn rule_primary(r: &RuleBuilder) -> Combinator {
    // Split choice to stay under 12-element limit
    r.choice((
        r.parse("literal"),
        r.parse("identifier"),
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
        r.token("NUMBER"),
        r.token("STRING"),
        r.token("TRUE"),
        r.token("FALSE"),
        r.token("NULL"),
        r.token("UNDEFINED"),
    ))
}

fn rule_identifier(r: &RuleBuilder) -> Combinator {
    r.token("IDENTIFIER")
}

fn rule_this_expression(r: &RuleBuilder) -> Combinator {
    r.token("THIS")
}

fn rule_super_expression(r: &RuleBuilder) -> Combinator {
    r.token("SUPER")
}

fn rule_array_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACKET"),
        r.optional(r.separated_by_trailing(r.parse("array_element"), r.token("COMMA"))),
        r.token("RBRACKET"),
    ))
}

fn rule_array_element(r: &RuleBuilder) -> Combinator {
    r.choice((r.parse("spread_element"), r.parse("expression")))
}

fn rule_object_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACE"),
        r.optional(r.separated_by_trailing(r.parse("object_property"), r.token("COMMA"))),
        r.token("RBRACE"),
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
    r.sequence((r.parse("property_key"), r.token("COLON"), r.parse("expression")))
}

fn rule_shorthand_property(r: &RuleBuilder) -> Combinator {
    r.parse("identifier")
}

fn rule_method_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.token("ASYNC")),
        r.optional(r.token("STAR")),
        r.parse("property_key"),
        r.token("LPAREN"),
        r.optional(r.parse("parameter_list")),
        r.token("RPAREN"),
        r.parse("block_statement"),
    ))
}

fn rule_getter_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("GET"),
        r.parse("property_key"),
        r.token("LPAREN"),
        r.token("RPAREN"),
        r.parse("block_statement"),
    ))
}

fn rule_setter_property(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("SET"),
        r.parse("property_key"),
        r.token("LPAREN"),
        r.parse("pattern"),
        r.token("RPAREN"),
        r.parse("block_statement"),
    ))
}

fn rule_property_key(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("identifier"),
        r.token("STRING"),
        r.token("NUMBER"),
        r.sequence((r.token("LBRACKET"), r.parse("expression"), r.token("RBRACKET"))),
    ))
}

fn rule_spread_element(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("DOT_DOT_DOT"), r.parse("expression")))
}

fn rule_function_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.token("ASYNC")),
        r.token("FUNCTION"),
        r.optional(r.token("STAR")),
        r.optional(r.parse("identifier")),
        r.optional(r.parse("type_parameters")),
        r.token("LPAREN"),
        r.optional(r.parse("parameter_list")),
        r.token("RPAREN"),
        r.optional(r.parse("type_annotation")),
        r.parse("block_statement"),
    ))
}

fn rule_arrow_function(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.sequence((
            r.optional(r.token("ASYNC")),
            r.optional(r.parse("type_parameters")),
            r.token("LPAREN"),
            r.optional(r.parse("parameter_list")),
            r.token("RPAREN"),
            r.optional(r.parse("type_annotation")),
            r.token("ARROW"),
            r.parse("arrow_body"),
        )),
        r.sequence((
            r.optional(r.token("ASYNC")),
            r.parse("identifier"),
            r.token("ARROW"),
            r.parse("arrow_body"),
        )),
    ))
}

fn rule_arrow_body(r: &RuleBuilder) -> Combinator {
    r.choice((r.parse("block_statement"), r.parse("expression")))
}

fn rule_class_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("CLASS"),
        r.optional(r.parse("identifier")),
        r.optional(r.parse("type_parameters")),
        r.optional(r.sequence((r.token("EXTENDS"), r.parse("expression")))),
        r.optional(r.sequence((
            r.token("IMPLEMENTS"),
            r.separated_by(r.parse("type_reference"), r.token("COMMA")),
        ))),
        r.parse("class_body"),
    ))
}

fn rule_template_literal(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.token("TEMPLATE_STRING"),
        r.sequence((
            r.token("TEMPLATE_HEAD"),
            r.parse("expression"),
            r.zero_or_more(r.sequence((
                r.token("TEMPLATE_MIDDLE"),
                r.parse("expression"),
            ))),
            r.token("TEMPLATE_TAIL"),
        )),
    ))
}

fn rule_new_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("NEW"),
        r.parse("expression"),
        r.optional(r.sequence((
            r.token("LPAREN"),
            r.optional(r.parse("argument_list")),
            r.token("RPAREN"),
        ))),
    ))
}

fn rule_yield_expression(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("YIELD"),
        r.optional(r.token("STAR")),
        r.optional(r.parse("expression")),
    ))
}

fn rule_parenthesized(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("LPAREN"), r.parse("expression"), r.token("RPAREN")))
}

fn rule_argument_list(r: &RuleBuilder) -> Combinator {
    r.separated_by(r.parse("argument"), r.token("COMMA"))
}

fn rule_argument(r: &RuleBuilder) -> Combinator {
    r.choice((r.parse("spread_element"), r.parse("expression")))
}

// Parameters

fn rule_parameter_list(r: &RuleBuilder) -> Combinator {
    r.separated_by_trailing(r.parse("parameter"), r.token("COMMA"))
}

fn rule_parameter(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.zero_or_more(r.parse("decorator")),
        r.optional(r.parse("accessibility_modifier")),
        r.optional(r.token("READONLY")),
        r.parse("pattern"),
        r.optional(r.token("QUESTION")),
        r.optional(r.parse("type_annotation")),
        r.optional(r.sequence((r.token("EQ"), r.parse("expression")))),
    ))
}

fn rule_accessibility_modifier(r: &RuleBuilder) -> Combinator {
    r.choice((r.token("PUBLIC"), r.token("PRIVATE"), r.token("PROTECTED")))
}

fn rule_decorator(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("AT"), r.parse("expression")))
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
        r.token("LBRACE"),
        r.optional(r.separated_by_trailing(
            r.parse("object_pattern_property"),
            r.token("COMMA"),
        )),
        r.token("RBRACE"),
    ))
}

fn rule_object_pattern_property(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.parse("rest_pattern"),
        r.sequence((
            r.parse("property_key"),
            r.optional(r.sequence((r.token("COLON"), r.parse("pattern")))),
            r.optional(r.sequence((r.token("EQ"), r.parse("expression")))),
        )),
    ))
}

fn rule_array_pattern(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACKET"),
        r.optional(r.separated_by_trailing(
            r.optional(r.parse("array_pattern_element")),
            r.token("COMMA"),
        )),
        r.token("RBRACKET"),
    ))
}

fn rule_array_pattern_element(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("pattern"),
        r.optional(r.sequence((r.token("EQ"), r.parse("expression")))),
    ))
}

fn rule_rest_pattern(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("DOT_DOT_DOT"), r.parse("pattern")))
}

// Type Annotations

fn rule_type_annotation(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("COLON"), r.parse("type")))
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
        r.token("ANY"),
        r.token("UNKNOWN"),
        r.token("NEVER"),
        r.token("VOID"),
        r.token("NULL"),
        r.token("UNDEFINED"),
        r.token("BOOLEAN"),
        r.token("NUMBER_KW"),
        r.token("STRING_KW"),
        r.token("SYMBOL"),
        r.token("BIGINT"),
        r.token("OBJECT_KW"),
    ))
}

fn rule_type_reference(r: &RuleBuilder) -> Combinator {
    r.sequence((r.parse("identifier"), r.optional(r.parse("type_arguments"))))
}

fn rule_type_arguments(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LT"),
        r.separated_by(r.parse("type"), r.token("COMMA")),
        r.token("GT"),
    ))
}

fn rule_type_parameters(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LT"),
        r.separated_by(r.parse("type_parameter"), r.token("COMMA")),
        r.token("GT"),
    ))
}

fn rule_type_parameter(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("identifier"),
        r.optional(r.sequence((r.token("EXTENDS"), r.parse("type")))),
        r.optional(r.sequence((r.token("EQ"), r.parse("type")))),
    ))
}

fn rule_literal_type(r: &RuleBuilder) -> Combinator {
    r.choice((
        r.token("STRING"),
        r.token("NUMBER"),
        r.token("TRUE"),
        r.token("FALSE"),
        r.token("NULL"),
    ))
}

fn rule_object_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACE"),
        r.zero_or_more(r.parse("type_member")),
        r.token("RBRACE"),
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
        r.optional(r.token("READONLY")),
        r.parse("property_key"),
        r.optional(r.token("QUESTION")),
        r.optional(r.parse("type_annotation")),
        r.parse("type_member_separator"),
    ))
}

fn rule_method_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("property_key"),
        r.optional(r.token("QUESTION")),
        r.optional(r.parse("type_parameters")),
        r.token("LPAREN"),
        r.optional(r.parse("parameter_list")),
        r.token("RPAREN"),
        r.optional(r.parse("type_annotation")),
        r.parse("type_member_separator"),
    ))
}

fn rule_index_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.token("READONLY")),
        r.token("LBRACKET"),
        r.parse("identifier"),
        r.token("COLON"),
        r.parse("type"),
        r.token("RBRACKET"),
        r.token("COLON"),
        r.parse("type"),
        r.parse("type_member_separator"),
    ))
}

fn rule_call_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.parse("type_parameters")),
        r.token("LPAREN"),
        r.optional(r.parse("parameter_list")),
        r.token("RPAREN"),
        r.optional(r.parse("type_annotation")),
        r.parse("type_member_separator"),
    ))
}

fn rule_construct_signature(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("NEW"),
        r.optional(r.parse("type_parameters")),
        r.token("LPAREN"),
        r.optional(r.parse("parameter_list")),
        r.token("RPAREN"),
        r.optional(r.parse("type_annotation")),
        r.parse("type_member_separator"),
    ))
}

fn rule_type_member_separator(r: &RuleBuilder) -> Combinator {
    r.choice((r.token("SEMICOLON"), r.token("COMMA")))
}

fn rule_array_type(r: &RuleBuilder) -> Combinator {
    r.sequence((r.parse("primary_type"), r.token("LBRACKET"), r.token("RBRACKET")))
}

fn rule_tuple_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACKET"),
        r.optional(r.separated_by(r.parse("type"), r.token("COMMA"))),
        r.token("RBRACKET"),
    ))
}

fn rule_union_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("primary_type"),
        r.one_or_more(r.sequence((r.token("PIPE"), r.parse("primary_type")))),
    ))
}

fn rule_intersection_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("primary_type"),
        r.one_or_more(r.sequence((r.token("AMP"), r.parse("primary_type")))),
    ))
}

fn rule_function_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.optional(r.parse("type_parameters")),
        r.token("LPAREN"),
        r.optional(r.parse("parameter_list")),
        r.token("RPAREN"),
        r.token("ARROW"),
        r.parse("type"),
    ))
}

fn rule_conditional_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.parse("primary_type"),
        r.token("EXTENDS"),
        r.parse("type"),
        r.token("QUESTION"),
        r.parse("type"),
        r.token("COLON"),
        r.parse("type"),
    ))
}

fn rule_typeof_type(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("TYPEOF"), r.parse("identifier")))
}

fn rule_keyof_type(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("KEYOF"), r.parse("type")))
}

fn rule_infer_type(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("INFER"), r.parse("type_parameter")))
}

fn rule_mapped_type(r: &RuleBuilder) -> Combinator {
    r.sequence((
        r.token("LBRACE"),
        r.optional(r.choice((
            r.sequence((r.token("PLUS"), r.token("READONLY"))),
            r.sequence((r.token("MINUS"), r.token("READONLY"))),
            r.token("READONLY"),
        ))),
        r.token("LBRACKET"),
        r.parse("type_parameter"),
        r.optional(r.sequence((r.token("AS"), r.parse("type")))),
        r.token("RBRACKET"),
        r.optional(r.choice((
            r.sequence((r.token("PLUS"), r.token("QUESTION"))),
            r.sequence((r.token("MINUS"), r.token("QUESTION"))),
            r.token("QUESTION"),
        ))),
        r.optional(r.parse("type_annotation")),
        r.token("RBRACE"),
    ))
}

fn rule_parenthesized_type(r: &RuleBuilder) -> Combinator {
    r.sequence((r.token("LPAREN"), r.parse("type"), r.token("RPAREN")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grammar_builds() {
        let grammar = typescript_grammar();
        let compiled = grammar.build();
        let code = compiled.generate();

        // Basic sanity checks
        assert!(code.contains("pub enum TokenKind"));
        assert!(code.contains("IDENTIFIER"));
        assert!(code.contains("pub struct Parser"));
    }

    #[test]
    fn test_grammar_generates_tokens() {
        let grammar = typescript_grammar();
        let compiled = grammar.build();
        let code = compiled.generate();

        // Check keywords (TokenKind enum)
        assert!(code.contains("TokenKind::LET"));
        assert!(code.contains("TokenKind::CONST"));
        assert!(code.contains("TokenKind::FUNCTION"));
        assert!(code.contains("TokenKind::CLASS"));
        assert!(code.contains("TokenKind::IF"));
        assert!(code.contains("TokenKind::ELSE"));
        assert!(code.contains("TokenKind::RETURN"));
        assert!(code.contains("TokenKind::IMPORT"));
        assert!(code.contains("TokenKind::EXPORT"));
        assert!(code.contains("TokenKind::ASYNC"));
        assert!(code.contains("TokenKind::AWAIT"));

        // Check operators
        assert!(code.contains("TokenKind::PLUS"));
        assert!(code.contains("TokenKind::MINUS"));
        assert!(code.contains("TokenKind::STAR"));
        assert!(code.contains("TokenKind::EQ"));
        assert!(code.contains("TokenKind::EQ_EQ"));
        assert!(code.contains("TokenKind::EQ_EQ_EQ"));
        assert!(code.contains("TokenKind::ARROW"));
    }

    #[test]
    fn test_grammar_generates_rules() {
        let grammar = typescript_grammar();
        let compiled = grammar.build();
        let code = compiled.generate();

        // Check Work enum variants are generated (PascalCase names)
        assert!(code.contains("Work::Program_Start"));
        assert!(code.contains("Work::Statement_Start"));
        assert!(code.contains("Work::Expression_Start"));
        assert!(code.contains("Work::VariableDeclaration_Start"));
        assert!(code.contains("Work::FunctionDeclaration_Start"));
        assert!(code.contains("Work::ClassDeclaration_Start"));
    }

    #[test]
    fn test_grammar_has_pratt_parsing() {
        let grammar = typescript_grammar();
        let compiled = grammar.build();
        let code = compiled.generate();

        // Pratt parsing for expressions should generate binding power logic
        assert!(code.contains("Work::Expression_Start"));
        // Should have pratt-related code
        assert!(code.contains("binding_power") || code.contains("Pratt"));
    }

    #[test]
    fn test_generated_code_size() {
        let grammar = typescript_grammar();
        let compiled = grammar.build();
        let code = compiled.generate();

        // TypeScript grammar should generate substantial code
        // The generated parser should be > 5000 lines
        let line_count = code.lines().count();
        assert!(
            line_count > 5000,
            "Expected > 5000 lines, got {}",
            line_count
        );
    }
}
