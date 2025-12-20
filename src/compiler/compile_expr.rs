//! Expression compilation
//!
//! Compiles AST expressions to bytecode instructions.

use super::bytecode::{ConstantIndex, Op, Register};
use super::Compiler;
use crate::ast::{
    Argument, ArrayElement, AssignmentOp, AssignmentTarget, BinaryOp, Expression, LiteralValue,
    LogicalOp, MemberProperty, ObjectProperty, ObjectPropertyKey, PropertyKind, UnaryOp, UpdateOp,
};
use crate::error::JsError;
use crate::value::CheapClone;

/// Information about a member key (const or computed)
enum MemberKeyInfo {
    Const(ConstantIndex),
    Computed(Register),
}

impl Compiler {
    /// Compile an expression, placing result in the specified destination register
    pub fn compile_expression(&mut self, expr: &Expression, dst: Register) -> Result<(), JsError> {
        self.builder.set_span(expr.span());

        match expr {
            Expression::Literal(lit) => self.compile_literal(&lit.value, dst),

            Expression::Identifier(id) => {
                // Special handling for magic identifiers
                if id.name.as_str() == "arguments" {
                    // Use LoadArguments opcode for accessing the arguments object
                    self.builder.emit(Op::LoadArguments { dst });
                    return Ok(());
                }

                let name_idx = self.builder.add_string(id.name.cheap_clone())?;
                self.builder.emit(Op::GetVar {
                    dst,
                    name: name_idx,
                });
                Ok(())
            }

            Expression::This(_) => {
                self.builder.emit(Op::LoadThis { dst });
                Ok(())
            }

            Expression::Super(_) => {
                // Super is typically used in member access or calls, handled separately
                Err(JsError::syntax_error_simple(
                    "'super' keyword is only valid inside a class",
                ))
            }

            Expression::Array(arr) => self.compile_array_expression(arr, dst),

            Expression::Object(obj) => self.compile_object_expression(obj, dst),

            Expression::Function(func) => self.compile_function_expression(func, dst),

            Expression::ArrowFunction(arrow) => self.compile_arrow_function(arrow, dst),

            Expression::Class(class) => self.compile_class_expression(class, dst),

            Expression::Template(template) => self.compile_template_literal(template, dst),

            Expression::TaggedTemplate(tagged) => self.compile_tagged_template(tagged, dst),

            Expression::Unary(unary) => self.compile_unary_expression(unary, dst),

            Expression::Binary(binary) => self.compile_binary_expression(binary, dst),

            Expression::Logical(logical) => self.compile_logical_expression(logical, dst),

            Expression::Conditional(cond) => self.compile_conditional_expression(cond, dst),

            Expression::Assignment(assign) => self.compile_assignment_expression(assign, dst),

            Expression::Update(update) => self.compile_update_expression(update, dst),

            Expression::Sequence(seq) => self.compile_sequence_expression(seq, dst),

            Expression::Member(member) => self.compile_member_expression(member, dst),

            Expression::OptionalChain(opt) => self.compile_optional_chain(opt, dst),

            Expression::Call(call) => self.compile_call_expression(call, dst),

            Expression::New(new) => self.compile_new_expression(new, dst),

            Expression::Spread(spread) => {
                // Spread is handled specially in array/object/call contexts
                self.compile_expression(&spread.argument, dst)
            }

            Expression::Yield(yield_expr) => self.compile_yield_expression(yield_expr, dst),

            Expression::Await(await_expr) => self.compile_await_expression(await_expr, dst),

            Expression::TypeAssertion(ta) => {
                // Type assertions are no-ops at runtime
                self.compile_expression(&ta.expression, dst)
            }

            Expression::NonNull(nn) => {
                // Non-null assertions are no-ops at runtime
                self.compile_expression(&nn.expression, dst)
            }

            Expression::Parenthesized(inner, _) => self.compile_expression(inner, dst),
        }
    }

    /// Compile a literal value
    pub(crate) fn compile_literal(
        &mut self,
        value: &LiteralValue,
        dst: Register,
    ) -> Result<(), JsError> {
        match value {
            LiteralValue::Null => {
                self.builder.emit(Op::LoadNull { dst });
            }
            LiteralValue::Undefined => {
                self.builder.emit(Op::LoadUndefined { dst });
            }
            LiteralValue::Boolean(b) => {
                self.builder.emit(Op::LoadBool { dst, value: *b });
            }
            LiteralValue::Number(n) => {
                self.builder.emit_load_number(dst, *n)?;
            }
            LiteralValue::String(s) => {
                self.builder.emit_load_string(dst, s.cheap_clone())?;
            }
            LiteralValue::BigInt(_) => {
                // BigInt not yet supported
                return Err(JsError::type_error("BigInt is not supported"));
            }
            LiteralValue::RegExp { pattern, flags } => {
                let pattern_str: crate::value::JsString = pattern.as_str().into();
                let flags_str: crate::value::JsString = flags.as_str().into();
                let idx = self
                    .builder
                    .add_constant(super::bytecode::Constant::RegExp {
                        pattern: pattern_str,
                        flags: flags_str,
                    })?;
                self.builder.emit(Op::LoadConst { dst, idx });
            }
        }
        Ok(())
    }

    /// Compile an array expression
    fn compile_array_expression(
        &mut self,
        arr: &crate::ast::ArrayExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        let count = arr.elements.len();

        if count == 0 {
            // Empty array
            self.builder.emit(Op::CreateArray {
                dst,
                start: 0,
                count: 0,
            });
            return Ok(());
        }

        // Reserve registers for elements
        let start = self.builder.reserve_registers(count as u8)?;

        // Compile each element
        for (i, elem) in arr.elements.iter().enumerate() {
            let reg = start + i as u8;
            match elem {
                Some(ArrayElement::Expression(expr)) => {
                    self.compile_expression(expr, reg)?;
                }
                Some(ArrayElement::Spread(spread)) => {
                    // For spread, we need special handling
                    // For now, compile the expression and mark for spreading
                    self.compile_expression(&spread.argument, reg)?;
                    // TODO: Handle spread at runtime
                }
                None => {
                    // Hole in array - load undefined
                    self.builder.emit(Op::LoadUndefined { dst: reg });
                }
            }
        }

        // Create the array
        self.builder.emit(Op::CreateArray {
            dst,
            start,
            count: count as u16,
        });

        Ok(())
    }

    /// Compile an object expression
    fn compile_object_expression(
        &mut self,
        obj: &crate::ast::ObjectExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Create empty object
        self.builder.emit(Op::CreateObject { dst });

        // Add properties
        for prop in &obj.properties {
            match prop {
                ObjectProperty::Property(p) => {
                    self.compile_object_property(dst, p)?;
                }
                ObjectProperty::Spread(spread) => {
                    // Compile spread source
                    let src = self.builder.alloc_register()?;
                    self.compile_expression(&spread.argument, src)?;
                    // TODO: Emit spread operation to copy properties
                    self.builder.free_register(src);
                }
            }
        }

        Ok(())
    }

    /// Compile a single object property
    fn compile_object_property(
        &mut self,
        obj: Register,
        prop: &crate::ast::Property,
    ) -> Result<(), JsError> {
        // Compile the value
        let value_reg = self.builder.alloc_register()?;

        match prop.kind {
            PropertyKind::Init => {
                self.compile_expression(&prop.value, value_reg)?;
            }
            PropertyKind::Get | PropertyKind::Set => {
                // Getters/setters need special handling
                self.compile_expression(&prop.value, value_reg)?;
                // TODO: Use DefineAccessor
            }
        }

        // Set the property based on key type
        match &prop.key {
            ObjectPropertyKey::Identifier(id) => {
                let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                self.builder.emit(Op::SetPropertyConst {
                    obj,
                    key: key_idx,
                    value: value_reg,
                });
            }
            ObjectPropertyKey::String(s) => {
                let key_idx = self.builder.add_string(s.value.cheap_clone())?;
                self.builder.emit(Op::SetPropertyConst {
                    obj,
                    key: key_idx,
                    value: value_reg,
                });
            }
            ObjectPropertyKey::Number(lit) => {
                // Compile number as key
                let key_reg = self.builder.alloc_register()?;
                self.compile_literal(&lit.value, key_reg)?;
                self.builder.emit(Op::SetProperty {
                    obj,
                    key: key_reg,
                    value: value_reg,
                });
                self.builder.free_register(key_reg);
            }
            ObjectPropertyKey::Computed(expr) => {
                let key_reg = self.builder.alloc_register()?;
                self.compile_expression(expr, key_reg)?;
                self.builder.emit(Op::SetProperty {
                    obj,
                    key: key_reg,
                    value: value_reg,
                });
                self.builder.free_register(key_reg);
            }
            ObjectPropertyKey::PrivateIdentifier(_) => {
                // Private fields handled separately
                return Err(JsError::syntax_error_simple(
                    "Private fields not yet supported in bytecode compiler",
                ));
            }
        }

        self.builder.free_register(value_reg);
        Ok(())
    }

    /// Compile a unary expression
    fn compile_unary_expression(
        &mut self,
        unary: &crate::ast::UnaryExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        match unary.operator {
            UnaryOp::Delete => {
                // Delete needs special handling based on target
                self.compile_delete_expression(&unary.argument, dst)
            }
            _ => {
                // Compile operand
                let src = self.builder.alloc_register()?;
                self.compile_expression(&unary.argument, src)?;

                // Emit the unary operation
                let op = match unary.operator {
                    UnaryOp::Minus => Op::Neg { dst, src },
                    UnaryOp::Plus => Op::Plus { dst, src },
                    UnaryOp::Not => Op::Not { dst, src },
                    UnaryOp::BitNot => Op::BitNot { dst, src },
                    UnaryOp::Typeof => Op::Typeof { dst, src },
                    UnaryOp::Void => Op::Void { dst, src },
                    UnaryOp::Delete => {
                        // Handled above, but need to return something for match completeness
                        return Err(JsError::internal_error(
                            "Delete should be handled separately",
                        ));
                    }
                };
                self.builder.emit(op);

                self.builder.free_register(src);
                Ok(())
            }
        }
    }

    /// Compile a delete expression
    fn compile_delete_expression(
        &mut self,
        expr: &Expression,
        dst: Register,
    ) -> Result<(), JsError> {
        match expr {
            Expression::Member(member) => {
                let obj_reg = self.builder.alloc_register()?;
                self.compile_expression(&member.object, obj_reg)?;

                match &member.property {
                    MemberProperty::Identifier(id) => {
                        let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                        self.builder.emit(Op::DeletePropertyConst {
                            dst,
                            obj: obj_reg,
                            key: key_idx,
                        });
                    }
                    MemberProperty::Expression(expr) => {
                        let key_reg = self.builder.alloc_register()?;
                        self.compile_expression(expr, key_reg)?;
                        self.builder.emit(Op::DeleteProperty {
                            dst,
                            obj: obj_reg,
                            key: key_reg,
                        });
                        self.builder.free_register(key_reg);
                    }
                    MemberProperty::PrivateIdentifier(_) => {
                        return Err(JsError::syntax_error_simple("Cannot delete private field"));
                    }
                }

                self.builder.free_register(obj_reg);
            }
            Expression::Identifier(_) => {
                // delete identifier - in strict mode this is an error
                // For now, just return true (non-strict behavior)
                self.builder.emit(Op::LoadBool { dst, value: true });
            }
            _ => {
                // delete on non-reference always returns true
                // But we still need to evaluate the expression for side effects
                let tmp = self.builder.alloc_register()?;
                self.compile_expression(expr, tmp)?;
                self.builder.free_register(tmp);
                self.builder.emit(Op::LoadBool { dst, value: true });
            }
        }
        Ok(())
    }

    /// Compile a binary expression
    fn compile_binary_expression(
        &mut self,
        binary: &crate::ast::BinaryExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Compile left operand
        let left = self.builder.alloc_register()?;
        self.compile_expression(&binary.left, left)?;

        // Compile right operand
        let right = self.builder.alloc_register()?;
        self.compile_expression(&binary.right, right)?;

        // Emit the binary operation
        let op = match binary.operator {
            BinaryOp::Add => Op::Add { dst, left, right },
            BinaryOp::Sub => Op::Sub { dst, left, right },
            BinaryOp::Mul => Op::Mul { dst, left, right },
            BinaryOp::Div => Op::Div { dst, left, right },
            BinaryOp::Mod => Op::Mod { dst, left, right },
            BinaryOp::Exp => Op::Exp { dst, left, right },
            BinaryOp::Eq => Op::Eq { dst, left, right },
            BinaryOp::NotEq => Op::NotEq { dst, left, right },
            BinaryOp::StrictEq => Op::StrictEq { dst, left, right },
            BinaryOp::StrictNotEq => Op::StrictNotEq { dst, left, right },
            BinaryOp::Lt => Op::Lt { dst, left, right },
            BinaryOp::LtEq => Op::LtEq { dst, left, right },
            BinaryOp::Gt => Op::Gt { dst, left, right },
            BinaryOp::GtEq => Op::GtEq { dst, left, right },
            BinaryOp::BitAnd => Op::BitAnd { dst, left, right },
            BinaryOp::BitOr => Op::BitOr { dst, left, right },
            BinaryOp::BitXor => Op::BitXor { dst, left, right },
            BinaryOp::LShift => Op::LShift { dst, left, right },
            BinaryOp::RShift => Op::RShift { dst, left, right },
            BinaryOp::URShift => Op::URShift { dst, left, right },
            BinaryOp::In => Op::In { dst, left, right },
            BinaryOp::Instanceof => Op::Instanceof { dst, left, right },
        };
        self.builder.emit(op);

        self.builder.free_register(right);
        self.builder.free_register(left);
        Ok(())
    }

    /// Compile a logical expression (with short-circuit evaluation)
    fn compile_logical_expression(
        &mut self,
        logical: &crate::ast::LogicalExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Compile left operand
        self.compile_expression(&logical.left, dst)?;

        // Emit conditional jump based on operator
        let skip_right = match logical.operator {
            LogicalOp::And => {
                // If left is falsy, skip right and keep left result
                self.builder.emit_jump_if_false(dst)
            }
            LogicalOp::Or => {
                // If left is truthy, skip right and keep left result
                self.builder.emit_jump_if_true(dst)
            }
            LogicalOp::NullishCoalescing => {
                // If left is NOT nullish, skip right and keep left result
                // If left IS nullish, fall through and evaluate right
                let skip_right = self.builder.emit_jump_if_not_nullish(dst);
                // Compile right operand
                self.compile_expression(&logical.right, dst)?;
                // Patch the skip jump to point here
                self.builder.patch_jump(skip_right);
                return Ok(());
            }
        };

        // Compile right operand (only reached if short-circuit didn't happen)
        self.compile_expression(&logical.right, dst)?;

        // Patch the skip jump
        self.builder.patch_jump(skip_right);

        Ok(())
    }

    /// Compile a conditional (ternary) expression
    fn compile_conditional_expression(
        &mut self,
        cond: &crate::ast::ConditionalExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Compile test
        let test_reg = self.builder.alloc_register()?;
        self.compile_expression(&cond.test, test_reg)?;

        // Jump to alternate if test is falsy
        let jump_to_alt = self.builder.emit_jump_if_false(test_reg);
        self.builder.free_register(test_reg);

        // Compile consequent
        self.compile_expression(&cond.consequent, dst)?;

        // Jump over alternate
        let jump_to_end = self.builder.emit_jump();

        // Patch jump to alternate
        self.builder.patch_jump(jump_to_alt);

        // Compile alternate
        self.compile_expression(&cond.alternate, dst)?;

        // Patch jump to end
        self.builder.patch_jump(jump_to_end);

        Ok(())
    }

    /// Compile an assignment expression
    fn compile_assignment_expression(
        &mut self,
        assign: &crate::ast::AssignmentExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        match &assign.left {
            AssignmentTarget::Identifier(id) => {
                self.compile_identifier_assignment(id, &assign.operator, &assign.right, dst)
            }
            AssignmentTarget::Member(member) => {
                self.compile_member_assignment(member, &assign.operator, &assign.right, dst)
            }
            AssignmentTarget::Pattern(pattern) => {
                // Destructuring assignment
                if assign.operator != AssignmentOp::Assign {
                    return Err(JsError::syntax_error_simple(
                        "Compound assignment to pattern is not allowed",
                    ));
                }
                // Compile the right side
                self.compile_expression(&assign.right, dst)?;
                // Destructure into the pattern
                self.compile_pattern_assignment(pattern, dst)
            }
        }
    }

    /// Compile assignment to an identifier
    fn compile_identifier_assignment(
        &mut self,
        id: &crate::ast::Identifier,
        op: &AssignmentOp,
        right: &Expression,
        dst: Register,
    ) -> Result<(), JsError> {
        let name_idx = self.builder.add_string(id.name.cheap_clone())?;

        if *op == AssignmentOp::Assign {
            // Simple assignment
            self.compile_expression(right, dst)?;
            self.builder.emit(Op::SetVar {
                name: name_idx,
                src: dst,
            });
        } else {
            // Compound assignment
            // Load current value
            self.builder.emit(Op::GetVar {
                dst,
                name: name_idx,
            });

            // Handle short-circuit operators specially
            match op {
                AssignmentOp::AndAssign => {
                    let skip = self.builder.emit_jump_if_false(dst);
                    self.compile_expression(right, dst)?;
                    self.builder.patch_jump(skip);
                }
                AssignmentOp::OrAssign => {
                    let skip = self.builder.emit_jump_if_true(dst);
                    self.compile_expression(right, dst)?;
                    self.builder.patch_jump(skip);
                }
                AssignmentOp::NullishAssign => {
                    let not_nullish = self.builder.emit(Op::JumpIfNotNullish {
                        cond: dst,
                        target: 0,
                    });
                    self.compile_expression(right, dst)?;
                    self.builder.patch_jump(super::JumpPlaceholder {
                        instruction_index: not_nullish,
                    });
                }
                _ => {
                    // Regular compound assignment
                    let right_reg = self.builder.alloc_register()?;
                    self.compile_expression(right, right_reg)?;

                    let binary_op = self.compound_to_binary_op(*op)?;
                    self.emit_binary_op(binary_op, dst, dst, right_reg);

                    self.builder.free_register(right_reg);
                }
            }

            self.builder.emit(Op::SetVar {
                name: name_idx,
                src: dst,
            });
        }

        Ok(())
    }

    /// Compile assignment to a member expression
    fn compile_member_assignment(
        &mut self,
        member: &crate::ast::MemberExpression,
        op: &AssignmentOp,
        right: &Expression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Compile object
        let obj_reg = self.builder.alloc_register()?;
        self.compile_expression(&member.object, obj_reg)?;

        // Get key
        let key_info = self.get_member_key_info(&member.property)?;

        if *op == AssignmentOp::Assign {
            // Simple assignment
            self.compile_expression(right, dst)?;
            self.emit_set_property(obj_reg, &key_info, dst)?;
        } else {
            // Compound assignment - load current value first
            self.emit_get_property(dst, obj_reg, &key_info)?;

            // Handle short-circuit operators
            match op {
                AssignmentOp::AndAssign => {
                    let skip = self.builder.emit_jump_if_false(dst);
                    self.compile_expression(right, dst)?;
                    self.builder.patch_jump(skip);
                }
                AssignmentOp::OrAssign => {
                    let skip = self.builder.emit_jump_if_true(dst);
                    self.compile_expression(right, dst)?;
                    self.builder.patch_jump(skip);
                }
                AssignmentOp::NullishAssign => {
                    let not_nullish = self.builder.emit(Op::JumpIfNotNullish {
                        cond: dst,
                        target: 0,
                    });
                    self.compile_expression(right, dst)?;
                    self.builder.patch_jump(super::JumpPlaceholder {
                        instruction_index: not_nullish,
                    });
                }
                _ => {
                    let right_reg = self.builder.alloc_register()?;
                    self.compile_expression(right, right_reg)?;

                    let binary_op = self.compound_to_binary_op(*op)?;
                    self.emit_binary_op(binary_op, dst, dst, right_reg);

                    self.builder.free_register(right_reg);
                }
            }

            self.emit_set_property(obj_reg, &key_info, dst)?;
        }

        // Free key register if computed
        if let MemberKeyInfo::Computed(key_reg) = key_info {
            self.builder.free_register(key_reg);
        }
        self.builder.free_register(obj_reg);

        Ok(())
    }

    /// Helper to convert compound assignment to binary op
    fn compound_to_binary_op(&self, op: AssignmentOp) -> Result<BinaryOp, JsError> {
        Ok(match op {
            AssignmentOp::AddAssign => BinaryOp::Add,
            AssignmentOp::SubAssign => BinaryOp::Sub,
            AssignmentOp::MulAssign => BinaryOp::Mul,
            AssignmentOp::DivAssign => BinaryOp::Div,
            AssignmentOp::ModAssign => BinaryOp::Mod,
            AssignmentOp::ExpAssign => BinaryOp::Exp,
            AssignmentOp::BitAndAssign => BinaryOp::BitAnd,
            AssignmentOp::BitOrAssign => BinaryOp::BitOr,
            AssignmentOp::BitXorAssign => BinaryOp::BitXor,
            AssignmentOp::LShiftAssign => BinaryOp::LShift,
            AssignmentOp::RShiftAssign => BinaryOp::RShift,
            AssignmentOp::URShiftAssign => BinaryOp::URShift,
            _ => {
                return Err(JsError::internal_error(
                    "Invalid compound assignment operator",
                ))
            }
        })
    }

    /// Emit a binary operation
    fn emit_binary_op(&mut self, op: BinaryOp, dst: Register, left: Register, right: Register) {
        let instr = match op {
            BinaryOp::Add => Op::Add { dst, left, right },
            BinaryOp::Sub => Op::Sub { dst, left, right },
            BinaryOp::Mul => Op::Mul { dst, left, right },
            BinaryOp::Div => Op::Div { dst, left, right },
            BinaryOp::Mod => Op::Mod { dst, left, right },
            BinaryOp::Exp => Op::Exp { dst, left, right },
            BinaryOp::Eq => Op::Eq { dst, left, right },
            BinaryOp::NotEq => Op::NotEq { dst, left, right },
            BinaryOp::StrictEq => Op::StrictEq { dst, left, right },
            BinaryOp::StrictNotEq => Op::StrictNotEq { dst, left, right },
            BinaryOp::Lt => Op::Lt { dst, left, right },
            BinaryOp::LtEq => Op::LtEq { dst, left, right },
            BinaryOp::Gt => Op::Gt { dst, left, right },
            BinaryOp::GtEq => Op::GtEq { dst, left, right },
            BinaryOp::BitAnd => Op::BitAnd { dst, left, right },
            BinaryOp::BitOr => Op::BitOr { dst, left, right },
            BinaryOp::BitXor => Op::BitXor { dst, left, right },
            BinaryOp::LShift => Op::LShift { dst, left, right },
            BinaryOp::RShift => Op::RShift { dst, left, right },
            BinaryOp::URShift => Op::URShift { dst, left, right },
            BinaryOp::In => Op::In { dst, left, right },
            BinaryOp::Instanceof => Op::Instanceof { dst, left, right },
        };
        self.builder.emit(instr);
    }

    /// Compile an update expression (++/--)
    fn compile_update_expression(
        &mut self,
        update: &crate::ast::UpdateExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        match update.argument.as_ref() {
            Expression::Identifier(id) => {
                let name_idx = self.builder.add_string(id.name.cheap_clone())?;

                // Load current value
                self.builder.emit(Op::GetVar {
                    dst,
                    name: name_idx,
                });

                if !update.prefix {
                    // Postfix: save original value
                    let original = self.builder.alloc_register()?;
                    self.builder.emit(Op::Move {
                        dst: original,
                        src: dst,
                    });

                    // Perform update
                    let one = self.builder.alloc_register()?;
                    self.builder.emit(Op::LoadInt { dst: one, value: 1 });

                    if update.operator == UpdateOp::Increment {
                        self.builder.emit(Op::Add {
                            dst,
                            left: dst,
                            right: one,
                        });
                    } else {
                        self.builder.emit(Op::Sub {
                            dst,
                            left: dst,
                            right: one,
                        });
                    }

                    // Store updated value
                    self.builder.emit(Op::SetVar {
                        name: name_idx,
                        src: dst,
                    });

                    // Return original value
                    self.builder.emit(Op::Move { dst, src: original });

                    self.builder.free_register(one);
                    self.builder.free_register(original);
                } else {
                    // Prefix: update in place
                    let one = self.builder.alloc_register()?;
                    self.builder.emit(Op::LoadInt { dst: one, value: 1 });

                    if update.operator == UpdateOp::Increment {
                        self.builder.emit(Op::Add {
                            dst,
                            left: dst,
                            right: one,
                        });
                    } else {
                        self.builder.emit(Op::Sub {
                            dst,
                            left: dst,
                            right: one,
                        });
                    }

                    // Store and return updated value
                    self.builder.emit(Op::SetVar {
                        name: name_idx,
                        src: dst,
                    });

                    self.builder.free_register(one);
                }
            }
            Expression::Member(member) => {
                // Similar but for member access
                let obj_reg = self.builder.alloc_register()?;
                self.compile_expression(&member.object, obj_reg)?;

                let key_info = self.get_member_key_info(&member.property)?;

                // Load current value
                self.emit_get_property(dst, obj_reg, &key_info)?;

                let one = self.builder.alloc_register()?;
                self.builder.emit(Op::LoadInt { dst: one, value: 1 });

                if !update.prefix {
                    // Postfix
                    let original = self.builder.alloc_register()?;
                    self.builder.emit(Op::Move {
                        dst: original,
                        src: dst,
                    });

                    if update.operator == UpdateOp::Increment {
                        self.builder.emit(Op::Add {
                            dst,
                            left: dst,
                            right: one,
                        });
                    } else {
                        self.builder.emit(Op::Sub {
                            dst,
                            left: dst,
                            right: one,
                        });
                    }

                    self.emit_set_property(obj_reg, &key_info, dst)?;
                    self.builder.emit(Op::Move { dst, src: original });

                    self.builder.free_register(original);
                } else {
                    // Prefix
                    if update.operator == UpdateOp::Increment {
                        self.builder.emit(Op::Add {
                            dst,
                            left: dst,
                            right: one,
                        });
                    } else {
                        self.builder.emit(Op::Sub {
                            dst,
                            left: dst,
                            right: one,
                        });
                    }

                    self.emit_set_property(obj_reg, &key_info, dst)?;
                }

                self.builder.free_register(one);
                if let MemberKeyInfo::Computed(key_reg) = key_info {
                    self.builder.free_register(key_reg);
                }
                self.builder.free_register(obj_reg);
            }
            _ => {
                return Err(JsError::syntax_error_simple(
                    "Invalid left-hand side in update expression",
                ));
            }
        }

        Ok(())
    }

    /// Compile a sequence expression
    fn compile_sequence_expression(
        &mut self,
        seq: &crate::ast::SequenceExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Compile all expressions, keeping only the last result
        for (i, expr) in seq.expressions.iter().enumerate() {
            if i == seq.expressions.len() - 1 {
                // Last expression - result goes to dst
                self.compile_expression(expr, dst)?;
            } else {
                // Not last - compile for side effects only
                let tmp = self.builder.alloc_register()?;
                self.compile_expression(expr, tmp)?;
                self.builder.free_register(tmp);
            }
        }
        Ok(())
    }

    /// Compile a member expression
    fn compile_member_expression(
        &mut self,
        member: &crate::ast::MemberExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Handle super.x access
        if matches!(member.object.as_ref(), Expression::Super(_)) {
            match &member.property {
                MemberProperty::Identifier(id) => {
                    let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                    self.builder.emit(Op::SuperGetConst { dst, key: key_idx });
                }
                MemberProperty::Expression(expr) => {
                    let key_reg = self.builder.alloc_register()?;
                    self.compile_expression(expr, key_reg)?;
                    self.builder.emit(Op::SuperGet { dst, key: key_reg });
                    self.builder.free_register(key_reg);
                }
                MemberProperty::PrivateIdentifier(_) => {
                    return Err(JsError::syntax_error_simple(
                        "Private fields not supported on super",
                    ));
                }
            }
            return Ok(());
        }

        // Compile object
        let obj_reg = self.builder.alloc_register()?;
        self.compile_expression(&member.object, obj_reg)?;

        // Get property
        match &member.property {
            MemberProperty::Identifier(id) => {
                let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                self.builder.emit(Op::GetPropertyConst {
                    dst,
                    obj: obj_reg,
                    key: key_idx,
                });
            }
            MemberProperty::Expression(expr) => {
                let key_reg = self.builder.alloc_register()?;
                self.compile_expression(expr, key_reg)?;
                self.builder.emit(Op::GetProperty {
                    dst,
                    obj: obj_reg,
                    key: key_reg,
                });
                self.builder.free_register(key_reg);
            }
            MemberProperty::PrivateIdentifier(_) => {
                return Err(JsError::syntax_error_simple(
                    "Private fields not yet supported in bytecode compiler",
                ));
            }
        }

        self.builder.free_register(obj_reg);
        Ok(())
    }

    /// Compile optional chain expression
    /// The optional chain wraps an expression that may contain ?. operators.
    /// When any ?. encounters null/undefined, the whole chain short-circuits to undefined.
    fn compile_optional_chain(
        &mut self,
        opt: &crate::ast::OptionalChainExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Compile the base expression, collecting jump placeholders for short-circuits
        // We use a stack to track all the short-circuit jumps
        let short_circuit_jumps = self.compile_optional_chain_inner(&opt.base, dst)?;

        // If there were any short-circuit jumps, patch them to jump to here
        // and load undefined as the result
        if !short_circuit_jumps.is_empty() {
            // Jump over the "load undefined" block if we completed normally
            let skip_undefined = self.builder.emit_jump();

            // This is where all short-circuit jumps land
            let short_circuit_target = self.builder.current_offset();
            for jump in short_circuit_jumps {
                self.builder
                    .patch_jump_to(jump, short_circuit_target as super::bytecode::JumpTarget);
            }

            // Load undefined as the short-circuit result
            self.builder.emit(Op::LoadUndefined { dst });

            // Patch the skip jump to after the undefined load
            self.builder.patch_jump(skip_undefined);
        }

        Ok(())
    }

    /// Recursively compile an optional chain expression, returning jump placeholders
    /// for each short-circuit point (where ?. encounters null/undefined).
    fn compile_optional_chain_inner(
        &mut self,
        expr: &Expression,
        dst: Register,
    ) -> Result<Vec<super::JumpPlaceholder>, JsError> {
        match expr {
            Expression::Member(member) => self.compile_member_expression_optional(member, dst),
            Expression::Call(call) => self.compile_call_expression_optional(call, dst),
            Expression::OptionalChain(inner) => {
                // Nested optional chain - compile recursively
                self.compile_optional_chain_inner(&inner.base, dst)
            }
            _ => {
                // Not a member or call - just compile normally
                self.compile_expression(expr, dst)?;
                Ok(Vec::new())
            }
        }
    }

    /// Compile a member expression that may be optional, returning short-circuit jumps
    fn compile_member_expression_optional(
        &mut self,
        member: &crate::ast::MemberExpression,
        dst: Register,
    ) -> Result<Vec<super::JumpPlaceholder>, JsError> {
        let mut short_circuit_jumps = Vec::new();

        // Handle super.x access (no optional chaining for super)
        if matches!(member.object.as_ref(), Expression::Super(_)) {
            // Delegate to regular member access for super
            self.compile_member_expression(member, dst)?;
            return Ok(short_circuit_jumps);
        }

        // Compile the object, recursively collecting short-circuit jumps
        let obj_reg = self.builder.alloc_register()?;

        // If the object is itself a member/call expression, handle it recursively
        let inner_jumps = match member.object.as_ref() {
            Expression::Member(inner_member) => {
                self.compile_member_expression_optional(inner_member, obj_reg)?
            }
            Expression::Call(inner_call) => {
                self.compile_call_expression_optional(inner_call, obj_reg)?
            }
            Expression::OptionalChain(inner_opt) => {
                self.compile_optional_chain_inner(&inner_opt.base, obj_reg)?
            }
            _ => {
                self.compile_expression(&member.object, obj_reg)?;
                Vec::new()
            }
        };
        short_circuit_jumps.extend(inner_jumps);

        // If this is an optional member access (?.), check for null/undefined
        if member.optional {
            // If obj is nullish, short-circuit to the end
            let jump = self.builder.emit_jump_if_nullish(obj_reg);
            short_circuit_jumps.push(jump);
        }

        // Get the property
        match &member.property {
            MemberProperty::Identifier(id) => {
                let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                self.builder.emit(Op::GetPropertyConst {
                    dst,
                    obj: obj_reg,
                    key: key_idx,
                });
            }
            MemberProperty::Expression(expr) => {
                let key_reg = self.builder.alloc_register()?;
                self.compile_expression(expr, key_reg)?;
                self.builder.emit(Op::GetProperty {
                    dst,
                    obj: obj_reg,
                    key: key_reg,
                });
                self.builder.free_register(key_reg);
            }
            MemberProperty::PrivateIdentifier(_) => {
                return Err(JsError::syntax_error_simple(
                    "Private fields not yet supported in bytecode compiler",
                ));
            }
        }

        self.builder.free_register(obj_reg);
        Ok(short_circuit_jumps)
    }

    /// Compile a call expression that may be optional, returning short-circuit jumps
    fn compile_call_expression_optional(
        &mut self,
        call: &crate::ast::CallExpression,
        dst: Register,
    ) -> Result<Vec<super::JumpPlaceholder>, JsError> {
        let mut short_circuit_jumps = Vec::new();

        // Check if this is a method call (obj.method() or obj?.method())
        if let Expression::Member(member) = call.callee.as_ref() {
            // Compile the object
            let obj_reg = self.builder.alloc_register()?;

            // Recursively handle nested optional chains in the object
            let inner_jumps = match member.object.as_ref() {
                Expression::Member(inner_member) => {
                    self.compile_member_expression_optional(inner_member, obj_reg)?
                }
                Expression::Call(inner_call) => {
                    self.compile_call_expression_optional(inner_call, obj_reg)?
                }
                Expression::OptionalChain(inner_opt) => {
                    self.compile_optional_chain_inner(&inner_opt.base, obj_reg)?
                }
                _ => {
                    self.compile_expression(&member.object, obj_reg)?;
                    Vec::new()
                }
            };
            short_circuit_jumps.extend(inner_jumps);

            // If member access is optional (?.), check for null/undefined
            if member.optional {
                let jump = self.builder.emit_jump_if_nullish(obj_reg);
                short_circuit_jumps.push(jump);
            }

            // Get the method
            let method_reg = self.builder.alloc_register()?;
            match &member.property {
                MemberProperty::Identifier(id) => {
                    let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                    self.builder.emit(Op::GetPropertyConst {
                        dst: method_reg,
                        obj: obj_reg,
                        key: key_idx,
                    });
                }
                MemberProperty::Expression(expr) => {
                    let key_reg = self.builder.alloc_register()?;
                    self.compile_expression(expr, key_reg)?;
                    self.builder.emit(Op::GetProperty {
                        dst: method_reg,
                        obj: obj_reg,
                        key: key_reg,
                    });
                    self.builder.free_register(key_reg);
                }
                MemberProperty::PrivateIdentifier(_) => {
                    return Err(JsError::syntax_error_simple(
                        "Private fields not yet supported in bytecode compiler",
                    ));
                }
            }

            // If call is optional (?.()), check if method is callable
            if call.optional {
                let jump = self.builder.emit_jump_if_nullish(method_reg);
                short_circuit_jumps.push(jump);
            }

            // Compile arguments
            let (args_start, argc) = self.compile_arguments(&call.arguments)?;

            // Call the method
            self.builder.emit(Op::Call {
                dst,
                callee: method_reg,
                this: obj_reg,
                args_start,
                argc,
            });

            self.builder.free_register(method_reg);
            self.builder.free_register(obj_reg);
        } else {
            // Regular call (not method call)
            let callee_reg = self.builder.alloc_register()?;

            // Handle nested optional chains in callee
            let inner_jumps = match call.callee.as_ref() {
                Expression::Member(inner_member) => {
                    self.compile_member_expression_optional(inner_member, callee_reg)?
                }
                Expression::Call(inner_call) => {
                    self.compile_call_expression_optional(inner_call, callee_reg)?
                }
                Expression::OptionalChain(inner_opt) => {
                    self.compile_optional_chain_inner(&inner_opt.base, callee_reg)?
                }
                _ => {
                    self.compile_expression(&call.callee, callee_reg)?;
                    Vec::new()
                }
            };
            short_circuit_jumps.extend(inner_jumps);

            // If call is optional (?.()), check if callee is callable
            if call.optional {
                let jump = self.builder.emit_jump_if_nullish(callee_reg);
                short_circuit_jumps.push(jump);
            }

            // Compile arguments
            let (args_start, argc) = self.compile_arguments(&call.arguments)?;

            // this is undefined for regular calls
            let this_reg = self.builder.alloc_register()?;
            self.builder.emit(Op::LoadUndefined { dst: this_reg });

            // Call
            self.builder.emit(Op::Call {
                dst,
                callee: callee_reg,
                this: this_reg,
                args_start,
                argc,
            });

            self.builder.free_register(this_reg);
            self.builder.free_register(callee_reg);
        }

        Ok(short_circuit_jumps)
    }

    /// Compile a call expression
    fn compile_call_expression(
        &mut self,
        call: &crate::ast::CallExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Handle super() call
        if matches!(call.callee.as_ref(), Expression::Super(_)) {
            // Compile arguments
            let (args_start, argc) = self.compile_arguments(&call.arguments)?;

            self.builder.emit(Op::SuperCall {
                dst,
                args_start,
                argc,
            });
            return Ok(());
        }

        // Handle super.method() call
        if let Expression::Member(member) = call.callee.as_ref() {
            if matches!(member.object.as_ref(), Expression::Super(_)) {
                // Super method call
                if let MemberProperty::Identifier(method_name) = &member.property {
                    let method_idx = self.builder.add_string(method_name.name.cheap_clone())?;

                    // Get super.method
                    let method_reg = self.builder.alloc_register()?;
                    self.builder.emit(Op::SuperGetConst {
                        dst: method_reg,
                        key: method_idx,
                    });

                    // Compile arguments
                    let (args_start, argc) = self.compile_arguments(&call.arguments)?;

                    // Call with `this` as the receiver
                    let this_reg = self.builder.alloc_register()?;
                    self.builder.emit(Op::LoadThis { dst: this_reg });

                    self.builder.emit(Op::Call {
                        dst,
                        callee: method_reg,
                        this: this_reg,
                        args_start,
                        argc,
                    });

                    self.builder.free_register(this_reg);
                    self.builder.free_register(method_reg);
                    return Ok(());
                }
            }
        }

        // Check for method call pattern: obj.method(args)
        if let Expression::Member(member) = call.callee.as_ref() {
            if let MemberProperty::Identifier(method_name) = &member.property {
                // Method call - we can use CallMethod for optimization
                let obj_reg = self.builder.alloc_register()?;
                self.compile_expression(&member.object, obj_reg)?;

                let method_idx = self.builder.add_string(method_name.name.cheap_clone())?;

                // Compile arguments
                let (args_start, argc) = self.compile_arguments(&call.arguments)?;

                self.builder.emit(Op::CallMethod {
                    dst,
                    obj: obj_reg,
                    method: method_idx,
                    args_start,
                    argc,
                });

                self.builder.free_register(obj_reg);
                return Ok(());
            }
        }

        // Regular call
        let callee_reg = self.builder.alloc_register()?;
        self.compile_expression(&call.callee, callee_reg)?;

        // `this` is undefined for regular calls
        let this_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::LoadUndefined { dst: this_reg });

        // Compile arguments
        let (args_start, argc) = self.compile_arguments(&call.arguments)?;

        self.builder.emit(Op::Call {
            dst,
            callee: callee_reg,
            this: this_reg,
            args_start,
            argc,
        });

        self.builder.free_register(this_reg);
        self.builder.free_register(callee_reg);

        Ok(())
    }

    /// Compile arguments for a call
    fn compile_arguments(&mut self, args: &[Argument]) -> Result<(Register, u8), JsError> {
        let argc = args.len();
        if argc > 255 {
            return Err(JsError::syntax_error_simple("Too many arguments"));
        }

        if argc == 0 {
            return Ok((0, 0));
        }

        let args_start = self.builder.reserve_registers(argc as u8)?;

        for (i, arg) in args.iter().enumerate() {
            let reg = args_start + i as u8;
            match arg {
                Argument::Expression(expr) => {
                    self.compile_expression(expr, reg)?;
                }
                Argument::Spread(spread) => {
                    // TODO: Handle spread arguments properly
                    self.compile_expression(&spread.argument, reg)?;
                }
            }
        }

        Ok((args_start, argc as u8))
    }

    /// Compile a new expression
    fn compile_new_expression(
        &mut self,
        new: &crate::ast::NewExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        let callee_reg = self.builder.alloc_register()?;
        self.compile_expression(&new.callee, callee_reg)?;

        let (args_start, argc) = self.compile_arguments(&new.arguments)?;

        self.builder.emit(Op::Construct {
            dst,
            callee: callee_reg,
            args_start,
            argc,
        });

        self.builder.free_register(callee_reg);
        Ok(())
    }

    /// Compile yield expression
    fn compile_yield_expression(
        &mut self,
        yield_expr: &crate::ast::YieldExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Compile argument (or undefined)
        let value_reg = self.builder.alloc_register()?;
        if let Some(arg) = &yield_expr.argument {
            self.compile_expression(arg, value_reg)?;
        } else {
            self.builder.emit(Op::LoadUndefined { dst: value_reg });
        }

        if yield_expr.delegate {
            // yield*
            self.builder.emit(Op::YieldStar {
                dst,
                iterable: value_reg,
            });
        } else {
            // yield
            self.builder.emit(Op::Yield {
                dst,
                value: value_reg,
            });
        }

        self.builder.free_register(value_reg);
        Ok(())
    }

    /// Compile await expression
    fn compile_await_expression(
        &mut self,
        await_expr: &crate::ast::AwaitExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        let promise_reg = self.builder.alloc_register()?;
        self.compile_expression(&await_expr.argument, promise_reg)?;

        self.builder.emit(Op::Await {
            dst,
            promise: promise_reg,
        });

        self.builder.free_register(promise_reg);
        Ok(())
    }

    /// Compile template literal
    fn compile_template_literal(
        &mut self,
        template: &crate::ast::TemplateLiteral,
        dst: Register,
    ) -> Result<(), JsError> {
        // Interleave quasis and expressions
        let total_parts = template.quasis.len() + template.expressions.len();

        if total_parts == 0 {
            self.builder.emit_load_string(dst, "".into())?;
            return Ok(());
        }

        if total_parts == 1 && template.expressions.is_empty() {
            // Single quasi, no expressions
            let quasi = template
                .quasis
                .first()
                .ok_or_else(|| JsError::internal_error("Template literal with no quasis"))?;
            self.builder
                .emit_load_string(dst, quasi.value.cheap_clone())?;
            return Ok(());
        }

        // Reserve registers for all parts
        let start = self.builder.reserve_registers(total_parts as u8)?;

        let mut reg_idx = 0;
        for (i, quasi) in template.quasis.iter().enumerate() {
            // Add quasi
            if !quasi.value.is_empty() {
                self.builder
                    .emit_load_string(start + reg_idx, quasi.value.cheap_clone())?;
                reg_idx += 1;
            }

            // Add expression (if not at the end)
            if let Some(expr) = template.expressions.get(i) {
                self.compile_expression(expr, start + reg_idx)?;
                reg_idx += 1;
            }
        }

        // Concatenate all parts
        self.builder.emit(Op::TemplateConcat {
            dst,
            start,
            count: reg_idx,
        });

        Ok(())
    }

    /// Compile tagged template
    fn compile_tagged_template(
        &mut self,
        tagged: &crate::ast::TaggedTemplateExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Create template strings constant (cooked and raw arrays)
        let cooked: Vec<crate::value::JsString> = tagged
            .quasi
            .quasis
            .iter()
            .map(|q| q.value.cheap_clone())
            .collect();
        let raw: Vec<crate::value::JsString> = tagged
            .quasi
            .quasis
            .iter()
            .map(|q| q.value.cheap_clone())
            .collect();

        let template_idx = self
            .builder
            .add_constant(super::bytecode::Constant::TemplateStrings { cooked, raw })?;

        // Compile the tag function
        let tag_reg = self.builder.alloc_register()?;

        // Check if it's a method call (obj.tag`template`)
        let this_reg = if let Expression::Member(member) = tagged.tag.as_ref() {
            // Compile object for `this`
            let obj_reg = self.builder.alloc_register()?;
            self.compile_expression(&member.object, obj_reg)?;

            // Get the method
            match &member.property {
                MemberProperty::Identifier(id) => {
                    let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                    self.builder.emit(Op::GetPropertyConst {
                        dst: tag_reg,
                        obj: obj_reg,
                        key: key_idx,
                    });
                }
                MemberProperty::Expression(expr) => {
                    let key_reg = self.builder.alloc_register()?;
                    self.compile_expression(expr, key_reg)?;
                    self.builder.emit(Op::GetProperty {
                        dst: tag_reg,
                        obj: obj_reg,
                        key: key_reg,
                    });
                    self.builder.free_register(key_reg);
                }
                MemberProperty::PrivateIdentifier(_) => {
                    return Err(JsError::syntax_error_simple(
                        "Private fields not yet supported in bytecode compiler",
                    ));
                }
            }
            Some(obj_reg)
        } else {
            // Regular call - no `this`
            self.compile_expression(&tagged.tag, tag_reg)?;
            None
        };

        // Compile expression arguments
        let exprs_count = tagged.quasi.expressions.len();
        let exprs_start = if exprs_count > 0 {
            let start = self.builder.reserve_registers(exprs_count as u8)?;
            for (i, expr) in tagged.quasi.expressions.iter().enumerate() {
                self.compile_expression(expr, start + i as u8)?;
            }
            start
        } else {
            0
        };

        // Set up `this` for the call (undefined if not a method call)
        let final_this_reg = match this_reg {
            Some(obj_reg) => obj_reg,
            None => {
                let reg = self.builder.alloc_register()?;
                self.builder.emit(Op::LoadUndefined { dst: reg });
                reg
            }
        };

        // Emit the tagged template call
        self.builder.emit(Op::TaggedTemplate {
            dst,
            tag: tag_reg,
            this: final_this_reg,
            template: template_idx,
            exprs_start,
            exprs_count: exprs_count as u8,
        });

        // Clean up
        self.builder.free_register(final_this_reg);
        self.builder.free_register(tag_reg);

        Ok(())
    }

    /// Compile function expression
    fn compile_function_expression(
        &mut self,
        func: &crate::ast::FunctionExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Extract function metadata
        let name = func.id.as_ref().map(|id| id.name.cheap_clone());

        // Use the existing compile_function_body from compile_stmt
        let chunk = self.compile_function_body(
            &func.params,
            &func.body.body,
            name,
            func.generator,
            func.async_,
            false, // is_arrow = false
        )?;

        // Add the chunk to constants
        let chunk_idx = self
            .builder
            .add_constant(super::bytecode::Constant::Chunk(std::rc::Rc::new(chunk)))?;

        // Emit the appropriate closure creation opcode
        if func.generator {
            self.builder.emit(Op::CreateGenerator { dst, chunk_idx });
        } else if func.async_ {
            self.builder.emit(Op::CreateAsync { dst, chunk_idx });
        } else {
            self.builder.emit(Op::CreateClosure { dst, chunk_idx });
        }

        Ok(())
    }

    /// Compile arrow function
    fn compile_arrow_function(
        &mut self,
        arrow: &crate::ast::ArrowFunctionExpression,
        dst: Register,
    ) -> Result<(), JsError> {
        // Compile the arrow function body
        let chunk = match arrow.body.as_ref() {
            crate::ast::ArrowFunctionBody::Block(block) => self.compile_function_body(
                &arrow.params,
                &block.body,
                None, // arrows are anonymous
                false,
                arrow.async_,
                true, // is_arrow = true
            )?,
            crate::ast::ArrowFunctionBody::Expression(expr) => {
                self.compile_arrow_expression_body(&arrow.params, expr, arrow.async_)?
            }
        };

        // Add chunk to constants
        let chunk_idx = self
            .builder
            .add_constant(super::bytecode::Constant::Chunk(std::rc::Rc::new(chunk)))?;

        // Arrow functions use CreateArrow (captures lexical this)
        self.builder.emit(Op::CreateArrow { dst, chunk_idx });

        Ok(())
    }

    /// Compile an expression-bodied arrow function into a BytecodeChunk
    fn compile_arrow_expression_body(
        &mut self,
        params: &[crate::ast::FunctionParam],
        expr: &crate::ast::Expression,
        is_async: bool,
    ) -> Result<super::BytecodeChunk, JsError> {
        use super::FunctionInfo;

        // Create a new compiler for the function body
        let mut func_compiler = super::Compiler::new();

        // Compile parameter declarations
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

                        let arg_reg = idx as u8;
                        let name_idx = func_compiler.builder.add_string(id.name.cheap_clone())?;
                        func_compiler.builder.emit(Op::DeclareVar {
                            name: name_idx,
                            init: arg_reg,
                            mutable: true,
                        });
                    }
                }
                _ => {}
            }
        }

        // Compile the expression and return it
        let result_reg = func_compiler.builder.alloc_register()?;
        func_compiler.compile_expression(expr, result_reg)?;
        func_compiler.builder.emit(Op::Return { value: result_reg });

        // Build the chunk with function info
        let mut chunk = func_compiler.builder.finish();
        chunk.function_info = Some(FunctionInfo {
            name: None,
            param_count: params.len(),
            is_generator: false,
            is_async,
            is_arrow: true,
            uses_arguments: false,
            uses_this: false,
            param_names,
            rest_param,
        });

        Ok(chunk)
    }

    /// Compile class expression
    fn compile_class_expression(
        &mut self,
        _class: &crate::ast::ClassExpression,
        _dst: Register,
    ) -> Result<(), JsError> {
        // TODO: Implement class compilation
        Err(JsError::syntax_error_simple(
            "Class expressions not yet supported in bytecode compiler",
        ))
    }

    // Helper methods for member access

    /// Get key info for a member property
    fn get_member_key_info(&mut self, prop: &MemberProperty) -> Result<MemberKeyInfo, JsError> {
        match prop {
            MemberProperty::Identifier(id) => {
                let idx = self.builder.add_string(id.name.cheap_clone())?;
                Ok(MemberKeyInfo::Const(idx))
            }
            MemberProperty::Expression(expr) => {
                let reg = self.builder.alloc_register()?;
                self.compile_expression(expr, reg)?;
                Ok(MemberKeyInfo::Computed(reg))
            }
            MemberProperty::PrivateIdentifier(_) => Err(JsError::syntax_error_simple(
                "Private fields not yet supported in bytecode compiler",
            )),
        }
    }

    /// Emit get property based on key info
    fn emit_get_property(
        &mut self,
        dst: Register,
        obj: Register,
        key_info: &MemberKeyInfo,
    ) -> Result<(), JsError> {
        match key_info {
            MemberKeyInfo::Const(idx) => {
                self.builder.emit(Op::GetPropertyConst {
                    dst,
                    obj,
                    key: *idx,
                });
            }
            MemberKeyInfo::Computed(reg) => {
                self.builder.emit(Op::GetProperty {
                    dst,
                    obj,
                    key: *reg,
                });
            }
        }
        Ok(())
    }

    /// Emit set property based on key info
    fn emit_set_property(
        &mut self,
        obj: Register,
        key_info: &MemberKeyInfo,
        value: Register,
    ) -> Result<(), JsError> {
        match key_info {
            MemberKeyInfo::Const(idx) => {
                self.builder.emit(Op::SetPropertyConst {
                    obj,
                    key: *idx,
                    value,
                });
            }
            MemberKeyInfo::Computed(reg) => {
                self.builder.emit(Op::SetProperty {
                    obj,
                    key: *reg,
                    value,
                });
            }
        }
        Ok(())
    }
}
