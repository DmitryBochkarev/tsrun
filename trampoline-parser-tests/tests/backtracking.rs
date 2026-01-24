//! Tests demonstrating exponential backtracking issues in grammars.
//!
//! These tests document problematic grammar patterns that cause O(2^n) parsing time
//! due to repeated re-parsing of shared prefixes during backtracking.
//!
//! Also tests the automatic detection and optimization features:
//! - `analyze_backtracking()`: Detects problematic patterns
//! - `optimize_backtracking()`: Rewrites grammars to eliminate the issue

use trampoline_parser::BacktrackingSeverity;
use trampoline_parser_tests::backtracking_bad_parser as bad;
use trampoline_parser_tests::backtracking_good_parser as good;

/// Demonstrates the exponential backtracking problem.
///
/// Grammar pattern that causes the issue:
/// ```text
/// list -> empty_list | dotted_list | proper_list
/// dotted_list -> '(' datum+ '.' datum ')'
/// proper_list -> '(' datum+ ')'
/// ```
///
/// When parsing `((((x))))`:
/// 1. dotted_list matches '(', then recursively parses inner content
/// 2. After parsing all inner content, dotted_list expects '.' but finds ')'
/// 3. Backtracks and tries proper_list
/// 4. proper_list re-parses the ENTIRE inner content again
///
/// At each nesting level, the inner content is parsed twice,
/// leading to O(2^n) time complexity.
#[test]
fn test_backtracking_grows_exponentially() {
    let mut times = Vec::new();

    for depth in [3, 6, 9, 12, 15] {
        let input = format!("{}{}{}", "(".repeat(depth), "x", ")".repeat(depth));

        let start = std::time::Instant::now();
        let mut parser = bad::Parser::new(&input);
        let result = parser.parse();
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Parsing failed at depth {}", depth);
        times.push((depth, elapsed));

        println!("Depth {:2}: {:?}", depth, elapsed);
    }

    // Check that time grows roughly exponentially (each +3 depth ~= 8x slower)
    // We allow some variance, but the pattern should be clear
    if times.len() >= 2 {
        let (d1, t1) = times[0];
        let (d2, t2) = times[times.len() - 1];

        let depth_ratio = (d2 - d1) as f64 / 3.0; // How many "doublings" of 3 levels
        let time_ratio = t2.as_secs_f64() / t1.as_secs_f64().max(0.0001);

        // For exponential growth with base ~2, each 3 levels should ~8x the time
        // Log base 8 of the time ratio should be roughly equal to depth_ratio
        let expected_ratio = 8.0_f64.powf(depth_ratio);

        println!("\nTime growth analysis:");
        println!("  Depth increase: {} -> {} (+{})", d1, d2, d2 - d1);
        println!("  Time ratio: {:.1}x", time_ratio);
        println!("  Expected for O(2^n): ~{:.1}x", expected_ratio);

        // The bad grammar should show significant slowdown
        // (we don't assert exact ratio as it varies by machine)
        assert!(
            time_ratio > 10.0,
            "Expected significant slowdown due to backtracking, got only {:.1}x",
            time_ratio
        );
    }
}

/// Shows that factoring out common prefix eliminates exponential backtracking.
///
/// Fixed grammar pattern:
/// ```text
/// list -> empty_list | non_empty_list
/// non_empty_list -> '(' datum+ dotted_tail? ')'
/// dotted_tail -> '.' datum
/// ```
///
/// Now the common prefix (datum+) is parsed only once, and we just check
/// if there's an optional dotted tail. This is O(n) instead of O(2^n).
#[test]
fn test_factored_grammar_is_linear() {
    let mut times = Vec::new();

    for depth in [3, 10, 25, 50, 100] {
        let input = format!("{}{}{}", "(".repeat(depth), "x", ")".repeat(depth));

        let start = std::time::Instant::now();
        let mut parser = good::Parser::new(&input);
        let result = parser.parse();
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Parsing failed at depth {}", depth);
        times.push((depth, elapsed));

        println!("Depth {:3}: {:?}", depth, elapsed);
    }

    // Check that time grows roughly linearly
    if times.len() >= 2 {
        let (d1, t1) = times[0];
        let (d2, t2) = times[times.len() - 1];

        let depth_ratio = d2 as f64 / d1 as f64;
        let time_ratio = t2.as_secs_f64() / t1.as_secs_f64().max(0.0001);

        println!("\nTime growth analysis:");
        println!("  Depth increase: {} -> {} ({:.1}x)", d1, d2, depth_ratio);
        println!("  Time ratio: {:.1}x", time_ratio);
        println!("  Expected for O(n): ~{:.1}x", depth_ratio);

        // For linear growth, time ratio should be roughly proportional to depth ratio
        // Allow 3x variance for system noise
        assert!(
            time_ratio < depth_ratio * 3.0,
            "Expected linear growth (~{:.1}x), got {:.1}x - might have backtracking issue",
            depth_ratio,
            time_ratio
        );
    }
}

/// Compares bad vs good grammar on the same input.
#[test]
fn test_compare_bad_vs_good() {
    let depth = 12;
    let input = format!("{}{}{}", "(".repeat(depth), "x", ")".repeat(depth));

    // Bad grammar (exponential)
    let start_bad = std::time::Instant::now();
    let mut parser_bad = bad::Parser::new(&input);
    let result_bad = parser_bad.parse();
    let time_bad = start_bad.elapsed();

    // Good grammar (linear)
    let start_good = std::time::Instant::now();
    let mut parser_good = good::Parser::new(&input);
    let result_good = parser_good.parse();
    let time_good = start_good.elapsed();

    assert!(result_bad.is_ok());
    assert!(result_good.is_ok());

    let speedup = time_bad.as_secs_f64() / time_good.as_secs_f64().max(0.0001);

    println!(
        "Depth {}: bad={:?}, good={:?}, speedup={:.1}x",
        depth, time_bad, time_good, speedup
    );

    // The good grammar should be significantly faster
    assert!(
        speedup > 5.0,
        "Expected good grammar to be much faster, got only {:.1}x speedup",
        speedup
    );
}

/// Documents what the detector should flag.
///
/// The pattern to detect:
/// - Two or more alternatives in a choice
/// - The alternatives share a common prefix
/// - The prefix contains recursive rules (like `datum+`)
///
/// Suggested fix:
/// - Factor out the common prefix
/// - Make the differing suffix optional or a nested choice
#[test]
fn test_pattern_description() {
    // This test just documents the pattern - detection would be compile-time

    let bad_pattern = r#"
        // BAD: Shared prefix with recursive content
        .rule("list", |r| {
            r.choice((
                r.parse("dotted_list"),  // '(' datum+ '.' datum ')'
                r.parse("proper_list"),  // '(' datum+ ')'
            ))
        })
    "#;

    let good_pattern = r#"
        // GOOD: Factored common prefix
        .rule("list", |r| {
            r.sequence((
                r.char('('),
                r.one_or_more(r.parse("datum")),
                r.optional(r.sequence((r.char('.'), r.parse("datum")))),
                r.char(')'),
            ))
        })
    "#;

    println!(
        "Bad pattern (causes exponential backtracking):\n{}",
        bad_pattern
    );
    println!("Good pattern (linear time):\n{}", good_pattern);
}

// ============================================================================
// Tests for automatic detection and optimization
// ============================================================================

use trampoline_parser::Grammar;

/// Helper to create a simple grammar with inline shared prefix (easier to detect)
fn create_simple_bad_grammar() -> trampoline_parser::CompiledGrammar {
    Grammar::new()
        .rule("expr", |r| {
            r.choice((
                // Both alternatives share '(' datum+ prefix
                r.sequence((
                    r.char('('),
                    r.one_or_more(r.parse("datum")),
                    r.char('.'),
                    r.parse("datum"),
                    r.char(')'),
                )),
                r.sequence((r.char('('), r.one_or_more(r.parse("datum")), r.char(')'))),
            ))
        })
        .rule("datum", |r| {
            r.choice((r.parse("expr"), r.capture(r.one_or_more(r.alpha()))))
        })
        .build()
}

/// Test that analyze_backtracking detects exponential patterns
#[test]
fn test_detection_finds_exponential_patterns() {
    // Test with simple inline grammar (direct shared prefix)
    let simple_grammar = create_simple_bad_grammar();
    let warnings = simple_grammar.analyze_backtracking();

    println!("Simple grammar warnings:");
    for w in &warnings {
        println!(
            "  - {}: {} (severity: {:?})",
            w.rule_name, w.description, w.severity
        );
    }

    assert!(
        warnings
            .iter()
            .any(|w| w.severity == BacktrackingSeverity::Exponential),
        "Expected to detect exponential backtracking in simple grammar"
    );
}

/// Test that analyze_backtracking detects patterns through rule references
/// when only SOME alternatives share a prefix.
///
/// Currently, the algorithm only detects shared prefixes when ALL alternatives
/// in a choice share the same prefix. This is a known limitation.
///
/// TODO: Enhance to detect pairwise shared prefixes among subsets of alternatives.
#[test]
fn test_detection_expands_rule_references() {
    // Create a grammar where ALL choice alternatives share a prefix when expanded
    let grammar = Grammar::new()
        .rule("list", |r| {
            r.choice((
                r.parse("dotted_list"), // '(' datum+ '.' datum ')'
                r.parse("proper_list"), // '(' datum+ ')'
                                        // Note: no empty_list - all alternatives share prefix
            ))
        })
        .rule("dotted_list", |r| {
            r.sequence((
                r.char('('),
                r.parse("ws"),
                r.one_or_more(r.sequence((r.parse("datum"), r.parse("ws")))),
                r.char('.'),
                r.parse("ws"),
                r.parse("datum"),
                r.parse("ws"),
                r.char(')'),
            ))
        })
        .rule("proper_list", |r| {
            r.sequence((
                r.char('('),
                r.parse("ws"),
                r.one_or_more(r.sequence((r.parse("datum"), r.parse("ws")))),
                r.char(')'),
            ))
        })
        .rule("datum", |r| r.choice((r.parse("list"), r.parse("symbol"))))
        .rule("symbol", |r| r.capture(r.one_or_more(r.alpha())))
        .rule("ws", |r| r.skip(r.zero_or_more(r.ws())))
        .build();

    let warnings = grammar.analyze_backtracking();

    println!("Grammar with rule references warnings:");
    for w in &warnings {
        println!(
            "  - {}: {} (severity: {:?})",
            w.rule_name, w.description, w.severity
        );
    }

    // The 'list' rule has choice between dotted_list and proper_list
    // which share a prefix when expanded
    assert!(
        warnings
            .iter()
            .any(|w| w.severity == BacktrackingSeverity::Exponential),
        "Expected to detect exponential backtracking through rule references"
    );
}

/// Test that optimize_backtracking produces a grammar with better performance
#[test]
fn test_optimization_improves_performance() {
    let simple_grammar = create_simple_bad_grammar();

    // Analyze before optimization
    let warnings_before = simple_grammar.analyze_backtracking();
    assert!(
        warnings_before
            .iter()
            .any(|w| w.severity == BacktrackingSeverity::Exponential),
        "Pre-condition: grammar should have exponential backtracking"
    );

    // Optimize
    let optimized = simple_grammar.optimize_backtracking();

    // Analyze after optimization
    let warnings_after = optimized.analyze_backtracking();
    println!("Warnings after optimization:");
    for w in &warnings_after {
        println!(
            "  - {}: {} (severity: {:?})",
            w.rule_name, w.description, w.severity
        );
    }

    assert!(
        !warnings_after
            .iter()
            .any(|w| w.severity == BacktrackingSeverity::Exponential),
        "After optimization, should have no exponential backtracking"
    );
}
