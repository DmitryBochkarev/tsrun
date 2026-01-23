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
            ValidationError::NullableLoop { rule_name, description } => {
                write!(f, "Nullable loop in rule '{}': {}", rule_name, description)
            }
            ValidationError::LeftRecursion { rule_name, path } => {
                write!(f, "Left recursion in rule '{}': {}", rule_name, path.join(" -> "))
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
fn is_nullable(comb: &Combinator, rule_map: &HashMap<&str, &Combinator>, visited: &mut HashSet<String>) -> bool {
    match comb {
        Combinator::Token(_) => false, // Tokens always consume input
        Combinator::Rule(name) => {
            // Prevent infinite recursion
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
        Combinator::Sequence(items) => {
            // Sequence is nullable if ALL items are nullable
            items.iter().all(|item| is_nullable(item, rule_map, visited))
        }
        Combinator::Choice(items) => {
            // Choice is nullable if ANY item is nullable
            items.iter().any(|item| is_nullable(item, rule_map, visited))
        }
        Combinator::ZeroOrMore(_) => true, // Can always match empty
        Combinator::OneOrMore(inner) => is_nullable(inner, rule_map, visited),
        Combinator::Optional(_) => true, // Can always match empty
        Combinator::Skip(inner) => is_nullable(inner, rule_map, visited),
        Combinator::SeparatedBy { .. } => {
            // SeparatedBy requires at least one item (not nullable)
            false
        }
        Combinator::Pratt(pratt) => {
            // Pratt is nullable if operand is nullable
            if let Some(ref operand) = *pratt.operand {
                is_nullable(operand, rule_map, visited)
            } else {
                false
            }
        }
        Combinator::Mapped { inner, .. } => is_nullable(inner, rule_map, visited),
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
            // Recursively check inner
            check_nullable_loops(rule_name, inner, rule_map, errors);
        }
        Combinator::SeparatedBy { item, separator, .. } => {
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
            // Recursively check
            check_nullable_loops(rule_name, item, rule_map, errors);
            check_nullable_loops(rule_name, separator, rule_map, errors);
        }
        Combinator::Sequence(items) => {
            for item in items {
                check_nullable_loops(rule_name, item, rule_map, errors);
            }
        }
        Combinator::Choice(items) => {
            for item in items {
                check_nullable_loops(rule_name, item, rule_map, errors);
            }
        }
        Combinator::Optional(inner) | Combinator::Skip(inner) => {
            check_nullable_loops(rule_name, inner, rule_map, errors);
        }
        Combinator::Pratt(pratt) => {
            if let Some(ref operand) = *pratt.operand {
                check_nullable_loops(rule_name, operand, rule_map, errors);
            }
        }
        Combinator::Mapped { inner, .. } => {
            check_nullable_loops(rule_name, inner, rule_map, errors);
        }
        Combinator::Token(_) | Combinator::Rule(_) => {
            // No nested loops to check
        }
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
                return true; // Found left recursion
            }
            if visited.contains(name) {
                return false; // Already checked this rule
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
            // Check items until we find one that must consume input
            for item in items {
                if has_left_recursion(target_rule, item, rule_map, path, visited) {
                    return true;
                }
                // If this item must consume input, stop checking
                let mut null_visited = HashSet::new();
                if !is_nullable(item, rule_map, &mut null_visited) {
                    break;
                }
            }
            false
        }
        Combinator::Choice(items) => {
            // Check all alternatives (any could cause left recursion)
            for item in items {
                if has_left_recursion(target_rule, item, rule_map, path, visited) {
                    return true;
                }
            }
            false
        }
        Combinator::Optional(inner) | Combinator::ZeroOrMore(inner) => {
            // These are nullable, so check inner
            has_left_recursion(target_rule, inner, rule_map, path, visited)
        }
        Combinator::OneOrMore(inner) => {
            has_left_recursion(target_rule, inner, rule_map, path, visited)
        }
        Combinator::Skip(inner) | Combinator::Mapped { inner, .. } => {
            has_left_recursion(target_rule, inner, rule_map, path, visited)
        }
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
        Combinator::Token(_) => false, // Tokens consume input, stop here
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Combinator;

    #[test]
    fn test_nullable_token() {
        let rule_map = HashMap::new();
        let mut visited = HashSet::new();
        assert!(!is_nullable(&Combinator::Token("FOO".to_string()), &rule_map, &mut visited));
    }

    #[test]
    fn test_nullable_optional() {
        let rule_map = HashMap::new();
        let mut visited = HashSet::new();
        let comb = Combinator::Optional(Box::new(Combinator::Token("FOO".to_string())));
        assert!(is_nullable(&comb, &rule_map, &mut visited));
    }

    #[test]
    fn test_nullable_zero_or_more() {
        let rule_map = HashMap::new();
        let mut visited = HashSet::new();
        let comb = Combinator::ZeroOrMore(Box::new(Combinator::Token("FOO".to_string())));
        assert!(is_nullable(&comb, &rule_map, &mut visited));
    }
}
