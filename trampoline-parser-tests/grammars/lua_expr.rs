//! Lua expression parser (for testing expressions independently).
//!
//! Whitespace is handled entirely in the grammar - no automatic ws skipping.
//! - Infix operators: use patterns with leading ws rule to handle "a.x * b" patterns
//!   (after postfix member access, position is right after identifier with leading space)
//! - Postfix operators: match directly since operand consumes trailing ws
//! - Prefix operators: handled as grammar rules (not Pratt) so ws is consumed before them

use trampoline_parser::{Assoc, CombinatorExt, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // Main entry: expression with Pratt for infix/postfix only
        // Infix operators use patterns with leading ws rule to handle "a.x * b" patterns:
        // After postfix member access, position is right after identifier (e.g., at " * b"),
        // so infix patterns need to consume leading whitespace before matching the operator.
        .rule("expr", |r| {
            r.pratt(r.parse("unary"), |ops| {
                ops
                    // Postfix operators (highest binding)
                    .postfix_call("(", ")", ",", 18, "|callee, args, _| Ok(call_expr(callee, args))")
                    .postfix_index("[", "]", 18, "|obj, idx, _| Ok(make_index(obj, idx))")
                    // Use pattern to prevent matching ".." as member access
                    .postfix_member_pattern(r.sequence((r.lit("."), r.not_followed_by(r.char('.')))), 18, "|obj, prop, _| Ok(member_expr(obj, prop))")

                    // Power (right-associative)
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

        .rule("primary", |r| {
            r.choice((
                r.parse("nil"),
                r.parse("true"),
                r.parse("false"),
                r.parse("number"),
                r.parse("string"),
                r.parse("table"),
                r.parse("paren_expr"),
                r.parse("identifier"),
            ))
        })

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

        .rule("int_number", |r| r.one_or_more(r.digit()))

        .rule("exponent", |r| {
            r.sequence((
                r.choice((r.char('e'), r.char('E'))),
                r.optional(r.choice((r.char('+'), r.char('-')))),
                r.one_or_more(r.digit()),
            ))
        })

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

        // Table
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

        .rule("field_list", |r| {
            r.separated_by_trailing(r.parse("field"), r.parse("field_sep"))
                .ast("|r, _| { if let ParseResult::List(items) = r { let fields: Vec<Field> = items.into_iter().filter_map(|item| if let ParseResult::Field(f) = item { Some(f) } else { None }).collect(); Ok(ParseResult::Fields(fields)) } else { Ok(ParseResult::Fields(vec![])) } }")
        })

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

        .rule("comment", |r| {
            r.sequence((
                r.lit("--"),
                r.zero_or_more(r.sequence((
                    r.not_followed_by(r.char('\n')),
                    r.any_char(),
                ))),
            ))
        })

        .rule("ws", |r| r.skip(r.zero_or_more(r.choice((r.ws(), r.parse("comment"))))))

        .ast_config(|c| {
            c.helper(HELPER_CODE)
                .result_variant("Expr", "Expr")
                .result_variant("Field", "Field")
                .result_variant("Fields", "Vec<Field>")
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Array(Expr),
    Named(String, Expr),
    Computed(Expr, Expr),
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
        ParseResult::Field(_) => Expr::Nil,
        ParseResult::Fields(_) => Expr::Nil,
    }
}
"#;
