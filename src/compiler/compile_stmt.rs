//! Statement compilation
//!
//! Compiles AST statements to bytecode instructions.

use super::bytecode::Op;
use super::Compiler;
use crate::ast::{
    BlockStatement, BreakStatement, ContinueStatement, DoWhileStatement, ForInOfLeft,
    ForInStatement, ForInit, ForOfStatement, ForStatement, IfStatement, LabeledStatement,
    ReturnStatement, Statement, SwitchStatement, ThrowStatement, TryStatement, VariableDeclaration,
    VariableKind, WhileStatement,
};
use crate::error::JsError;
use crate::value::{CheapClone, JsString};

impl Compiler {
    /// Compile a statement
    pub fn compile_statement(&mut self, stmt: &Statement) -> Result<(), JsError> {
        match stmt {
            Statement::Expression(expr_stmt) => {
                self.builder.set_span(expr_stmt.span);
                let dst = self.builder.alloc_register()?;
                self.compile_expression(&expr_stmt.expression, dst)?;
                // Discard the result
                self.builder.free_register(dst);
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

            // TypeScript declarations - no-ops at runtime
            Statement::TypeAlias(_)
            | Statement::InterfaceDeclaration(_)
            | Statement::EnumDeclaration(_)
            | Statement::NamespaceDeclaration(_) => {
                // TODO: Enum declarations should create runtime objects
                Ok(())
            }

            // Module declarations
            Statement::Import(_) | Statement::Export(_) => {
                // TODO: Implement module compilation
                Err(JsError::syntax_error_simple(
                    "Module imports/exports not yet supported in bytecode compiler",
                ))
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

            // Compile initializer (or undefined)
            let init_reg = self.builder.alloc_register()?;
            if let Some(init) = &declarator.init {
                self.compile_expression(init, init_reg)?;
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

        // Compile statements
        for stmt in block.body.iter() {
            self.compile_statement(stmt)?;
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
        self.compile_statement(&if_stmt.consequent)?;

        if let Some(alternate) = &if_stmt.alternate {
            // Jump over else block
            let jump_to_end = self.builder.emit_jump();

            // Patch jump to else
            self.builder.patch_jump(jump_to_else);

            // Compile alternate
            self.compile_statement(alternate)?;

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
        self.compile_statement(&while_stmt.body)?;

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
        self.compile_statement(&do_while.body)?;

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
        self.compile_statement(&for_stmt.body)?;

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

    /// Compile a for-in statement
    fn compile_for_in(&mut self, for_in: &ForInStatement) -> Result<(), JsError> {
        self.builder.set_span(for_in.span);

        // Push scope
        self.builder.emit(Op::PushScope);

        // Compile the right side (object to iterate)
        let obj_reg = self.builder.alloc_register()?;
        self.compile_expression(&for_in.right, obj_reg)?;

        // Get iterator for keys
        let iter_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::GetIterator {
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
        self.compile_statement(&for_in.body)?;

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

        // For await...of, await the result
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

        // Bind to left side
        self.compile_for_in_of_left(&for_of.left, value_reg)?;

        // Compile body
        self.compile_statement(&for_of.body)?;

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
                self.compile_statement(stmt)?;
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
                self.compile_statement(stmt)?;
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
                self.compile_statement(stmt)?;
            }
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
        self.compile_statement(&labeled.body)?;

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

        // Compile parameter declarations
        // Parameters will be passed via registers and need to be bound to the environment
        let mut param_names = Vec::with_capacity(params.len());
        let mut rest_param = None;

        for (idx, param) in params.iter().enumerate() {
            match &param.pattern {
                crate::ast::Pattern::Identifier(id) => {
                    param_names.push(id.name.cheap_clone());

                    // Load argument from register and declare variable
                    let arg_reg = idx as u8;
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
                        let arg_reg = idx as u8;
                        let name_idx = func_compiler.builder.add_string(id.name.cheap_clone())?;
                        func_compiler.builder.emit(Op::DeclareVar {
                            name: name_idx,
                            init: arg_reg,
                            mutable: true,
                        });
                    }
                }
                // TODO: Handle destructuring patterns
                _ => {}
            }
        }

        // Compile the body statements
        for stmt in body {
            func_compiler.compile_statement(stmt)?;
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
    fn compile_class_declaration(
        &mut self,
        _class: &crate::ast::ClassDeclaration,
    ) -> Result<(), JsError> {
        // TODO: Implement class declaration compilation
        Err(JsError::syntax_error_simple(
            "Class declarations not yet supported in bytecode compiler",
        ))
    }
}
