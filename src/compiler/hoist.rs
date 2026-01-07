//! Variable hoisting for var declarations
//!
//! JavaScript `var` declarations are hoisted to the top of their function scope.
//! This module provides functions to collect var declarations and emit them at
//! the beginning of the scope.

use super::Compiler;
use super::bytecode::Op;
use crate::ast::{ForInOfLeft, ForInit, Pattern, Statement, VariableDeclaration, VariableKind};
use crate::error::JsError;
use crate::prelude::*;
use crate::value::{CheapClone, JsString};

impl Compiler {
    /// Emit hoisted var declarations for a list of statements
    /// This should be called at the beginning of a scope (program or function body)
    pub fn emit_hoisted_declarations(&mut self, statements: &[Statement]) -> Result<(), JsError> {
        // Collect all var names that need to be hoisted
        let mut var_names: Vec<JsString> = Vec::new();
        collect_hoisted_vars(statements, &mut var_names);

        // Emit DeclareVarHoisted for each unique var (with undefined as initial value)
        let undefined_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::LoadUndefined { dst: undefined_reg });

        for name in var_names {
            // Track that this var has been hoisted
            if self.hoisted_vars.insert(name.cheap_clone()) {
                // Only emit declaration if not already hoisted
                let name_idx = self.builder.add_string(name)?;
                self.builder.emit(Op::DeclareVarHoisted {
                    name: name_idx,
                    init: undefined_reg,
                });
            }
        }

        self.builder.free_register(undefined_reg);
        Ok(())
    }

    /// Check if a variable name has been hoisted
    pub fn is_hoisted(&self, name: &JsString) -> bool {
        self.hoisted_vars.contains(name)
    }
}

/// Recursively collect all var declaration names from statements
/// This only collects top-level vars within the current scope - it doesn't
/// descend into function bodies (which create new scopes)
fn collect_hoisted_vars(statements: &[Statement], names: &mut Vec<JsString>) {
    for stmt in statements {
        collect_hoisted_vars_stmt(stmt, names);
    }
}

/// Collect hoisted vars from a single statement
fn collect_hoisted_vars_stmt(stmt: &Statement, names: &mut Vec<JsString>) {
    match stmt {
        Statement::VariableDeclaration(decl) => {
            if decl.kind == VariableKind::Var {
                collect_hoisted_vars_decl(decl, names);
            }
        }

        // Block statements - vars hoist out of blocks
        Statement::Block(block) => {
            for s in block.body.iter() {
                collect_hoisted_vars_stmt(s, names);
            }
        }

        // Control flow - vars hoist out
        Statement::If(if_stmt) => {
            collect_hoisted_vars_stmt(&if_stmt.consequent, names);
            if let Some(ref alt) = if_stmt.alternate {
                collect_hoisted_vars_stmt(alt, names);
            }
        }

        Statement::While(while_stmt) => {
            collect_hoisted_vars_stmt(&while_stmt.body, names);
        }

        Statement::DoWhile(do_while) => {
            collect_hoisted_vars_stmt(&do_while.body, names);
        }

        Statement::For(for_stmt) => {
            // var in for init hoists
            if let Some(ForInit::Variable(decl)) = &for_stmt.init
                && decl.kind == VariableKind::Var
            {
                collect_hoisted_vars_decl(decl, names);
            }
            collect_hoisted_vars_stmt(&for_stmt.body, names);
        }

        Statement::ForIn(for_in) => {
            if let ForInOfLeft::Variable(decl) = &for_in.left
                && decl.kind == VariableKind::Var
            {
                collect_hoisted_vars_decl(decl, names);
            }
            collect_hoisted_vars_stmt(&for_in.body, names);
        }

        Statement::ForOf(for_of) => {
            if let ForInOfLeft::Variable(decl) = &for_of.left
                && decl.kind == VariableKind::Var
            {
                collect_hoisted_vars_decl(decl, names);
            }
            collect_hoisted_vars_stmt(&for_of.body, names);
        }

        Statement::Switch(switch_stmt) => {
            for case in switch_stmt.cases.iter() {
                for s in case.consequent.iter() {
                    collect_hoisted_vars_stmt(s, names);
                }
            }
        }

        Statement::Try(try_stmt) => {
            for s in try_stmt.block.body.iter() {
                collect_hoisted_vars_stmt(s, names);
            }
            if let Some(ref handler) = try_stmt.handler {
                for s in handler.body.body.iter() {
                    collect_hoisted_vars_stmt(s, names);
                }
            }
            if let Some(ref finalizer) = try_stmt.finalizer {
                for s in finalizer.body.iter() {
                    collect_hoisted_vars_stmt(s, names);
                }
            }
        }

        Statement::Labeled(labeled) => {
            collect_hoisted_vars_stmt(&labeled.body, names);
        }

        // Function declarations do NOT have their body scanned for var hoisting
        // (the function body is a separate scope), but the function name itself
        // is considered hoisted (function hoisting)
        Statement::FunctionDeclaration(_) => {
            // Function declarations are handled separately - their names are hoisted
            // but we handle that via DeclareVarHoisted when compiling the function
        }

        // Class declarations don't hoist the same way
        Statement::ClassDeclaration(_) => {}

        // These don't contain nested statements with var declarations
        Statement::Expression(_)
        | Statement::Return(_)
        | Statement::Break(_)
        | Statement::Continue(_)
        | Statement::Throw(_)
        | Statement::Empty
        | Statement::Debugger
        | Statement::TypeAlias(_)
        | Statement::InterfaceDeclaration(_)
        | Statement::EnumDeclaration(_)
        | Statement::NamespaceDeclaration(_)
        | Statement::Import(_)
        | Statement::Export(_) => {}
    }
}

/// Collect var names from a variable declaration
fn collect_hoisted_vars_decl(decl: &VariableDeclaration, names: &mut Vec<JsString>) {
    for declarator in decl.declarations.iter() {
        collect_pattern_var_names(&declarator.id, names);
    }
}

/// Count the number of bindings that will be created in a function's environment.
/// This is used to pre-size the environment HashMap to avoid resizing.
///
/// Counts:
/// - Parameters (including destructured bindings)
/// - Hoisted var declarations (unique names)
/// - Hoisted function declarations
/// - `this` binding (for non-arrow functions)
pub fn count_function_bindings(
    params: &[crate::ast::FunctionParam],
    body: &[Statement],
    is_arrow: bool,
) -> usize {
    let mut seen: FxHashSet<JsString> = FxHashSet::default();
    let mut count: usize = 0;

    // Count parameter bindings
    for param in params {
        count_pattern_bindings(&param.pattern, &mut seen, &mut count);
    }

    // Count hoisted var declarations (unique names only)
    let mut var_names: Vec<JsString> = Vec::new();
    collect_hoisted_vars(body, &mut var_names);
    for name in var_names {
        if seen.insert(name) {
            count += 1;
        }
    }

    // Count hoisted function declarations
    collect_function_decl_names(body, &mut seen, &mut count);

    // Add 1 for `this` binding (non-arrow functions)
    if !is_arrow {
        count += 1;
    }

    // Add some slack for potential additional bindings (arguments object, etc.)
    count + 2
}

/// Count bindings from a pattern (for parameters and destructuring)
fn count_pattern_bindings(pattern: &Pattern, seen: &mut FxHashSet<JsString>, count: &mut usize) {
    match pattern {
        Pattern::Identifier(id) => {
            if seen.insert(id.name.cheap_clone()) {
                *count += 1;
            }
        }
        Pattern::Object(obj_pat) => {
            for prop in &obj_pat.properties {
                match prop {
                    crate::ast::ObjectPatternProperty::KeyValue { value, .. } => {
                        count_pattern_bindings(value, seen, count);
                    }
                    crate::ast::ObjectPatternProperty::Rest(rest) => {
                        count_pattern_bindings(&rest.argument, seen, count);
                    }
                }
            }
        }
        Pattern::Array(arr_pat) => {
            for p in arr_pat.elements.iter().flatten() {
                count_pattern_bindings(p, seen, count);
            }
        }
        Pattern::Assignment(assign_pat) => {
            count_pattern_bindings(&assign_pat.left, seen, count);
        }
        Pattern::Rest(rest) => {
            count_pattern_bindings(&rest.argument, seen, count);
        }
    }
}

/// Collect function declaration names (hoisted)
fn collect_function_decl_names(
    statements: &[Statement],
    seen: &mut FxHashSet<JsString>,
    count: &mut usize,
) {
    for stmt in statements {
        collect_function_decl_names_stmt(stmt, seen, count);
    }
}

/// Collect function declaration names from a single statement
fn collect_function_decl_names_stmt(
    stmt: &Statement,
    seen: &mut FxHashSet<JsString>,
    count: &mut usize,
) {
    match stmt {
        Statement::FunctionDeclaration(func) => {
            if let Some(ref id) = func.id
                && seen.insert(id.name.cheap_clone())
            {
                *count += 1;
            }
        }
        // Recurse into blocks and control flow (function declarations hoist from blocks)
        Statement::Block(block) => {
            for s in block.body.iter() {
                collect_function_decl_names_stmt(s, seen, count);
            }
        }
        Statement::If(if_stmt) => {
            collect_function_decl_names_stmt(&if_stmt.consequent, seen, count);
            if let Some(ref alt) = if_stmt.alternate {
                collect_function_decl_names_stmt(alt, seen, count);
            }
        }
        Statement::While(while_stmt) => {
            collect_function_decl_names_stmt(&while_stmt.body, seen, count);
        }
        Statement::DoWhile(do_while) => {
            collect_function_decl_names_stmt(&do_while.body, seen, count);
        }
        Statement::For(for_stmt) => {
            collect_function_decl_names_stmt(&for_stmt.body, seen, count);
        }
        Statement::ForIn(for_in) => {
            collect_function_decl_names_stmt(&for_in.body, seen, count);
        }
        Statement::ForOf(for_of) => {
            collect_function_decl_names_stmt(&for_of.body, seen, count);
        }
        Statement::Switch(switch_stmt) => {
            for case in switch_stmt.cases.iter() {
                for s in case.consequent.iter() {
                    collect_function_decl_names_stmt(s, seen, count);
                }
            }
        }
        Statement::Try(try_stmt) => {
            for s in try_stmt.block.body.iter() {
                collect_function_decl_names_stmt(s, seen, count);
            }
            if let Some(ref handler) = try_stmt.handler {
                for s in handler.body.body.iter() {
                    collect_function_decl_names_stmt(s, seen, count);
                }
            }
            if let Some(ref finalizer) = try_stmt.finalizer {
                for s in finalizer.body.iter() {
                    collect_function_decl_names_stmt(s, seen, count);
                }
            }
        }
        Statement::Labeled(labeled) => {
            collect_function_decl_names_stmt(&labeled.body, seen, count);
        }
        // Other statements don't contain function declarations at this level
        _ => {}
    }
}

/// Collect var names from a pattern (for destructuring)
fn collect_pattern_var_names(pattern: &Pattern, names: &mut Vec<JsString>) {
    match pattern {
        Pattern::Identifier(id) => {
            names.push(id.name.cheap_clone());
        }
        Pattern::Object(obj_pat) => {
            for prop in &obj_pat.properties {
                match prop {
                    crate::ast::ObjectPatternProperty::KeyValue { value, .. } => {
                        collect_pattern_var_names(value, names);
                    }
                    crate::ast::ObjectPatternProperty::Rest(rest) => {
                        collect_pattern_var_names(&rest.argument, names);
                    }
                }
            }
        }
        Pattern::Array(arr_pat) => {
            for p in arr_pat.elements.iter().flatten() {
                collect_pattern_var_names(p, names);
            }
        }
        Pattern::Assignment(assign_pat) => {
            collect_pattern_var_names(&assign_pat.left, names);
        }
        Pattern::Rest(rest) => {
            collect_pattern_var_names(&rest.argument, names);
        }
    }
}
