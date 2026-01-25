//! Grammar validation to detect problematic patterns
//!
//! This module provides compile-time validation to detect:
//! - Nullable loops (loops where inner can match empty input - causes infinite loops)
//! - Direct left recursion (rules that call themselves without consuming input)

use crate::ir::{Combinator, RuleDef};
use std::collections::{HashMap, HashSet};

/// Validation errors for grammar
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// A loop combinator (ZeroOrMore, OneOrMore, SeparatedBy) has a nullable inner
    NullableLoop {
        rule_name: String,
        description: String,
    },
    /// A rule has direct left recursion
    LeftRecursion {
        rule_name: String,
        path: Vec<String>,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::NullableLoop {
                rule_name,
                description,
            } => {
                write!(f, "Nullable loop in rule '{}': {}", rule_name, description)
            }
            ValidationError::LeftRecursion { rule_name, path } => {
                write!(
                    f,
                    "Left recursion in rule '{}': {}",
                    rule_name,
                    path.join(" -> ")
                )
            }
        }
    }
}

/// Validate a grammar and return any errors found
pub fn validate_grammar(rules: &[RuleDef]) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Build a map of rule names to their combinators for lookup
    let rule_map: HashMap<&str, &Combinator> = rules
        .iter()
        .map(|r| (r.name.as_str(), &r.combinator))
        .collect();

    // Check each rule
    for rule in rules {
        // Check for nullable loops
        check_nullable_loops(&rule.name, &rule.combinator, &rule_map, &mut errors);

        // Check for left recursion
        check_left_recursion(&rule.name, &rule.combinator, &rule_map, &mut errors);
    }

    errors
}

/// Check if a combinator can match empty input (is nullable)
pub fn is_nullable(
    comb: &Combinator,
    rule_map: &HashMap<&str, &Combinator>,
    visited: &mut HashSet<String>,
) -> bool {
    match comb {
        // Character-level primitives - never nullable (always consume input)
        Combinator::Literal(s) => s.is_empty(), // Empty string is nullable
        Combinator::Char(_) => false,
        Combinator::CharClass(_) => false,
        Combinator::CharRange(_, _) => false,
        Combinator::AnyChar => false,

        // Lookahead - doesn't consume input
        Combinator::NotFollowedBy(_) => true,
        Combinator::FollowedBy(_) => true,

        // Capture - nullable if inner is nullable
        Combinator::Capture(inner) => is_nullable(inner, rule_map, visited),

        // Rule reference
        Combinator::Rule(name) => {
            if visited.contains(name) {
                return false; // Assume not nullable for recursive rules
            }
            visited.insert(name.clone());
            if let Some(rule_comb) = rule_map.get(name.as_str()) {
                is_nullable(rule_comb, rule_map, visited)
            } else {
                false
            }
        }

        // Combinators
        Combinator::Sequence(items) => items
            .iter()
            .all(|item| is_nullable(item, rule_map, visited)),
        Combinator::Choice(items) => items
            .iter()
            .any(|item| is_nullable(item, rule_map, visited)),
        Combinator::ZeroOrMore(_) => true,
        Combinator::OneOrMore(inner) => is_nullable(inner, rule_map, visited),
        Combinator::Optional(_) => true,
        Combinator::Skip(inner) => is_nullable(inner, rule_map, visited),
        Combinator::SeparatedBy { .. } => false, // Requires at least one item
        Combinator::Pratt(pratt) => {
            if let Some(ref operand) = *pratt.operand {
                is_nullable(operand, rule_map, visited)
            } else {
                false
            }
        }
        Combinator::Mapped { inner, .. } | Combinator::Memoize { inner, .. } => {
            is_nullable(inner, rule_map, visited)
        }
    }
}

/// Check for nullable loops in a combinator
fn check_nullable_loops(
    rule_name: &str,
    comb: &Combinator,
    rule_map: &HashMap<&str, &Combinator>,
    errors: &mut Vec<ValidationError>,
) {
    match comb {
        Combinator::ZeroOrMore(inner) | Combinator::OneOrMore(inner) => {
            let mut visited = HashSet::new();
            if is_nullable(inner, rule_map, &mut visited) {
                errors.push(ValidationError::NullableLoop {
                    rule_name: rule_name.to_string(),
                    description: "Loop body can match empty input".to_string(),
                });
            }
            check_nullable_loops(rule_name, inner, rule_map, errors);
        }
        Combinator::SeparatedBy {
            item, separator, ..
        } => {
            let mut visited = HashSet::new();
            if is_nullable(item, rule_map, &mut visited) {
                errors.push(ValidationError::NullableLoop {
                    rule_name: rule_name.to_string(),
                    description: "SeparatedBy item can match empty input".to_string(),
                });
            }
            visited.clear();
            if is_nullable(separator, rule_map, &mut visited) {
                errors.push(ValidationError::NullableLoop {
                    rule_name: rule_name.to_string(),
                    description: "SeparatedBy separator can match empty input".to_string(),
                });
            }
            check_nullable_loops(rule_name, item, rule_map, errors);
            check_nullable_loops(rule_name, separator, rule_map, errors);
        }
        Combinator::Sequence(items) | Combinator::Choice(items) => {
            for item in items {
                check_nullable_loops(rule_name, item, rule_map, errors);
            }
        }
        Combinator::Optional(inner)
        | Combinator::Skip(inner)
        | Combinator::Capture(inner)
        | Combinator::NotFollowedBy(inner)
        | Combinator::FollowedBy(inner) => {
            check_nullable_loops(rule_name, inner, rule_map, errors);
        }
        Combinator::Pratt(pratt) => {
            if let Some(ref operand) = *pratt.operand {
                check_nullable_loops(rule_name, operand, rule_map, errors);
            }
        }
        Combinator::Mapped { inner, .. } | Combinator::Memoize { inner, .. } => {
            check_nullable_loops(rule_name, inner, rule_map, errors);
        }
        // Leaf combinators - no nested loops
        Combinator::Rule(_)
        | Combinator::Literal(_)
        | Combinator::Char(_)
        | Combinator::CharClass(_)
        | Combinator::CharRange(_, _)
        | Combinator::AnyChar => {}
    }
}

/// Check for left recursion starting from a combinator
fn check_left_recursion(
    rule_name: &str,
    comb: &Combinator,
    rule_map: &HashMap<&str, &Combinator>,
    errors: &mut Vec<ValidationError>,
) {
    let mut path = vec![rule_name.to_string()];
    let mut visited = HashSet::new();
    visited.insert(rule_name.to_string());

    if has_left_recursion(rule_name, comb, rule_map, &mut path, &mut visited) {
        errors.push(ValidationError::LeftRecursion {
            rule_name: rule_name.to_string(),
            path,
        });
    }
}

/// Check if a combinator can reach the given rule name without consuming input
fn has_left_recursion(
    target_rule: &str,
    comb: &Combinator,
    rule_map: &HashMap<&str, &Combinator>,
    path: &mut Vec<String>,
    visited: &mut HashSet<String>,
) -> bool {
    match comb {
        Combinator::Rule(name) => {
            if name == target_rule {
                return true;
            }
            if visited.contains(name) {
                return false;
            }
            visited.insert(name.clone());
            path.push(name.clone());

            if let Some(rule_comb) = rule_map.get(name.as_str()) {
                let result = has_left_recursion(target_rule, rule_comb, rule_map, path, visited);
                if !result {
                    path.pop();
                }
                result
            } else {
                path.pop();
                false
            }
        }
        Combinator::Sequence(items) => {
            for item in items {
                if has_left_recursion(target_rule, item, rule_map, path, visited) {
                    return true;
                }
                let mut null_visited = HashSet::new();
                if !is_nullable(item, rule_map, &mut null_visited) {
                    break;
                }
            }
            false
        }
        Combinator::Choice(items) => {
            for item in items {
                if has_left_recursion(target_rule, item, rule_map, path, visited) {
                    return true;
                }
            }
            false
        }
        Combinator::Optional(inner) | Combinator::ZeroOrMore(inner) => {
            has_left_recursion(target_rule, inner, rule_map, path, visited)
        }
        Combinator::OneOrMore(inner)
        | Combinator::Skip(inner)
        | Combinator::Capture(inner)
        | Combinator::Mapped { inner, .. }
        | Combinator::Memoize { inner, .. } => {
            has_left_recursion(target_rule, inner, rule_map, path, visited)
        }
        // Lookahead doesn't consume input but can't cause left recursion by itself
        Combinator::NotFollowedBy(_) | Combinator::FollowedBy(_) => false,
        Combinator::SeparatedBy { item, .. } => {
            has_left_recursion(target_rule, item, rule_map, path, visited)
        }
        Combinator::Pratt(pratt) => {
            if let Some(ref operand) = *pratt.operand {
                has_left_recursion(target_rule, operand, rule_map, path, visited)
            } else {
                false
            }
        }
        // Leaf combinators that consume input - stop here
        Combinator::Literal(s) if !s.is_empty() => false,
        Combinator::Literal(_) => true, // Empty literal is nullable
        Combinator::Char(_)
        | Combinator::CharClass(_)
        | Combinator::CharRange(_, _)
        | Combinator::AnyChar => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Combinator;

    #[test]
    fn test_nullable_literal() {
        let rule_map = HashMap::new();
        let mut visited = HashSet::new();
        assert!(!is_nullable(
            &Combinator::Literal("foo".to_string()),
            &rule_map,
            &mut visited
        ));

        visited.clear();
        assert!(is_nullable(
            &Combinator::Literal("".to_string()),
            &rule_map,
            &mut visited
        ));
    }

    #[test]
    fn test_nullable_optional() {
        let rule_map = HashMap::new();
        let mut visited = HashSet::new();
        let comb = Combinator::Optional(Box::new(Combinator::Literal("foo".to_string())));
        assert!(is_nullable(&comb, &rule_map, &mut visited));
    }

    #[test]
    fn test_nullable_zero_or_more() {
        let rule_map = HashMap::new();
        let mut visited = HashSet::new();
        let comb = Combinator::ZeroOrMore(Box::new(Combinator::Literal("foo".to_string())));
        assert!(is_nullable(&comb, &rule_map, &mut visited));
    }

    #[test]
    fn test_nullable_char_class() {
        let rule_map = HashMap::new();
        let mut visited = HashSet::new();
        assert!(!is_nullable(
            &Combinator::CharClass(crate::ir::CharClass::Digit),
            &rule_map,
            &mut visited
        ));
    }
}
