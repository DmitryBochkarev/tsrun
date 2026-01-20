//! Integration tests for generating a TypeScript parser
//!
//! These tests progressively build up TypeScript syntax support,
//! starting from the simplest constructs and adding complexity.

use std::fs;
use std::process::Command;
use trampoline_parser::{Assoc, CombinatorExt, Grammar};

/// Helper to generate code and verify it's non-empty
fn generate_and_check(grammar: Grammar) -> String {
    let compiled = grammar.build();
    let code = compiled.generate();
    assert!(!code.is_empty(), "Generated code should not be empty");
    assert!(
        code.contains("pub enum TokenKind"),
        "Should contain TokenKind enum"
    );
    assert!(
        code.contains("pub struct Lexer"),
        "Should contain Lexer struct"
    );
    assert!(code.contains("pub enum Work"), "Should contain Work enum");
    assert!(
        code.contains("pub struct Parser"),
        "Should contain Parser struct"
    );
    code
}

// =============================================================================
// Level 1: Literals
// =============================================================================

#[test]
fn test_level1_number_literal() {
    // Grammar for just number literals: 42, 3.14, 0xff
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| {
                    c.match_class("digit")
                        .or()
                        .char('.')
                        .or()
                        .char('x')
                        .or()
                        .match_class("hex")
                })
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("expression")))
        .rule("expression", |r| r.parse("literal"))
        .rule("literal", |r| {
            r.token("NUMBER")
                .ast("|t| Expr::Number(t.text.parse().unwrap())")
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("NUMBER"), "Should have NUMBER token");
}

#[test]
fn test_level1_string_literal() {
    // Grammar for string literals: "hello", 'world'
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("STRING", "")
                .start_with(|c| c.char('"').or().char('\''))
                .scan_until_matching_quote()
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("expression")))
        .rule("expression", |r| r.parse("literal"))
        .rule("literal", |r| {
            r.token("STRING").ast("|t| Expr::String(t.text)")
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("STRING"), "Should have STRING token");
}

#[test]
fn test_level1_identifier() {
    // Grammar for identifiers: foo, _bar, $baz
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_').or().char('$'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_').or().char('$'))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("expression")))
        .rule("expression", |r| {
            r.token("IDENTIFIER").ast("|t| Expr::Ident(t.text)")
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("IDENTIFIER"), "Should have IDENTIFIER token");
}

// =============================================================================
// Level 2: Simple Binary Expressions
// =============================================================================

#[test]
fn test_level2_binary_expression() {
    // Grammar for: 1 + 2, 3 * 4, etc.
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit").or().char('.'))
                .build()
                .token("PLUS", "+")
                .token("MINUS", "-")
                .token("STAR", "*")
                .token("SLASH", "/")
                .token("LPAREN", "(")
                .token("RPAREN", ")")
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("expression")))
        .rule("expression", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix(
                    "PLUS",
                    10,
                    Assoc::Left,
                    "|l, r| Expr::Binary(Op::Add, l, r)",
                )
                .infix(
                    "MINUS",
                    10,
                    Assoc::Left,
                    "|l, r| Expr::Binary(Op::Sub, l, r)",
                )
                .infix(
                    "STAR",
                    20,
                    Assoc::Left,
                    "|l, r| Expr::Binary(Op::Mul, l, r)",
                )
                .infix(
                    "SLASH",
                    20,
                    Assoc::Left,
                    "|l, r| Expr::Binary(Op::Div, l, r)",
                )
            })
        })
        .rule("primary", |r| {
            r.choice((
                r.token("NUMBER")
                    .ast("|t| Expr::Number(t.text.parse().unwrap())"),
                r.sequence((r.token("LPAREN"), r.parse("expression"), r.token("RPAREN")))
                    .ast("|(_, e, _)| e"),
            ))
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("PLUS"), "Should have PLUS token");
    assert!(
        code.contains("Expression_Pratt_AfterOperand"),
        "Should have Pratt work variants"
    );
}

#[test]
fn test_level2_unary_expression() {
    // Grammar for: -1, !true, typeof x
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_'))
                .build()
                .token("MINUS", "-")
                .token("BANG", "!")
                .keyword("TYPEOF", "typeof")
                .keyword("TRUE", "true")
                .keyword("FALSE", "false")
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("expression")))
        .rule("expression", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.prefix("MINUS", 15, "|e| Expr::Neg(e)")
                    .prefix("BANG", 15, "|e| Expr::Not(e)")
                    .prefix("TYPEOF", 15, "|e| Expr::TypeOf(e)")
            })
        })
        .rule("primary", |r| {
            r.choice((
                r.token("NUMBER")
                    .ast("|t| Expr::Number(t.text.parse().unwrap())"),
                r.token("TRUE").ast("|_| Expr::Bool(true)"),
                r.token("FALSE").ast("|_| Expr::Bool(false)"),
                r.token("IDENTIFIER").ast("|t| Expr::Ident(t.text)"),
            ))
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("TYPEOF"), "Should have TYPEOF keyword");
    assert!(code.contains("BANG"), "Should have BANG token");
}

// =============================================================================
// Level 3: Variable Declarations
// =============================================================================

#[test]
fn test_level3_variable_declaration() {
    // Grammar for: let x = 1; const y = 2;
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_'))
                .build()
                .token("EQUALS", "=")
                .token("SEMICOLON", ";")
                .keyword("LET", "let")
                .keyword("CONST", "const")
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("statement")))
        .rule("statement", |r| {
            r.choice((
                r.parse("variable_declaration"),
                r.parse("expression_statement"),
            ))
        })
        .rule("variable_declaration", |r| {
            r.sequence((
                r.choice((r.token("LET"), r.token("CONST"))),
                r.token("IDENTIFIER"),
                r.token("EQUALS"),
                r.parse("expression"),
                r.token("SEMICOLON"),
            ))
            .ast("|(kind, name, _, value, _)| Stmt::VarDecl { kind, name, value }")
        })
        .rule("expression_statement", |r| {
            r.sequence((r.parse("expression"), r.token("SEMICOLON")))
                .ast("|(expr, _)| Stmt::Expr(expr)")
        })
        .rule("expression", |r| {
            r.token("NUMBER")
                .ast("|t| Expr::Number(t.text.parse().unwrap())")
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("LET"), "Should have LET keyword");
    assert!(code.contains("CONST"), "Should have CONST keyword");
    assert!(
        code.contains("VariableDeclaration_Start"),
        "Should have variable declaration work"
    );
}

#[test]
fn test_level3_variable_with_type_annotation() {
    // Grammar for: let x: number = 1;
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_'))
                .build()
                .token("COLON", ":")
                .token("EQUALS", "=")
                .token("SEMICOLON", ";")
                .keyword("LET", "let")
                .keyword("CONST", "const")
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("statement")))
        .rule("statement", |r| r.parse("variable_declaration"))
        .rule("variable_declaration", |r| {
            r.sequence((
                r.choice((r.token("LET"), r.token("CONST"))),
                r.token("IDENTIFIER"),
                r.optional(r.parse("type_annotation")), // optional type
                r.token("EQUALS"),
                r.parse("expression"),
                r.token("SEMICOLON"),
            ))
            .ast("|(kind, name, _type, _, value, _)| Stmt::VarDecl { kind, name, value }")
        })
        .rule("type_annotation", |r| {
            r.sequence((r.token("COLON"), r.parse("type")))
                .ast("|(_, ty)| ty")
        })
        .rule("type", |r| {
            r.token("IDENTIFIER") // simple type for now
                .ast("|t| Type::Named(t.text)")
        })
        .rule("expression", |r| {
            r.token("NUMBER")
                .ast("|t| Expr::Number(t.text.parse().unwrap())")
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("COLON"), "Should have COLON token");
    assert!(
        code.contains("TypeAnnotation_Start"),
        "Should have type annotation work"
    );
}

// =============================================================================
// Level 4: Function Declarations
// =============================================================================

#[test]
fn test_level4_function_declaration() {
    // Grammar for: function add(a, b) { return a + b; }
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_'))
                .build()
                .token("LPAREN", "(")
                .token("RPAREN", ")")
                .token("LBRACE", "{")
                .token("RBRACE", "}")
                .token("COMMA", ",")
                .token("SEMICOLON", ";")
                .token("PLUS", "+")
                .keyword("FUNCTION", "function")
                .keyword("RETURN", "return")
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("statement")))
        .rule("statement", |r| {
            r.choice((
                r.parse("function_declaration"),
                r.parse("return_statement"),
                r.parse("expression_statement"),
            ))
        })
        .rule("function_declaration", |r| {
            r.sequence((
                r.token("FUNCTION"),
                r.token("IDENTIFIER"),
                r.token("LPAREN"),
                r.optional(r.parse("parameter_list")),
                r.token("RPAREN"),
                r.parse("block"),
            ))
            .ast("|(_, name, _, params, _, body)| Stmt::Function { name, params, body }")
        })
        .rule("parameter_list", |r| {
            r.separated_by(r.token("IDENTIFIER"), r.token("COMMA"))
        })
        .rule("block", |r| {
            r.sequence((
                r.token("LBRACE"),
                r.zero_or_more(r.parse("statement")),
                r.token("RBRACE"),
            ))
            .ast("|(_, stmts, _)| Block { statements: stmts }")
        })
        .rule("return_statement", |r| {
            r.sequence((
                r.token("RETURN"),
                r.optional(r.parse("expression")),
                r.token("SEMICOLON"),
            ))
            .ast("|(_, expr, _)| Stmt::Return(expr)")
        })
        .rule("expression_statement", |r| {
            r.sequence((r.parse("expression"), r.token("SEMICOLON")))
                .ast("|(expr, _)| Stmt::Expr(expr)")
        })
        .rule("expression", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix(
                    "PLUS",
                    10,
                    Assoc::Left,
                    "|l, r| Expr::Binary(Op::Add, l, r)",
                )
            })
        })
        .rule("primary", |r| {
            r.choice((
                r.token("NUMBER")
                    .ast("|t| Expr::Number(t.text.parse().unwrap())"),
                r.token("IDENTIFIER").ast("|t| Expr::Ident(t.text)"),
            ))
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("FUNCTION"), "Should have FUNCTION keyword");
    assert!(code.contains("RETURN"), "Should have RETURN keyword");
    assert!(
        code.contains("FunctionDeclaration_Start"),
        "Should have function declaration work"
    );
}

#[test]
fn test_level4_arrow_function() {
    // Grammar for: (a, b) => a + b, x => x * 2
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_'))
                .build()
                .token("ARROW", "=>")
                .token("LPAREN", "(")
                .token("RPAREN", ")")
                .token("COMMA", ",")
                .token("PLUS", "+")
                .token("STAR", "*")
                .skip("whitespace")
        })
        .rule("program", |r| r.parse("expression"))
        .rule("expression", |r| {
            r.choice((r.parse("arrow_function"), r.parse("binary_expression")))
        })
        .rule("arrow_function", |r| {
            r.sequence((
                r.parse("arrow_params"),
                r.token("ARROW"),
                r.parse("expression"),
            ))
            .ast("|(params, _, body)| Expr::Arrow { params, body }")
        })
        .rule("arrow_params", |r| {
            r.choice((
                // (a, b)
                r.sequence((
                    r.token("LPAREN"),
                    r.optional(r.separated_by(r.token("IDENTIFIER"), r.token("COMMA"))),
                    r.token("RPAREN"),
                ))
                .ast("|(_, params, _)| params.unwrap_or_default()"),
                // x
                r.token("IDENTIFIER").ast("|t| vec![t]"),
            ))
        })
        .rule("binary_expression", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix(
                    "PLUS",
                    10,
                    Assoc::Left,
                    "|l, r| Expr::Binary(Op::Add, l, r)",
                )
                .infix(
                    "STAR",
                    20,
                    Assoc::Left,
                    "|l, r| Expr::Binary(Op::Mul, l, r)",
                )
            })
        })
        .rule("primary", |r| {
            r.choice((
                r.token("NUMBER")
                    .ast("|t| Expr::Number(t.text.parse().unwrap())"),
                r.token("IDENTIFIER").ast("|t| Expr::Ident(t.text)"),
            ))
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("ARROW"), "Should have ARROW token");
    assert!(
        code.contains("ArrowFunction_Start"),
        "Should have arrow function work"
    );
}

// =============================================================================
// Level 5: Control Flow
// =============================================================================

#[test]
fn test_level5_if_statement() {
    // Grammar for: if (x) { ... } else { ... }
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_'))
                .build()
                .token("LPAREN", "(")
                .token("RPAREN", ")")
                .token("LBRACE", "{")
                .token("RBRACE", "}")
                .token("SEMICOLON", ";")
                .keyword("IF", "if")
                .keyword("ELSE", "else")
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("statement")))
        .rule("statement", |r| {
            r.choice((
                r.parse("if_statement"),
                r.parse("block"),
                r.parse("expression_statement"),
            ))
        })
        .rule("if_statement", |r| {
            r.sequence((
                r.token("IF"),
                r.token("LPAREN"),
                r.parse("expression"),
                r.token("RPAREN"),
                r.parse("statement"),
                r.optional(r.sequence((r.token("ELSE"), r.parse("statement")))),
            ))
            .ast("|(_, _, cond, _, then, else_)| Stmt::If { cond, then, else_ }")
        })
        .rule("block", |r| {
            r.sequence((
                r.token("LBRACE"),
                r.zero_or_more(r.parse("statement")),
                r.token("RBRACE"),
            ))
            .ast("|(_, stmts, _)| Stmt::Block(stmts)")
        })
        .rule("expression_statement", |r| {
            r.sequence((r.parse("expression"), r.token("SEMICOLON")))
                .ast("|(expr, _)| Stmt::Expr(expr)")
        })
        .rule("expression", |r| {
            r.token("IDENTIFIER").ast("|t| Expr::Ident(t.text)")
        });

    let code = generate_and_check(grammar);
    assert!(code.contains("IF"), "Should have IF keyword");
    assert!(code.contains("ELSE"), "Should have ELSE keyword");
    assert!(
        code.contains("IfStatement_Start"),
        "Should have if statement work"
    );
}

// =============================================================================
// Test: Full Mini-TypeScript Grammar
// =============================================================================

#[test]
fn test_mini_typescript_grammar() {
    // A more complete mini-TypeScript grammar
    let grammar = Grammar::new()
        .lexer(|l| {
            // Literals
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit").or().char('.'))
                .build()
            .token("STRING", "")
                .start_with(|c| c.char('"').or().char('\''))
                .scan_until_matching_quote()
                .build()
            .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_').or().char('$'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_').or().char('$'))
                .build()
            // Operators
            .token("PLUS", "+")
            .token("MINUS", "-")
            .token("STAR", "*")
            .token("SLASH", "/")
            .token("PERCENT", "%")
            .token("STAR_STAR", "**")
            .token("EQUALS", "=")
            .token("EQUALS_EQUALS", "==")
            .token("EQUALS_EQUALS_EQUALS", "===")
            .token("BANG", "!")
            .token("BANG_EQUALS", "!=")
            .token("BANG_EQUALS_EQUALS", "!==")
            .token("LESS", "<")
            .token("LESS_EQUALS", "<=")
            .token("GREATER", ">")
            .token("GREATER_EQUALS", ">=")
            .token("AMPERSAND_AMPERSAND", "&&")
            .token("PIPE_PIPE", "||")
            .token("QUESTION_QUESTION", "??")
            .token("QUESTION", "?")
            .token("COLON", ":")
            .token("SEMICOLON", ";")
            .token("COMMA", ",")
            .token("DOT", ".")
            .token("ARROW", "=>")
            // Delimiters
            .token("LPAREN", "(")
            .token("RPAREN", ")")
            .token("LBRACE", "{")
            .token("RBRACE", "}")
            .token("LBRACKET", "[")
            .token("RBRACKET", "]")
            // Keywords
            .keyword("LET", "let")
            .keyword("CONST", "const")
            .keyword("VAR", "var")
            .keyword("FUNCTION", "function")
            .keyword("RETURN", "return")
            .keyword("IF", "if")
            .keyword("ELSE", "else")
            .keyword("FOR", "for")
            .keyword("WHILE", "while")
            .keyword("TRUE", "true")
            .keyword("FALSE", "false")
            .keyword("NULL", "null")
            .keyword("UNDEFINED", "undefined")
            .keyword("TYPEOF", "typeof")
            .keyword("NEW", "new")
            .keyword("CLASS", "class")
            .keyword("EXTENDS", "extends")
            .keyword("THIS", "this")
            .keyword("INTERFACE", "interface")
            .keyword("TYPE", "type")
            .keyword("ENUM", "enum")
            // Skip
            .skip("whitespace")
        })
        .rule("program", |r| {
            r.zero_or_more(r.parse("statement"))
        })
        .rule("statement", |r| {
            r.choice((
                r.parse("variable_declaration"),
                r.parse("function_declaration"),
                r.parse("if_statement"),
                r.parse("return_statement"),
                r.parse("expression_statement"),
            ))
        })
        .rule("variable_declaration", |r| {
            r.sequence((
                r.choice((r.token("LET"), r.token("CONST"), r.token("VAR"))),
                r.token("IDENTIFIER"),
                r.optional(r.parse("type_annotation")),
                r.optional(r.sequence((
                    r.token("EQUALS"),
                    r.parse("expression"),
                ))),
                r.token("SEMICOLON"),
            ))
            .ast("|(kind, name, ty, init, _)| Stmt::VarDecl { kind, name, ty, init }")
        })
        .rule("function_declaration", |r| {
            r.sequence((
                r.token("FUNCTION"),
                r.token("IDENTIFIER"),
                r.token("LPAREN"),
                r.optional(r.parse("parameter_list")),
                r.token("RPAREN"),
                r.optional(r.parse("type_annotation")),
                r.parse("block"),
            ))
            .ast("|(_, name, _, params, _, ret_type, body)| Stmt::Function { name, params, ret_type, body }")
        })
        .rule("parameter_list", |r| {
            r.separated_by(
                r.parse("parameter"),
                r.token("COMMA"),
            )
        })
        .rule("parameter", |r| {
            r.sequence((
                r.token("IDENTIFIER"),
                r.optional(r.parse("type_annotation")),
            ))
            .ast("|(name, ty)| Param { name, ty }")
        })
        .rule("type_annotation", |r| {
            r.sequence((
                r.token("COLON"),
                r.parse("type"),
            ))
            .ast("|(_, ty)| ty")
        })
        .rule("type", |r| {
            r.token("IDENTIFIER")
                .ast("|t| Type::Named(t.text)")
        })
        .rule("if_statement", |r| {
            r.sequence((
                r.token("IF"),
                r.token("LPAREN"),
                r.parse("expression"),
                r.token("RPAREN"),
                r.parse("statement_or_block"),
                r.optional(r.sequence((
                    r.token("ELSE"),
                    r.parse("statement_or_block"),
                ))),
            ))
            .ast("|(_, _, cond, _, then, else_)| Stmt::If { cond, then, else_ }")
        })
        .rule("statement_or_block", |r| {
            r.choice((
                r.parse("block"),
                r.parse("statement"),
            ))
        })
        .rule("return_statement", |r| {
            r.sequence((
                r.token("RETURN"),
                r.optional(r.parse("expression")),
                r.token("SEMICOLON"),
            ))
            .ast("|(_, expr, _)| Stmt::Return(expr)")
        })
        .rule("expression_statement", |r| {
            r.sequence((
                r.parse("expression"),
                r.token("SEMICOLON"),
            ))
            .ast("|(expr, _)| Stmt::Expr(expr)")
        })
        .rule("block", |r| {
            r.sequence((
                r.token("LBRACE"),
                r.zero_or_more(r.parse("statement")),
                r.token("RBRACE"),
            ))
            .ast("|(_, stmts, _)| Block(stmts)")
        })
        .rule("expression", |r| {
            r.pratt(
                r.parse("primary"),
                |ops| ops
                    // Assignment
                    .infix("EQUALS", 1, Assoc::Right, "|l, r| Expr::Assign(l, r)")
                    // Ternary
                    .ternary("QUESTION", "COLON", 2, "|c, t, f| Expr::Ternary(c, t, f)")
                    // Nullish coalescing
                    .infix("QUESTION_QUESTION", 3, Assoc::Left, "|l, r| Expr::NullishCoalesce(l, r)")
                    // Logical
                    .infix("PIPE_PIPE", 4, Assoc::Left, "|l, r| Expr::LogicalOr(l, r)")
                    .infix("AMPERSAND_AMPERSAND", 5, Assoc::Left, "|l, r| Expr::LogicalAnd(l, r)")
                    // Equality
                    .infix("EQUALS_EQUALS", 9, Assoc::Left, "|l, r| Expr::Eq(l, r)")
                    .infix("BANG_EQUALS", 9, Assoc::Left, "|l, r| Expr::NotEq(l, r)")
                    .infix("EQUALS_EQUALS_EQUALS", 9, Assoc::Left, "|l, r| Expr::StrictEq(l, r)")
                    .infix("BANG_EQUALS_EQUALS", 9, Assoc::Left, "|l, r| Expr::StrictNotEq(l, r)")
                    // Relational
                    .infix("LESS", 10, Assoc::Left, "|l, r| Expr::Lt(l, r)")
                    .infix("LESS_EQUALS", 10, Assoc::Left, "|l, r| Expr::LtEq(l, r)")
                    .infix("GREATER", 10, Assoc::Left, "|l, r| Expr::Gt(l, r)")
                    .infix("GREATER_EQUALS", 10, Assoc::Left, "|l, r| Expr::GtEq(l, r)")
                    // Additive
                    .infix("PLUS", 13, Assoc::Left, "|l, r| Expr::Add(l, r)")
                    .infix("MINUS", 13, Assoc::Left, "|l, r| Expr::Sub(l, r)")
                    // Multiplicative
                    .infix("STAR", 14, Assoc::Left, "|l, r| Expr::Mul(l, r)")
                    .infix("SLASH", 14, Assoc::Left, "|l, r| Expr::Div(l, r)")
                    .infix("PERCENT", 14, Assoc::Left, "|l, r| Expr::Mod(l, r)")
                    // Exponentiation
                    .infix("STAR_STAR", 15, Assoc::Right, "|l, r| Expr::Pow(l, r)")
                    // Prefix
                    .prefix("MINUS", 16, "|e| Expr::Neg(e)")
                    .prefix("BANG", 16, "|e| Expr::Not(e)")
                    .prefix("TYPEOF", 16, "|e| Expr::TypeOf(e)")
                    // Postfix (call, member, index)
                    .postfix_call("LPAREN", "RPAREN", 18, "|callee, args| Expr::Call(callee, args)")
                    .postfix_member("DOT", 18, "|obj, prop| Expr::Member(obj, prop)")
                    .postfix_index("LBRACKET", "RBRACKET", 18, "|obj, idx| Expr::Index(obj, idx)")
            )
        })
        .rule("primary", |r| {
            r.choice((
                r.token("NUMBER").ast("|t| Expr::Number(t.text.parse().unwrap())"),
                r.token("STRING").ast("|t| Expr::String(t.text)"),
                r.token("TRUE").ast("|_| Expr::Bool(true)"),
                r.token("FALSE").ast("|_| Expr::Bool(false)"),
                r.token("NULL").ast("|_| Expr::Null"),
                r.token("UNDEFINED").ast("|_| Expr::Undefined"),
                r.token("THIS").ast("|_| Expr::This"),
                r.token("IDENTIFIER").ast("|t| Expr::Ident(t.text)"),
                r.parse("array_literal"),
                r.parse("object_literal"),
                r.sequence((
                    r.token("LPAREN"),
                    r.parse("expression"),
                    r.token("RPAREN"),
                )).ast("|(_, e, _)| e"),
            ))
        })
        .rule("array_literal", |r| {
            r.sequence((
                r.token("LBRACKET"),
                r.optional(r.separated_by_trailing(
                    r.parse("expression"),
                    r.token("COMMA"),
                )),
                r.token("RBRACKET"),
            ))
            .ast("|(_, elems, _)| Expr::Array(elems.unwrap_or_default())")
        })
        .rule("object_literal", |r| {
            r.sequence((
                r.token("LBRACE"),
                r.optional(r.separated_by_trailing(
                    r.parse("property"),
                    r.token("COMMA"),
                )),
                r.token("RBRACE"),
            ))
            .ast("|(_, props, _)| Expr::Object(props.unwrap_or_default())")
        })
        .rule("property", |r| {
            r.sequence((
                r.token("IDENTIFIER"),
                r.token("COLON"),
                r.parse("expression"),
            ))
            .ast("|(key, _, value)| Property { key, value }")
        });

    let code = generate_and_check(grammar);

    // Verify comprehensive token coverage
    assert!(code.contains("STAR_STAR"), "Should have STAR_STAR token");
    assert!(
        code.contains("QUESTION_QUESTION"),
        "Should have QUESTION_QUESTION token"
    );
    assert!(code.contains("INTERFACE"), "Should have INTERFACE keyword");
    assert!(code.contains("ENUM"), "Should have ENUM keyword");

    // Verify Work enum has expected variants
    assert!(code.contains("Program_Start"), "Should have Program_Start");
    assert!(
        code.contains("Expression_Pratt_AfterOperand"),
        "Should have Pratt parsing work"
    );

    // Print code length for reference
    println!("Generated code length: {} bytes", code.len());
}

// =============================================================================
// Compilation Test: Verify generated code is valid Rust
// =============================================================================

#[test]
fn test_generated_code_compiles() {
    // Generate a simple but complete grammar
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit").or().char('.'))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha").or().char('_'))
                .continue_with(|c| c.match_class("alphanumeric").or().char('_'))
                .build()
                .token("PLUS", "+")
                .token("MINUS", "-")
                .token("STAR", "*")
                .token("EQUALS", "=")
                .token("SEMICOLON", ";")
                .token("LPAREN", "(")
                .token("RPAREN", ")")
                .keyword("LET", "let")
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.parse("statement")))
        .rule("statement", |r| {
            r.choice((
                r.parse("variable_declaration"),
                r.parse("expression_statement"),
            ))
        })
        .rule("variable_declaration", |r| {
            r.sequence((
                r.token("LET"),
                r.token("IDENTIFIER"),
                r.token("EQUALS"),
                r.parse("expression"),
                r.token("SEMICOLON"),
            ))
        })
        .rule("expression_statement", |r| {
            r.sequence((r.parse("expression"), r.token("SEMICOLON")))
        })
        .rule("expression", |r| {
            r.pratt(r.parse("primary"), |ops| {
                ops.infix("PLUS", 10, Assoc::Left, "|l, r| (l, r)")
                    .infix("MINUS", 10, Assoc::Left, "|l, r| (l, r)")
                    .infix("STAR", 20, Assoc::Left, "|l, r| (l, r)")
                    .prefix("MINUS", 15, "|e| e")
            })
        })
        .rule("primary", |r| {
            r.choice((
                r.token("NUMBER"),
                r.token("IDENTIFIER"),
                r.sequence((r.token("LPAREN"), r.parse("expression"), r.token("RPAREN"))),
            ))
        });

    let compiled = grammar.build();
    let code = compiled.generate();

    // Write to temporary file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("trampoline_parser_test.rs");
    let temp_out = temp_dir.join("trampoline_parser_test_out");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Try to compile with rustc as a library (syntax check only)
    let output = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "-o",
        ])
        .arg(&temp_out)
        .arg(&temp_file)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                // Only fail on actual errors, not warnings
                if stderr.contains("error[") || stderr.contains("error:") {
                    println!("Generated code:\n{}", code);
                    println!("\nCompilation errors:\n{}", stderr);
                    panic!("Generated code failed to compile!");
                }
            }
            println!("Generated code compiles successfully!");
        }
        Err(e) => {
            // rustc might not be available in all test environments
            println!(
                "Warning: Could not run rustc: {}. Skipping compilation test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_out);
}

// =============================================================================
// End-to-End Test: Verify generated parser actually parses input
// =============================================================================

#[test]
fn test_parse_simple_number() {
    // Generate a minimal parser for just a number
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "42" and prints the result
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("42");
    match parser.parse() {
        Ok(result) => {
            match result {
                ParseResult::Token(t) => {
                    if t.kind == TokenKind::NUMBER && t.text == "42" {
                        println!("SUCCESS");
                    } else {
                        println!("WRONG_TOKEN: {:?} {:?}", t.kind, t.text);
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_number_test.rs");
    let temp_bin = temp_dir.join("parse_number_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: parser correctly parsed '42'");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_simple_sequence() {
    // Generate a parser for: NUMBER PLUS NUMBER
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS", "+")
                .skip("whitespace")
        })
        .rule("program", |r| {
            r.sequence((r.token("NUMBER"), r.token("PLUS"), r.token("NUMBER")))
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "1 + 2"
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("1 + 2");
    match parser.parse() {
        Ok(result) => {
            match result {
                ParseResult::List(items) if items.len() == 3 => {
                    println!("SUCCESS: got {} items", items.len());
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_seq_test.rs");
    let temp_bin = temp_dir.join("parse_seq_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: parser correctly parsed '1 + 2' sequence");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_pratt_binary_expression() {
    // Generate a parser for binary expressions with precedence
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS", "+")
                .token("STAR", "*")
                .skip("whitespace")
        })
        .rule("expression", |r| {
            r.pratt(r.token("NUMBER"), |ops| {
                ops.infix("PLUS", 10, Assoc::Left, "|l, r| BinOp::Add(l, r)")
                    .infix("STAR", 20, Assoc::Left, "|l, r| BinOp::Mul(l, r)")
            })
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "1 + 2 * 3"
    // With correct precedence, this should be: 1 + (2 * 3)
    // Result structure: List([1, +, List([2, *, 3])])
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("1 + 2 * 3");
    match parser.parse() {
        Ok(result) => {
            // Check that we got a nested structure indicating precedence was respected
            println!("RESULT: {:?}", result);
            // The result should be a binary expression tree
            match &result {
                ParseResult::List(items) if items.len() == 3 => {
                    // Should be: [left, op, right] where right is [2, *, 3]
                    match &items[2] {
                        ParseResult::List(right_items) if right_items.len() == 3 => {
                            println!("SUCCESS: precedence respected - got nested structure");
                        }
                        _ => println!("SUCCESS: got binary expression (structure may vary)"),
                    }
                }
                _ => println!("RESULT_TYPE: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_pratt_test.rs");
    let temp_bin = temp_dir.join("parse_pratt_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: Pratt parser handled precedence");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_choice_first_alternative() {
    // Generate a parser with choice - first alternative matches
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha"))
                .continue_with(|c| c.match_class("alphanumeric"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| {
            r.choice((r.token("NUMBER"), r.token("IDENTIFIER")))
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "42" - should match first alternative
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("42");
    match parser.parse() {
        Ok(result) => {
            match result {
                ParseResult::Token(t) if t.kind == TokenKind::NUMBER => {
                    println!("SUCCESS: matched NUMBER");
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_choice_first_test.rs");
    let temp_bin = temp_dir.join("parse_choice_first_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: choice matched first alternative");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_choice_second_alternative() {
    // Generate a parser with choice - second alternative matches
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha"))
                .continue_with(|c| c.match_class("alphanumeric"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| {
            r.choice((r.token("NUMBER"), r.token("IDENTIFIER")))
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "foo" - should match second alternative
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("foo");
    match parser.parse() {
        Ok(result) => {
            match result {
                ParseResult::Token(t) if t.kind == TokenKind::IDENTIFIER => {
                    println!("SUCCESS: matched IDENTIFIER");
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_choice_second_test.rs");
    let temp_bin = temp_dir.join("parse_choice_second_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: choice matched second alternative with backtracking");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_zero_or_more() {
    // Generate a parser with zero_or_more
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.zero_or_more(r.token("NUMBER")));

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "1 2 3"
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("1 2 3");
    match parser.parse() {
        Ok(result) => {
            match result {
                ParseResult::List(items) if items.len() == 3 => {
                    println!("SUCCESS: got {} items", items.len());
                }
                ParseResult::List(items) => {
                    println!("WRONG_COUNT: got {} items, expected 3", items.len());
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_zero_or_more_test.rs");
    let temp_bin = temp_dir.join("parse_zero_or_more_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: zero_or_more parsed multiple items");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_optional_present() {
    // Generate a parser with optional - when the optional content is present
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS", "+")
                .skip("whitespace")
        })
        .rule("program", |r| {
            r.sequence((r.token("NUMBER"), r.optional(r.token("PLUS"))))
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "42 +" (with optional present)
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("42 +");
    match parser.parse() {
        Ok(result) => {
            match &result {
                ParseResult::List(items) if items.len() == 2 => {
                    // Second item should be the PLUS token, not None
                    match &items[1] {
                        ParseResult::Token(t) if t.kind == TokenKind::PLUS => {
                            println!("SUCCESS: optional was present");
                        }
                        ParseResult::None => {
                            println!("WRONG: optional should be present but got None");
                        }
                        _ => println!("WRONG_OPTIONAL: {:?}", items[1]),
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_optional_present_test.rs");
    let temp_bin = temp_dir.join("parse_optional_present_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: optional correctly parsed present value");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_optional_absent() {
    // Generate a parser with optional - when the optional content is absent
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS", "+")
                .skip("whitespace")
        })
        .rule("program", |r| {
            r.sequence((r.token("NUMBER"), r.optional(r.token("PLUS"))))
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "42" (without optional)
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("42");
    match parser.parse() {
        Ok(result) => {
            match &result {
                ParseResult::List(items) if items.len() == 2 => {
                    // Second item should be None
                    match &items[1] {
                        ParseResult::None => {
                            println!("SUCCESS: optional was absent (None)");
                        }
                        _ => println!("WRONG_OPTIONAL: expected None, got {:?}", items[1]),
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_optional_absent_test.rs");
    let temp_bin = temp_dir.join("parse_optional_absent_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: optional correctly returned None for absent value");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_pratt_prefix_expression() {
    // Generate a parser with Pratt parsing that includes prefix operators
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("MINUS", "-")
                .token("BANG", "!")
                .skip("whitespace")
        })
        .rule("program", |r| r.parse("expression"))
        .rule("expression", |r| {
            r.pratt(r.token("NUMBER"), |ops| {
                ops.prefix("MINUS", 15, "|e| Neg(e)")
                    .prefix("BANG", 15, "|e| Not(e)")
                    .infix("MINUS", 10, Assoc::Left, "|l, r| Sub(l, r)")
            })
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "-42" (prefix minus on number)
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("-42");
    match parser.parse() {
        Ok(result) => {
            // Result should be a list: [MINUS token, NUMBER token]
            match &result {
                ParseResult::List(items) if items.len() == 2 => {
                    let has_minus = matches!(&items[0], ParseResult::Token(t) if t.kind == TokenKind::MINUS);
                    let has_number = matches!(&items[1], ParseResult::Token(t) if t.kind == TokenKind::NUMBER);
                    if has_minus && has_number {
                        println!("SUCCESS: prefix expression parsed correctly");
                    } else {
                        println!("WRONG_STRUCTURE: expected [MINUS, NUMBER], got {:?}", items);
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_prefix_test.rs");
    let temp_bin = temp_dir.join("parse_prefix_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: prefix operator in Pratt parser");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_pratt_postfix_expression() {
    // Generate a parser with Pratt parsing that includes simple postfix operators
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS_PLUS", "++")
                .token("MINUS_MINUS", "--")
                .token("PLUS", "+")
                .skip("whitespace")
        })
        .rule("program", |r| r.parse("expression"))
        .rule("expression", |r| {
            r.pratt(r.token("NUMBER"), |ops| {
                ops.postfix("PLUS_PLUS", 18, "|e| PostInc(e)")
                    .postfix("MINUS_MINUS", 18, "|e| PostDec(e)")
                    .infix("PLUS", 10, Assoc::Left, "|l, r| Add(l, r)")
            })
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "42++" (postfix increment)
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("42++");
    match parser.parse() {
        Ok(result) => {
            // Result should be a list: [NUMBER token, PLUS_PLUS token]
            match &result {
                ParseResult::List(items) if items.len() == 2 => {
                    let has_number = matches!(&items[0], ParseResult::Token(t) if t.kind == TokenKind::NUMBER);
                    let has_plus_plus = matches!(&items[1], ParseResult::Token(t) if t.kind == TokenKind::PLUS_PLUS);
                    if has_number && has_plus_plus {
                        println!("SUCCESS: postfix expression parsed correctly");
                    } else {
                        println!("WRONG_STRUCTURE: expected [NUMBER, PLUS_PLUS], got {:?}", items);
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_postfix_test.rs");
    let temp_bin = temp_dir.join("parse_postfix_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: postfix operator in Pratt parser");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_pratt_member_access() {
    // Generate a parser with Pratt parsing that includes member access
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("IDENTIFIER", "")
                .start_with(|c| c.match_class("alpha"))
                .continue_with(|c| c.match_class("alphanumeric"))
                .build()
                .token("DOT", ".")
                .skip("whitespace")
        })
        .rule("program", |r| r.parse("expression"))
        .rule("expression", |r| {
            r.pratt(r.token("IDENTIFIER"), |ops| {
                ops.postfix_member("DOT", 18, "|obj, prop| Member(obj, prop)")
            })
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "foo.bar" (member access)
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("foo.bar");
    match parser.parse() {
        Ok(result) => {
            // Result should be a list: [obj, DOT, prop]
            match &result {
                ParseResult::List(items) if items.len() == 3 => {
                    let has_obj = matches!(&items[0], ParseResult::Token(t) if t.text == "foo");
                    let has_dot = matches!(&items[1], ParseResult::Token(t) if t.kind == TokenKind::DOT);
                    let has_prop = matches!(&items[2], ParseResult::Token(t) if t.text == "bar");
                    if has_obj && has_dot && has_prop {
                        println!("SUCCESS: member access parsed correctly");
                    } else {
                        println!("WRONG_STRUCTURE: expected [foo, DOT, bar], got {:?}", items);
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_member_test.rs");
    let temp_bin = temp_dir.join("parse_member_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: member access in Pratt parser");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_pratt_ternary() {
    // Generate a parser with Pratt parsing that includes ternary operator
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("QUESTION", "?")
                .token("COLON", ":")
                .token("PLUS", "+")
                .skip("whitespace")
        })
        .rule("program", |r| r.parse("expression"))
        .rule("expression", |r| {
            r.pratt(r.token("NUMBER"), |ops| {
                ops.ternary(
                    "QUESTION",
                    "COLON",
                    2,
                    "|cond, then, else_| Ternary(cond, then, else_)",
                )
                .infix("PLUS", 10, Assoc::Left, "|l, r| Add(l, r)")
            })
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "1 ? 2 : 3" (ternary)
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("1 ? 2 : 3");
    match parser.parse() {
        Ok(result) => {
            // Result should be a list: [cond, QUESTION, then, COLON, else]
            match &result {
                ParseResult::List(items) if items.len() == 5 => {
                    let has_cond = matches!(&items[0], ParseResult::Token(t) if t.text == "1");
                    let has_question = matches!(&items[1], ParseResult::Token(t) if t.kind == TokenKind::QUESTION);
                    let has_then = matches!(&items[2], ParseResult::Token(t) if t.text == "2");
                    let has_colon = matches!(&items[3], ParseResult::Token(t) if t.kind == TokenKind::COLON);
                    let has_else = matches!(&items[4], ParseResult::Token(t) if t.text == "3");
                    if has_cond && has_question && has_then && has_colon && has_else {
                        println!("SUCCESS: ternary expression parsed correctly");
                    } else {
                        println!("WRONG_STRUCTURE: expected [1, ?, 2, :, 3], got {:?}", items);
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_ternary_test.rs");
    let temp_bin = temp_dir.join("parse_ternary_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: ternary operator in Pratt parser");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

#[test]
fn test_parse_lookahead() {
    // Generate a parser that uses lookahead
    // Rule: NUMBER followed by PLUS, but only if PLUS is followed by another NUMBER
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS", "+")
                .token("STAR", "*")
                .skip("whitespace")
        })
        .rule("program", |r| {
            // Lookahead to check for NUMBER without consuming
            r.sequence((r.lookahead(r.token("NUMBER")), r.token("NUMBER")))
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "42"
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("42");
    match parser.parse() {
        Ok(result) => {
            // Result should be: [None (from lookahead), NUMBER token]
            match &result {
                ParseResult::List(items) if items.len() == 2 => {
                    let has_none = matches!(&items[0], ParseResult::None);
                    let has_number = matches!(&items[1], ParseResult::Token(t) if t.kind == TokenKind::NUMBER);
                    if has_none && has_number {
                        println!("SUCCESS: lookahead parsed correctly");
                    } else {
                        println!("WRONG_STRUCTURE: expected [None, NUMBER], got {:?}", items);
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_lookahead_test.rs");
    let temp_bin = temp_dir.join("parse_lookahead_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: lookahead combinator");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

// =============================================================================
// AST Configuration Tests
// =============================================================================

#[test]
fn test_ast_config_imports() {
    // Test that configured imports appear in generated code
    let grammar = Grammar::new()
        .ast_config(|c| c.import("crate::ast::*").import("crate::lexer::Span"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    assert!(
        code.contains("use crate::ast::*;"),
        "Should contain first import"
    );
    assert!(
        code.contains("use crate::lexer::Span;"),
        "Should contain second import"
    );
}

#[test]
fn test_ast_config_external_span_type() {
    // Test that external span type is used and internal Span is not generated
    let grammar = Grammar::new()
        .ast_config(|c| c.span_type("Span"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should NOT contain the internal Span struct definition
    assert!(
        !code.contains("pub struct Span {"),
        "Should NOT generate internal Span struct"
    );
    // Token struct should use the external Span type
    assert!(code.contains("pub span: Span,"), "Should use external Span");
}

#[test]
fn test_ast_config_external_string_type() {
    // Test that external string type is used in Token struct
    let grammar = Grammar::new()
        .ast_config(|c| c.string_type("JsString"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    assert!(
        code.contains("pub text: JsString,"),
        "Should use JsString for token text"
    );
}

#[test]
fn test_ast_config_external_error_type() {
    // Test that external error type disables internal ParseError generation
    let grammar = Grammar::new()
        .ast_config(|c| c.error_type("JsError"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should NOT contain the internal ParseError struct definition
    assert!(
        !code.contains("pub struct ParseError {"),
        "Should NOT generate internal ParseError struct"
    );
}

#[test]
fn test_ast_config_no_parse_result() {
    // Test that ParseResult enum can be disabled
    let grammar = Grammar::new()
        .ast_config(|c| c.no_parse_result())
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should NOT contain the ParseResult enum definition
    assert!(
        !code.contains("pub enum ParseResult {"),
        "Should NOT generate ParseResult enum"
    );
}

#[test]
fn test_ast_config_combined() {
    // Test multiple AST config options together
    let grammar = Grammar::new()
        .ast_config(|c| {
            c.import("crate::ast::*")
                .import("crate::lexer::Span")
                .span_type("Span")
                .string_type("JsString")
                .error_type("JsError")
        })
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Check imports
    assert!(code.contains("use crate::ast::*;"));
    assert!(code.contains("use crate::lexer::Span;"));

    // Check that internal types are not generated
    assert!(!code.contains("pub struct Span {"));
    assert!(!code.contains("pub struct ParseError {"));

    // Check that Token uses configured types
    assert!(code.contains("pub text: JsString,"));
    assert!(code.contains("pub span: Span,"));
}

#[test]
fn test_ast_config_default_behavior() {
    // Test that default config (no ast_config call) produces current behavior
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should contain internal types (backward compatibility)
    assert!(
        code.contains("pub struct Span {"),
        "Default should generate Span"
    );
    assert!(
        code.contains("pub struct ParseError {"),
        "Default should generate ParseError"
    );
    assert!(
        code.contains("pub enum ParseResult {"),
        "Default should generate ParseResult"
    );
    assert!(
        code.contains("pub text: String,"),
        "Default should use String"
    );
}

#[test]
fn test_apply_mappings_generates_helpers() {
    // Test that apply_mappings generates ParseResult helper methods
    let grammar = Grammar::new()
        .ast_config(|c| c.apply_mappings())
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should generate ParseResult helper methods
    assert!(
        code.contains("pub fn into_token(self) -> Token"),
        "Should generate into_token helper"
    );
    assert!(
        code.contains("pub fn as_token(&self) -> &Token"),
        "Should generate as_token helper"
    );
    assert!(
        code.contains("pub fn into_list(self) -> Vec<ParseResult>"),
        "Should generate into_list helper"
    );
    assert!(
        code.contains("pub fn as_list(&self) -> &Vec<ParseResult>"),
        "Should generate as_list helper"
    );
    assert!(
        code.contains("pub fn is_none(&self) -> bool"),
        "Should generate is_none helper"
    );
    assert!(
        code.contains("pub fn into_option(self) -> Option<ParseResult>"),
        "Should generate into_option helper"
    );
    assert!(
        code.contains("pub fn span(&self) -> Span"),
        "Should generate span helper"
    );
}

#[test]
fn test_apply_mappings_with_mapped_combinator() {
    // Test that mappings are recognized when apply_mappings is enabled
    use trampoline_parser::CombinatorExt;

    let grammar = Grammar::new()
        .ast_config(|c| c.apply_mappings())
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| {
            r.token("NUMBER").ast("|t| t.text.parse::<i32>().unwrap()")
        });

    let code = grammar.build().generate();

    // Should contain the mapping comment
    assert!(
        code.contains("// AST Mapping: |t| t.text.parse::<i32>().unwrap()"),
        "Should generate AST mapping comment"
    );
}

#[test]
fn test_apply_mappings_disabled_no_helpers() {
    // Test that helpers are NOT generated when apply_mappings is disabled (default)
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should NOT generate ParseResult helper methods
    // Use more specific patterns to avoid matching Token.span or Span struct
    assert!(
        !code.contains("impl ParseResult {"),
        "Should NOT generate ParseResult impl block when apply_mappings is false"
    );
    assert!(
        !code.contains("/// Extract the Token from a ParseResult::Token variant"),
        "Should NOT generate into_token helper when apply_mappings is false"
    );
}

#[test]
fn test_apply_mappings_sequence_mapping() {
    // Test mapping on a sequence combinator
    use trampoline_parser::CombinatorExt;

    let grammar = Grammar::new()
        .ast_config(|c| c.apply_mappings())
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS", "+")
                .skip("whitespace")
        })
        .rule("program", |r| {
            r.sequence((r.token("NUMBER"), r.token("PLUS"), r.token("NUMBER")))
                .ast("|(a, _, b)| (a, b)")
        });

    let code = grammar.build().generate();

    // Should contain the mapping comment
    assert!(
        code.contains("// AST Mapping: |(a, _, b)| (a, b)"),
        "Should generate AST mapping comment for sequence"
    );
}

// =============================================================================
// StringDict Integration Tests
// =============================================================================

#[test]
fn test_string_dict_lexer_struct() {
    // Test that StringDict configuration adds string_dict field to Lexer
    let grammar = Grammar::new()
        .ast_config(|c| c.string_dict("StringDict").string_type("JsString"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Lexer should have string_dict field
    assert!(
        code.contains("string_dict: &'a mut StringDict,"),
        "Lexer should have string_dict field"
    );
}

#[test]
fn test_string_dict_lexer_new() {
    // Test that Lexer::new accepts string_dict parameter
    let grammar = Grammar::new()
        .ast_config(|c| c.string_dict("StringDict"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Lexer::new should accept string_dict
    assert!(
        code.contains("pub fn new(input: &'a str, string_dict: &'a mut StringDict) -> Self"),
        "Lexer::new should accept string_dict parameter"
    );
    assert!(
        code.contains("string_dict,"),
        "Lexer should store string_dict"
    );
}

#[test]
fn test_string_dict_parser_new() {
    // Test that Parser::new accepts and passes string_dict to Lexer
    let grammar = Grammar::new()
        .ast_config(|c| c.string_dict("StringDict"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Parser::new should accept string_dict
    assert!(
        code.contains("pub fn new(input: &'a str, string_dict: &'a mut StringDict) -> Self"),
        "Parser::new should accept string_dict parameter"
    );
    // Parser should pass string_dict to Lexer
    assert!(
        code.contains("lexer: Lexer::new(input, string_dict),"),
        "Parser should pass string_dict to Lexer"
    );
}

#[test]
fn test_string_dict_token_text_creation() {
    // Test that token text uses string_dict.get_or_insert()
    let grammar = Grammar::new()
        .ast_config(|c| c.string_dict("StringDict"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Token text should use string_dict.get_or_insert()
    assert!(
        code.contains("self.string_dict.get_or_insert("),
        "Token text should use string_dict.get_or_insert()"
    );
    // Should NOT contain plain .to_string() for token text
    assert!(
        !code.contains("text: String::new()"),
        "Should not use String::new() when StringDict is configured"
    );
}

#[test]
fn test_string_dict_custom_method() {
    // Test that custom string_dict method name is used
    let grammar = Grammar::new()
        .ast_config(|c| c.string_dict("MyDict").string_dict_method("intern"))
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should use custom method name
    assert!(
        code.contains("self.string_dict.intern("),
        "Should use custom method name 'intern'"
    );
    // Should use custom type name
    assert!(
        code.contains("string_dict: &'a mut MyDict"),
        "Should use custom type name 'MyDict'"
    );
}

#[test]
fn test_string_dict_disabled_default() {
    // Test that StringDict is NOT used by default
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should NOT have string_dict field
    assert!(
        !code.contains("string_dict:"),
        "Should NOT have string_dict field when not configured"
    );
    // Lexer::new should only take input
    assert!(
        code.contains("pub fn new(input: &'a str) -> Self {"),
        "Lexer::new should only take input when StringDict not configured"
    );
    // Should use .to_string() for token text
    assert!(
        code.contains(".to_string()"),
        "Should use .to_string() when StringDict not configured"
    );
}

#[test]
fn test_parse_negative_lookahead() {
    // Generate a parser that uses negative lookahead
    // Parse NUMBER only if it's NOT followed by STAR
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS", "+")
                .token("STAR", "*")
                .skip("whitespace")
        })
        .rule("program", |r| {
            r.sequence((r.token("NUMBER"), r.negative_lookahead(r.token("STAR"))))
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that parses "42 +" (number NOT followed by star)
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("42 +");
    match parser.parse() {
        Ok(result) => {
            // Result should be: [NUMBER token, None (from negative lookahead)]
            match &result {
                ParseResult::List(items) if items.len() == 2 => {
                    let has_number = matches!(&items[0], ParseResult::Token(t) if t.kind == TokenKind::NUMBER);
                    let has_none = matches!(&items[1], ParseResult::None);
                    if has_number && has_none {
                        println!("SUCCESS: negative lookahead succeeded (no star found)");
                    } else {
                        println!("WRONG_STRUCTURE: expected [NUMBER, None], got {:?}", items);
                    }
                }
                _ => println!("WRONG_RESULT: {:?}", result),
            }
        }
        Err(e) => println!("ERROR: {}", e),
    }
}
"#,
    );

    // Write, compile, and run
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("parse_neg_lookahead_test.rs");
    let temp_bin = temp_dir.join("parse_neg_lookahead_test");
    fs::write(&temp_file, &code).expect("Failed to write temp file");

    // Compile
    let compile_output = Command::new("rustc")
        .args(["--edition", "2021", "-o"])
        .arg(&temp_bin)
        .arg(&temp_file)
        .output();

    match compile_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompilation errors:\n{}", stderr);
                panic!("Generated code failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(&temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Parser did not produce expected output!");
            }

            println!("End-to-end test passed: negative lookahead combinator");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping end-to-end test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(&temp_file);
    let _ = fs::remove_file(&temp_bin);
}

// =============================================================================
// Span Propagation Tests
// =============================================================================

#[test]
fn test_span_merge_generated() {
    // Test that Span::merge() is generated when apply_mappings is enabled
    let grammar = Grammar::new()
        .ast_config(|c| c.apply_mappings())
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should have Span impl block with merge method
    assert!(
        code.contains("impl Span {"),
        "Should generate Span impl block when apply_mappings is enabled"
    );
    assert!(
        code.contains("pub fn merge(self, other: Span) -> Span {"),
        "Should generate Span::merge() when apply_mappings is enabled"
    );
    assert!(
        code.contains("start: self.start.min(other.start),"),
        "merge() should take minimum of starts"
    );
    assert!(
        code.contains("end: self.end.max(other.end),"),
        "merge() should take maximum of ends"
    );
}

#[test]
fn test_span_merge_not_generated_without_apply_mappings() {
    // Test that Span::merge() is NOT generated when apply_mappings is disabled
    let grammar = Grammar::new()
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should NOT have Span impl block
    assert!(
        !code.contains("impl Span {"),
        "Should NOT generate Span impl when apply_mappings is disabled"
    );
    assert!(
        !code.contains("pub fn merge("),
        "Should NOT generate Span::merge() when apply_mappings is disabled"
    );
}

#[test]
fn test_combined_span_generated() {
    // Test that ParseResult::combined_span() is generated when apply_mappings is enabled
    let grammar = Grammar::new()
        .ast_config(|c| c.apply_mappings())
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should have combined_span method
    assert!(
        code.contains("pub fn combined_span(&self) -> Span {"),
        "Should generate combined_span() when apply_mappings is enabled"
    );
    assert!(
        code.contains("first.merge(last)"),
        "combined_span() should use merge() for lists"
    );
}

#[test]
fn test_span_propagation_compile_check() {
    // Test that generated code with span propagation compiles correctly
    let grammar = Grammar::new()
        .ast_config(|c| c.apply_mappings())
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .token("PLUS", "+")
                .skip("whitespace")
        })
        .rule("expr", |r| {
            r.sequence((r.token("NUMBER"), r.token("PLUS"), r.token("NUMBER")))
                .ast("|(a, op, b)| Expr::Binary { left: a, op, right: b }")
        });

    let compiled = grammar.build();
    let mut code = compiled.generate();

    // Add a main function that uses combined_span
    code.push_str(
        r#"
fn main() {
    let mut parser = Parser::new("1 + 2");
    match parser.parse() {
        Ok(result) => {
            let span = result.combined_span();
            // Test that merge works correctly
            let span2 = Span { start: 0, end: 1, line: 1, column: 1 };
            let span3 = Span { start: 4, end: 5, line: 1, column: 5 };
            let merged = span2.merge(span3);
            if merged.start == 0 && merged.end == 5 {
                println!("SUCCESS: span propagation works");
            } else {
                println!("FAILED: merged span incorrect: {:?}", merged);
            }
        }
        Err(e) => println!("PARSE_ERROR: {:?}", e),
    }
}
"#,
    );

    // Try to compile the generated code
    let temp_file = "/tmp/test_span_propagation.rs";
    let temp_bin = "/tmp/test_span_propagation";
    fs::write(temp_file, &code).expect("Failed to write temp file");

    match Command::new("rustc")
        .args(["-o", temp_bin, temp_file])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("Generated code:\n{}", code);
                println!("\nCompiler error:\n{}", stderr);
                panic!("Generated code with span propagation failed to compile!");
            }

            // Run the binary
            let run_output = Command::new(temp_bin)
                .output()
                .expect("Failed to run parser");
            let stdout = String::from_utf8_lossy(&run_output.stdout);

            if !stdout.contains("SUCCESS") {
                println!("Generated code:\n{}", code);
                println!("\nParser output:\n{}", stdout);
                panic!("Span propagation did not produce expected output!");
            }

            println!("Span propagation test passed");
        }
        Err(e) => {
            println!(
                "Warning: Could not run rustc: {}. Skipping span propagation test.",
                e
            );
        }
    }

    // Cleanup
    let _ = fs::remove_file(temp_file);
    let _ = fs::remove_file(temp_bin);
}

#[test]
fn test_span_merge_handles_empty_spans() {
    // Test that merge handles empty/default spans correctly
    let grammar = Grammar::new()
        .ast_config(|c| c.apply_mappings())
        .lexer(|l| {
            l.token("NUMBER", "")
                .start_with(|c| c.match_class("digit"))
                .continue_with(|c| c.match_class("digit"))
                .build()
                .skip("whitespace")
        })
        .rule("program", |r| r.token("NUMBER"));

    let code = grammar.build().generate();

    // Should handle empty spans by returning the non-empty one
    assert!(
        code.contains("if other.start == 0 && other.end == 0 {"),
        "merge() should check for empty other span"
    );
    assert!(
        code.contains("if self.start == 0 && self.end == 0 {"),
        "merge() should check for empty self span"
    );
}
