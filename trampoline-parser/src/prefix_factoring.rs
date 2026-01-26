//! Automatic detection and rewriting of exponential backtracking patterns.
//!
//! When `Choice` alternatives share a common prefix containing recursive rules,
//! parsing becomes O(2^n). This module detects such patterns and rewrites them
//! to factor out the common prefix, achieving O(n) parsing time.
//!
//! ## Example
//!
//! ```text
//! // BAD: O(2^n) - shared prefix '(' datum+ is re-parsed on backtrack
//! Choice([
//!     Sequence(['(', datum+, '.', datum, ')']),  // dotted_list
//!     Sequence(['(', datum+, ')']),              // proper_list
//! ])
//!
//! // GOOD: O(n) - prefix parsed once, suffix is optional
//! Sequence([
//!     '(',
//!     datum+,
//!     Optional(Sequence(['.', datum])),
//!     ')',
//! ])
//! ```

use crate::ir::{Combinator, InfixOp, PostfixOp, PrattDef, PrefixOp, RuleDef, TernaryOp};
use crate::validation;
use std::collections::{HashMap, HashSet};

/// Severity of a backtracking issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacktrackingSeverity {
    /// No shared prefix between alternatives
    None,
    /// Shared prefix exists but contains no recursion (O(k*n) worst case)
    Linear,
    /// Shared prefix contains recursive rules (O(2^n) worst case)
    Exponential,
}

/// Result of analyzing a Choice node for common prefixes
#[derive(Debug)]
pub struct PrefixAnalysis {
    /// The shared prefix elements
    pub prefix: Vec<Combinator>,
    /// What remains after the prefix for each alternative
    pub suffixes: Vec<Suffix>,
    /// How severe the backtracking issue is
    pub severity: BacktrackingSeverity,
}

/// What remains of an alternative after factoring out the common prefix
#[derive(Debug, Clone)]
pub enum Suffix {
    /// The alternative was exactly the prefix (nothing remains)
    Empty,
    /// A single combinator remains
    Single(Combinator),
    /// Multiple combinators remain
    Sequence(Vec<Combinator>),
}

/// Warning about a backtracking issue in a grammar
#[derive(Debug)]
pub struct BacktrackingWarning {
    /// Name of the rule containing the problematic choice
    pub rule_name: String,
    /// Human-readable description of the issue
    pub description: String,
    /// Severity of the issue
    pub severity: BacktrackingSeverity,
}

// ============================================================================
// Combinator Equality
// ============================================================================

/// Check if two combinators are structurally equal.
///
/// This is a deep structural comparison. Two combinators may match the same
/// language but not be structurally equal (e.g., different orderings in choice).
pub fn combinators_equal(a: &Combinator, b: &Combinator) -> bool {
    match (a, b) {
        // Leaf nodes - direct comparison
        (Combinator::Literal(s1), Combinator::Literal(s2)) => s1 == s2,
        (Combinator::Char(c1), Combinator::Char(c2)) => c1 == c2,
        (Combinator::CharClass(cc1), Combinator::CharClass(cc2)) => cc1 == cc2,
        (Combinator::CharRange(a1, b1), Combinator::CharRange(a2, b2)) => a1 == a2 && b1 == b2,
        (Combinator::AnyChar, Combinator::AnyChar) => true,
        (Combinator::Rule(r1), Combinator::Rule(r2)) => r1 == r2,

        // Recursive nodes - compare children
        (Combinator::Sequence(items1), Combinator::Sequence(items2)) => {
            items1.len() == items2.len()
                && items1
                    .iter()
                    .zip(items2)
                    .all(|(a, b)| combinators_equal(a, b))
        }
        (Combinator::Choice(items1), Combinator::Choice(items2)) => {
            items1.len() == items2.len()
                && items1
                    .iter()
                    .zip(items2)
                    .all(|(a, b)| combinators_equal(a, b))
        }

        // Single-child wrappers
        (Combinator::ZeroOrMore(inner1), Combinator::ZeroOrMore(inner2))
        | (Combinator::OneOrMore(inner1), Combinator::OneOrMore(inner2))
        | (Combinator::Optional(inner1), Combinator::Optional(inner2))
        | (Combinator::Skip(inner1), Combinator::Skip(inner2))
        | (Combinator::Capture(inner1), Combinator::Capture(inner2))
        | (Combinator::NotFollowedBy(inner1), Combinator::NotFollowedBy(inner2))
        | (Combinator::FollowedBy(inner1), Combinator::FollowedBy(inner2)) => {
            combinators_equal(inner1, inner2)
        }

        // SeparatedBy
        (
            Combinator::SeparatedBy {
                item: i1,
                separator: s1,
                trailing: t1,
            },
            Combinator::SeparatedBy {
                item: i2,
                separator: s2,
                trailing: t2,
            },
        ) => t1 == t2 && combinators_equal(i1, i2) && combinators_equal(s1, s2),

        // Mapped
        (
            Combinator::Mapped {
                inner: i1,
                mapping: m1,
            },
            Combinator::Mapped {
                inner: i2,
                mapping: m2,
            },
        ) => m1 == m2 && combinators_equal(i1, i2),

        // Pratt - compare all parts
        (Combinator::Pratt(p1), Combinator::Pratt(p2)) => pratt_equal(p1, p2),

        // Different variants are not equal
        _ => false,
    }
}

fn pratt_equal(p1: &PrattDef, p2: &PrattDef) -> bool {
    // Compare operands
    match (p1.operand.as_ref(), p2.operand.as_ref()) {
        (Some(o1), Some(o2)) => {
            if !combinators_equal(o1, o2) {
                return false;
            }
        }
        (None, None) => {}
        _ => return false,
    }

    // Compare prefix ops
    if p1.prefix_ops.len() != p2.prefix_ops.len() {
        return false;
    }
    for (op1, op2) in p1.prefix_ops.iter().zip(&p2.prefix_ops) {
        if !prefix_op_equal(op1, op2) {
            return false;
        }
    }

    // Compare infix ops
    if p1.infix_ops.len() != p2.infix_ops.len() {
        return false;
    }
    for (op1, op2) in p1.infix_ops.iter().zip(&p2.infix_ops) {
        if !infix_op_equal(op1, op2) {
            return false;
        }
    }

    // Compare postfix ops
    if p1.postfix_ops.len() != p2.postfix_ops.len() {
        return false;
    }
    for (op1, op2) in p1.postfix_ops.iter().zip(&p2.postfix_ops) {
        if !postfix_op_equal(op1, op2) {
            return false;
        }
    }

    // Compare ternary
    match (&p1.ternary, &p2.ternary) {
        (Some(t1), Some(t2)) => ternary_op_equal(t1, t2),
        (None, None) => true,
        _ => false,
    }
}

fn prefix_op_equal(op1: &PrefixOp, op2: &PrefixOp) -> bool {
    op1.precedence == op2.precedence
        && op1.mapping == op2.mapping
        && combinators_equal(&op1.pattern, &op2.pattern)
}

fn infix_op_equal(op1: &InfixOp, op2: &InfixOp) -> bool {
    op1.precedence == op2.precedence
        && op1.assoc == op2.assoc
        && op1.mapping == op2.mapping
        && combinators_equal(&op1.pattern, &op2.pattern)
}

fn postfix_op_equal(op1: &PostfixOp, op2: &PostfixOp) -> bool {
    match (op1, op2) {
        (
            PostfixOp::Simple {
                pattern: p1,
                precedence: prec1,
                mapping: m1,
            },
            PostfixOp::Simple {
                pattern: p2,
                precedence: prec2,
                mapping: m2,
            },
        ) => prec1 == prec2 && m1 == m2 && combinators_equal(p1, p2),

        (
            PostfixOp::Call {
                open: o1,
                close: c1,
                separator: s1,
                arg_rule: ar1,
                precedence: prec1,
                mapping: m1,
            },
            PostfixOp::Call {
                open: o2,
                close: c2,
                separator: s2,
                arg_rule: ar2,
                precedence: prec2,
                mapping: m2,
            },
        ) => {
            prec1 == prec2
                && m1 == m2
                && ar1 == ar2
                && combinators_equal(o1, o2)
                && combinators_equal(c1, c2)
                && combinators_equal(s1, s2)
        }

        (
            PostfixOp::Index {
                open: o1,
                close: c1,
                precedence: prec1,
                mapping: m1,
            },
            PostfixOp::Index {
                open: o2,
                close: c2,
                precedence: prec2,
                mapping: m2,
            },
        ) => prec1 == prec2 && m1 == m2 && combinators_equal(o1, o2) && combinators_equal(c1, c2),

        (
            PostfixOp::Member {
                pattern: p1,
                precedence: prec1,
                mapping: m1,
            },
            PostfixOp::Member {
                pattern: p2,
                precedence: prec2,
                mapping: m2,
            },
        ) => prec1 == prec2 && m1 == m2 && combinators_equal(p1, p2),

        _ => false,
    }
}

fn ternary_op_equal(t1: &TernaryOp, t2: &TernaryOp) -> bool {
    t1.precedence == t2.precedence
        && t1.mapping == t2.mapping
        && combinators_equal(&t1.first, &t2.first)
        && combinators_equal(&t1.second, &t2.second)
}

// ============================================================================
// Recursion Detection
// ============================================================================

/// Check if a combinator tree contains rules that can be recursive.
///
/// A rule is considered recursive if following rule references leads back
/// to a rule we've already seen.
fn contains_recursion(
    comb: &Combinator,
    rule_map: &HashMap<&str, &Combinator>,
    visited: &mut HashSet<String>,
) -> bool {
    match comb {
        Combinator::Rule(name) => {
            if visited.contains(name) {
                return true; // Found cycle = recursion
            }
            visited.insert(name.clone());
            if let Some(rule_comb) = rule_map.get(name.as_str()) {
                contains_recursion(rule_comb, rule_map, visited)
            } else {
                false
            }
        }
        Combinator::Sequence(items) | Combinator::Choice(items) => items
            .iter()
            .any(|c| contains_recursion(c, rule_map, visited)),
        Combinator::ZeroOrMore(inner)
        | Combinator::OneOrMore(inner)
        | Combinator::Optional(inner)
        | Combinator::Skip(inner)
        | Combinator::Capture(inner)
        | Combinator::NotFollowedBy(inner)
        | Combinator::FollowedBy(inner)
        | Combinator::Mapped { inner, .. }
        | Combinator::Memoize { inner, .. } => contains_recursion(inner, rule_map, visited),
        Combinator::SeparatedBy {
            item, separator, ..
        } => {
            contains_recursion(item, rule_map, visited)
                || contains_recursion(separator, rule_map, visited)
        }
        Combinator::Pratt(pratt) => {
            if let Some(ref operand) = *pratt.operand {
                if contains_recursion(operand, rule_map, visited) {
                    return true;
                }
            }
            // Check operator patterns too
            for op in &pratt.prefix_ops {
                if contains_recursion(&op.pattern, rule_map, visited) {
                    return true;
                }
            }
            for op in &pratt.infix_ops {
                if contains_recursion(&op.pattern, rule_map, visited) {
                    return true;
                }
            }
            false
        }
        // Leaf nodes
        Combinator::Literal(_)
        | Combinator::Char(_)
        | Combinator::CharClass(_)
        | Combinator::CharRange(_, _)
        | Combinator::AnyChar => false,
    }
}

// ============================================================================
// Common Prefix Detection
// ============================================================================

/// A view of a combinator as a sequence of elements.
/// Single items become 1-element sequences.
struct SequenceView<'a> {
    items: Vec<&'a Combinator>,
}

impl<'a> SequenceView<'a> {
    fn from_combinator(c: &'a Combinator) -> Self {
        match c {
            Combinator::Sequence(items) => SequenceView {
                items: items.iter().collect(),
            },
            other => SequenceView { items: vec![other] },
        }
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn get(&self, i: usize) -> Option<&'a Combinator> {
        self.items.get(i).copied()
    }
}

/// Expand a combinator by resolving rule references to their definitions.
/// Only expands one level - the top-level combinator.
fn expand_combinator<'a>(
    comb: &'a Combinator,
    rule_map: &'a HashMap<&str, &'a Combinator>,
) -> &'a Combinator {
    if let Combinator::Rule(name) = comb {
        if let Some(expanded) = rule_map.get(name.as_str()) {
            return expanded;
        }
    }
    comb
}

/// Find the longest common prefix among Choice alternatives.
///
/// Returns a `PrefixAnalysis` with:
/// - The shared prefix elements
/// - The suffixes (what remains) for each alternative
/// - The severity of the backtracking issue
///
/// This function expands rule references to find hidden shared prefixes.
/// For example, if `dotted_list` and `proper_list` both start with `'(' datum+`,
/// this will detect that shared prefix.
pub fn find_common_prefix(
    alternatives: &[Combinator],
    rule_map: &HashMap<&str, &Combinator>,
) -> PrefixAnalysis {
    if alternatives.len() < 2 {
        return PrefixAnalysis {
            prefix: vec![],
            suffixes: alternatives
                .iter()
                .map(|c| Suffix::Single(c.clone()))
                .collect(),
            severity: BacktrackingSeverity::None,
        };
    }

    // Expand rule references to get their actual content
    let expanded: Vec<&Combinator> = alternatives
        .iter()
        .map(|c| expand_combinator(c, rule_map))
        .collect();

    // Normalize to sequence views
    let views: Vec<SequenceView> = expanded
        .iter()
        .map(|c| SequenceView::from_combinator(c))
        .collect();

    // Find the minimum length across all alternatives
    let min_len = views.iter().map(|v| v.len()).min().unwrap_or(0);

    // Find LCP length by comparing element-by-element
    let mut prefix_len = 0;
    for i in 0..min_len {
        let first = match views[0].get(i) {
            Some(c) => c,
            None => break,
        };

        let all_equal = views.iter().skip(1).all(|v| {
            v.get(i)
                .map(|c| combinators_equal(first, c))
                .unwrap_or(false)
        });

        if all_equal {
            prefix_len = i + 1;
        } else {
            break;
        }
    }

    if prefix_len == 0 {
        return PrefixAnalysis {
            prefix: vec![],
            suffixes: alternatives
                .iter()
                .map(|c| Suffix::Single(c.clone()))
                .collect(),
            severity: BacktrackingSeverity::None,
        };
    }

    // Extract prefix
    let prefix: Vec<Combinator> = (0..prefix_len)
        .filter_map(|i| views[0].get(i).cloned())
        .collect();

    // Extract suffixes
    let suffixes: Vec<Suffix> = views
        .iter()
        .map(|v| {
            let remaining: Vec<Combinator> = (prefix_len..v.len())
                .filter_map(|i| v.get(i).cloned())
                .collect();
            match remaining.len() {
                0 => Suffix::Empty,
                1 => Suffix::Single(remaining.into_iter().next().unwrap_or_else(|| {
                    // This shouldn't happen due to the length check
                    Combinator::Literal(String::new())
                })),
                _ => Suffix::Sequence(remaining),
            }
        })
        .collect();

    // Check if prefix consumes input (is non-nullable)
    let prefix_as_seq = Combinator::Sequence(prefix.clone());
    let mut visited = HashSet::new();
    let prefix_is_nullable = validation::is_nullable(&prefix_as_seq, rule_map, &mut visited);

    // Check if prefix contains recursive rules
    let mut recursion_visited = HashSet::new();
    let prefix_has_recursion = prefix
        .iter()
        .any(|c| contains_recursion(c, rule_map, &mut recursion_visited));

    // Determine severity
    let severity = if prefix_is_nullable {
        // Nullable prefix doesn't cause backtracking issues
        BacktrackingSeverity::None
    } else if prefix_has_recursion {
        BacktrackingSeverity::Exponential
    } else {
        BacktrackingSeverity::Linear
    };

    PrefixAnalysis {
        prefix,
        suffixes,
        severity,
    }
}

// ============================================================================
// Transformation
// ============================================================================

/// Transform a Choice with common prefix into factored form.
///
/// Before: `Choice([Seq([A, B, C]), Seq([A, B, D])])`
/// After:  `Seq([A, B, Choice([C, D])])`
///
/// If one suffix is empty, wraps the tail choice in `Optional`.
pub fn factor_common_prefix(analysis: &PrefixAnalysis) -> Option<Combinator> {
    if analysis.severity == BacktrackingSeverity::None || analysis.prefix.is_empty() {
        return None; // Nothing to factor
    }

    let prefix = &analysis.prefix;
    let suffixes = &analysis.suffixes;

    // Build the suffix alternatives (non-empty ones)
    let suffix_alternatives: Vec<Combinator> = suffixes
        .iter()
        .filter_map(|s| match s {
            Suffix::Empty => None,
            Suffix::Single(c) => Some(c.clone()),
            Suffix::Sequence(items) => Some(Combinator::Sequence(items.clone())),
        })
        .collect();

    let has_empty_suffix = suffixes.iter().any(|s| matches!(s, Suffix::Empty));

    // Build the tail (what comes after the prefix)
    let tail = if suffix_alternatives.is_empty() {
        // All suffixes were empty - just return the prefix
        None
    } else if suffix_alternatives.len() == 1 && has_empty_suffix {
        // One real suffix + empty = optional(suffix)
        Some(Combinator::Optional(Box::new(
            suffix_alternatives.into_iter().next()?,
        )))
    } else if has_empty_suffix {
        // Multiple suffixes + empty = optional(choice(suffixes))
        Some(Combinator::Optional(Box::new(Combinator::Choice(
            suffix_alternatives,
        ))))
    } else {
        // No empty suffix = choice(suffixes)
        Some(Combinator::Choice(suffix_alternatives))
    };

    // Combine prefix and tail
    let mut result_items = prefix.clone();
    if let Some(tail) = tail {
        result_items.push(tail);
    }

    // Return as sequence, or single item if only one element
    Some(if result_items.len() == 1 {
        result_items.into_iter().next()?
    } else {
        Combinator::Sequence(result_items)
    })
}

// ============================================================================
// Grammar Optimization
// ============================================================================

/// Recursively optimize all Choice nodes in a combinator tree.
///
/// Only transforms choices with `Exponential` severity.
pub fn optimize_combinator(comb: &Combinator, rule_map: &HashMap<&str, &Combinator>) -> Combinator {
    match comb {
        Combinator::Choice(alternatives) => {
            // First optimize children
            let optimized_alts: Vec<Combinator> = alternatives
                .iter()
                .map(|c| optimize_combinator(c, rule_map))
                .collect();

            // Then check for common prefix
            let analysis = find_common_prefix(&optimized_alts, rule_map);

            if analysis.severity == BacktrackingSeverity::Exponential {
                if let Some(factored) = factor_common_prefix(&analysis) {
                    return factored;
                }
            }

            Combinator::Choice(optimized_alts)
        }
        Combinator::Sequence(items) => Combinator::Sequence(
            items
                .iter()
                .map(|c| optimize_combinator(c, rule_map))
                .collect(),
        ),
        Combinator::ZeroOrMore(inner) => {
            Combinator::ZeroOrMore(Box::new(optimize_combinator(inner, rule_map)))
        }
        Combinator::OneOrMore(inner) => {
            Combinator::OneOrMore(Box::new(optimize_combinator(inner, rule_map)))
        }
        Combinator::Optional(inner) => {
            Combinator::Optional(Box::new(optimize_combinator(inner, rule_map)))
        }
        Combinator::Skip(inner) => Combinator::Skip(Box::new(optimize_combinator(inner, rule_map))),
        Combinator::Capture(inner) => {
            Combinator::Capture(Box::new(optimize_combinator(inner, rule_map)))
        }
        Combinator::NotFollowedBy(inner) => {
            Combinator::NotFollowedBy(Box::new(optimize_combinator(inner, rule_map)))
        }
        Combinator::FollowedBy(inner) => {
            Combinator::FollowedBy(Box::new(optimize_combinator(inner, rule_map)))
        }
        Combinator::Mapped { inner, mapping } => Combinator::Mapped {
            inner: Box::new(optimize_combinator(inner, rule_map)),
            mapping: mapping.clone(),
        },
        Combinator::Memoize { inner, id } => Combinator::Memoize {
            inner: Box::new(optimize_combinator(inner, rule_map)),
            id: *id,
        },
        Combinator::SeparatedBy {
            item,
            separator,
            trailing,
        } => Combinator::SeparatedBy {
            item: Box::new(optimize_combinator(item, rule_map)),
            separator: Box::new(optimize_combinator(separator, rule_map)),
            trailing: *trailing,
        },
        Combinator::Pratt(pratt) => {
            // Optimize operand if present
            let optimized_operand = pratt
                .operand
                .as_ref()
                .as_ref()
                .map(|o| optimize_combinator(o, rule_map));
            Combinator::Pratt(PrattDef {
                operand: Box::new(optimized_operand),
                prefix_ops: pratt.prefix_ops.clone(),
                infix_ops: pratt.infix_ops.clone(),
                postfix_ops: pratt.postfix_ops.clone(),
                ternary: pratt.ternary.clone(),
            })
        }
        // Leaf nodes - return unchanged
        Combinator::Rule(_)
        | Combinator::Literal(_)
        | Combinator::Char(_)
        | Combinator::CharClass(_)
        | Combinator::CharRange(_, _)
        | Combinator::AnyChar => comb.clone(),
    }
}

// ============================================================================
// Grammar Analysis
// ============================================================================

/// Analyze a grammar for backtracking issues.
///
/// Returns warnings for each problematic Choice node found.
pub fn analyze_grammar(rules: &[RuleDef]) -> Vec<BacktrackingWarning> {
    let rule_map: HashMap<&str, &Combinator> = rules
        .iter()
        .map(|r| (r.name.as_str(), &r.combinator))
        .collect();

    let mut warnings = Vec::new();

    for rule in rules {
        analyze_combinator_for_backtracking(&rule.name, &rule.combinator, &rule_map, &mut warnings);
    }

    warnings
}

fn analyze_combinator_for_backtracking(
    rule_name: &str,
    comb: &Combinator,
    rule_map: &HashMap<&str, &Combinator>,
    warnings: &mut Vec<BacktrackingWarning>,
) {
    match comb {
        Combinator::Choice(alternatives) => {
            let analysis = find_common_prefix(alternatives, rule_map);

            if analysis.severity == BacktrackingSeverity::Exponential {
                warnings.push(BacktrackingWarning {
                    rule_name: rule_name.to_string(),
                    description: format!(
                        "Choice with {} alternatives shares a prefix of {} elements containing recursive rules. \
                         This causes O(2^n) parsing time. Consider factoring out the common prefix.",
                        alternatives.len(),
                        analysis.prefix.len()
                    ),
                    severity: analysis.severity,
                });
            }

            // Recurse into alternatives
            for alt in alternatives {
                analyze_combinator_for_backtracking(rule_name, alt, rule_map, warnings);
            }
        }
        Combinator::Sequence(items) => {
            for item in items {
                analyze_combinator_for_backtracking(rule_name, item, rule_map, warnings);
            }
        }
        Combinator::ZeroOrMore(inner)
        | Combinator::OneOrMore(inner)
        | Combinator::Optional(inner)
        | Combinator::Skip(inner)
        | Combinator::Capture(inner)
        | Combinator::NotFollowedBy(inner)
        | Combinator::FollowedBy(inner)
        | Combinator::Mapped { inner, .. }
        | Combinator::Memoize { inner, .. } => {
            analyze_combinator_for_backtracking(rule_name, inner, rule_map, warnings);
        }
        Combinator::SeparatedBy {
            item, separator, ..
        } => {
            analyze_combinator_for_backtracking(rule_name, item, rule_map, warnings);
            analyze_combinator_for_backtracking(rule_name, separator, rule_map, warnings);
        }
        Combinator::Pratt(pratt) => {
            if let Some(ref operand) = *pratt.operand {
                analyze_combinator_for_backtracking(rule_name, operand, rule_map, warnings);
            }
        }
        // Leaf nodes - nothing to analyze
        Combinator::Rule(_)
        | Combinator::Literal(_)
        | Combinator::Char(_)
        | Combinator::CharClass(_)
        | Combinator::CharRange(_, _)
        | Combinator::AnyChar => {}
    }
}

// ============================================================================
// Memoization Candidate Detection
// ============================================================================

/// Identify rules that should be memoized to avoid exponential backtracking.
///
/// This function analyzes the grammar to find rules that:
/// 1. Appear at the start of Choice alternatives that share a common prefix
/// 2. Contain recursion (either directly or through rule references)
/// 3. Would cause exponential backtracking without memoization
///
/// Returns a set of rule names that should be wrapped with `.memoize()`.
pub fn identify_memoization_candidates(rules: &[RuleDef]) -> HashSet<String> {
    let rule_map: HashMap<&str, &Combinator> = rules
        .iter()
        .map(|r| (r.name.as_str(), &r.combinator))
        .collect();

    let mut candidates = HashSet::new();

    for rule in rules {
        find_memoization_candidates_in_combinator(&rule.combinator, &rule_map, &mut candidates);
    }

    candidates
}

/// Recursively search a combinator for memoization candidates.
fn find_memoization_candidates_in_combinator(
    comb: &Combinator,
    rule_map: &HashMap<&str, &Combinator>,
    candidates: &mut HashSet<String>,
) {
    match comb {
        Combinator::Choice(alternatives) => {
            // Check if this choice has exponential backtracking potential
            let analysis = find_common_prefix(alternatives, rule_map);

            if analysis.severity == BacktrackingSeverity::Exponential {
                // Find the first rule reference in the common prefix
                // That rule (and rules it calls) are memoization candidates
                for prefix_elem in &analysis.prefix {
                    collect_rule_references(prefix_elem, candidates);
                }
            }

            // Also check for patterns where one alternative starts with another
            // E.g., Choice([generic_call, identifier]) where generic_call starts with identifier
            find_overlapping_rule_starts(alternatives, rule_map, candidates);

            // Recurse into alternatives
            for alt in alternatives {
                find_memoization_candidates_in_combinator(alt, rule_map, candidates);
            }
        }
        Combinator::Sequence(items) => {
            for item in items {
                find_memoization_candidates_in_combinator(item, rule_map, candidates);
            }
        }
        Combinator::ZeroOrMore(inner)
        | Combinator::OneOrMore(inner)
        | Combinator::Optional(inner)
        | Combinator::Skip(inner)
        | Combinator::Capture(inner)
        | Combinator::NotFollowedBy(inner)
        | Combinator::FollowedBy(inner)
        | Combinator::Mapped { inner, .. }
        | Combinator::Memoize { inner, .. } => {
            find_memoization_candidates_in_combinator(inner, rule_map, candidates);
        }
        Combinator::SeparatedBy {
            item, separator, ..
        } => {
            find_memoization_candidates_in_combinator(item, rule_map, candidates);
            find_memoization_candidates_in_combinator(separator, rule_map, candidates);
        }
        Combinator::Pratt(pratt) => {
            if let Some(ref operand) = *pratt.operand {
                find_memoization_candidates_in_combinator(operand, rule_map, candidates);
            }
        }
        // Leaf nodes - nothing to recurse into
        Combinator::Rule(_)
        | Combinator::Literal(_)
        | Combinator::Char(_)
        | Combinator::CharClass(_)
        | Combinator::CharRange(_, _)
        | Combinator::AnyChar => {}
    }
}

/// Find cases where one alternative starts with a rule that another alternative would match.
///
/// E.g., Choice([generic_call, identifier]) where generic_call starts with identifier.
/// In this case, generic_call should be memoized because after matching identifier
/// and failing to complete generic_call, we'll backtrack and try identifier again.
fn find_overlapping_rule_starts(
    alternatives: &[Combinator],
    rule_map: &HashMap<&str, &Combinator>,
    candidates: &mut HashSet<String>,
) {
    // Collect the first rule reference from each alternative
    let mut first_rules: Vec<(usize, &str)> = Vec::new();
    for (idx, alt) in alternatives.iter().enumerate() {
        if let Some(first_rule) = get_first_rule(alt, rule_map) {
            first_rules.push((idx, first_rule));
        }
    }

    // For each alternative that's a rule reference, check if its expansion
    // starts with any other alternative's first rule
    for alt in alternatives {
        if let Combinator::Rule(rule_name) = alt {
            if let Some(rule_def) = rule_map.get(rule_name.as_str()) {
                // Get what this rule starts with
                if let Some(starts_with) = get_first_rule(rule_def, rule_map) {
                    // Check if any other alternative is this same rule
                    for (_, other_first) in &first_rules {
                        if *other_first == starts_with && *other_first != rule_name.as_str() {
                            // This rule starts with something another alternative matches
                            // Mark this rule as a memoization candidate
                            candidates.insert(rule_name.clone());
                        }
                    }
                }
            }
        }
    }
}

/// Get the first rule reference at the start of a combinator.
fn get_first_rule<'a>(
    comb: &'a Combinator,
    _rule_map: &HashMap<&str, &'a Combinator>,
) -> Option<&'a str> {
    match comb {
        Combinator::Rule(name) => Some(name.as_str()),
        Combinator::Sequence(items) if !items.is_empty() => get_first_rule(&items[0], _rule_map),
        Combinator::Optional(inner) => get_first_rule(inner, _rule_map),
        Combinator::Skip(inner) => get_first_rule(inner, _rule_map),
        Combinator::Capture(inner) => get_first_rule(inner, _rule_map),
        Combinator::Mapped { inner, .. } => get_first_rule(inner, _rule_map),
        Combinator::Memoize { inner, .. } => get_first_rule(inner, _rule_map),
        _ => None,
    }
}

/// Collect all rule references from a combinator.
fn collect_rule_references(comb: &Combinator, rules: &mut HashSet<String>) {
    match comb {
        Combinator::Rule(name) => {
            rules.insert(name.clone());
        }
        Combinator::Sequence(items) | Combinator::Choice(items) => {
            for item in items {
                collect_rule_references(item, rules);
            }
        }
        Combinator::ZeroOrMore(inner)
        | Combinator::OneOrMore(inner)
        | Combinator::Optional(inner)
        | Combinator::Skip(inner)
        | Combinator::Capture(inner)
        | Combinator::NotFollowedBy(inner)
        | Combinator::FollowedBy(inner)
        | Combinator::Mapped { inner, .. }
        | Combinator::Memoize { inner, .. } => {
            collect_rule_references(inner, rules);
        }
        Combinator::SeparatedBy {
            item, separator, ..
        } => {
            collect_rule_references(item, rules);
            collect_rule_references(separator, rules);
        }
        Combinator::Pratt(pratt) => {
            if let Some(ref operand) = *pratt.operand {
                collect_rule_references(operand, rules);
            }
            for op in &pratt.prefix_ops {
                collect_rule_references(&op.pattern, rules);
            }
            for op in &pratt.infix_ops {
                collect_rule_references(&op.pattern, rules);
            }
        }
        // Leaf nodes
        Combinator::Literal(_)
        | Combinator::Char(_)
        | Combinator::CharClass(_)
        | Combinator::CharRange(_, _)
        | Combinator::AnyChar => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combinators_equal_literals() {
        assert!(combinators_equal(
            &Combinator::Literal("foo".to_string()),
            &Combinator::Literal("foo".to_string())
        ));
        assert!(!combinators_equal(
            &Combinator::Literal("foo".to_string()),
            &Combinator::Literal("bar".to_string())
        ));
    }

    #[test]
    fn test_combinators_equal_sequences() {
        let seq1 = Combinator::Sequence(vec![
            Combinator::Char('('),
            Combinator::Rule("datum".to_string()),
        ]);
        let seq2 = Combinator::Sequence(vec![
            Combinator::Char('('),
            Combinator::Rule("datum".to_string()),
        ]);
        let seq3 = Combinator::Sequence(vec![
            Combinator::Char('('),
            Combinator::Rule("other".to_string()),
        ]);

        assert!(combinators_equal(&seq1, &seq2));
        assert!(!combinators_equal(&seq1, &seq3));
    }

    #[test]
    fn test_find_common_prefix_simple() {
        let rule_map = HashMap::new();

        let alternatives = vec![
            Combinator::Sequence(vec![
                Combinator::Char('('),
                Combinator::Char('a'),
                Combinator::Char('.'),
            ]),
            Combinator::Sequence(vec![
                Combinator::Char('('),
                Combinator::Char('a'),
                Combinator::Char(')'),
            ]),
        ];

        let analysis = find_common_prefix(&alternatives, &rule_map);

        assert_eq!(analysis.prefix.len(), 2);
        assert!(combinators_equal(
            &analysis.prefix[0],
            &Combinator::Char('(')
        ));
        assert!(combinators_equal(
            &analysis.prefix[1],
            &Combinator::Char('a')
        ));
        assert_eq!(analysis.suffixes.len(), 2);
    }

    #[test]
    fn test_factor_common_prefix() {
        let analysis = PrefixAnalysis {
            prefix: vec![Combinator::Char('('), Combinator::Char('a')],
            suffixes: vec![
                Suffix::Single(Combinator::Char('.')),
                Suffix::Single(Combinator::Char(')')),
            ],
            severity: BacktrackingSeverity::Exponential,
        };

        let factored = factor_common_prefix(&analysis).unwrap();

        // Should be: Sequence(['(', 'a', Choice(['.', ')'])])
        if let Combinator::Sequence(items) = factored {
            assert_eq!(items.len(), 3);
            assert!(combinators_equal(&items[0], &Combinator::Char('(')));
            assert!(combinators_equal(&items[1], &Combinator::Char('a')));
            if let Combinator::Choice(alts) = &items[2] {
                assert_eq!(alts.len(), 2);
            } else {
                panic!("Expected Choice");
            }
        } else {
            panic!("Expected Sequence");
        }
    }

    #[test]
    fn test_factor_with_empty_suffix() {
        let analysis = PrefixAnalysis {
            prefix: vec![Combinator::Char('('), Combinator::Char('a')],
            suffixes: vec![
                Suffix::Single(Combinator::Char('.')),
                Suffix::Empty, // One alternative is just the prefix
            ],
            severity: BacktrackingSeverity::Exponential,
        };

        let factored = factor_common_prefix(&analysis).unwrap();

        // Should be: Sequence(['(', 'a', Optional('.')])
        if let Combinator::Sequence(items) = factored {
            assert_eq!(items.len(), 3);
            if let Combinator::Optional(_) = &items[2] {
                // Good - wrapped in Optional
            } else {
                panic!("Expected Optional for empty suffix case");
            }
        } else {
            panic!("Expected Sequence");
        }
    }

    #[test]
    fn test_identify_memoization_candidates() {
        // Create a grammar with potential exponential backtracking
        // Similar to the generic_call vs identifier pattern
        let rules = vec![
            RuleDef {
                name: "primary_inner".to_string(),
                combinator: Combinator::Choice(vec![
                    Combinator::Rule("generic_call".to_string()),
                    Combinator::Rule("identifier".to_string()),
                ]),
            },
            RuleDef {
                name: "generic_call".to_string(),
                combinator: Combinator::Sequence(vec![
                    Combinator::Rule("identifier".to_string()),
                    Combinator::Rule("type_arguments".to_string()),
                    Combinator::Literal("(".to_string()),
                    Combinator::Literal(")".to_string()),
                ]),
            },
            RuleDef {
                name: "type_arguments".to_string(),
                combinator: Combinator::Sequence(vec![
                    Combinator::Literal("<".to_string()),
                    Combinator::Rule("type".to_string()),
                    Combinator::Literal(">".to_string()),
                ]),
            },
            RuleDef {
                name: "type".to_string(),
                combinator: Combinator::Choice(vec![
                    Combinator::Rule("type_reference".to_string()),
                    Combinator::Rule("identifier".to_string()),
                ]),
            },
            RuleDef {
                name: "type_reference".to_string(),
                combinator: Combinator::Sequence(vec![
                    Combinator::Rule("identifier".to_string()),
                    Combinator::Optional(Box::new(Combinator::Rule("type_arguments".to_string()))),
                ]),
            },
            RuleDef {
                name: "identifier".to_string(),
                combinator: Combinator::Capture(Box::new(Combinator::OneOrMore(Box::new(
                    Combinator::CharClass(crate::ir::CharClass::Alpha),
                )))),
            },
        ];

        let candidates = identify_memoization_candidates(&rules);

        // The choice in primary_inner has generic_call and identifier as alternatives.
        // generic_call contains identifier as its first element, so there's a shared prefix.
        // generic_call should be identified as a memoization candidate.
        assert!(
            candidates.contains("generic_call"),
            "Should identify generic_call as memoization candidate, got: {:?}",
            candidates
        );
    }
}
