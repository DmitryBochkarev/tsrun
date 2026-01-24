//! Tests for the Scheme parser.
//!
//! Exercises underused trampoline-parser features:
//! - Multiple prefix operators (quote family)
//! - Extended symbol character classes
//! - Character literals
//! - Dotted pairs
//! - Hash-prefixed disambiguation

use trampoline_parser_tests::scheme_parser::{ParseResult, Parser, SchemeValue};

fn parse(input: &str) -> Result<SchemeValue, String> {
    let mut parser = Parser::new(input);
    let result = parser.parse().map_err(|e| e.message)?;
    match result {
        ParseResult::Scheme(v) => Ok(v),
        other => Err(format!("Expected Scheme value, got {:?}", other)),
    }
}

fn sym(s: &str) -> SchemeValue {
    SchemeValue::Symbol(s.to_string())
}

fn num(n: f64) -> SchemeValue {
    SchemeValue::Number(n)
}

fn string(s: &str) -> SchemeValue {
    SchemeValue::String(s.to_string())
}

fn list(items: Vec<SchemeValue>) -> SchemeValue {
    let mut result = SchemeValue::Nil;
    for item in items.into_iter().rev() {
        result = SchemeValue::Pair(Box::new(item), Box::new(result));
    }
    result
}

fn pair(a: SchemeValue, b: SchemeValue) -> SchemeValue {
    SchemeValue::Pair(Box::new(a), Box::new(b))
}

fn quote(v: SchemeValue) -> SchemeValue {
    SchemeValue::Quote(Box::new(v))
}

fn quasiquote(v: SchemeValue) -> SchemeValue {
    SchemeValue::Quasiquote(Box::new(v))
}

fn unquote(v: SchemeValue) -> SchemeValue {
    SchemeValue::Unquote(Box::new(v))
}

fn unquote_splicing(v: SchemeValue) -> SchemeValue {
    SchemeValue::UnquoteSplicing(Box::new(v))
}

// === Numbers ===

#[test]
fn test_number_integer() {
    assert_eq!(parse("42"), Ok(num(42.0)));
}

#[test]
fn test_number_negative() {
    assert_eq!(parse("-17"), Ok(num(-17.0)));
}

#[test]
fn test_number_positive() {
    assert_eq!(parse("+5"), Ok(num(5.0)));
}

#[test]
fn test_number_zero() {
    assert_eq!(parse("0"), Ok(num(0.0)));
}

#[test]
fn test_number_float() {
    assert_eq!(parse("3.14159"), Ok(num(3.14159)));
}

#[test]
fn test_number_float_no_fraction() {
    assert_eq!(parse("3."), Ok(num(3.0)));
}

#[test]
fn test_number_scientific() {
    assert_eq!(parse("1e5"), Ok(num(100000.0)));
}

#[test]
fn test_number_scientific_negative_exp() {
    assert_eq!(parse("1e-5"), Ok(num(0.00001)));
}

#[test]
fn test_number_scientific_positive_exp() {
    assert_eq!(parse("1e+5"), Ok(num(100000.0)));
}

#[test]
fn test_number_float_scientific() {
    assert_eq!(parse("3.14e2"), Ok(num(314.0)));
}

// === Symbols ===

#[test]
fn test_symbol_simple() {
    assert_eq!(parse("hello"), Ok(sym("hello")));
}

#[test]
fn test_symbol_with_dash() {
    assert_eq!(parse("hello-world"), Ok(sym("hello-world")));
}

#[test]
fn test_symbol_predicate() {
    assert_eq!(parse("null?"), Ok(sym("null?")));
}

#[test]
fn test_symbol_mutator() {
    assert_eq!(parse("set!"), Ok(sym("set!")));
}

#[test]
fn test_symbol_conversion() {
    assert_eq!(parse("string->list"), Ok(sym("string->list")));
}

#[test]
fn test_symbol_with_numbers() {
    assert_eq!(parse("vector-ref2"), Ok(sym("vector-ref2")));
}

#[test]
fn test_symbol_underscore() {
    assert_eq!(parse("_private"), Ok(sym("_private")));
}

#[test]
fn test_symbol_special_chars() {
    assert_eq!(parse("make-hash-table"), Ok(sym("make-hash-table")));
}

#[test]
fn test_symbol_peculiar_plus() {
    assert_eq!(parse("+"), Ok(sym("+")));
}

#[test]
fn test_symbol_peculiar_minus() {
    assert_eq!(parse("-"), Ok(sym("-")));
}

#[test]
fn test_symbol_peculiar_ellipsis() {
    assert_eq!(parse("..."), Ok(sym("...")));
}

#[test]
fn test_symbol_math() {
    assert_eq!(parse("*"), Ok(sym("*")));
}

#[test]
fn test_symbol_division() {
    assert_eq!(parse("/"), Ok(sym("/")));
}

#[test]
fn test_symbol_less_than() {
    assert_eq!(parse("<"), Ok(sym("<")));
}

#[test]
fn test_symbol_greater_than() {
    assert_eq!(parse(">"), Ok(sym(">")));
}

#[test]
fn test_symbol_equals() {
    assert_eq!(parse("="), Ok(sym("=")));
}

#[test]
fn test_symbol_less_or_equal() {
    assert_eq!(parse("<="), Ok(sym("<=")));
}

#[test]
fn test_symbol_arrow() {
    assert_eq!(parse("->"), Ok(sym("->")));
}

// === Strings ===

#[test]
fn test_string_simple() {
    assert_eq!(parse("\"hello\""), Ok(string("hello")));
}

#[test]
fn test_string_empty() {
    assert_eq!(parse("\"\""), Ok(string("")));
}

#[test]
fn test_string_with_spaces() {
    assert_eq!(parse("\"hello world\""), Ok(string("hello world")));
}

#[test]
fn test_string_escape_newline() {
    assert_eq!(parse("\"line1\\nline2\""), Ok(string("line1\nline2")));
}

#[test]
fn test_string_escape_tab() {
    assert_eq!(parse("\"col1\\tcol2\""), Ok(string("col1\tcol2")));
}

#[test]
fn test_string_escape_quote() {
    assert_eq!(parse("\"say \\\"hi\\\"\""), Ok(string("say \"hi\"")));
}

#[test]
fn test_string_escape_backslash() {
    assert_eq!(parse("\"path\\\\file\""), Ok(string("path\\file")));
}

// === Booleans ===

#[test]
fn test_boolean_true() {
    assert_eq!(parse("#t"), Ok(SchemeValue::Boolean(true)));
}

#[test]
fn test_boolean_false() {
    assert_eq!(parse("#f"), Ok(SchemeValue::Boolean(false)));
}

// === Characters ===

#[test]
fn test_character_simple() {
    assert_eq!(parse("#\\a"), Ok(SchemeValue::Character('a')));
}

#[test]
fn test_character_uppercase() {
    assert_eq!(parse("#\\Z"), Ok(SchemeValue::Character('Z')));
}

#[test]
fn test_character_newline() {
    assert_eq!(parse("#\\newline"), Ok(SchemeValue::Character('\n')));
}

#[test]
fn test_character_space() {
    assert_eq!(parse("#\\space"), Ok(SchemeValue::Character(' ')));
}

#[test]
fn test_character_tab() {
    assert_eq!(parse("#\\tab"), Ok(SchemeValue::Character('\t')));
}

#[test]
fn test_character_digit() {
    assert_eq!(parse("#\\5"), Ok(SchemeValue::Character('5')));
}

#[test]
fn test_character_special() {
    assert_eq!(parse("#\\("), Ok(SchemeValue::Character('(')));
}

// === Lists ===

#[test]
fn test_empty_list() {
    assert_eq!(parse("()"), Ok(SchemeValue::Nil));
}

#[test]
fn test_single_element_list() {
    assert_eq!(parse("(a)"), Ok(list(vec![sym("a")])));
}

#[test]
fn test_simple_list() {
    assert_eq!(
        parse("(a b c)"),
        Ok(list(vec![sym("a"), sym("b"), sym("c")]))
    );
}

#[test]
fn test_list_with_numbers() {
    assert_eq!(
        parse("(1 2 3)"),
        Ok(list(vec![num(1.0), num(2.0), num(3.0)]))
    );
}

#[test]
fn test_nested_list() {
    let expected = list(vec![
        list(vec![sym("a"), sym("b")]),
        list(vec![sym("c"), sym("d")]),
    ]);
    assert_eq!(parse("((a b) (c d))"), Ok(expected));
}

#[test]
fn test_deeply_nested_list() {
    let expected = list(vec![list(vec![list(vec![sym("x")])])]);
    assert_eq!(parse("(((x)))"), Ok(expected));
}

#[test]
fn test_mixed_list() {
    let expected = list(vec![num(1.0), sym("a"), string("hello")]);
    assert_eq!(parse("(1 a \"hello\")"), Ok(expected));
}

#[test]
fn test_list_with_empty_list() {
    let expected = list(vec![sym("a"), SchemeValue::Nil, sym("b")]);
    assert_eq!(parse("(a () b)"), Ok(expected));
}

// === Dotted Pairs ===

#[test]
fn test_dotted_pair_simple() {
    assert_eq!(parse("(a . b)"), Ok(pair(sym("a"), sym("b"))));
}

#[test]
fn test_dotted_pair_numbers() {
    assert_eq!(parse("(1 . 2)"), Ok(pair(num(1.0), num(2.0))));
}

#[test]
fn test_dotted_list_complex() {
    // (a b . c) = Pair(a, Pair(b, c))
    let expected = pair(sym("a"), pair(sym("b"), sym("c")));
    assert_eq!(parse("(a b . c)"), Ok(expected));
}

#[test]
fn test_dotted_list_three_elements() {
    // (1 2 3 . 4) = Pair(1, Pair(2, Pair(3, 4)))
    let expected = pair(num(1.0), pair(num(2.0), pair(num(3.0), num(4.0))));
    assert_eq!(parse("(1 2 3 . 4)"), Ok(expected));
}

#[test]
fn test_dotted_pair_with_list() {
    // (a . (b c)) = (a b c) - dotted pair with proper list tail
    let expected = pair(sym("a"), list(vec![sym("b"), sym("c")]));
    assert_eq!(parse("(a . (b c))"), Ok(expected));
}

// === Vectors ===

#[test]
fn test_empty_vector() {
    assert_eq!(parse("#()"), Ok(SchemeValue::Vector(vec![])));
}

#[test]
fn test_simple_vector() {
    assert_eq!(
        parse("#(1 2 3)"),
        Ok(SchemeValue::Vector(vec![num(1.0), num(2.0), num(3.0)]))
    );
}

#[test]
fn test_vector_with_symbols() {
    assert_eq!(
        parse("#(a b c)"),
        Ok(SchemeValue::Vector(vec![sym("a"), sym("b"), sym("c")]))
    );
}

#[test]
fn test_nested_vectors() {
    assert_eq!(
        parse("#(#(1) #(2))"),
        Ok(SchemeValue::Vector(vec![
            SchemeValue::Vector(vec![num(1.0)]),
            SchemeValue::Vector(vec![num(2.0)]),
        ]))
    );
}

#[test]
fn test_vector_with_list() {
    assert_eq!(
        parse("#((a b) (c d))"),
        Ok(SchemeValue::Vector(vec![
            list(vec![sym("a"), sym("b")]),
            list(vec![sym("c"), sym("d")]),
        ]))
    );
}

// === Quote Family ===

#[test]
fn test_quote_symbol() {
    assert_eq!(parse("'x"), Ok(quote(sym("x"))));
}

#[test]
fn test_quote_number() {
    assert_eq!(parse("'42"), Ok(quote(num(42.0))));
}

#[test]
fn test_quote_list() {
    assert_eq!(parse("'(a b)"), Ok(quote(list(vec![sym("a"), sym("b")]))));
}

#[test]
fn test_quote_empty_list() {
    assert_eq!(parse("'()"), Ok(quote(SchemeValue::Nil)));
}

#[test]
fn test_quasiquote_symbol() {
    assert_eq!(parse("`x"), Ok(quasiquote(sym("x"))));
}

#[test]
fn test_quasiquote_list() {
    assert_eq!(
        parse("`(a b)"),
        Ok(quasiquote(list(vec![sym("a"), sym("b")])))
    );
}

#[test]
fn test_unquote_symbol() {
    assert_eq!(parse(",x"), Ok(unquote(sym("x"))));
}

#[test]
fn test_unquote_splicing_symbol() {
    assert_eq!(parse(",@x"), Ok(unquote_splicing(sym("x"))));
}

#[test]
fn test_unquote_splicing_list() {
    assert_eq!(
        parse(",@(a b)"),
        Ok(unquote_splicing(list(vec![sym("a"), sym("b")])))
    );
}

#[test]
fn test_nested_quotes() {
    // ''x = (quote (quote x))
    assert_eq!(parse("''x"), Ok(quote(quote(sym("x")))));
}

#[test]
fn test_triple_quotes() {
    // '''x = (quote (quote (quote x)))
    assert_eq!(parse("'''x"), Ok(quote(quote(quote(sym("x"))))));
}

#[test]
fn test_quasiquote_with_unquote() {
    // `(a ,b c) = (quasiquote (a (unquote b) c))
    let expected = quasiquote(list(vec![sym("a"), unquote(sym("b")), sym("c")]));
    assert_eq!(parse("`(a ,b c)"), Ok(expected));
}

#[test]
fn test_quasiquote_with_unquote_splicing() {
    // `(a ,@b c) = (quasiquote (a (unquote-splicing b) c))
    let expected = quasiquote(list(vec![sym("a"), unquote_splicing(sym("b")), sym("c")]));
    assert_eq!(parse("`(a ,@b c)"), Ok(expected));
}

#[test]
fn test_quote_vector() {
    assert_eq!(
        parse("'#(1 2 3)"),
        Ok(quote(SchemeValue::Vector(vec![
            num(1.0),
            num(2.0),
            num(3.0)
        ])))
    );
}

// === Comments ===

#[test]
fn test_line_comment_before() {
    assert_eq!(parse("; comment\n42"), Ok(num(42.0)));
}

#[test]
fn test_line_comment_after() {
    assert_eq!(parse("42 ; comment"), Ok(num(42.0)));
}

#[test]
fn test_comment_in_list() {
    let expected = list(vec![sym("a"), sym("b")]);
    assert_eq!(parse("(a ; comment\n b)"), Ok(expected));
}

#[test]
fn test_multiple_comments() {
    assert_eq!(parse("; line 1\n; line 2\n42"), Ok(num(42.0)));
}

// === Whitespace ===

#[test]
fn test_leading_whitespace() {
    assert_eq!(parse("   42"), Ok(num(42.0)));
}

#[test]
fn test_trailing_whitespace() {
    assert_eq!(parse("42   "), Ok(num(42.0)));
}

#[test]
fn test_list_with_extra_whitespace() {
    let expected = list(vec![sym("a"), sym("b"), sym("c")]);
    assert_eq!(parse("(  a   b   c  )"), Ok(expected));
}

#[test]
fn test_newlines_in_list() {
    let expected = list(vec![sym("a"), sym("b"), sym("c")]);
    assert_eq!(parse("(a\nb\nc)"), Ok(expected));
}

// === Stress Tests ===

#[test]
fn test_deeply_nested_lists_100() {
    let depth = 100;
    let input = format!("{}{}{}", "(".repeat(depth), "x", ")".repeat(depth));
    let result = parse(&input);
    assert!(result.is_ok());
    // Verify structure is correct
    let mut current = result.unwrap();
    for _ in 0..depth {
        match current {
            SchemeValue::Pair(first, rest) => {
                current = *first;
                assert_eq!(*rest, SchemeValue::Nil);
            }
            _ => panic!("Expected nested list structure"),
        }
    }
    assert_eq!(current, sym("x"));
}

#[test]
fn test_deeply_nested_quotes_100() {
    let depth = 100;
    let input = format!("{}x", "'".repeat(depth));
    let result = parse(&input);
    assert!(result.is_ok());
    // Verify structure is correct
    let mut current = result.unwrap();
    for _ in 0..depth {
        match current {
            SchemeValue::Quote(inner) => current = *inner,
            _ => panic!("Expected quote"),
        }
    }
    assert_eq!(current, sym("x"));
}

#[test]
fn test_long_list_1000_elements() {
    let elements: String = (0..1000).map(|i| format!("x{} ", i)).collect();
    let input = format!("({})", elements);
    let result = parse(&input);
    assert!(result.is_ok());
    // Count elements
    let mut count = 0;
    let mut current = result.unwrap();
    while let SchemeValue::Pair(_, rest) = current {
        count += 1;
        current = *rest;
    }
    assert_eq!(count, 1000);
}

#[test]
fn test_long_symbol_name() {
    let name = "a".repeat(1000);
    let result = parse(&name);
    assert_eq!(result, Ok(sym(&name)));
}

#[test]
fn test_long_string() {
    let content = "x".repeat(10000);
    let input = format!("\"{}\"", content);
    let result = parse(&input);
    assert_eq!(result, Ok(string(&content)));
}

#[test]
fn test_mixed_deep_nesting() {
    // Mix of lists, quotes, and vectors
    // '(#(((a)))) = quote of list containing vector containing ((a))
    let input = "'(#(((a))))";
    let expected = quote(list(vec![SchemeValue::Vector(vec![list(vec![list(
        vec![sym("a")],
    )])])]));
    assert_eq!(parse(input), Ok(expected));
}

#[test]
fn test_complex_quasiquote() {
    // `(let ((x ,a)) ,@body)
    let input = "`(let ((x ,a)) ,@body)";
    let expected = quasiquote(list(vec![
        sym("let"),
        list(vec![list(vec![sym("x"), unquote(sym("a"))])]),
        unquote_splicing(sym("body")),
    ]));
    assert_eq!(parse(input), Ok(expected));
}

// === Hash Prefix Disambiguation ===

#[test]
fn test_hash_t_not_char() {
    // #t should be boolean, not character 't'
    assert_eq!(parse("#t"), Ok(SchemeValue::Boolean(true)));
}

#[test]
fn test_hash_f_not_char() {
    // #f should be boolean, not character 'f'
    assert_eq!(parse("#f"), Ok(SchemeValue::Boolean(false)));
}

#[test]
fn test_hash_backslash_t() {
    // #\t should be character 't'
    assert_eq!(parse("#\\t"), Ok(SchemeValue::Character('t')));
}

#[test]
fn test_hash_paren_is_vector() {
    // #( should start a vector
    assert_eq!(parse("#(1)"), Ok(SchemeValue::Vector(vec![num(1.0)])));
}

// === Error Cases ===

#[test]
fn test_unclosed_list() {
    let result = parse("(a b");
    assert!(result.is_err());
}

#[test]
fn test_unclosed_string() {
    let result = parse("\"hello");
    assert!(result.is_err());
}

#[test]
fn test_unclosed_vector() {
    let result = parse("#(1 2");
    assert!(result.is_err());
}

#[test]
fn test_invalid_hash() {
    // #x is not valid (not #t, #f, #\, or #()
    let result = parse("#x");
    assert!(result.is_err());
}

// === Real Scheme Code ===

#[test]
fn test_define_function() {
    // (define (square x) (* x x))
    let expected = list(vec![
        sym("define"),
        list(vec![sym("square"), sym("x")]),
        list(vec![sym("*"), sym("x"), sym("x")]),
    ]);
    assert_eq!(parse("(define (square x) (* x x))"), Ok(expected));
}

#[test]
fn test_lambda() {
    // (lambda (x y) (+ x y))
    let expected = list(vec![
        sym("lambda"),
        list(vec![sym("x"), sym("y")]),
        list(vec![sym("+"), sym("x"), sym("y")]),
    ]);
    assert_eq!(parse("(lambda (x y) (+ x y))"), Ok(expected));
}

#[test]
fn test_if_expression() {
    // (if (> x 0) 'positive 'non-positive)
    let expected = list(vec![
        sym("if"),
        list(vec![sym(">"), sym("x"), num(0.0)]),
        quote(sym("positive")),
        quote(sym("non-positive")),
    ]);
    assert_eq!(parse("(if (> x 0) 'positive 'non-positive)"), Ok(expected));
}

#[test]
fn test_cond_expression() {
    // (cond ((= x 0) 'zero) ((> x 0) 'positive) (else 'negative))
    let expected = list(vec![
        sym("cond"),
        list(vec![
            list(vec![sym("="), sym("x"), num(0.0)]),
            quote(sym("zero")),
        ]),
        list(vec![
            list(vec![sym(">"), sym("x"), num(0.0)]),
            quote(sym("positive")),
        ]),
        list(vec![sym("else"), quote(sym("negative"))]),
    ]);
    assert_eq!(
        parse("(cond ((= x 0) 'zero) ((> x 0) 'positive) (else 'negative))"),
        Ok(expected)
    );
}

#[test]
fn test_let_expression() {
    // (let ((x 1) (y 2)) (+ x y))
    let expected = list(vec![
        sym("let"),
        list(vec![
            list(vec![sym("x"), num(1.0)]),
            list(vec![sym("y"), num(2.0)]),
        ]),
        list(vec![sym("+"), sym("x"), sym("y")]),
    ]);
    assert_eq!(parse("(let ((x 1) (y 2)) (+ x y))"), Ok(expected));
}

#[test]
fn test_quasiquote_macro_template() {
    // `(lambda (,@args) ,body)
    let expected = quasiquote(list(vec![
        sym("lambda"),
        list(vec![unquote_splicing(sym("args"))]),
        unquote(sym("body")),
    ]));
    assert_eq!(parse("`(lambda (,@args) ,body)"), Ok(expected));
}
