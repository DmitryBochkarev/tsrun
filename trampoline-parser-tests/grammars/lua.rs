//! Parser for Lua expressions and statements.
//!
//! This grammar stress-tests the trampoline-parser with:
//! - Right-associative operators (.. and ^)
//! - Keyword operators (and, or, not)
//! - Postfix chains (calls, indexing, member access)
//! - Tables with mixed field types
//! - Control flow structures

use trampoline_parser::{Assoc, CombinatorExt, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // Main entry: a chunk is a sequence of statements
        .rule("chunk", |r| {
            r.sequence((
                r.parse("ws"),
                r.zero_or_more(r.sequence((
                    r.parse("statement"),
                    r.parse("ws"),
                ))),
            ))
            // Structure: List([ws_result, List([List([Stmt, ws]), List([Stmt, ws]), ...])])
            // items[0] = ws result (None due to skip)
            // items[1] = zero_or_more result: List of iterations
            // Each iteration is List([Stmt, ws])
            .ast("|r, _| { if let ParseResult::List(items) = r { if let Some(ParseResult::List(iterations)) = items.into_iter().nth(1) { let stmts: Vec<Stmt> = iterations.into_iter().filter_map(|iter| { if let ParseResult::List(parts) = iter { parts.into_iter().find_map(|p| if let ParseResult::Stmt(s) = p { Some(s) } else { None }) } else { None } }).collect(); Ok(ParseResult::Stmts(stmts)) } else { Ok(ParseResult::Stmts(vec![])) } } else { Ok(ParseResult::Stmts(vec![])) } }")
        })

        // Statements
        .rule("statement", |r| {
            r.choice((
                r.parse("local_decl"),
                r.parse("if_statement"),
                r.parse("while_statement"),
                r.parse("for_statement"),
                r.parse("repeat_statement"),
                r.parse("function_decl"),
                r.parse("return_statement"),
                r.parse("assignment_or_call"),
            ))
        })

        // Local declaration: local name [= expr]
        .rule("local_decl", |r| {
            r.sequence((
                r.parse("kw_local"),
                r.parse("ws1"),
                r.parse("name_list"),
                r.parse("ws"),
                r.optional(r.sequence((
                    r.lit("="),
                    r.parse("ws"),
                    r.parse("expr_list"),
                ))),
            ))
            .ast("|r, _| { if let ParseResult::List(parts) = r { let names = if let ParseResult::Names(n) = &parts[2] { n.clone() } else { vec![] }; let exprs = parts.get(4).and_then(|p| if let ParseResult::List(inner) = p { inner.get(2).and_then(|e| if let ParseResult::Exprs(es) = e { Some(es.clone()) } else { None }) } else { None }).unwrap_or_default(); Ok(ParseResult::Stmt(Stmt::Local(names, exprs))) } else { Ok(ParseResult::None) } }")
        })

        // If statement: if expr then block {elseif expr then block} [else block] end
        .rule("if_statement", |r| {
            r.sequence((
                r.parse("kw_if"),
                r.parse("ws1"),
                r.parse("expr"),
                r.parse("ws"),  // ws not ws1: trailing ws already consumed by expr
                r.parse("kw_then"),
                r.parse("ws"),
                r.parse("block"),
                r.parse("ws"),
                r.zero_or_more(r.sequence((
                    r.parse("kw_elseif"),
                    r.parse("ws1"),
                    r.parse("expr"),
                    r.parse("ws"),  // ws not ws1: trailing ws already consumed by expr
                    r.parse("kw_then"),
                    r.parse("ws"),
                    r.parse("block"),
                    r.parse("ws"),
                ))),
                r.optional(r.sequence((
                    r.parse("kw_else"),
                    r.parse("ws"),
                    r.parse("block"),
                    r.parse("ws"),
                ))),
                r.parse("kw_end"),
            ))
            .ast("|r, _| Ok(ParseResult::Stmt(Stmt::If))")
        })

        // While statement: while expr do block end
        .rule("while_statement", |r| {
            r.sequence((
                r.parse("kw_while"),
                r.parse("ws1"),
                r.parse("expr"),
                r.parse("ws"),  // ws not ws1: trailing ws already consumed by expr
                r.parse("kw_do"),
                r.parse("ws"),
                r.parse("block"),
                r.parse("ws"),
                r.parse("kw_end"),
            ))
            .ast("|r, _| Ok(ParseResult::Stmt(Stmt::While))")
        })

        // For statement: for name = expr, expr [, expr] do block end
        .rule("for_statement", |r| {
            r.sequence(vec![
                r.parse("kw_for"),
                r.parse("ws1"),
                r.parse("identifier"),
                r.parse("ws"),
                r.lit("="),
                r.parse("ws"),
                r.parse("expr"),
                r.parse("ws"),
                r.lit(","),
                r.parse("ws"),
                r.parse("expr"),
                r.optional(r.sequence((
                    r.parse("ws"),
                    r.lit(","),
                    r.parse("ws"),
                    r.parse("expr"),
                ))),
                r.parse("ws"),  // ws not ws1: trailing ws already consumed by expr
                r.parse("kw_do"),
                r.parse("ws"),
                r.parse("block"),
                r.parse("ws"),
                r.parse("kw_end"),
            ])
            .ast("|r, _| Ok(ParseResult::Stmt(Stmt::For))")
        })

        // Repeat statement: repeat block until expr
        .rule("repeat_statement", |r| {
            r.sequence((
                r.parse("kw_repeat"),
                r.parse("ws"),
                r.parse("block"),
                r.parse("ws"),
                r.parse("kw_until"),
                r.parse("ws1"),
                r.parse("expr"),
            ))
            .ast("|r, _| Ok(ParseResult::Stmt(Stmt::Repeat))")
        })

        // Function declaration: function name ( [params] ) block end
        .rule("function_decl", |r| {
            r.sequence(vec![
                r.parse("kw_function"),
                r.parse("ws1"),
                r.parse("func_name"),
                r.parse("ws"),
                r.lit("("),
                r.parse("ws"),
                r.optional(r.parse("param_list")),
                r.parse("ws"),
                r.lit(")"),
                r.parse("ws"),
                r.parse("block"),
                r.parse("ws"),
                r.parse("kw_end"),
            ])
            .ast("|r, _| Ok(ParseResult::Stmt(Stmt::Function))")
        })

        // Return statement: return [expr_list]
        .rule("return_statement", |r| {
            r.sequence((
                r.parse("kw_return"),
                r.optional(r.sequence((
                    r.parse("ws1"),
                    r.parse("expr_list"),
                ))),
            ))
            .ast("|r, _| Ok(ParseResult::Stmt(Stmt::Return))")
        })

        // Assignment or function call (disambiguated by context)
        .rule("assignment_or_call", |r| {
            r.sequence((
                r.parse("prefix_expr"),
                r.optional(r.sequence((
                    r.parse("ws"),
                    r.lit("="),
                    r.parse("ws"),
                    r.parse("expr_list"),
                ))),
            ))
            .ast("|r, _| Ok(ParseResult::Stmt(Stmt::AssignOrCall))")
        })

        // Block: sequence of statements (used inside control structures)
        .rule("block", |r| {
            r.zero_or_more(r.sequence((
                r.parse("statement"),
                r.parse("ws"),
            )))
            .ast("|_, _| Ok(ParseResult::None)")
        })

        // Expression using Pratt parsing - infix/postfix only
        // Prefix operators handled in grammar rules for proper ws handling
        // Infix operators use patterns with leading ws rule to handle "a.x * b" patterns
        .rule("expr", |r| {
            r.pratt(r.parse("unary"), |ops| {
                ops
                    // Postfix operators (highest binding)
                    .postfix_call("(", ")", ",", 18, "|callee, args, _| Ok(call_expr(callee, args))")
                    .postfix_index("[", "]", 18, "|obj, idx, _| Ok(make_index(obj, idx))")
                    // Use pattern to prevent matching ".." as member access
                    .postfix_member_pattern(r.sequence((r.lit("."), r.not_followed_by(r.char('.')))), 18, "|obj, prop, _| Ok(member_expr(obj, prop))")

                    // Power (right-associative, very high precedence)
                    .infix(r.sequence((r.parse("ws"), r.lit("^"))), 12, Assoc::Right, "|l, r, _| Ok(binary_expr(l, r, BinOp::Pow))")

                    // Multiplicative
                    .infix(r.sequence((r.parse("ws"), r.lit("*"))), 10, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Mul))")
                    .infix(r.sequence((r.parse("ws"), r.lit("//"))), 10, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::FloorDiv))")
                    .infix(r.sequence((r.parse("ws"), r.lit("/"))), 10, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Div))")
                    .infix(r.sequence((r.parse("ws"), r.lit("%"))), 10, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Mod))")

                    // Additive
                    .infix(r.sequence((r.parse("ws"), r.lit("+"))), 9, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Add))")
                    // Use pattern to ensure "-" isn't followed by "-" (which would be a comment)
                    .infix(r.sequence((r.parse("ws"), r.lit("-"), r.not_followed_by(r.lit("-")))), 9, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Sub))")

                    // String concatenation (right-associative)
                    .infix(r.sequence((r.parse("ws"), r.lit(".."))), 8, Assoc::Right, "|l, r, _| Ok(binary_expr(l, r, BinOp::Concat))")

                    // Comparison
                    .infix(r.sequence((r.parse("ws"), r.lit("=="))), 4, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Eq))")
                    .infix(r.sequence((r.parse("ws"), r.lit("~="))), 4, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::NotEq))")
                    .infix(r.sequence((r.parse("ws"), r.lit("<="))), 4, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Le))")
                    .infix(r.sequence((r.parse("ws"), r.lit(">="))), 4, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Ge))")
                    .infix(r.sequence((r.parse("ws"), r.lit("<"))), 4, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Lt))")
                    .infix(r.sequence((r.parse("ws"), r.lit(">"))), 4, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Gt))")

                    // Logical (keyword operators - include ws in pattern)
                    .infix(r.sequence((r.parse("ws"), r.lit("and"), r.not_followed_by(r.ident_cont()))), 3, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::And))")
                    .infix(r.sequence((r.parse("ws"), r.lit("or"), r.not_followed_by(r.ident_cont()))), 2, Assoc::Left, "|l, r, _| Ok(binary_expr(l, r, BinOp::Or))")
            })
        })

        // Unary expressions - prefix operators handled here (not in Pratt)
        // This allows ws to be consumed before checking for prefix ops
        .rule("unary", |r| {
            r.sequence((r.parse("ws"), r.parse("unary_inner"), r.parse("ws")))
                .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.remove(1)) } else { Ok(r) } }")
        })

        .rule("unary_inner", |r| {
            r.choice((
                r.parse("prefix_not"),
                r.parse("prefix_neg"),
                r.parse("prefix_len"),
                r.parse("primary"),
            ))
        })

        .rule("prefix_not", |r| {
            r.sequence((
                r.lit("not"),
                r.not_followed_by(r.ident_cont()),
                r.parse("unary"),  // recursive - handles ws and more prefixes
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let e = items.into_iter().last().unwrap_or(ParseResult::None); Ok(unary_expr(e, UnOp::Not)) } else { Ok(r) } }")
        })

        .rule("prefix_neg", |r| {
            r.sequence((
                r.lit("-"),
                r.not_followed_by(r.lit("-")),  // not a comment
                r.parse("unary"),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let e = items.into_iter().last().unwrap_or(ParseResult::None); Ok(unary_expr(e, UnOp::Neg)) } else { Ok(r) } }")
        })

        .rule("prefix_len", |r| {
            r.sequence((
                r.lit("#"),
                r.parse("unary"),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let e = items.into_iter().last().unwrap_or(ParseResult::None); Ok(unary_expr(e, UnOp::Len)) } else { Ok(r) } }")
        })

        // Primary expression
        .rule("primary", |r| {
            r.choice((
                r.parse("nil"),
                r.parse("true"),
                r.parse("false"),
                r.parse("number"),
                r.parse("string"),
                r.parse("table"),
                r.parse("function_expr"),
                r.parse("paren_expr"),
                r.parse("identifier"),
            ))
        })

        // Prefix expression (for assignment LHS)
        .rule("prefix_expr", |r| {
            r.pratt(r.parse("prefix_primary"), |ops| {
                ops
                    .postfix_call("(", ")", ",", 18, "|callee, args, _| Ok(call_expr(callee, args))")
                    .postfix_index("[", "]", 18, "|obj, idx, _| Ok(make_index(obj, idx))")
                    // Use pattern to prevent matching ".." as member access
                    .postfix_member_pattern(r.sequence((r.lit("."), r.not_followed_by(r.char('.')))), 18, "|obj, prop, _| Ok(member_expr(obj, prop))")
            })
        })

        .rule("prefix_primary", |r| {
            r.choice((
                r.parse("paren_expr"),
                r.parse("identifier"),
            ))
        })

        // Parenthesized expression
        .rule("paren_expr", |r| {
            r.sequence((
                r.lit("("),
                r.parse("ws"),
                r.parse("expr"),
                r.parse("ws"),
                r.lit(")"),
            ))
            .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.remove(2)) } else { Ok(r) } }")
        })

        // Function expression: function ( [params] ) block end
        .rule("function_expr", |r| {
            r.sequence((
                r.parse("kw_function"),
                r.parse("ws"),
                r.lit("("),
                r.parse("ws"),
                r.optional(r.parse("param_list")),
                r.parse("ws"),
                r.lit(")"),
                r.parse("ws"),
                r.parse("block"),
                r.parse("ws"),
                r.parse("kw_end"),
            ))
            .ast("|_, _| Ok(ParseResult::Expr(Expr::Function))")
        })

        // Table constructor
        .rule("table", |r| {
            r.sequence((
                r.lit("{"),
                r.parse("ws"),
                r.optional(r.parse("field_list")),
                r.parse("ws"),
                r.lit("}"),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let fields = items.get(2).and_then(|f| if let ParseResult::Fields(fs) = f { Some(fs.clone()) } else { None }).unwrap_or_default(); Ok(ParseResult::Expr(Expr::Table(fields))) } else { Ok(ParseResult::Expr(Expr::Table(vec![]))) } }")
        })

        // Field list: field { fieldsep field } [fieldsep]
        .rule("field_list", |r| {
            r.sequence((
                r.parse("field"),
                r.zero_or_more(r.sequence((
                    r.parse("field_sep"),
                    r.parse("field"),
                ))),
                r.optional(r.parse("field_sep")),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let mut fields = vec![]; if let ParseResult::Field(f) = &items[0] { fields.push(f.clone()); } if let ParseResult::List(rest) = &items[1] { for item in rest.iter() { if let ParseResult::List(pair) = item { if let Some(ParseResult::Field(f)) = pair.get(1) { fields.push(f.clone()); } } } } Ok(ParseResult::Fields(fields)) } else { Ok(ParseResult::Fields(vec![])) } }")
        })

        // Field: [expr] = expr | name = expr | expr
        .rule("field", |r| {
            r.choice((
                r.parse("computed_field"),
                r.parse("named_field"),
                r.parse("array_field"),
            ))
        })

        .rule("computed_field", |r| {
            r.sequence((
                r.lit("["),
                r.parse("ws"),
                r.parse("expr"),
                r.parse("ws"),
                r.lit("]"),
                r.parse("ws"),
                r.lit("="),
                r.parse("ws"),
                r.parse("expr"),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let key = to_expr(items.get(2).cloned().unwrap_or(ParseResult::None)); let val = to_expr(items.get(8).cloned().unwrap_or(ParseResult::None)); Ok(ParseResult::Field(Field::Computed(key, val))) } else { Ok(ParseResult::None) } }")
        })

        .rule("named_field", |r| {
            r.sequence((
                r.parse("identifier"),
                r.parse("ws"),
                r.lit("="),
                r.parse("ws"),
                r.parse("expr"),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let name = if let ParseResult::Expr(Expr::Ident(n)) = &items[0] { n.clone() } else { String::new() }; let val = to_expr(items.get(4).cloned().unwrap_or(ParseResult::None)); Ok(ParseResult::Field(Field::Named(name, val))) } else { Ok(ParseResult::None) } }")
        })

        .rule("array_field", |r| {
            r.parse("expr")
            .ast("|r, _| { let e = to_expr(r); Ok(ParseResult::Field(Field::Array(e))) }")
        })

        .rule("field_sep", |r| {
            r.sequence((
                r.parse("ws"),
                r.choice((r.lit(","), r.lit(";"))),
                r.parse("ws"),
            ))
        })

        // Literals
        .rule("nil", |r| {
            r.sequence((
                r.lit("nil"),
                r.not_followed_by(r.ident_cont()),
            ))
            .ast("|_, _| Ok(ParseResult::Expr(Expr::Nil))")
        })

        .rule("true", |r| {
            r.sequence((
                r.lit("true"),
                r.not_followed_by(r.ident_cont()),
            ))
            .ast("|_, _| Ok(ParseResult::Expr(Expr::Bool(true)))")
        })

        .rule("false", |r| {
            r.sequence((
                r.lit("false"),
                r.not_followed_by(r.ident_cont()),
            ))
            .ast("|_, _| Ok(ParseResult::Expr(Expr::Bool(false)))")
        })

        // Number: integer, float, hex, scientific
        .rule("number", |r| {
            r.capture(r.choice((
                r.parse("hex_number"),
                r.parse("float_number"),
                r.parse("int_number"),
            )))
            .ast("|r, _| { if let ParseResult::Text(s, _) = r { Ok(ParseResult::Expr(Expr::Number(s))) } else { Ok(ParseResult::None) } }")
        })

        .rule("hex_number", |r| {
            r.sequence((
                r.lit("0"),
                r.choice((r.char('x'), r.char('X'))),
                r.one_or_more(r.hex_digit()),
            ))
        })

        .rule("float_number", |r| {
            r.sequence((
                r.one_or_more(r.digit()),
                r.choice((
                    r.sequence((
                        r.lit("."),
                        r.zero_or_more(r.digit()),
                        r.optional(r.parse("exponent")),
                    )),
                    r.parse("exponent"),
                )),
            ))
        })

        .rule("int_number", |r| {
            r.one_or_more(r.digit())
        })

        .rule("exponent", |r| {
            r.sequence((
                r.choice((r.char('e'), r.char('E'))),
                r.optional(r.choice((r.char('+'), r.char('-')))),
                r.one_or_more(r.digit()),
            ))
        })

        // String: double-quoted, single-quoted, or raw [[...]]
        .rule("string", |r| {
            r.choice((
                r.parse("double_string"),
                r.parse("single_string"),
                r.parse("raw_string"),
            ))
        })

        .rule("double_string", |r| {
            r.sequence((
                r.char('"'),
                r.capture(r.zero_or_more(r.choice((
                    r.sequence((r.char('\\'), r.any_char())),
                    r.sequence((
                        r.not_followed_by(r.choice((r.char('"'), r.char('\\')))),
                        r.any_char(),
                    )),
                )))),
                r.char('"'),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { if let ParseResult::Text(s, _) = &items[1] { Ok(ParseResult::Expr(Expr::String(s.clone()))) } else { Ok(ParseResult::Expr(Expr::String(String::new()))) } } else { Ok(ParseResult::None) } }")
        })

        .rule("single_string", |r| {
            r.sequence((
                r.char('\''),
                r.capture(r.zero_or_more(r.choice((
                    r.sequence((r.char('\\'), r.any_char())),
                    r.sequence((
                        r.not_followed_by(r.choice((r.char('\''), r.char('\\')))),
                        r.any_char(),
                    )),
                )))),
                r.char('\''),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { if let ParseResult::Text(s, _) = &items[1] { Ok(ParseResult::Expr(Expr::String(s.clone()))) } else { Ok(ParseResult::Expr(Expr::String(String::new()))) } } else { Ok(ParseResult::None) } }")
        })

        .rule("raw_string", |r| {
            r.sequence((
                r.lit("[["),
                r.capture(r.zero_or_more(r.sequence((
                    r.not_followed_by(r.lit("]]")),
                    r.any_char(),
                )))),
                r.lit("]]"),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { if let ParseResult::Text(s, _) = &items[1] { Ok(ParseResult::Expr(Expr::String(s.clone()))) } else { Ok(ParseResult::Expr(Expr::String(String::new()))) } } else { Ok(ParseResult::None) } }")
        })

        // Identifier (not a keyword)
        .rule("identifier", |r| {
            r.sequence((
                r.not_followed_by(r.parse("keyword")),
                r.capture(r.sequence((
                    r.choice((r.alpha(), r.char('_'))),
                    r.zero_or_more(r.choice((r.alpha_num(), r.char('_')))),
                ))),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { if let ParseResult::Text(s, _) = &items[1] { Ok(ParseResult::Expr(Expr::Ident(s.clone()))) } else { Ok(ParseResult::None) } } else { Ok(ParseResult::None) } }")
        })

        // Keywords (for negative lookahead)
        .rule("keyword", |r| {
            r.sequence((
                r.choice(vec![
                    r.lit("and"), r.lit("break"), r.lit("do"), r.lit("else"),
                    r.lit("elseif"), r.lit("end"), r.lit("false"), r.lit("for"),
                    r.lit("function"), r.lit("if"), r.lit("in"), r.lit("local"),
                    r.lit("nil"), r.lit("not"), r.lit("or"), r.lit("repeat"),
                    r.lit("return"), r.lit("then"), r.lit("true"), r.lit("until"),
                    r.lit("while"),
                ]),
                r.not_followed_by(r.choice((r.alpha_num(), r.char('_')))),
            ))
        })

        // Individual keywords with boundary checking
        .rule("kw_and", |r| r.sequence((r.lit("and"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_do", |r| r.sequence((r.lit("do"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_else", |r| r.sequence((r.lit("else"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_elseif", |r| r.sequence((r.lit("elseif"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_end", |r| r.sequence((r.lit("end"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_false", |r| r.sequence((r.lit("false"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_for", |r| r.sequence((r.lit("for"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_function", |r| r.sequence((r.lit("function"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_if", |r| r.sequence((r.lit("if"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_in", |r| r.sequence((r.lit("in"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_local", |r| r.sequence((r.lit("local"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_nil", |r| r.sequence((r.lit("nil"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_not", |r| r.sequence((r.lit("not"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_or", |r| r.sequence((r.lit("or"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_repeat", |r| r.sequence((r.lit("repeat"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_return", |r| r.sequence((r.lit("return"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_then", |r| r.sequence((r.lit("then"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_true", |r| r.sequence((r.lit("true"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_until", |r| r.sequence((r.lit("until"), r.not_followed_by(r.ident_cont()))))
        .rule("kw_while", |r| r.sequence((r.lit("while"), r.not_followed_by(r.ident_cont()))))

        // Helper rules
        .rule("name_list", |r| {
            r.sequence((
                r.parse("identifier"),
                r.zero_or_more(r.sequence((
                    r.parse("ws"),
                    r.lit(","),
                    r.parse("ws"),
                    r.parse("identifier"),
                ))),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let mut names = vec![]; if let ParseResult::Expr(Expr::Ident(n)) = &items[0] { names.push(n.clone()); } if let ParseResult::List(rest) = &items[1] { for item in rest.iter() { if let ParseResult::List(parts) = item { if let Some(ParseResult::Expr(Expr::Ident(n))) = parts.get(3) { names.push(n.clone()); } } } } Ok(ParseResult::Names(names)) } else { Ok(ParseResult::Names(vec![])) } }")
        })

        .rule("expr_list", |r| {
            r.sequence((
                r.parse("expr"),
                r.zero_or_more(r.sequence((
                    r.parse("ws"),
                    r.lit(","),
                    r.parse("ws"),
                    r.parse("expr"),
                ))),
            ))
            .ast("|r, _| { if let ParseResult::List(items) = r { let mut exprs = vec![to_expr(items[0].clone())]; if let ParseResult::List(rest) = &items[1] { for item in rest.iter() { if let ParseResult::List(parts) = item { if let Some(e) = parts.get(3) { exprs.push(to_expr(e.clone())); } } } } Ok(ParseResult::Exprs(exprs)) } else { Ok(ParseResult::Exprs(vec![])) } }")
        })

        .rule("param_list", |r| {
            r.choice((
                r.sequence((
                    r.parse("name_list"),
                    r.optional(r.sequence((
                        r.parse("ws"),
                        r.lit(","),
                        r.parse("ws"),
                        r.lit("..."),
                    ))),
                )),
                r.lit("..."),
            ))
        })

        .rule("func_name", |r| {
            r.sequence((
                r.parse("identifier"),
                r.zero_or_more(r.sequence((
                    r.lit("."),
                    r.parse("identifier"),
                ))),
                r.optional(r.sequence((
                    r.lit(":"),
                    r.parse("identifier"),
                ))),
            ))
        })

        // Whitespace
        .rule("ws", |r| r.skip(r.zero_or_more(r.choice((
            r.ws(),
            r.parse("line_comment"),
        )))))

        .rule("ws1", |r| r.skip(r.one_or_more(r.choice((
            r.ws(),
            r.parse("line_comment"),
        )))))

        .rule("line_comment", |r| {
            r.sequence((
                r.lit("--"),
                r.not_followed_by(r.lit("[")),
                r.zero_or_more(r.sequence((
                    r.not_followed_by(r.char('\n')),
                    r.any_char(),
                ))),
            ))
        })

        .ast_config(|c| {
            c.helper(HELPER_CODE)
                .result_variant("Expr", "Expr")
                .result_variant("Exprs", "Vec<Expr>")
                .result_variant("Stmt", "Stmt")
                .result_variant("Stmts", "Vec<Stmt>")
                .result_variant("Field", "Field")
                .result_variant("Fields", "Vec<Field>")
                .result_variant("Names", "Vec<String>")
                .apply_mappings()
        })
        .build()
}

const HELPER_CODE: &str = r#"
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, FloorDiv, Mod, Pow, Concat,
    Eq, NotEq, Lt, Le, Gt, Ge, And, Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp { Neg, Not, Len }

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Nil,
    Bool(bool),
    Number(String),
    String(String),
    Ident(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    Table(Vec<Field>),
    Function,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Array(Expr),
    Named(String, Expr),
    Computed(Expr, Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Local(Vec<String>, Vec<Expr>),
    If,
    While,
    For,
    Repeat,
    Function,
    Return,
    AssignOrCall,
}

fn call_expr(callee: ParseResult, args: Vec<ParseResult>) -> ParseResult {
    let callee = to_expr(callee);
    let args = args.into_iter().map(to_expr).collect();
    ParseResult::Expr(Expr::Call(Box::new(callee), args))
}

fn make_index(obj: ParseResult, idx: ParseResult) -> ParseResult {
    let obj = to_expr(obj);
    let idx = to_expr(idx);
    ParseResult::Expr(Expr::Index(Box::new(obj), Box::new(idx)))
}

fn member_expr(obj: ParseResult, prop: String) -> ParseResult {
    let obj = to_expr(obj);
    ParseResult::Expr(Expr::Member(Box::new(obj), prop))
}

fn binary_expr(l: ParseResult, r: ParseResult, op: BinOp) -> ParseResult {
    let l = to_expr(l);
    let r = to_expr(r);
    ParseResult::Expr(Expr::Binary(Box::new(l), op, Box::new(r)))
}

fn unary_expr(e: ParseResult, op: UnOp) -> ParseResult {
    let e = to_expr(e);
    ParseResult::Expr(Expr::Unary(op, Box::new(e)))
}

fn to_expr(r: ParseResult) -> Expr {
    match r {
        ParseResult::Expr(e) => e,
        ParseResult::Text(s, _) => {
            if let Ok(_) = s.parse::<f64>() {
                Expr::Number(s)
            } else {
                Expr::Ident(s)
            }
        }
        ParseResult::None => Expr::Nil,
        ParseResult::List(items) => {
            if let Some(first) = items.into_iter().next() {
                to_expr(first)
            } else {
                Expr::Nil
            }
        }
        ParseResult::Exprs(es) => es.into_iter().next().unwrap_or(Expr::Nil),
        ParseResult::Stmt(_) => Expr::Nil,
        ParseResult::Stmts(_) => Expr::Nil,
        ParseResult::Field(_) => Expr::Nil,
        ParseResult::Fields(_) => Expr::Nil,
        ParseResult::Names(_) => Expr::Nil,
    }
}
"#;
