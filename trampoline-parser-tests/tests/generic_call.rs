//! Tests demonstrating exponential backtracking with generic call expressions.
//!
//! The pattern `identifier < type_args > ( args )` can cause exponential backtracking
//! when parsing input with many `<` characters because the parser must consider
//! whether each `<` starts a type argument list or is a comparison operator.
//!
//! This test documents the issue for future fixes in the trampoline-parser.

use trampoline_parser_tests::generic_call_auto_memoized_parser as auto_memoized;
use trampoline_parser_tests::generic_call_bad_parser as bad;
use trampoline_parser_tests::generic_call_good_parser as good;
use trampoline_parser_tests::generic_call_memoized_parser as memoized;

/// Tests that simple generic calls work correctly.
#[test]
fn test_simple_generic_call() {
    // identity<number>(x)
    let mut parser = good::Parser::new("identity<number>(x)");
    let result = parser.parse();
    assert!(result.is_ok(), "Simple generic call should parse");
}

/// Tests that comparisons still work correctly.
#[test]
fn test_comparison_not_generic_call() {
    // a < b should be comparison, not start of generic call
    let mut parser = good::Parser::new("a < b");
    let result = parser.parse();
    assert!(result.is_ok(), "Comparison should parse");
}

/// Demonstrates the exponential backtracking problem.
///
/// When parsing input like `a<b<c<d<...`, the bad grammar explores many possibilities:
/// - Is `<b` a type argument or comparison?
/// - If type argument, is `<c` nested or end of type args?
/// etc.
///
/// This leads to exponential time as each `<` doubles the possibilities.
#[test]
#[ignore] // TODO: Fix exponential backtracking in trampoline-parser
fn test_bad_grammar_exponential_backtracking() {
    use std::time::{Duration, Instant};

    // Input with many < characters - causes exponential backtracking
    let input = "a<b<c<d<e<f<g<h<i<j";

    let start = Instant::now();
    let mut parser = bad::Parser::new(input);
    let _ = parser.parse();
    let elapsed = start.elapsed();

    // This should complete in under 1 second but with exponential backtracking
    // it may take much longer
    assert!(
        elapsed < Duration::from_secs(1),
        "Bad grammar took too long: {:?} (expected < 1s)",
        elapsed
    );
}

/// Shows that using simple type arguments avoids the backtracking issue.
#[test]
fn test_good_grammar_fast() {
    use std::time::{Duration, Instant};

    // Same input - but good grammar uses simpler type matching
    let input = "a<b<c<d<e<f<g<h<i<j";

    let start = Instant::now();
    let mut parser = good::Parser::new(input);
    let _ = parser.parse();
    let elapsed = start.elapsed();

    // With simple type arguments, this should complete quickly
    assert!(
        elapsed < Duration::from_secs(1),
        "Good grammar took too long: {:?} (expected < 1s)",
        elapsed
    );
}

/// Documents the pattern that causes the issue.
///
/// When generic_call is tried before identifier in a choice, the parser:
/// 1. Matches identifier `a`
/// 2. Tries to match `<` as start of type_arguments
/// 3. Recursively tries to parse type (which can include type_reference with more type_arguments)
/// 4. Each level of nesting doubles the search space
///
/// The fix is to use simpler type matching that doesn't allow arbitrary nesting,
/// or to use lookahead to determine if `<` is followed by a valid type argument list.
#[test]
fn test_pattern_documentation() {
    let bad_pattern = r#"
        // BAD: generic_call with full type_arguments (allows nesting)
        .rule("primary_inner", |r| {
            r.choice((
                r.parse("generic_call"),  // identifier<types>(args)
                r.parse("identifier"),
            ))
        })
        .rule("type_arguments", |r| {
            r.sequence((
                op(r, "<"),
                r.separated_by(r.parse("type"), op(r, ",")),  // type can nest!
                op(r, ">"),
            ))
        })
    "#;

    let good_pattern = r#"
        // GOOD: generic_call with simple_type_arguments (no nesting)
        .rule("primary_inner", |r| {
            r.choice((
                r.parse("generic_call"),
                r.parse("identifier"),
            ))
        })
        .rule("simple_type_arguments", |r| {
            r.sequence((
                op(r, "<"),
                r.separated_by(r.parse("identifier"), op(r, ",")),  // just identifiers!
                op(r, ">"),
            ))
        })
    "#;

    println!("Bad pattern (exponential backtracking):\n{}", bad_pattern);
    println!("Good pattern (linear time):\n{}", good_pattern);
}

/// Tests that memoization fixes the exponential backtracking.
///
/// The memoized grammar uses the same complex type_arguments that cause
/// exponential backtracking in the bad grammar, but wraps generic_call
/// with `.memoize()` to cache results at each position.
///
/// Without memoization: O(2^n) time
/// With memoization: O(n) time
#[test]
fn test_memoized_grammar_fast() {
    use std::time::{Duration, Instant};

    // Same input that causes exponential backtracking in bad grammar
    let input = "a<b<c<d<e<f<g<h<i<j";

    let start = Instant::now();
    let mut parser = memoized::Parser::new(input);
    let _ = parser.parse();
    let elapsed = start.elapsed();

    // With memoization, this should complete quickly even with complex type args
    assert!(
        elapsed < Duration::from_secs(1),
        "Memoized grammar took too long: {:?} (expected < 1s)",
        elapsed
    );
}

/// Verifies memoized grammar correctly parses generic calls.
#[test]
fn test_memoized_simple_generic_call() {
    let mut parser = memoized::Parser::new("identity<number>(x)");
    let result = parser.parse();
    assert!(result.is_ok(), "Memoized grammar should parse generic call");
}

/// Verifies memoized grammar correctly parses comparisons.
#[test]
fn test_memoized_comparison() {
    let mut parser = memoized::Parser::new("a < b");
    let result = parser.parse();
    assert!(result.is_ok(), "Memoized grammar should parse comparison");
}

/// Tests that automatic memoization via build_with_memoization() works.
///
/// The auto-memoized grammar uses the same complex type_arguments as the bad
/// grammar, but should be fast because the rules are automatically wrapped
/// with memoization based on backtracking analysis.
#[test]
fn test_auto_memoized_grammar_fast() {
    use std::time::{Duration, Instant};

    let input = "a<b<c<d<e<f<g<h<i<j";

    let start = Instant::now();
    let mut parser = auto_memoized::Parser::new(input);
    let _ = parser.parse();
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "Auto-memoized grammar took too long: {:?} (expected < 1s)",
        elapsed
    );
}

/// Verifies auto-memoized grammar correctly parses generic calls.
#[test]
fn test_auto_memoized_simple_generic_call() {
    let mut parser = auto_memoized::Parser::new("identity<number>(x)");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Auto-memoized grammar should parse generic call"
    );
}

/// Verifies auto-memoized grammar correctly parses comparisons.
#[test]
fn test_auto_memoized_comparison() {
    let mut parser = auto_memoized::Parser::new("a < b");
    let result = parser.parse();
    assert!(
        result.is_ok(),
        "Auto-memoized grammar should parse comparison"
    );
}
