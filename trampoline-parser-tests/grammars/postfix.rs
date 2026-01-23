//! Parser for expressions with postfix operators (call, index, member, simple).

use trampoline_parser::{Assoc, CombinatorExt, CompiledGrammar, Grammar};

pub fn grammar() -> CompiledGrammar {
    Grammar::new()
        // This grammar demonstrates grammar-controlled whitespace handling.
        // The operand rule wraps primary with whitespace to handle "a.x * b" patterns:
        // - Leading ws handles whitespace before operand
        // - Trailing ws handles whitespace after operand (before postfix or infix ops)
        // Infix operators also consume leading ws for whitespace between postfix and infix.
        .rule("expr", |r| {
            r.pratt(r.parse("operand"), |ops| {
                ops
                    // Postfix operators (highest precedence for binding)
                    .postfix_call("(", ")", ",", 18, "|callee, args, _| Ok(call(callee, args))")
                    .postfix_index("[", "]", 18, "|obj, idx, _| Ok(index(obj, idx))")
                    .postfix_member(".", 18, "|obj, prop, _| Ok(member(obj, prop))")
                    .postfix("++", 17, "|e, _| Ok(postinc(e))")
                    .postfix("--", 17, "|e, _| Ok(postdec(e))")
                    // Binary operators - use patterns with leading ws rule
                    // This enables "a.x * b" to work: after ".x" postfix, ws consumes " " before "*"
                    .infix(r.sequence((r.parse("ws"), r.lit("+"))), 1, Assoc::Left, "|l, r, _| Ok(binary(l, r, BinOp::Add))")
                    .infix(r.sequence((r.parse("ws"), r.lit("*"))), 2, Assoc::Left, "|l, r, _| Ok(binary(l, r, BinOp::Mul))")
            })
        })
        // Operand wraps primary with whitespace
        .rule("operand", |r| {
            r.sequence((r.parse("ws"), r.parse("primary"), r.parse("ws")))
                .ast("|r, _| { if let ParseResult::List(mut items) = r { Ok(items.remove(1)) } else { Ok(r) } }")
        })
        .rule("primary", |r| {
            r.choice((
                r.parse("paren_expr"),
                r.parse("identifier"),
                r.parse("number"),
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
        .rule("identifier", |r| {
            r.capture(r.sequence((
                r.ident_start(),
                r.zero_or_more(r.ident_cont()),
            )))
        })
        .rule("number", |r| r.capture(r.one_or_more(r.digit())))
        .rule("ws", |r| r.skip(r.zero_or_more(r.ws())))
        .ast_config(|c| {
            c.helper(HELPER_CODE)
                .result_variant("Expr", "Expr")
                .apply_mappings()
        })
        .build()
}

const HELPER_CODE: &str = r#"
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp { Add, Mul }

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(String),
    Num(i64),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    PostInc(Box<Expr>),
    PostDec(Box<Expr>),
}

fn call(callee: ParseResult, args: Vec<ParseResult>) -> ParseResult {
    let callee = to_expr(callee);
    let args = args.into_iter().map(to_expr).collect();
    ParseResult::Expr(Expr::Call(Box::new(callee), args))
}

fn index(obj: ParseResult, idx: ParseResult) -> ParseResult {
    let obj = to_expr(obj);
    let idx = to_expr(idx);
    ParseResult::Expr(Expr::Index(Box::new(obj), Box::new(idx)))
}

fn member(obj: ParseResult, prop: String) -> ParseResult {
    let obj = to_expr(obj);
    ParseResult::Expr(Expr::Member(Box::new(obj), prop))
}

fn postinc(e: ParseResult) -> ParseResult {
    let e = to_expr(e);
    ParseResult::Expr(Expr::PostInc(Box::new(e)))
}

fn postdec(e: ParseResult) -> ParseResult {
    let e = to_expr(e);
    ParseResult::Expr(Expr::PostDec(Box::new(e)))
}

fn binary(l: ParseResult, r: ParseResult, op: BinOp) -> ParseResult {
    let l = to_expr(l);
    let r = to_expr(r);
    ParseResult::Expr(Expr::Binary(Box::new(l), op, Box::new(r)))
}

fn to_expr(r: ParseResult) -> Expr {
    match r {
        ParseResult::Text(s, _) => {
            if let Ok(n) = s.parse::<i64>() {
                Expr::Num(n)
            } else {
                Expr::Ident(s)
            }
        }
        ParseResult::Expr(e) => e,
        ParseResult::None => Expr::Num(0),
        ParseResult::List(items) => {
            if let Some(first) = items.into_iter().next() {
                to_expr(first)
            } else {
                Expr::Num(0)
            }
        }
    }
}
"#;
