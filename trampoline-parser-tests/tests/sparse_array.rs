//! Tests for sparse array parsing (arrays with holes like [, , 3])
//! Also tests Pratt expressions inside separated lists

use trampoline_parser_tests::*;

// =============================================================================
// Basic Array Tests
// =============================================================================

#[test]
fn sparse_array_empty() {
    let mut parser = sparse_array_parser::Parser::new("[]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse empty array");
}

#[test]
fn sparse_array_single_element() {
    let mut parser = sparse_array_parser::Parser::new("[foo]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse single element array");
}

#[test]
fn sparse_array_multiple_elements() {
    let mut parser = sparse_array_parser::Parser::new("[foo,bar,baz]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse multiple element array");
}

#[test]
fn sparse_array_trailing_comma() {
    let mut parser = sparse_array_parser::Parser::new("[foo,bar,]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse array with trailing comma");
}

// =============================================================================
// Sparse Array Tests (arrays with holes)
// =============================================================================

// This test may hang/timeout if there's an infinite loop bug
#[test]
fn sparse_array_leading_hole() {
    // Array starting with a hole like [,a]
    // The first element is "empty" (comma without preceding value)
    let mut parser = sparse_array_parser::Parser::new("[,foo]");
    let result = parser.parse();
    // This SHOULD either:
    // 1. Parse successfully with an empty first element, OR
    // 2. Fail to parse (since our grammar requires elements)
    // It should NOT hang infinitely
    println!("Result: {:?}", result);
}

#[test]
fn sparse_array_only_hole() {
    // Array with just a comma (one hole)
    let mut parser = sparse_array_parser::Parser::new("[,]");
    let result = parser.parse();
    println!("Result: {:?}", result);
}

#[test]
fn sparse_array_multiple_leading_holes() {
    // Array starting with multiple holes
    let mut parser = sparse_array_parser::Parser::new("[,,foo]");
    let result = parser.parse();
    println!("Result: {:?}", result);
}

#[test]
fn sparse_array_middle_hole() {
    // Array with a hole in the middle
    let mut parser = sparse_array_parser::Parser::new("[foo,,bar]");
    let result = parser.parse();
    println!("Result: {:?}", result);
}

#[test]
fn sparse_array_all_holes() {
    // Array with only holes
    let mut parser = sparse_array_parser::Parser::new("[,,,]");
    let result = parser.parse();
    println!("Result: {:?}", result);
}

// =============================================================================
// Pratt Expressions in List Tests
// =============================================================================

#[test]
fn pratt_in_list_empty() {
    let mut parser = pratt_in_list_parser::Parser::new("[]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse empty array");
}

#[test]
fn pratt_in_list_single() {
    let mut parser = pratt_in_list_parser::Parser::new("[foo]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse single element");
}

#[test]
fn pratt_in_list_expression() {
    let mut parser = pratt_in_list_parser::Parser::new("[foo+bar]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse expression");
}

#[test]
fn pratt_in_list_multiple() {
    let mut parser = pratt_in_list_parser::Parser::new("[foo,bar,baz]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse multiple elements");
}

#[test]
fn pratt_in_list_trailing_comma() {
    let mut parser = pratt_in_list_parser::Parser::new("[foo,bar,]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse trailing comma");
}

// This test may hang/timeout if there's an infinite loop bug
#[test]
fn pratt_in_list_leading_hole() {
    // Array starting with a hole - this might trigger infinite loop
    let mut parser = pratt_in_list_parser::Parser::new("[,foo]");
    let result = parser.parse();
    println!("pratt_in_list_leading_hole: {:?}", result);
}

#[test]
fn pratt_in_list_only_comma() {
    let mut parser = pratt_in_list_parser::Parser::new("[,]");
    let result = parser.parse();
    println!("pratt_in_list_only_comma: {:?}", result);
}

#[test]
fn pratt_in_list_multiple_holes() {
    let mut parser = pratt_in_list_parser::Parser::new("[,,foo]");
    let result = parser.parse();
    println!("pratt_in_list_multiple_holes: {:?}", result);
}

// =============================================================================
// Pratt with Postfix Operators Tests
// =============================================================================

#[test]
fn pratt_postfix_empty() {
    let mut parser = pratt_in_list_postfix_parser::Parser::new("[]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse empty array");
}

#[test]
fn pratt_postfix_call_expr() {
    let mut parser = pratt_in_list_postfix_parser::Parser::new("[foo()]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse call expression");
}

#[test]
fn pratt_postfix_member_expr() {
    let mut parser = pratt_in_list_postfix_parser::Parser::new("[foo.bar]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse member expression");
}

#[test]
fn pratt_postfix_index_expr() {
    let mut parser = pratt_in_list_postfix_parser::Parser::new("[foo[0]]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse index expression");
}

// This test may hang/timeout if there's an infinite loop bug
#[test]
fn pratt_postfix_leading_hole() {
    let mut parser = pratt_in_list_postfix_parser::Parser::new("[,foo]");
    let result = parser.parse();
    println!("pratt_postfix_leading_hole: {:?}", result);
}

#[test]
fn pratt_postfix_only_comma() {
    let mut parser = pratt_in_list_postfix_parser::Parser::new("[,]");
    let result = parser.parse();
    println!("pratt_postfix_only_comma: {:?}", result);
}

// =============================================================================
// TypeScript-like Grammar Tests (with ws, choice wrapper, nested arrays)
// =============================================================================

#[test]
fn ts_like_empty() {
    let mut parser = pratt_in_list_ts_parser::Parser::new("[]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse empty array");
}

#[test]
fn ts_like_single() {
    let mut parser = pratt_in_list_ts_parser::Parser::new("[foo]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse single element");
}

#[test]
fn ts_like_nested() {
    let mut parser = pratt_in_list_ts_parser::Parser::new("[[foo]]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse nested array");
}

#[test]
fn ts_like_spread() {
    let mut parser = pratt_in_list_ts_parser::Parser::new("[...foo]");
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse spread element");
}

// This test may hang/timeout if there's an infinite loop bug
#[test]
fn ts_like_leading_hole() {
    let mut parser = pratt_in_list_ts_parser::Parser::new("[, foo]");
    let result = parser.parse();
    println!("ts_like_leading_hole: {:?}", result);
}

#[test]
fn ts_like_only_comma() {
    let mut parser = pratt_in_list_ts_parser::Parser::new("[,]");
    let result = parser.parse();
    println!("ts_like_only_comma: {:?}", result);
}

#[test]
fn ts_like_multiple_holes() {
    let mut parser = pratt_in_list_ts_parser::Parser::new("[, , foo]");
    let result = parser.parse();
    println!("ts_like_multiple_holes: {:?}", result);
}
