//! Statement compilation
//!
//! Compiles AST statements to bytecode instructions.

use super::bytecode::{Op, Register};
use super::Compiler;
use crate::ast::{
    BlockStatement, BreakStatement, ClassConstructor, ClassDeclaration, ClassMember, ClassMethod,
    ClassProperty, ContinueStatement, DoWhileStatement, ForInOfLeft, ForInStatement, ForInit,
    ForOfStatement, ForStatement, IfStatement, LabeledStatement, MethodKind, ObjectPatternProperty,
    ObjectPropertyKey, Pattern, ReturnStatement, Statement, SwitchStatement, ThrowStatement,
    TryStatement, VariableDeclaration, VariableKind, WhileStatement,
};
use crate::error::JsError;
use crate::value::{CheapClone, JsString};

impl Compiler {
    /// Compile a statement
    pub fn compile_statement_impl(&mut self, stmt: &Statement) -> Result<(), JsError> {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.builder.set_span(expr_stmt.span);
                if self.track_completion {
                    // When tracking completion, compile directly to register 0
                    self.compile_expression(&expr_stmt.expression, 0)?;
                } else {
                    let dst = self.builder.alloc_register()?;
                    self.compile_expression(&expr_stmt.expression, dst)?;
                    // Discard the result
                    self.builder.free_register(dst);
                }
                Ok(())
            }

            Statement::VariableDeclaration(decl) => self.compile_variable_declaration(decl),

            Statement::Block(block) => self.compile_block(block),

            Statement::If(if_stmt) => self.compile_if(if_stmt),

            Statement::While(while_stmt) => self.compile_while(while_stmt),

            Statement::DoWhile(do_while) => self.compile_do_while(do_while),

            Statement::For(for_stmt) => self.compile_for(for_stmt),

            Statement::ForIn(for_in) => self.compile_for_in(for_in),

            Statement::ForOf(for_of) => self.compile_for_of(for_of),

            Statement::Switch(switch_stmt) => self.compile_switch(switch_stmt),

            Statement::Return(return_stmt) => self.compile_return(return_stmt),

            Statement::Break(break_stmt) => self.compile_break(break_stmt),

            Statement::Continue(continue_stmt) => self.compile_continue(continue_stmt),

            Statement::Throw(throw_stmt) => self.compile_throw(throw_stmt),

            Statement::Try(try_stmt) => self.compile_try(try_stmt),

            Statement::Labeled(labeled) => self.compile_labeled(labeled),

            Statement::FunctionDeclaration(func) => self.compile_function_declaration(func),

            Statement::ClassDeclaration(class) => self.compile_class_declaration(class),

            Statement::Empty => {
                // No-op
                Ok(())
            }

            Statement::Debugger => {
                self.builder.emit(Op::Debugger);
                Ok(())
            }

            // TypeScript declarations - no-ops at runtime (except enum)
            Statement::TypeAlias(_) | Statement::InterfaceDeclaration(_) => Ok(()),

            // Enum declarations create runtime objects
            Statement::EnumDeclaration(decl) => self.compile_enum_declaration(decl),

            // Namespace declarations
            Statement::NamespaceDeclaration(decl) => self.compile_namespace_declaration(decl),

            // Module declarations
            Statement::Import(_) => {
                // TODO: Implement module compilation
                Err(JsError::syntax_error_simple(
                    "Module imports not yet supported in bytecode compiler",
                ))
            }

            // Export declaration - compile the inner declaration if any
            // (exports are only meaningful in module context, but we support
            // them within namespaces)
            Statement::Export(export) => {
                if let Some(ref decl) = export.declaration {
                    self.compile_statement_impl(decl)?;
                }
                Ok(())
            }
        }
    }

    /// Compile a variable declaration
    fn compile_variable_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), JsError> {
        self.builder.set_span(decl.span);

        let mutable = decl.kind != VariableKind::Const;
        let is_var = decl.kind == VariableKind::Var;

        for declarator in decl.declarations.iter() {
            self.builder.set_span(declarator.span);

            // Get the inferred name from simple identifier patterns
            let inferred_name = match &declarator.id {
                Pattern::Identifier(id) => Some(id.name.cheap_clone()),
                _ => None,
            };

            // Compile initializer (or undefined)
            let init_reg = self.builder.alloc_register()?;
            if let Some(init) = &declarator.init {
                // Check if this is an anonymous function being assigned to an identifier
                // If so, pass the inferred name for function.name property
                self.compile_expression_with_inferred_name(init, init_reg, inferred_name)?;
            } else {
                self.builder.emit(Op::LoadUndefined { dst: init_reg });
            }

            // Bind to pattern
            self.compile_pattern_binding(&declarator.id, init_reg, mutable, is_var)?;

            self.builder.free_register(init_reg);
        }

        Ok(())
    }

    /// Compile a block statement
    fn compile_block(&mut self, block: &BlockStatement) -> Result<(), JsError> {
        self.builder.set_span(block.span);

        // Push a new scope
        self.builder.emit(Op::PushScope);

        if block.body.is_empty() && self.track_completion {
            // Empty block has completion value undefined
            self.builder.emit(Op::LoadUndefined { dst: 0 });
        } else {
            // Compile statements
            for stmt in block.body.iter() {
                self.compile_statement_impl(stmt)?;
            }
        }

        // Pop scope
        self.builder.emit(Op::PopScope);

        Ok(())
    }

    /// Compile an if statement
    fn compile_if(&mut self, if_stmt: &IfStatement) -> Result<(), JsError> {
        self.builder.set_span(if_stmt.span);

        // Compile test
        let test_reg = self.builder.alloc_register()?;
        self.compile_expression(&if_stmt.test, test_reg)?;

        // Jump to else/end if test is falsy
        let jump_to_else = self.builder.emit_jump_if_false(test_reg);
        self.builder.free_register(test_reg);

        // Compile consequent
        self.compile_statement_impl(&if_stmt.consequent)?;

        if let Some(alternate) = &if_stmt.alternate {
            // Jump over else block
            let jump_to_end = self.builder.emit_jump();

            // Patch jump to else
            self.builder.patch_jump(jump_to_else);

            // Compile alternate
            self.compile_statement_impl(alternate)?;

            // Patch jump to end
            self.builder.patch_jump(jump_to_end);
        } else {
            // Patch jump to end (no else block)
            self.builder.patch_jump(jump_to_else);
        }

        Ok(())
    }

    /// Compile a while statement
    fn compile_while(&mut self, while_stmt: &WhileStatement) -> Result<(), JsError> {
        self.builder.set_span(while_stmt.span);

        // Loop start (for continue)
        let loop_start = self.builder.current_offset();

        // Push loop context
        self.push_loop(None);
        self.set_continue_target(loop_start);

        // Compile test
        let test_reg = self.builder.alloc_register()?;
        self.compile_expression(&while_stmt.test, test_reg)?;

        // Jump to end if test is falsy
        let jump_to_end = self.builder.emit_jump_if_false(test_reg);
        self.builder.free_register(test_reg);

        // Compile body
        self.compile_statement_impl(&while_stmt.body)?;

        // Jump back to start
        self.builder.emit_jump_to(loop_start);

        // Patch end jump
        self.builder.patch_jump(jump_to_end);

        // Pop loop context (patches break jumps)
        self.pop_loop();

        Ok(())
    }

    /// Compile a do-while statement
    fn compile_do_while(&mut self, do_while: &DoWhileStatement) -> Result<(), JsError> {
        self.builder.set_span(do_while.span);

        // Loop start
        let loop_start = self.builder.current_offset();

        // Push loop context
        self.push_loop(None);

        // Compile body first
        self.compile_statement_impl(&do_while.body)?;

        // Continue target is here (after body, before test)
        let continue_target = self.builder.current_offset();
        self.set_continue_target(continue_target);

        // Compile test
        let test_reg = self.builder.alloc_register()?;
        self.compile_expression(&do_while.test, test_reg)?;

        // Jump back to start if test is truthy
        self.builder.emit(Op::JumpIfTrue {
            cond: test_reg,
            target: loop_start as u32,
        });
        self.builder.free_register(test_reg);

        // Pop loop context
        self.pop_loop();

        Ok(())
    }

    /// Compile a for statement
    fn compile_for(&mut self, for_stmt: &ForStatement) -> Result<(), JsError> {
        self.builder.set_span(for_stmt.span);

        // Check if this is a for loop with let/const declaration (needs per-iteration binding)
        let per_iteration_vars = self.get_per_iteration_vars(&for_stmt.init);

        if per_iteration_vars.is_empty() {
            // No let/const vars: use simple compilation
            self.compile_for_simple(for_stmt)
        } else {
            // Has let/const vars: use per-iteration binding semantics
            self.compile_for_per_iteration(for_stmt, &per_iteration_vars)
        }
    }

    /// Get variable names that need per-iteration binding (let/const in for init)
    fn get_per_iteration_vars(&self, init: &Option<ForInit>) -> Vec<JsString> {
        if let Some(ForInit::Variable(decl)) = init {
            if decl.kind == VariableKind::Let || decl.kind == VariableKind::Const {
                // Extract variable names from declarations
                let mut names = Vec::new();
                for declarator in decl.declarations.iter() {
                    Self::collect_pattern_names(&declarator.id, &mut names);
                }
                return names;
            }
        }
        Vec::new()
    }

    /// Collect variable names from a pattern
    fn collect_pattern_names(pattern: &Pattern, names: &mut Vec<JsString>) {
        match pattern {
            Pattern::Identifier(id) => names.push(id.name.cheap_clone()),
            Pattern::Object(obj) => {
                for prop in &obj.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { value, .. } => {
                            Self::collect_pattern_names(value, names);
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            Self::collect_pattern_names(&rest.argument, names);
                        }
                    }
                }
            }
            Pattern::Array(arr) => {
                for elem in arr.elements.iter().flatten() {
                    Self::collect_pattern_names(elem, names);
                }
            }
            Pattern::Rest(rest) => {
                Self::collect_pattern_names(&rest.argument, names);
            }
            Pattern::Assignment(assign) => {
                Self::collect_pattern_names(&assign.left, names);
            }
        }
    }

    /// Compile for loop without per-iteration bindings (var or expression init)
    fn compile_for_simple(&mut self, for_stmt: &ForStatement) -> Result<(), JsError> {
        // Push scope for loop variable
        self.builder.emit(Op::PushScope);

        // Compile init
        if let Some(init) = &for_stmt.init {
            match init {
                ForInit::Variable(decl) => {
                    self.compile_variable_declaration(decl)?;
                }
                ForInit::Expression(expr) => {
                    let tmp = self.builder.alloc_register()?;
                    self.compile_expression(expr, tmp)?;
                    self.builder.free_register(tmp);
                }
            }
        }

        // Loop test position
        let loop_test = self.builder.current_offset();

        // Push loop context
        self.push_loop(None);

        // Compile test (if any)
        let jump_to_end = if let Some(test) = &for_stmt.test {
            let test_reg = self.builder.alloc_register()?;
            self.compile_expression(test, test_reg)?;
            let jump = self.builder.emit_jump_if_false(test_reg);
            self.builder.free_register(test_reg);
            Some(jump)
        } else {
            None
        };

        // Compile body
        self.compile_statement_impl(&for_stmt.body)?;

        // Continue target (before update)
        let continue_target = self.builder.current_offset();
        self.set_continue_target(continue_target);

        // Compile update (if any)
        if let Some(update) = &for_stmt.update {
            let tmp = self.builder.alloc_register()?;
            self.compile_expression(update, tmp)?;
            self.builder.free_register(tmp);
        }

        // Jump back to test
        self.builder.emit_jump_to(loop_test);

        // Patch end jump
        if let Some(jump) = jump_to_end {
            self.builder.patch_jump(jump);
        }

        // Pop loop context
        self.pop_loop();

        // Pop scope
        self.builder.emit(Op::PopScope);

        Ok(())
    }

    /// Compile for loop with per-iteration bindings for let/const vars
    /// Each iteration gets a fresh binding, with values copied between iterations.
    ///
    /// Key insight: closures must capture the PRE-update value. To achieve this,
    /// we don't modify the per-iteration bindings during update. Instead, we
    /// compile the update to compute the new value into a register, then use
    /// that register to initialize the NEXT iteration's bindings.
    fn compile_for_per_iteration(
        &mut self,
        for_stmt: &ForStatement,
        var_names: &[JsString],
    ) -> Result<(), JsError> {
        // Allocate registers to hold values between iterations
        let mut var_regs: Vec<(JsString, Register)> = Vec::new();
        for name in var_names {
            let reg = self.builder.alloc_register()?;
            var_regs.push((name.cheap_clone(), reg));
        }

        // Push outer scope for the init
        self.builder.emit(Op::PushScope);

        // Compile init (first iteration's values)
        if let Some(ForInit::Variable(decl)) = &for_stmt.init {
            self.compile_variable_declaration(decl)?;
        }

        // Copy initial values to registers
        for (name, reg) in &var_regs {
            let name_idx = self.builder.add_string(name.cheap_clone())?;
            self.builder.emit(Op::GetVar {
                dst: *reg,
                name: name_idx,
            });
        }

        // Pop the init scope (we'll create per-iteration scopes in the loop)
        self.builder.emit(Op::PopScope);

        // Loop start - push per-iteration scope and copy values from registers
        let loop_start = self.builder.current_offset();

        // Push per-iteration scope
        self.builder.emit(Op::PushScope);

        // Declare and initialize vars from registers (these are the values closures will capture)
        for (name, reg) in &var_regs {
            let name_idx = self.builder.add_string(name.cheap_clone())?;
            self.builder.emit(Op::DeclareVar {
                name: name_idx,
                init: *reg,
                mutable: true, // let vars are mutable
            });
        }

        // Push loop context
        self.push_loop(None);

        // Compile test (if any)
        let jump_to_end = if let Some(test) = &for_stmt.test {
            let test_reg = self.builder.alloc_register()?;
            self.compile_expression(test, test_reg)?;
            let jump = self.builder.emit_jump_if_false(test_reg);
            self.builder.free_register(test_reg);
            Some(jump)
        } else {
            None
        };

        // Compile body (closures capture the per-iteration scope's bindings)
        self.compile_statement_impl(&for_stmt.body)?;

        // Continue target
        let continue_target = self.builder.current_offset();
        self.set_continue_target(continue_target);

        // Compile update with special handling for loop variables:
        // Instead of modifying the scope's bindings (which closures captured),
        // we evaluate the update and store results to registers for the next iteration.
        if let Some(update) = &for_stmt.update {
            // Enable loop variable redirection: any assignment to loop vars
            // will be redirected to their corresponding registers
            self.set_loop_var_redirects(var_regs.clone());

            let tmp = self.builder.alloc_register()?;
            self.compile_expression(update, tmp)?;
            self.builder.free_register(tmp);

            // Disable redirection
            self.clear_loop_var_redirects();
        } else {
            // No update expression, but body may have modified loop variables.
            // Copy current scope values back to registers for next iteration.
            for (name, reg) in &var_regs {
                let name_idx = self.builder.add_string(name.cheap_clone())?;
                self.builder.emit(Op::GetVar {
                    dst: *reg,
                    name: name_idx,
                });
            }
        }

        // Pop per-iteration scope
        self.builder.emit(Op::PopScope);

        // Jump back to loop start
        self.builder.emit_jump_to(loop_start);

        // Patch end jump (jump here when test fails)
        if let Some(jump) = jump_to_end {
            self.builder.patch_jump(jump);
        }

        // If jumping out due to test failure, need to pop scope
        self.builder.emit(Op::PopScope);

        // Pop loop context
        self.pop_loop();

        // Free registers
        for (_, reg) in var_regs {
            self.builder.free_register(reg);
        }

        Ok(())
    }

    /// Compile a for-in statement
    fn compile_for_in(&mut self, for_in: &ForInStatement) -> Result<(), JsError> {
        self.builder.set_span(for_in.span);

        // Push scope
        self.builder.emit(Op::PushScope);

        // Compile the right side (object to iterate)
        let obj_reg = self.builder.alloc_register()?;
        self.compile_expression(&for_in.right, obj_reg)?;

        // Get keys iterator for for-in (iterates over enumerable property keys)
        let iter_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::GetKeysIterator {
            dst: iter_reg,
            obj: obj_reg,
        });

        // Loop start
        let loop_start = self.builder.current_offset();

        // Push loop context
        self.push_loop(None);
        self.set_continue_target(loop_start);

        // Get next key
        let result_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::IteratorNext {
            dst: result_reg,
            iterator: iter_reg,
        });

        // Check if done
        let jump_to_end = self.builder.emit(Op::IteratorDone {
            result: result_reg,
            target: 0,
        });
        let jump_placeholder = super::JumpPlaceholder {
            instruction_index: jump_to_end,
        };

        // Get the value
        let value_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::IteratorValue {
            dst: value_reg,
            result: result_reg,
        });

        // Bind to left side
        self.compile_for_in_of_left(&for_in.left, value_reg)?;

        // Compile body
        self.compile_statement_impl(&for_in.body)?;

        // Jump back to start
        self.builder.emit_jump_to(loop_start);

        // Patch end jump
        self.builder.patch_jump(jump_placeholder);

        // Pop loop context
        self.pop_loop();

        // Free registers
        self.builder.free_register(value_reg);
        self.builder.free_register(result_reg);
        self.builder.free_register(iter_reg);
        self.builder.free_register(obj_reg);

        // Pop scope
        self.builder.emit(Op::PopScope);

        Ok(())
    }

    /// Compile a for-of statement
    fn compile_for_of(&mut self, for_of: &ForOfStatement) -> Result<(), JsError> {
        self.builder.set_span(for_of.span);

        // Push scope
        self.builder.emit(Op::PushScope);

        // Compile the right side (iterable)
        let obj_reg = self.builder.alloc_register()?;
        self.compile_expression(&for_of.right, obj_reg)?;

        // Get iterator
        let iter_reg = self.builder.alloc_register()?;
        if for_of.await_ {
            self.builder.emit(Op::GetAsyncIterator {
                dst: iter_reg,
                obj: obj_reg,
            });
        } else {
            self.builder.emit(Op::GetIterator {
                dst: iter_reg,
                obj: obj_reg,
            });
        }

        // Loop start
        let loop_start = self.builder.current_offset();

        // Push loop context
        self.push_loop(None);
        self.set_continue_target(loop_start);

        // Get next value
        let result_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::IteratorNext {
            dst: result_reg,
            iterator: iter_reg,
        });

        // For await...of, await the result (for true async iterators, next() returns a promise)
        if for_of.await_ {
            self.builder.emit(Op::Await {
                dst: result_reg,
                promise: result_reg,
            });
        }

        // Check if done
        let jump_to_end = self.builder.emit(Op::IteratorDone {
            result: result_reg,
            target: 0,
        });
        let jump_placeholder = super::JumpPlaceholder {
            instruction_index: jump_to_end,
        };

        // Get the value
        let value_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::IteratorValue {
            dst: value_reg,
            result: result_reg,
        });

        // For await...of, also await the value (for sync iterables with promise values)
        if for_of.await_ {
            self.builder.emit(Op::Await {
                dst: value_reg,
                promise: value_reg,
            });
        }

        // Bind to left side
        self.compile_for_in_of_left(&for_of.left, value_reg)?;

        // Compile body
        self.compile_statement_impl(&for_of.body)?;

        // Jump back to start
        self.builder.emit_jump_to(loop_start);

        // Patch end jump
        self.builder.patch_jump(jump_placeholder);

        // Pop loop context
        self.pop_loop();

        // Free registers
        self.builder.free_register(value_reg);
        self.builder.free_register(result_reg);
        self.builder.free_register(iter_reg);
        self.builder.free_register(obj_reg);

        // Pop scope
        self.builder.emit(Op::PopScope);

        Ok(())
    }

    /// Compile the left side of a for-in/for-of
    fn compile_for_in_of_left(
        &mut self,
        left: &ForInOfLeft,
        value_reg: super::Register,
    ) -> Result<(), JsError> {
        match left {
            ForInOfLeft::Variable(decl) => {
                // Should have exactly one declarator
                if let Some(declarator) = decl.declarations.first() {
                    let mutable = decl.kind != VariableKind::Const;
                    let is_var = decl.kind == VariableKind::Var;
                    self.compile_pattern_binding(&declarator.id, value_reg, mutable, is_var)?;
                }
            }
            ForInOfLeft::Pattern(pattern) => {
                // Assignment to existing binding
                self.compile_pattern_assignment(pattern, value_reg)?;
            }
        }
        Ok(())
    }

    /// Compile a switch statement
    fn compile_switch(&mut self, switch_stmt: &SwitchStatement) -> Result<(), JsError> {
        self.builder.set_span(switch_stmt.span);

        // Compile discriminant
        let disc_reg = self.builder.alloc_register()?;
        self.compile_expression(&switch_stmt.discriminant, disc_reg)?;

        // Push loop context for break (switch uses the same break mechanism)
        self.push_loop(None);

        // Collect case targets
        let mut case_jumps: Vec<super::JumpPlaceholder> = Vec::new();
        let mut default_jump: Option<super::JumpPlaceholder> = None;

        // First pass: emit comparison and jumps
        for case in switch_stmt.cases.iter() {
            if let Some(test) = &case.test {
                // Regular case
                let test_reg = self.builder.alloc_register()?;
                self.compile_expression(test, test_reg)?;

                // Compare with discriminant (strict equality)
                let cmp_reg = self.builder.alloc_register()?;
                self.builder.emit(Op::StrictEq {
                    dst: cmp_reg,
                    left: disc_reg,
                    right: test_reg,
                });

                // Jump to case body if equal
                case_jumps.push(self.builder.emit_jump_if_true(cmp_reg));

                self.builder.free_register(cmp_reg);
                self.builder.free_register(test_reg);
            } else {
                // Default case - save for later
                default_jump = Some(self.builder.emit_jump());
            }
        }

        // Jump to end if no case matched (and no default)
        let jump_to_end = if default_jump.is_none() {
            Some(self.builder.emit_jump())
        } else {
            None
        };

        // Second pass: emit case bodies
        let mut case_jumps_iter = case_jumps.into_iter();
        for case in switch_stmt.cases.iter() {
            if case.test.is_some() {
                // Patch case jump to here
                if let Some(jump) = case_jumps_iter.next() {
                    self.builder.patch_jump(jump);
                }
            } else {
                // Patch default jump to here
                if let Some(jump) = default_jump.take() {
                    self.builder.patch_jump(jump);
                }
            }

            // Compile case statements
            for stmt in case.consequent.iter() {
                self.compile_statement_impl(stmt)?;
            }
        }

        // Patch jump to end
        if let Some(jump) = jump_to_end {
            self.builder.patch_jump(jump);
        }

        // Pop loop context (patches break jumps)
        self.pop_loop();

        self.builder.free_register(disc_reg);

        Ok(())
    }

    /// Compile a return statement
    fn compile_return(&mut self, return_stmt: &ReturnStatement) -> Result<(), JsError> {
        self.builder.set_span(return_stmt.span);

        if let Some(argument) = &return_stmt.argument {
            let reg = self.builder.alloc_register()?;
            self.compile_expression(argument, reg)?;
            self.builder.emit(Op::Return { value: reg });
            self.builder.free_register(reg);
        } else {
            self.builder.emit(Op::ReturnUndefined);
        }

        Ok(())
    }

    /// Compile a break statement
    fn compile_break(&mut self, break_stmt: &BreakStatement) -> Result<(), JsError> {
        self.builder.set_span(break_stmt.span);

        let label = break_stmt.label.as_ref().map(|id| &id.name);
        self.add_break_jump(label)?;

        Ok(())
    }

    /// Compile a continue statement
    fn compile_continue(&mut self, continue_stmt: &ContinueStatement) -> Result<(), JsError> {
        self.builder.set_span(continue_stmt.span);

        let label = continue_stmt.label.as_ref().map(|id| &id.name);
        self.add_continue_jump(label)?;

        Ok(())
    }

    /// Compile a throw statement
    fn compile_throw(&mut self, throw_stmt: &ThrowStatement) -> Result<(), JsError> {
        self.builder.set_span(throw_stmt.span);

        let reg = self.builder.alloc_register()?;
        self.compile_expression(&throw_stmt.argument, reg)?;
        self.builder.emit(Op::Throw { value: reg });
        self.builder.free_register(reg);

        Ok(())
    }

    /// Compile a try statement
    fn compile_try(&mut self, try_stmt: &TryStatement) -> Result<(), JsError> {
        self.builder.set_span(try_stmt.span);

        self.try_depth += 1;

        // Emit PushTry - targets will be patched
        let push_try_idx = self.builder.emit(Op::PushTry {
            catch_target: 0,
            finally_target: 0,
        });

        // Compile try block
        self.compile_block(&try_stmt.block)?;

        // Pop try handler after successful completion
        self.builder.emit(Op::PopTry);

        // Jump to finally (if exists) or end
        let jump_after_try = self.builder.emit_jump();

        // Catch handler
        let catch_start = self.builder.current_offset();
        if let Some(handler) = &try_stmt.handler {
            self.builder.set_span(handler.span);

            // Push scope for catch variable
            self.builder.emit(Op::PushScope);

            // Bind exception to parameter
            if let Some(param) = &handler.param {
                let exc_reg = self.builder.alloc_register()?;
                self.builder.emit(Op::GetException { dst: exc_reg });
                self.compile_pattern_binding(param, exc_reg, true, false)?;
                self.builder.free_register(exc_reg);
            }

            // Compile catch body
            for stmt in handler.body.body.iter() {
                self.compile_statement_impl(stmt)?;
            }

            // Pop scope
            self.builder.emit(Op::PopScope);
        }

        // Jump to finally (if exists) or end
        let jump_after_catch = self.builder.emit_jump();

        // Finally handler
        let finally_start = self.builder.current_offset();
        if let Some(finalizer) = &try_stmt.finalizer {
            self.builder.set_span(finalizer.span);

            // Compile finally block
            for stmt in finalizer.body.iter() {
                self.compile_statement_impl(stmt)?;
            }

            // FinallyEnd completes any pending return/throw
            self.builder.emit(Op::FinallyEnd);
        }

        // End of try-catch-finally
        let end_offset = self.builder.current_offset();

        // Patch jumps: if there's a finally block, jump TO it, otherwise jump past everything
        if try_stmt.finalizer.is_some() {
            // Jump to finally block (which will naturally fall through to end)
            self.builder
                .patch_jump_to(jump_after_try, finally_start as super::JumpTarget);
            self.builder
                .patch_jump_to(jump_after_catch, finally_start as super::JumpTarget);
        } else {
            // No finally, jump to end
            self.builder
                .patch_jump_to(jump_after_try, end_offset as super::JumpTarget);
            self.builder
                .patch_jump_to(jump_after_catch, end_offset as super::JumpTarget);
        }

        // Patch PushTry targets
        self.builder.patch_try_targets(
            push_try_idx,
            if try_stmt.handler.is_some() {
                catch_start as u32
            } else {
                0
            },
            if try_stmt.finalizer.is_some() {
                finally_start as u32
            } else {
                0
            },
        );

        self.try_depth -= 1;

        Ok(())
    }

    /// Compile a labeled statement
    fn compile_labeled(&mut self, labeled: &LabeledStatement) -> Result<(), JsError> {
        self.builder.set_span(labeled.span);

        // Push loop context with label
        self.push_loop(Some(labeled.label.name.cheap_clone()));

        // Compile the body
        self.compile_statement_impl(&labeled.body)?;

        // Pop loop context
        self.pop_loop();

        Ok(())
    }

    /// Compile a function declaration
    fn compile_function_declaration(
        &mut self,
        func: &crate::ast::FunctionDeclaration,
    ) -> Result<(), JsError> {
        self.builder.set_span(func.span);

        // Get function name
        let name = func.id.as_ref().map(|id| id.name.cheap_clone());

        // Compile the function body to a nested chunk
        let chunk = self.compile_function_body(
            &func.params,
            &func.body.body,
            name.clone(),
            func.generator,
            func.async_,
            false, // not an arrow function
        )?;

        // Add the chunk to constants
        let chunk_idx = self.builder.add_chunk(chunk)?;

        // Allocate register for the function object
        let dst = self.builder.alloc_register()?;

        // Emit CreateClosure instruction
        if func.generator && func.async_ {
            self.builder
                .emit(Op::CreateAsyncGenerator { dst, chunk_idx });
        } else if func.generator {
            self.builder.emit(Op::CreateGenerator { dst, chunk_idx });
        } else if func.async_ {
            self.builder.emit(Op::CreateAsync { dst, chunk_idx });
        } else {
            self.builder.emit(Op::CreateClosure { dst, chunk_idx });
        }

        // If the function has a name, declare it as a variable
        if let Some(ref fn_name) = name {
            let name_idx = self.builder.add_string(fn_name.cheap_clone())?;
            self.builder.emit(Op::DeclareVarHoisted {
                name: name_idx,
                init: dst,
            });
        }

        self.builder.free_register(dst);

        Ok(())
    }

    /// Compile function body to a nested BytecodeChunk
    pub fn compile_function_body(
        &mut self,
        params: &[crate::ast::FunctionParam],
        body: &[Statement],
        name: Option<JsString>,
        is_generator: bool,
        is_async: bool,
        is_arrow: bool,
    ) -> Result<super::BytecodeChunk, JsError> {
        use super::FunctionInfo;

        // Create a new compiler for the function body
        let mut func_compiler = Compiler::new();

        // Copy class context so private members can be accessed inside nested functions
        func_compiler.class_context_stack = self.class_context_stack.clone();

        // Reserve registers for parameters - they are passed in registers 0, 1, 2...
        // We must reserve these before any other register allocation
        if !params.is_empty() {
            func_compiler
                .builder
                .reserve_registers(params.len() as u8)?;
        }

        // Compile parameter declarations
        // Parameters will be passed via registers and need to be bound to the environment
        let mut param_names = Vec::with_capacity(params.len());
        let mut rest_param = None;

        for (idx, param) in params.iter().enumerate() {
            let arg_reg = idx as u8;

            match &param.pattern {
                crate::ast::Pattern::Identifier(id) => {
                    param_names.push(id.name.cheap_clone());

                    // Load argument from register and declare variable
                    let name_idx = func_compiler.builder.add_string(id.name.cheap_clone())?;
                    func_compiler.builder.emit(Op::DeclareVar {
                        name: name_idx,
                        init: arg_reg,
                        mutable: true,
                    });
                }
                crate::ast::Pattern::Rest(rest) => {
                    rest_param = Some(idx);
                    if let crate::ast::Pattern::Identifier(id) = &*rest.argument {
                        param_names.push(id.name.cheap_clone());

                        // For rest params, we'll need special handling (not fully implemented)
                        let name_idx = func_compiler.builder.add_string(id.name.cheap_clone())?;
                        func_compiler.builder.emit(Op::DeclareVar {
                            name: name_idx,
                            init: arg_reg,
                            mutable: true,
                        });
                    }
                }
                crate::ast::Pattern::Object(_) | crate::ast::Pattern::Array(_) => {
                    // Handle destructuring patterns in function parameters
                    // The argument is in arg_reg, compile the pattern binding
                    param_names.push(JsString::from(format!("__param{}__", idx)));
                    func_compiler.compile_pattern_binding(&param.pattern, arg_reg, true, false)?;
                }
                crate::ast::Pattern::Assignment(assign_pat) => {
                    // Parameter with default value: param = defaultValue
                    // If argument is undefined, use default value
                    let actual_value = func_compiler.builder.alloc_register()?;

                    // Check if arg is undefined
                    let is_undefined = func_compiler.builder.alloc_register()?;
                    func_compiler
                        .builder
                        .emit(Op::LoadUndefined { dst: is_undefined });
                    func_compiler.builder.emit(Op::StrictEq {
                        dst: is_undefined,
                        left: arg_reg,
                        right: is_undefined,
                    });

                    let skip_default = func_compiler.builder.emit_jump_if_false(is_undefined);
                    func_compiler.builder.free_register(is_undefined);

                    // Use default value
                    func_compiler.compile_expression(&assign_pat.right, actual_value)?;
                    let skip_arg = func_compiler.builder.emit_jump();

                    // Use provided argument
                    func_compiler.builder.patch_jump(skip_default);
                    func_compiler.builder.emit(Op::Move {
                        dst: actual_value,
                        src: arg_reg,
                    });

                    func_compiler.builder.patch_jump(skip_arg);

                    // Bind the inner pattern
                    func_compiler.compile_pattern_binding(
                        &assign_pat.left,
                        actual_value,
                        true,
                        false,
                    )?;

                    // Extract name for param_names
                    if let crate::ast::Pattern::Identifier(id) = assign_pat.left.as_ref() {
                        param_names.push(id.name.cheap_clone());
                    } else {
                        param_names.push(JsString::from(format!("__param{}__", idx)));
                    }

                    func_compiler.builder.free_register(actual_value);
                }
            }
        }

        // Hoist var declarations in the function body
        func_compiler.emit_hoisted_declarations(body)?;

        // Compile the body statements
        for stmt in body {
            func_compiler.compile_statement_impl(stmt)?;
        }

        // Emit implicit return undefined at end
        let undefined_reg = func_compiler.builder.alloc_register()?;
        func_compiler
            .builder
            .emit(Op::LoadUndefined { dst: undefined_reg });
        func_compiler.builder.emit(Op::Return {
            value: undefined_reg,
        });

        // Build the chunk with function info
        let mut chunk = func_compiler.builder.finish();
        chunk.function_info = Some(FunctionInfo {
            name,
            param_count: params.len(),
            is_generator,
            is_async,
            is_arrow,
            uses_arguments: false, // TODO: analyze function body
            uses_this: !is_arrow,
            param_names,
            rest_param,
        });

        // Make sure we have enough registers for parameters
        if chunk.register_count < params.len() as u8 {
            chunk.register_count = params.len() as u8;
        }

        Ok(chunk)
    }

    /// Compile a class declaration
    fn compile_class_declaration(&mut self, class: &ClassDeclaration) -> Result<(), JsError> {
        self.builder.set_span(class.span);

        // Allocate register for the class constructor
        let class_reg = self.builder.alloc_register()?;

        // Compile class body to register
        // Note: compile_class_body now also declares the class variable (if named)
        // so that static blocks can reference the class by name
        self.compile_class_body(class, class_reg)?;

        self.builder.free_register(class_reg);
        Ok(())
    }

    /// Compile class body - shared by class declarations and expressions
    ///
    /// `inferred_name`: Optional inferred name for anonymous class expressions.
    /// This is used only for the `.name` property, NOT for creating a scope binding.
    pub fn compile_class_body(
        &mut self,
        class: &ClassDeclaration,
        dst: super::bytecode::Register,
    ) -> Result<(), JsError> {
        self.compile_class_body_with_name(class, dst, None)
    }

    /// Compile class body with optional inferred name (for anonymous class expressions)
    pub fn compile_class_body_with_name(
        &mut self,
        class: &ClassDeclaration,
        dst: super::bytecode::Register,
        inferred_name: Option<JsString>,
    ) -> Result<(), JsError> {
        // Get class name for constructor - prefer explicit id, fallback to inferred name
        let class_name = class
            .id
            .as_ref()
            .map(|id| id.name.cheap_clone())
            .or(inferred_name);
        // Only create inner binding if class has explicit id (not inferred name)
        let has_explicit_name = class.id.is_some();

        // Compile class decorators first - they are evaluated before the class is created
        // We store them in reverse order so we can apply them bottom-to-top later
        let mut decorator_regs: Vec<super::bytecode::Register> = Vec::new();
        for decorator in class.decorators.iter().rev() {
            let dec_reg = self.builder.alloc_register()?;
            self.compile_expression(&decorator.expression, dec_reg)?;
            decorator_regs.push(dec_reg);
        }

        // Generate a unique class brand for private field access
        let class_brand = self.new_class_brand();

        // Collect class members and build private member info
        let mut constructor: Option<&ClassConstructor> = None;
        let mut instance_methods: Vec<&ClassMethod> = Vec::new();
        let mut static_methods: Vec<&ClassMethod> = Vec::new();
        let mut instance_fields: Vec<&ClassProperty> = Vec::new();
        let mut static_fields: Vec<&ClassProperty> = Vec::new();
        let mut static_blocks: Vec<&crate::ast::BlockStatement> = Vec::new();
        let mut instance_private_fields: Vec<&ClassProperty> = Vec::new();
        let mut static_private_fields: Vec<&ClassProperty> = Vec::new();
        let mut instance_private_methods: Vec<&ClassMethod> = Vec::new();
        let mut static_private_methods: Vec<&ClassMethod> = Vec::new();
        let mut private_members = rustc_hash::FxHashMap::default();

        for member in &class.body.members {
            match member {
                ClassMember::Constructor(ctor) => {
                    constructor = Some(ctor);
                }
                ClassMember::Method(method) => {
                    if let ObjectPropertyKey::PrivateIdentifier(id) = &method.key {
                        // This is a private method
                        private_members.insert(
                            id.name.cheap_clone(),
                            super::PrivateMemberInfo {
                                is_method: true,
                                is_static: method.static_,
                            },
                        );
                        if method.static_ {
                            static_private_methods.push(method);
                        } else {
                            instance_private_methods.push(method);
                        }
                    } else {
                        // Regular public method
                        if method.static_ {
                            static_methods.push(method);
                        } else {
                            instance_methods.push(method);
                        }
                    }
                }
                ClassMember::Property(prop) => {
                    if let ObjectPropertyKey::PrivateIdentifier(id) = &prop.key {
                        // This is a private field
                        private_members.insert(
                            id.name.cheap_clone(),
                            super::PrivateMemberInfo {
                                is_method: false,
                                is_static: prop.static_,
                            },
                        );
                        if prop.static_ {
                            static_private_fields.push(prop);
                        } else {
                            instance_private_fields.push(prop);
                        }
                    } else {
                        // Regular public field
                        if prop.static_ {
                            static_fields.push(prop);
                        } else {
                            instance_fields.push(prop);
                        }
                    }
                }
                ClassMember::StaticBlock(block) => {
                    static_blocks.push(block);
                }
            }
        }

        // Push class context for private member access
        self.class_context_stack.push(super::ClassContext {
            brand: class_brand,
            private_members,
        });

        // Compile constructor (or create default one)
        let ctor_chunk = if let Some(ctor) = constructor {
            self.compile_constructor_body(
                ctor,
                &instance_fields,
                &instance_private_fields,
                &instance_private_methods,
                class_brand,
                class_name.clone(),
            )?
        } else {
            self.compile_default_constructor(
                &instance_fields,
                &instance_private_fields,
                &instance_private_methods,
                class_brand,
                class_name.clone(),
            )?
        };

        let ctor_chunk_idx = self.builder.add_chunk(ctor_chunk)?;

        // Create constructor register and emit CreateClosure
        let ctor_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::CreateClosure {
            dst: ctor_reg,
            chunk_idx: ctor_chunk_idx,
        });

        // Compile superclass if present
        let super_reg = self.builder.alloc_register()?;
        if let Some(super_class) = &class.super_class {
            self.compile_expression(super_class, super_reg)?;
        } else {
            self.builder.emit(Op::LoadUndefined { dst: super_reg });
        }

        // Create the class
        self.builder.emit(Op::CreateClass {
            dst,
            constructor: ctor_reg,
            super_class: super_reg,
        });

        self.builder.free_register(ctor_reg);
        self.builder.free_register(super_reg);

        // Define instance methods
        for method in &instance_methods {
            self.compile_class_method(dst, method, false)?;
        }

        // Define static methods
        for method in &static_methods {
            self.compile_class_method(dst, method, true)?;
        }

        // Initialize static fields (on the class constructor itself)
        for field in &static_fields {
            self.compile_static_field_initializer(dst, field)?;
        }

        // Before running static blocks, bind the class name so code in static blocks
        // can reference the class by name (e.g., `Config.value = 42`)
        // Only create inner binding for explicit class names, not inferred names.
        // For `var C = class {}`, the binding is handled by the var declaration.
        // For `class C {}` or `var x = class C {}`, C needs an inner immutable binding.
        // If decorators are present, the binding must be mutable so we can update it
        // after decorators are applied (since decorators can replace the class).
        let has_decorators = !decorator_regs.is_empty();
        if has_explicit_name {
            if let Some(ref name) = class_name {
                let name_idx = self.builder.add_string(name.cheap_clone())?;
                self.builder.emit(Op::DeclareVar {
                    name: name_idx,
                    init: dst,
                    mutable: has_decorators, // mutable if decorators might replace the class
                });
            }
        }

        // Execute static blocks with `this` bound to the class constructor
        for block in &static_blocks {
            self.compile_static_block(dst, block)?;
        }

        // Initialize static private fields (on the class constructor itself)
        for field in &static_private_fields {
            self.compile_static_private_field_initializer(dst, field, class_brand)?;
        }

        // Define static private methods
        for method in &static_private_methods {
            self.compile_private_method(dst, method, true, class_brand)?;
        }

        // Pop class context
        self.class_context_stack.pop();

        // Apply class decorators (in reverse order, bottom-to-top)
        // Decorators are already in reverse order in decorator_regs
        if !decorator_regs.is_empty() {
            // Get class name constant index (or MAX for no name)
            let class_name_idx = if let Some(ref name) = class_name {
                self.builder.add_string(name.cheap_clone())?
            } else {
                super::bytecode::ConstantIndex::MAX
            };

            for dec_reg in decorator_regs {
                self.builder.emit(Op::ApplyClassDecorator {
                    class: dst,
                    decorator: dec_reg,
                    class_name: class_name_idx,
                });
                self.builder.free_register(dec_reg);
            }

            // Update the class name binding with the final decorated class
            // (decorators may have replaced the class)
            if has_explicit_name {
                if let Some(ref name) = class_name {
                    let name_idx = self.builder.add_string(name.cheap_clone())?;
                    self.builder.emit(Op::SetVar {
                        name: name_idx,
                        src: dst,
                    });
                }
            }
        }

        Ok(())
    }

    /// Compile a class method and emit DefineMethod/DefineAccessor
    fn compile_class_method(
        &mut self,
        class_reg: super::bytecode::Register,
        method: &ClassMethod,
        is_static: bool,
    ) -> Result<(), JsError> {
        // Get method name
        let method_name: JsString = match &method.key {
            ObjectPropertyKey::Identifier(id) => id.name.cheap_clone(),
            ObjectPropertyKey::String(s) => s.value.cheap_clone(),
            ObjectPropertyKey::Number(lit) => {
                // Convert number to string for method name
                use crate::ast::LiteralValue;
                match &lit.value {
                    LiteralValue::Number(n) => JsString::from(crate::value::number_to_string(*n)),
                    _ => return Err(JsError::syntax_error_simple("Invalid method key")),
                }
            }
            ObjectPropertyKey::Computed(_) => {
                return Err(JsError::syntax_error_simple(
                    "Computed method names not yet supported in bytecode compiler",
                ))
            }
            ObjectPropertyKey::PrivateIdentifier(id) => id.name.cheap_clone(),
        };

        let name_idx = self.builder.add_string(method_name.cheap_clone())?;

        // Compile method body
        let func = &method.value;
        let method_chunk = self.compile_function_body(
            &func.params,
            &func.body.body,
            Some(method_name),
            func.generator,
            func.async_,
            false,
        )?;

        let chunk_idx = self.builder.add_chunk(method_chunk)?;

        // Allocate register for method function
        let method_reg = self.builder.alloc_register()?;

        // Create the method function
        if func.generator && func.async_ {
            self.builder.emit(Op::CreateAsyncGenerator {
                dst: method_reg,
                chunk_idx,
            });
        } else if func.generator {
            self.builder.emit(Op::CreateGenerator {
                dst: method_reg,
                chunk_idx,
            });
        } else if func.async_ {
            self.builder.emit(Op::CreateAsync {
                dst: method_reg,
                chunk_idx,
            });
        } else {
            self.builder.emit(Op::CreateClosure {
                dst: method_reg,
                chunk_idx,
            });
        }

        // Emit DefineMethod or DefineAccessor based on method kind
        match method.kind {
            MethodKind::Method => {
                self.builder.emit(Op::DefineMethod {
                    class: class_reg,
                    name: name_idx,
                    method: method_reg,
                    is_static,
                });
            }
            MethodKind::Get => {
                // Getter only - pass undefined for setter
                let undefined_reg = self.builder.alloc_register()?;
                self.builder.emit(Op::LoadUndefined { dst: undefined_reg });

                self.builder.emit(Op::DefineAccessor {
                    class: class_reg,
                    name: name_idx,
                    getter: method_reg,
                    setter: undefined_reg,
                    is_static,
                });

                self.builder.free_register(undefined_reg);
            }
            MethodKind::Set => {
                // Setter only - pass undefined for getter
                let undefined_reg = self.builder.alloc_register()?;
                self.builder.emit(Op::LoadUndefined { dst: undefined_reg });

                self.builder.emit(Op::DefineAccessor {
                    class: class_reg,
                    name: name_idx,
                    getter: undefined_reg,
                    setter: method_reg,
                    is_static,
                });

                self.builder.free_register(undefined_reg);
            }
        }

        self.builder.free_register(method_reg);
        Ok(())
    }

    /// Compile constructor body
    fn compile_constructor_body(
        &mut self,
        ctor: &ClassConstructor,
        instance_fields: &[&ClassProperty],
        instance_private_fields: &[&ClassProperty],
        instance_private_methods: &[&ClassMethod],
        class_brand: u32,
        name: Option<JsString>,
    ) -> Result<super::BytecodeChunk, JsError> {
        use super::FunctionInfo;

        let mut func_compiler = Compiler::new();

        // Copy the class context so private field access works inside the constructor
        func_compiler.class_context_stack = self.class_context_stack.clone();

        // Reserve registers for parameters
        if !ctor.params.is_empty() {
            func_compiler
                .builder
                .reserve_registers(ctor.params.len() as u8)?;
        }

        // Compile parameter declarations inline (same as compile_function_body)
        let mut param_names = Vec::with_capacity(ctor.params.len());
        for (idx, param) in ctor.params.iter().enumerate() {
            let arg_reg = idx as u8;

            match &param.pattern {
                crate::ast::Pattern::Identifier(id) => {
                    param_names.push(id.name.cheap_clone());
                    let name_idx = func_compiler.builder.add_string(id.name.cheap_clone())?;
                    func_compiler.builder.emit(Op::DeclareVar {
                        name: name_idx,
                        init: arg_reg,
                        mutable: true,
                    });
                }
                crate::ast::Pattern::Rest(rest) => {
                    if let crate::ast::Pattern::Identifier(id) = &*rest.argument {
                        param_names.push(id.name.cheap_clone());
                        let name_idx = func_compiler.builder.add_string(id.name.cheap_clone())?;
                        func_compiler.builder.emit(Op::DeclareVar {
                            name: name_idx,
                            init: arg_reg,
                            mutable: true,
                        });
                    }
                }
                crate::ast::Pattern::Object(_) | crate::ast::Pattern::Array(_) => {
                    param_names.push(JsString::from(format!("__param{}__", idx)));
                    func_compiler.compile_pattern_binding(&param.pattern, arg_reg, true, false)?;
                }
                crate::ast::Pattern::Assignment(assign_pat) => {
                    let actual_value = func_compiler.builder.alloc_register()?;
                    let is_undefined = func_compiler.builder.alloc_register()?;
                    func_compiler
                        .builder
                        .emit(Op::LoadUndefined { dst: is_undefined });
                    func_compiler.builder.emit(Op::StrictEq {
                        dst: is_undefined,
                        left: arg_reg,
                        right: is_undefined,
                    });

                    let skip_default = func_compiler.builder.emit_jump_if_false(is_undefined);
                    func_compiler.builder.free_register(is_undefined);

                    func_compiler.compile_expression(&assign_pat.right, actual_value)?;
                    let skip_arg = func_compiler.builder.emit_jump();

                    func_compiler.builder.patch_jump(skip_default);
                    func_compiler.builder.emit(Op::Move {
                        dst: actual_value,
                        src: arg_reg,
                    });

                    func_compiler.builder.patch_jump(skip_arg);
                    func_compiler.compile_pattern_binding(
                        &assign_pat.left,
                        actual_value,
                        true,
                        false,
                    )?;

                    if let crate::ast::Pattern::Identifier(id) = assign_pat.left.as_ref() {
                        param_names.push(id.name.cheap_clone());
                    } else {
                        param_names.push(JsString::from(format!("__param{}__", idx)));
                    }

                    func_compiler.builder.free_register(actual_value);
                }
            }
        }

        // Compile instance field initializers at the start of constructor
        // These run before the user's constructor body (after super() call if extending)
        for field in instance_fields {
            func_compiler.compile_instance_field_initializer(field)?;
        }

        // Initialize instance private fields
        for field in instance_private_fields {
            func_compiler.compile_instance_private_field_initializer(field, class_brand)?;
        }

        // Install instance private methods on 'this'
        for method in instance_private_methods {
            func_compiler.compile_instance_private_method_initializer(method, class_brand)?;
        }

        // Hoist var declarations in constructor body
        func_compiler.emit_hoisted_declarations(&ctor.body.body)?;

        // Compile constructor body
        for stmt in ctor.body.body.iter() {
            func_compiler.compile_statement_impl(stmt)?;
        }

        // Return this implicitly (constructor returns `this`)
        let this_reg = func_compiler.builder.alloc_register()?;
        func_compiler.builder.emit(Op::LoadThis { dst: this_reg });
        func_compiler.builder.emit(Op::Return { value: this_reg });

        let mut chunk = func_compiler.builder.finish();
        chunk.function_info = Some(FunctionInfo {
            name,
            param_count: ctor.params.len(),
            param_names,
            rest_param: None,
            is_generator: false,
            is_async: false,
            is_arrow: false,
            uses_arguments: false,
            uses_this: true, // constructors use this
        });

        Ok(chunk)
    }

    /// Compile default constructor (for classes without explicit constructor)
    fn compile_default_constructor(
        &mut self,
        instance_fields: &[&ClassProperty],
        instance_private_fields: &[&ClassProperty],
        instance_private_methods: &[&ClassMethod],
        class_brand: u32,
        name: Option<JsString>,
    ) -> Result<super::BytecodeChunk, JsError> {
        use super::FunctionInfo;

        let mut func_compiler = Compiler::new();

        // Copy the class context so private field access works inside the constructor
        func_compiler.class_context_stack = self.class_context_stack.clone();

        // Compile instance field initializers
        for field in instance_fields {
            func_compiler.compile_instance_field_initializer(field)?;
        }

        // Initialize instance private fields
        for field in instance_private_fields {
            func_compiler.compile_instance_private_field_initializer(field, class_brand)?;
        }

        // Install instance private methods on 'this'
        for method in instance_private_methods {
            func_compiler.compile_instance_private_method_initializer(method, class_brand)?;
        }

        // Return this
        let this_reg = func_compiler.builder.alloc_register()?;
        func_compiler.builder.emit(Op::LoadThis { dst: this_reg });
        func_compiler.builder.emit(Op::Return { value: this_reg });

        let mut chunk = func_compiler.builder.finish();
        chunk.function_info = Some(FunctionInfo {
            name,
            param_count: 0,
            param_names: vec![],
            rest_param: None,
            is_generator: false,
            is_async: false,
            is_arrow: false,
            uses_arguments: false,
            uses_this: true, // constructors use this
        });

        Ok(chunk)
    }

    /// Compile instance field initializer (this.field = value)
    fn compile_instance_field_initializer(&mut self, field: &ClassProperty) -> Result<(), JsError> {
        // Get field name
        let field_name: JsString = match &field.key {
            ObjectPropertyKey::Identifier(id) => id.name.cheap_clone(),
            ObjectPropertyKey::String(s) => s.value.cheap_clone(),
            _ => return Ok(()), // Skip computed/private for now
        };

        let name_idx = self.builder.add_string(field_name)?;

        // Get this
        let this_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::LoadThis { dst: this_reg });

        // Compile field initializer or use undefined
        let value_reg = self.builder.alloc_register()?;
        if let Some(init) = &field.value {
            self.compile_expression(init, value_reg)?;
        } else {
            self.builder.emit(Op::LoadUndefined { dst: value_reg });
        }

        // Set property on this
        self.builder.emit(Op::SetPropertyConst {
            obj: this_reg,
            key: name_idx,
            value: value_reg,
        });

        self.builder.free_register(value_reg);
        self.builder.free_register(this_reg);
        Ok(())
    }

    /// Compile a static field initializer (sets property on class constructor)
    fn compile_static_field_initializer(
        &mut self,
        class_reg: super::bytecode::Register,
        field: &ClassProperty,
    ) -> Result<(), JsError> {
        // Get field name
        let field_name: JsString = match &field.key {
            ObjectPropertyKey::Identifier(id) => id.name.cheap_clone(),
            ObjectPropertyKey::String(s) => s.value.cheap_clone(),
            _ => return Ok(()), // Skip computed/private for now
        };

        let name_idx = self.builder.add_string(field_name)?;

        // Compile field initializer or use undefined
        let value_reg = self.builder.alloc_register()?;
        if let Some(init) = &field.value {
            self.compile_expression(init, value_reg)?;
        } else {
            self.builder.emit(Op::LoadUndefined { dst: value_reg });
        }

        // Set property on class constructor
        self.builder.emit(Op::SetPropertyConst {
            obj: class_reg,
            key: name_idx,
            value: value_reg,
        });

        self.builder.free_register(value_reg);
        Ok(())
    }

    /// Compile an instance private field initializer (this.#field = value)
    fn compile_instance_private_field_initializer(
        &mut self,
        field: &ClassProperty,
        class_brand: u32,
    ) -> Result<(), JsError> {
        // Get field name
        let field_name: JsString = match &field.key {
            ObjectPropertyKey::PrivateIdentifier(id) => id.name.cheap_clone(),
            _ => return Ok(()), // Should only be called for private fields
        };

        let name_idx = self.builder.add_string(field_name)?;

        // Get this
        let this_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::LoadThis { dst: this_reg });

        // Compile field initializer or use undefined
        let value_reg = self.builder.alloc_register()?;
        if let Some(init) = &field.value {
            self.compile_expression(init, value_reg)?;
        } else {
            self.builder.emit(Op::LoadUndefined { dst: value_reg });
        }

        // Define private field on this
        self.builder.emit(Op::DefinePrivateField {
            obj: this_reg,
            class_brand,
            field_name: name_idx,
            value: value_reg,
        });

        self.builder.free_register(value_reg);
        self.builder.free_register(this_reg);
        Ok(())
    }

    /// Install an instance private method on 'this' during construction
    fn compile_instance_private_method_initializer(
        &mut self,
        method: &ClassMethod,
        class_brand: u32,
    ) -> Result<(), JsError> {
        // Get method name
        let method_name: JsString = match &method.key {
            ObjectPropertyKey::PrivateIdentifier(id) => id.name.cheap_clone(),
            _ => return Ok(()), // Should only be called for private methods
        };

        let name_idx = self.builder.add_string(method_name.cheap_clone())?;

        // Compile method body
        let func = &method.value;
        let method_chunk = self.compile_function_body(
            &func.params,
            &func.body.body,
            Some(method_name),
            func.generator,
            func.async_,
            false,
        )?;

        let chunk_idx = self.builder.add_chunk(method_chunk)?;

        // Create method function
        let method_reg = self.builder.alloc_register()?;
        if func.generator && func.async_ {
            self.builder.emit(Op::CreateAsyncGenerator {
                dst: method_reg,
                chunk_idx,
            });
        } else if func.generator {
            self.builder.emit(Op::CreateGenerator {
                dst: method_reg,
                chunk_idx,
            });
        } else if func.async_ {
            self.builder.emit(Op::CreateAsync {
                dst: method_reg,
                chunk_idx,
            });
        } else {
            self.builder.emit(Op::CreateClosure {
                dst: method_reg,
                chunk_idx,
            });
        }

        // Get this
        let this_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::LoadThis { dst: this_reg });

        // Define private method on this
        self.builder.emit(Op::DefinePrivateField {
            obj: this_reg,
            class_brand,
            field_name: name_idx,
            value: method_reg,
        });

        self.builder.free_register(this_reg);
        self.builder.free_register(method_reg);
        Ok(())
    }

    /// Compile a static private field initializer
    fn compile_static_private_field_initializer(
        &mut self,
        class_reg: super::bytecode::Register,
        field: &ClassProperty,
        class_brand: u32,
    ) -> Result<(), JsError> {
        // Get field name
        let field_name: JsString = match &field.key {
            ObjectPropertyKey::PrivateIdentifier(id) => id.name.cheap_clone(),
            _ => return Ok(()), // Should only be called for private fields
        };

        let name_idx = self.builder.add_string(field_name)?;

        // Compile field initializer or use undefined
        let value_reg = self.builder.alloc_register()?;
        if let Some(init) = &field.value {
            self.compile_expression(init, value_reg)?;
        } else {
            self.builder.emit(Op::LoadUndefined { dst: value_reg });
        }

        // Define private field on class constructor
        self.builder.emit(Op::DefinePrivateField {
            obj: class_reg,
            class_brand,
            field_name: name_idx,
            value: value_reg,
        });

        self.builder.free_register(value_reg);
        Ok(())
    }

    /// Compile a private method
    fn compile_private_method(
        &mut self,
        class_reg: super::bytecode::Register,
        method: &ClassMethod,
        is_static: bool,
        class_brand: u32,
    ) -> Result<(), JsError> {
        // Get method name
        let method_name: JsString = match &method.key {
            ObjectPropertyKey::PrivateIdentifier(id) => id.name.cheap_clone(),
            _ => return Ok(()), // Should only be called for private methods
        };

        let name_idx = self.builder.add_string(method_name.cheap_clone())?;

        // Compile method body
        let func = &method.value;
        let method_chunk = self.compile_function_body(
            &func.params,
            &func.body.body,
            Some(method_name),
            func.generator,
            func.async_,
            false,
        )?;

        let chunk_idx = self.builder.add_chunk(method_chunk)?;

        // Create method function
        let method_reg = self.builder.alloc_register()?;
        if func.generator && func.async_ {
            self.builder.emit(Op::CreateAsyncGenerator {
                dst: method_reg,
                chunk_idx,
            });
        } else if func.generator {
            self.builder.emit(Op::CreateGenerator {
                dst: method_reg,
                chunk_idx,
            });
        } else if func.async_ {
            self.builder.emit(Op::CreateAsync {
                dst: method_reg,
                chunk_idx,
            });
        } else {
            self.builder.emit(Op::CreateClosure {
                dst: method_reg,
                chunk_idx,
            });
        }

        if is_static {
            // Define on class constructor directly
            self.builder.emit(Op::DefinePrivateField {
                obj: class_reg,
                class_brand,
                field_name: name_idx,
                value: method_reg,
            });
        } else {
            // Store for later installation on instances
            self.builder.emit(Op::DefinePrivateMethod {
                class: class_reg,
                class_brand,
                method_name: name_idx,
                method: method_reg,
                is_static,
            });
        }

        self.builder.free_register(method_reg);
        Ok(())
    }

    /// Compile a static block - compiles the block body as a function and calls it with `this` = class
    fn compile_static_block(
        &mut self,
        class_reg: super::bytecode::Register,
        block: &crate::ast::BlockStatement,
    ) -> Result<(), JsError> {
        // Compile the static block body as an anonymous function
        // The function will be called immediately with `this` bound to the class constructor
        let empty_params: [crate::ast::FunctionParam; 0] = [];
        let block_chunk =
            self.compile_function_body(&empty_params, &block.body, None, false, false, false)?;
        let chunk_idx = self.builder.add_chunk(block_chunk)?;

        // Create a closure for the static block
        let block_fn_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::CreateClosure {
            dst: block_fn_reg,
            chunk_idx,
        });

        // Call the static block with `this` = class constructor
        // No arguments needed, so we use a dummy register for args_start
        let result_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::Call {
            dst: result_reg,
            callee: block_fn_reg,
            this: class_reg,
            args_start: 0, // Register is u8
            argc: 0,
        });

        self.builder.free_register(result_reg);
        self.builder.free_register(block_fn_reg);
        Ok(())
    }

    /// Compile an enum declaration
    /// TypeScript enums compile to an object with forward and reverse mappings
    fn compile_enum_declaration(
        &mut self,
        decl: &crate::ast::EnumDeclaration,
    ) -> Result<(), JsError> {
        self.builder.set_span(decl.span);

        // Create the enum object
        let enum_obj = self.builder.alloc_register()?;
        self.builder.emit(Op::CreateObject { dst: enum_obj });

        // Declare the enum variable FIRST so member initializers can reference prior members
        // via EnumName.MemberName or just MemberName (for const enums)
        let enum_name_idx = self.builder.add_string(decl.id.name.cheap_clone())?;
        self.builder.emit(Op::DeclareVar {
            name: enum_name_idx,
            init: enum_obj,
            mutable: true, // Enums are mutable like objects
        });

        // Track the current numeric value for auto-increment
        let mut current_value: i64 = 0;
        let value_reg = self.builder.alloc_register()?;
        let key_reg = self.builder.alloc_register()?;

        // Track prior member names for rewriting identifier references
        let mut prior_members: Vec<JsString> = Vec::new();

        for member in &decl.members {
            let member_name = member.id.name.cheap_clone();
            let name_idx = self.builder.add_string(member_name.cheap_clone())?;

            if let Some(ref init) = member.initializer {
                // Compile the initializer expression, rewriting references to prior enum members
                self.compile_enum_init_expression(init, value_reg, enum_obj, &prior_members)?;

                // Try to compute the numeric value for auto-increment
                // This is a simplified version - in reality, we'd need const evaluation
                if let crate::ast::Expression::Literal(lit) = init {
                    if let crate::ast::LiteralValue::Number(n) = &lit.value {
                        current_value = *n as i64 + 1;
                    }
                }
            } else {
                // Use auto-increment value
                self.builder.emit(Op::LoadInt {
                    dst: value_reg,
                    value: current_value as i32,
                });
                current_value += 1;
            }

            // Add this member to prior members for subsequent initializers
            prior_members.push(member_name.cheap_clone());

            // Set forward mapping: EnumName.MemberName = value
            self.builder.emit(Op::SetPropertyConst {
                obj: enum_obj,
                key: name_idx,
                value: value_reg,
            });

            // Set reverse mapping for numeric values: EnumName[value] = MemberName
            // Only for numeric values (not string enums)
            // We need to check if value is numeric at runtime for mixed enums
            let is_numeric = match &member.initializer {
                None => true,
                Some(init) => {
                    // Check for numeric literal
                    matches!(
                        init,
                        crate::ast::Expression::Literal(crate::ast::Literal { value: crate::ast::LiteralValue::Number(_), .. })
                    ) ||
                    // Check for unary minus of numeric literal (e.g., -10)
                    matches!(
                        init,
                        crate::ast::Expression::Unary(unary)
                            if unary.operator == crate::ast::UnaryOp::Minus
                            && matches!(
                                unary.argument.as_ref(),
                                crate::ast::Expression::Literal(crate::ast::Literal { value: crate::ast::LiteralValue::Number(_), .. })
                            )
                    )
                }
            };
            if is_numeric {
                // Load the member name as a string value
                self.builder.emit_load_string(key_reg, member_name)?;

                // Set reverse mapping: EnumName[value] = "MemberName"
                self.builder.emit(Op::SetProperty {
                    obj: enum_obj,
                    key: value_reg,
                    value: key_reg,
                });
            }
        }

        self.builder.free_register(key_reg);
        self.builder.free_register(value_reg);
        self.builder.free_register(enum_obj);
        Ok(())
    }

    /// Compile a namespace declaration
    fn compile_namespace_declaration(
        &mut self,
        decl: &crate::ast::NamespaceDeclaration,
    ) -> Result<(), JsError> {
        self.builder.set_span(decl.span);

        let name_idx = self.builder.add_string(decl.id.name.cheap_clone())?;
        let ns_obj = self.builder.alloc_register()?;

        // Check if namespace already exists (for merging)
        // Try to get existing namespace, use it if found, otherwise create new
        let existing_reg = self.builder.alloc_register()?;

        // Try to get the existing namespace variable (returns undefined if not found)
        self.builder.emit(Op::TryGetVar {
            dst: existing_reg,
            name: name_idx,
        });

        // Check if existing is undefined - use JumpIfNullish since undefined is nullish
        // If undefined/null, jump to create new object, else use existing
        let jump_to_create = self.builder.emit_jump_if_nullish(existing_reg);

        // Use existing namespace
        self.builder.emit(Op::Move {
            dst: ns_obj,
            src: existing_reg,
        });
        let jump_to_end = self.builder.emit_jump();

        // Create new namespace object
        self.builder.patch_jump(jump_to_create);
        self.builder.emit(Op::CreateObject { dst: ns_obj });
        self.builder.emit(Op::DeclareVar {
            name: name_idx,
            init: ns_obj,
            mutable: true,
        });

        self.builder.patch_jump(jump_to_end);

        // Free temporary registers
        self.builder.free_register(existing_reg);

        // Push a new scope for the namespace body
        self.builder.emit(Op::PushScope);

        // Compile the namespace body statements
        for stmt in decl.body.iter() {
            self.compile_statement_impl(stmt)?;

            // If the statement exports something, add it to the namespace object
            // For now, we handle exported declarations by adding them to the namespace
            if let Statement::Export(export) = stmt {
                if let Some(ref decl) = export.declaration {
                    self.add_export_to_namespace(ns_obj, decl)?;
                }
            }
        }

        // Pop the namespace scope
        self.builder.emit(Op::PopScope);

        self.builder.free_register(ns_obj);
        Ok(())
    }

    /// Add an exported declaration to a namespace object
    fn add_export_to_namespace(
        &mut self,
        ns_obj: super::Register,
        decl: &Statement,
    ) -> Result<(), JsError> {
        match decl {
            Statement::VariableDeclaration(var_decl) => {
                for declarator in var_decl.declarations.iter() {
                    if let crate::ast::Pattern::Identifier(id) = &declarator.id {
                        let value_reg = self.builder.alloc_register()?;
                        let name_idx = self.builder.add_string(id.name.cheap_clone())?;
                        self.builder.emit(Op::GetVar {
                            dst: value_reg,
                            name: name_idx,
                        });
                        self.builder.emit(Op::SetPropertyConst {
                            obj: ns_obj,
                            key: name_idx,
                            value: value_reg,
                        });
                        self.builder.free_register(value_reg);
                    }
                }
            }
            Statement::FunctionDeclaration(func_decl) => {
                if let Some(ref id) = func_decl.id {
                    let value_reg = self.builder.alloc_register()?;
                    let name_idx = self.builder.add_string(id.name.cheap_clone())?;
                    self.builder.emit(Op::GetVar {
                        dst: value_reg,
                        name: name_idx,
                    });
                    self.builder.emit(Op::SetPropertyConst {
                        obj: ns_obj,
                        key: name_idx,
                        value: value_reg,
                    });
                    self.builder.free_register(value_reg);
                }
            }
            Statement::ClassDeclaration(class_decl) => {
                if let Some(ref id) = class_decl.id {
                    let value_reg = self.builder.alloc_register()?;
                    let name_idx = self.builder.add_string(id.name.cheap_clone())?;
                    self.builder.emit(Op::GetVar {
                        dst: value_reg,
                        name: name_idx,
                    });
                    self.builder.emit(Op::SetPropertyConst {
                        obj: ns_obj,
                        key: name_idx,
                        value: value_reg,
                    });
                    self.builder.free_register(value_reg);
                }
            }
            Statement::EnumDeclaration(enum_decl) => {
                let value_reg = self.builder.alloc_register()?;
                let name_idx = self.builder.add_string(enum_decl.id.name.cheap_clone())?;
                self.builder.emit(Op::GetVar {
                    dst: value_reg,
                    name: name_idx,
                });
                self.builder.emit(Op::SetPropertyConst {
                    obj: ns_obj,
                    key: name_idx,
                    value: value_reg,
                });
                self.builder.free_register(value_reg);
            }
            Statement::NamespaceDeclaration(nested_ns) => {
                // Add nested namespace to parent namespace
                let value_reg = self.builder.alloc_register()?;
                let name_idx = self.builder.add_string(nested_ns.id.name.cheap_clone())?;
                self.builder.emit(Op::GetVar {
                    dst: value_reg,
                    name: name_idx,
                });
                self.builder.emit(Op::SetPropertyConst {
                    obj: ns_obj,
                    key: name_idx,
                    value: value_reg,
                });
                self.builder.free_register(value_reg);
            }
            _ => {}
        }
        Ok(())
    }

    /// Compile an enum initializer expression, rewriting references to prior enum members
    /// as property accesses on the enum object.
    fn compile_enum_init_expression(
        &mut self,
        expr: &crate::ast::Expression,
        dst: super::Register,
        enum_obj: super::Register,
        prior_members: &[JsString],
    ) -> Result<(), JsError> {
        use crate::ast::Expression;

        match expr {
            // Check if this is an identifier that matches a prior enum member
            Expression::Identifier(id) => {
                if prior_members.iter().any(|m| m.as_str() == id.name.as_str()) {
                    // This is a reference to a prior member - load from enum object
                    let name_idx = self.builder.add_string(id.name.cheap_clone())?;
                    self.builder.emit(Op::GetPropertyConst {
                        dst,
                        obj: enum_obj,
                        key: name_idx,
                    });
                    Ok(())
                } else {
                    // Not a prior member - compile normally
                    self.compile_expression(expr, dst)
                }
            }

            // For binary expressions, recursively handle operands
            Expression::Binary(bin) => {
                let left_reg = self.builder.alloc_register()?;
                let right_reg = self.builder.alloc_register()?;

                self.compile_enum_init_expression(&bin.left, left_reg, enum_obj, prior_members)?;
                self.compile_enum_init_expression(&bin.right, right_reg, enum_obj, prior_members)?;

                // Now emit the binary operation
                self.compile_binary_op(bin.operator, dst, left_reg, right_reg)?;

                self.builder.free_register(right_reg);
                self.builder.free_register(left_reg);
                Ok(())
            }

            // For unary expressions, recursively handle operand
            Expression::Unary(unary) => {
                let arg_reg = self.builder.alloc_register()?;
                self.compile_enum_init_expression(
                    &unary.argument,
                    arg_reg,
                    enum_obj,
                    prior_members,
                )?;

                // Emit the unary operation
                match unary.operator {
                    crate::ast::UnaryOp::Minus => {
                        self.builder.emit(Op::Neg { dst, src: arg_reg });
                    }
                    crate::ast::UnaryOp::Plus => {
                        self.builder.emit(Op::Plus { dst, src: arg_reg });
                    }
                    crate::ast::UnaryOp::Not => {
                        self.builder.emit(Op::Not { dst, src: arg_reg });
                    }
                    crate::ast::UnaryOp::BitNot => {
                        self.builder.emit(Op::BitNot { dst, src: arg_reg });
                    }
                    _ => {
                        // Fall back to normal compilation for other unary ops
                        self.builder.free_register(arg_reg);
                        return self.compile_expression(expr, dst);
                    }
                }

                self.builder.free_register(arg_reg);
                Ok(())
            }

            // For parenthesized expressions, handle the inner expression
            Expression::Parenthesized(expr, _) => {
                self.compile_enum_init_expression(expr, dst, enum_obj, prior_members)
            }

            // For other expressions (literals, etc.), compile normally
            _ => self.compile_expression(expr, dst),
        }
    }

    /// Compile a binary operator
    fn compile_binary_op(
        &mut self,
        operator: crate::ast::BinaryOp,
        dst: super::Register,
        left: super::Register,
        right: super::Register,
    ) -> Result<(), JsError> {
        use crate::ast::BinaryOp;
        match operator {
            BinaryOp::BitOr => self.builder.emit(Op::BitOr { dst, left, right }),
            BinaryOp::BitAnd => self.builder.emit(Op::BitAnd { dst, left, right }),
            BinaryOp::BitXor => self.builder.emit(Op::BitXor { dst, left, right }),
            BinaryOp::Add => self.builder.emit(Op::Add { dst, left, right }),
            BinaryOp::Sub => self.builder.emit(Op::Sub { dst, left, right }),
            BinaryOp::Mul => self.builder.emit(Op::Mul { dst, left, right }),
            BinaryOp::Div => self.builder.emit(Op::Div { dst, left, right }),
            BinaryOp::Mod => self.builder.emit(Op::Mod { dst, left, right }),
            BinaryOp::Exp => self.builder.emit(Op::Exp { dst, left, right }),
            BinaryOp::LShift => self.builder.emit(Op::LShift { dst, left, right }),
            BinaryOp::RShift => self.builder.emit(Op::RShift { dst, left, right }),
            BinaryOp::URShift => self.builder.emit(Op::URShift { dst, left, right }),
            _ => {
                return Err(JsError::internal_error(format!(
                    "Unsupported binary operator in enum initializer: {:?}",
                    operator
                )))
            }
        };
        Ok(())
    }
}
