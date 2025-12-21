//! Pattern compilation
//!
//! Compiles destructuring patterns to bytecode.

use super::bytecode::{Op, Register};
use super::Compiler;
use crate::ast::{ObjectPatternProperty, ObjectPropertyKey, Pattern};
use crate::error::JsError;
use crate::value::CheapClone;

impl Compiler {
    /// Compile a pattern binding (for variable declarations)
    /// Creates new variable bindings from the pattern
    pub fn compile_pattern_binding(
        &mut self,
        pattern: &Pattern,
        value_reg: Register,
        mutable: bool,
        is_var: bool,
    ) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                let name_idx = self.builder.add_string(id.name.cheap_clone())?;
                if is_var {
                    // For var declarations, check if already hoisted
                    if self.is_hoisted(&id.name) {
                        // Already hoisted - just assign
                        self.builder.emit(Op::SetVar {
                            name: name_idx,
                            src: value_reg,
                        });
                    } else {
                        // Not hoisted yet (e.g., inside eval or dynamic scope)
                        self.builder.emit(Op::DeclareVarHoisted {
                            name: name_idx,
                            init: value_reg,
                        });
                    }
                } else {
                    self.builder.emit(Op::DeclareVar {
                        name: name_idx,
                        init: value_reg,
                        mutable,
                    });
                }
                Ok(())
            }

            Pattern::Object(obj_pat) => {
                self.compile_object_pattern_binding(obj_pat, value_reg, mutable, is_var)
            }

            Pattern::Array(arr_pat) => {
                self.compile_array_pattern_binding(arr_pat, value_reg, mutable, is_var)
            }

            Pattern::Assignment(assign_pat) => {
                // Pattern with default value: pattern = defaultValue
                // If value is undefined, use default
                let actual_value = self.builder.alloc_register()?;

                // Check if value is undefined
                let is_undefined = self.builder.alloc_register()?;
                self.builder.emit(Op::LoadUndefined { dst: is_undefined });
                self.builder.emit(Op::StrictEq {
                    dst: is_undefined,
                    left: value_reg,
                    right: is_undefined,
                });

                // If undefined, use default; otherwise use value
                let skip_default = self.builder.emit_jump_if_false(is_undefined);
                self.builder.free_register(is_undefined);

                // Use default value
                self.compile_expression(&assign_pat.right, actual_value)?;
                let skip_value = self.builder.emit_jump();

                // Use provided value
                self.builder.patch_jump(skip_default);
                self.builder.emit(Op::Move {
                    dst: actual_value,
                    src: value_reg,
                });

                self.builder.patch_jump(skip_value);

                // Recursively bind to the inner pattern
                self.compile_pattern_binding(&assign_pat.left, actual_value, mutable, is_var)?;

                self.builder.free_register(actual_value);
                Ok(())
            }

            Pattern::Rest(rest) => {
                // Rest element in pattern - this should only appear in array patterns
                // At this level, value_reg already contains the rest array
                self.compile_pattern_binding(&rest.argument, value_reg, mutable, is_var)
            }
        }
    }

    /// Compile object pattern binding
    fn compile_object_pattern_binding(
        &mut self,
        obj_pat: &crate::ast::ObjectPattern,
        value_reg: Register,
        mutable: bool,
        is_var: bool,
    ) -> Result<(), JsError> {
        use crate::value::JsString;

        // First pass: collect keys that will be extracted (for rest handling)
        let mut extracted_keys: Vec<JsString> = Vec::new();
        let mut has_rest = false;

        for prop in &obj_pat.properties {
            match prop {
                ObjectPatternProperty::KeyValue { key, .. } => {
                    // Collect the key name for rest handling
                    match key {
                        ObjectPropertyKey::Identifier(id) => {
                            extracted_keys.push(id.name.cheap_clone());
                        }
                        ObjectPropertyKey::String(s) => {
                            extracted_keys.push(s.value.cheap_clone());
                        }
                        // Computed keys can't be statically known, skip them
                        // (rest will still work but may include some extra props)
                        ObjectPropertyKey::Computed(_) | ObjectPropertyKey::Number(_) => {}
                        ObjectPropertyKey::PrivateIdentifier(_) => {}
                    }
                }
                ObjectPatternProperty::Rest(_) => {
                    has_rest = true;
                }
            }
        }

        // Second pass: compile bindings
        for prop in &obj_pat.properties {
            match prop {
                ObjectPatternProperty::KeyValue {
                    key,
                    value,
                    shorthand: _,
                    span: _,
                } => {
                    // Get property from object
                    let prop_value = self.builder.alloc_register()?;

                    match key {
                        ObjectPropertyKey::Identifier(id) => {
                            let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                            self.builder.emit(Op::GetPropertyConst {
                                dst: prop_value,
                                obj: value_reg,
                                key: key_idx,
                            });
                        }
                        ObjectPropertyKey::String(s) => {
                            let key_idx = self.builder.add_string(s.value.cheap_clone())?;
                            self.builder.emit(Op::GetPropertyConst {
                                dst: prop_value,
                                obj: value_reg,
                                key: key_idx,
                            });
                        }
                        ObjectPropertyKey::Computed(expr) => {
                            let key_reg = self.builder.alloc_register()?;
                            self.compile_expression(expr, key_reg)?;
                            self.builder.emit(Op::GetProperty {
                                dst: prop_value,
                                obj: value_reg,
                                key: key_reg,
                            });
                            self.builder.free_register(key_reg);
                        }
                        ObjectPropertyKey::Number(lit) => {
                            let key_reg = self.builder.alloc_register()?;
                            self.compile_literal(&lit.value, key_reg)?;
                            self.builder.emit(Op::GetProperty {
                                dst: prop_value,
                                obj: value_reg,
                                key: key_reg,
                            });
                            self.builder.free_register(key_reg);
                        }
                        ObjectPropertyKey::PrivateIdentifier(_) => {
                            return Err(JsError::syntax_error_simple(
                                "Private identifiers not yet supported in destructuring",
                            ));
                        }
                    }

                    // Bind to the value pattern
                    self.compile_pattern_binding(value, prop_value, mutable, is_var)?;
                    self.builder.free_register(prop_value);
                }
                ObjectPatternProperty::Rest(rest) => {
                    // Rest in object destructuring: { a, ...rest }
                    // Create object with remaining properties (excluding extracted keys)
                    let rest_obj = self.builder.alloc_register()?;

                    if has_rest && !extracted_keys.is_empty() {
                        // Add excluded keys to constant pool
                        let excluded_idx =
                            self.builder.add_excluded_keys(extracted_keys.clone())?;
                        self.builder.emit(Op::CreateObjectRest {
                            dst: rest_obj,
                            src: value_reg,
                            excluded_keys: excluded_idx,
                        });
                    } else {
                        // No keys to exclude, copy all properties
                        let excluded_idx = self.builder.add_excluded_keys(vec![])?;
                        self.builder.emit(Op::CreateObjectRest {
                            dst: rest_obj,
                            src: value_reg,
                            excluded_keys: excluded_idx,
                        });
                    }

                    self.compile_pattern_binding(&rest.argument, rest_obj, mutable, is_var)?;
                    self.builder.free_register(rest_obj);
                }
            }
        }
        Ok(())
    }

    /// Compile array pattern binding
    fn compile_array_pattern_binding(
        &mut self,
        arr_pat: &crate::ast::ArrayPattern,
        value_reg: Register,
        mutable: bool,
        is_var: bool,
    ) -> Result<(), JsError> {
        // Get iterator for the value
        let iter_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::GetIterator {
            dst: iter_reg,
            obj: value_reg,
        });

        let result_reg = self.builder.alloc_register()?;
        let elem_value = self.builder.alloc_register()?;

        for (i, elem) in arr_pat.elements.iter().enumerate() {
            if let Some(pattern) = elem {
                // Check for rest pattern
                if let Pattern::Rest(rest) = pattern {
                    // Collect remaining elements into an array
                    let rest_arr = self.builder.alloc_register()?;
                    self.builder.emit(Op::CreateRestArray {
                        dst: rest_arr,
                        start_index: i as u8,
                    });
                    self.compile_pattern_binding(&rest.argument, rest_arr, mutable, is_var)?;
                    self.builder.free_register(rest_arr);
                    break;
                }

                // Get next iterator value
                self.builder.emit(Op::IteratorNext {
                    dst: result_reg,
                    iterator: iter_reg,
                });

                // Get the value (or undefined if done)
                self.builder.emit(Op::IteratorValue {
                    dst: elem_value,
                    result: result_reg,
                });

                // Bind to the pattern
                self.compile_pattern_binding(pattern, elem_value, mutable, is_var)?;
            } else {
                // Hole in array pattern - skip this element
                self.builder.emit(Op::IteratorNext {
                    dst: result_reg,
                    iterator: iter_reg,
                });
            }
        }

        self.builder.free_register(elem_value);
        self.builder.free_register(result_reg);
        self.builder.free_register(iter_reg);

        Ok(())
    }

    /// Compile a pattern assignment (for assignment expressions)
    /// Assigns to existing bindings from the pattern
    pub fn compile_pattern_assignment(
        &mut self,
        pattern: &Pattern,
        value_reg: Register,
    ) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                let name_idx = self.builder.add_string(id.name.cheap_clone())?;
                self.builder.emit(Op::SetVar {
                    name: name_idx,
                    src: value_reg,
                });
                Ok(())
            }

            Pattern::Object(obj_pat) => self.compile_object_pattern_assignment(obj_pat, value_reg),

            Pattern::Array(arr_pat) => self.compile_array_pattern_assignment(arr_pat, value_reg),

            Pattern::Assignment(assign_pat) => {
                // Pattern with default value
                let actual_value = self.builder.alloc_register()?;

                // Check if value is undefined
                let is_undefined = self.builder.alloc_register()?;
                self.builder.emit(Op::LoadUndefined { dst: is_undefined });
                self.builder.emit(Op::StrictEq {
                    dst: is_undefined,
                    left: value_reg,
                    right: is_undefined,
                });

                let skip_default = self.builder.emit_jump_if_false(is_undefined);
                self.builder.free_register(is_undefined);

                // Use default
                self.compile_expression(&assign_pat.right, actual_value)?;
                let skip_value = self.builder.emit_jump();

                // Use provided value
                self.builder.patch_jump(skip_default);
                self.builder.emit(Op::Move {
                    dst: actual_value,
                    src: value_reg,
                });

                self.builder.patch_jump(skip_value);

                // Recursively assign
                self.compile_pattern_assignment(&assign_pat.left, actual_value)?;

                self.builder.free_register(actual_value);
                Ok(())
            }

            Pattern::Rest(rest) => self.compile_pattern_assignment(&rest.argument, value_reg),
        }
    }

    /// Compile object pattern assignment
    fn compile_object_pattern_assignment(
        &mut self,
        obj_pat: &crate::ast::ObjectPattern,
        value_reg: Register,
    ) -> Result<(), JsError> {
        use crate::value::JsString;

        // First pass: collect keys that will be extracted (for rest handling)
        let mut extracted_keys: Vec<JsString> = Vec::new();
        let mut has_rest = false;

        for prop in &obj_pat.properties {
            match prop {
                ObjectPatternProperty::KeyValue { key, .. } => {
                    match key {
                        ObjectPropertyKey::Identifier(id) => {
                            extracted_keys.push(id.name.cheap_clone());
                        }
                        ObjectPropertyKey::String(s) => {
                            extracted_keys.push(s.value.cheap_clone());
                        }
                        ObjectPropertyKey::Computed(_) | ObjectPropertyKey::Number(_) => {}
                        ObjectPropertyKey::PrivateIdentifier(_) => {}
                    }
                }
                ObjectPatternProperty::Rest(_) => {
                    has_rest = true;
                }
            }
        }

        // Second pass: compile assignments
        for prop in &obj_pat.properties {
            match prop {
                ObjectPatternProperty::KeyValue {
                    key,
                    value,
                    shorthand: _,
                    span: _,
                } => {
                    let prop_value = self.builder.alloc_register()?;

                    match key {
                        ObjectPropertyKey::Identifier(id) => {
                            let key_idx = self.builder.add_string(id.name.cheap_clone())?;
                            self.builder.emit(Op::GetPropertyConst {
                                dst: prop_value,
                                obj: value_reg,
                                key: key_idx,
                            });
                        }
                        ObjectPropertyKey::String(s) => {
                            let key_idx = self.builder.add_string(s.value.cheap_clone())?;
                            self.builder.emit(Op::GetPropertyConst {
                                dst: prop_value,
                                obj: value_reg,
                                key: key_idx,
                            });
                        }
                        ObjectPropertyKey::Computed(expr) => {
                            let key_reg = self.builder.alloc_register()?;
                            self.compile_expression(expr, key_reg)?;
                            self.builder.emit(Op::GetProperty {
                                dst: prop_value,
                                obj: value_reg,
                                key: key_reg,
                            });
                            self.builder.free_register(key_reg);
                        }
                        ObjectPropertyKey::Number(lit) => {
                            let key_reg = self.builder.alloc_register()?;
                            self.compile_literal(&lit.value, key_reg)?;
                            self.builder.emit(Op::GetProperty {
                                dst: prop_value,
                                obj: value_reg,
                                key: key_reg,
                            });
                            self.builder.free_register(key_reg);
                        }
                        ObjectPropertyKey::PrivateIdentifier(_) => {
                            return Err(JsError::syntax_error_simple(
                                "Private identifiers not yet supported in destructuring",
                            ));
                        }
                    }

                    self.compile_pattern_assignment(value, prop_value)?;
                    self.builder.free_register(prop_value);
                }
                ObjectPatternProperty::Rest(rest) => {
                    let rest_obj = self.builder.alloc_register()?;

                    if has_rest {
                        let excluded_idx =
                            self.builder.add_excluded_keys(extracted_keys.clone())?;
                        self.builder.emit(Op::CreateObjectRest {
                            dst: rest_obj,
                            src: value_reg,
                            excluded_keys: excluded_idx,
                        });
                    } else {
                        let excluded_idx = self.builder.add_excluded_keys(vec![])?;
                        self.builder.emit(Op::CreateObjectRest {
                            dst: rest_obj,
                            src: value_reg,
                            excluded_keys: excluded_idx,
                        });
                    }

                    self.compile_pattern_assignment(&rest.argument, rest_obj)?;
                    self.builder.free_register(rest_obj);
                }
            }
        }
        Ok(())
    }

    /// Compile array pattern assignment
    fn compile_array_pattern_assignment(
        &mut self,
        arr_pat: &crate::ast::ArrayPattern,
        value_reg: Register,
    ) -> Result<(), JsError> {
        let iter_reg = self.builder.alloc_register()?;
        self.builder.emit(Op::GetIterator {
            dst: iter_reg,
            obj: value_reg,
        });

        let result_reg = self.builder.alloc_register()?;
        let elem_value = self.builder.alloc_register()?;

        for (i, elem) in arr_pat.elements.iter().enumerate() {
            if let Some(pattern) = elem {
                if let Pattern::Rest(rest) = pattern {
                    let rest_arr = self.builder.alloc_register()?;
                    self.builder.emit(Op::CreateRestArray {
                        dst: rest_arr,
                        start_index: i as u8,
                    });
                    self.compile_pattern_assignment(&rest.argument, rest_arr)?;
                    self.builder.free_register(rest_arr);
                    break;
                }

                self.builder.emit(Op::IteratorNext {
                    dst: result_reg,
                    iterator: iter_reg,
                });

                self.builder.emit(Op::IteratorValue {
                    dst: elem_value,
                    result: result_reg,
                });

                self.compile_pattern_assignment(pattern, elem_value)?;
            } else {
                self.builder.emit(Op::IteratorNext {
                    dst: result_reg,
                    iterator: iter_reg,
                });
            }
        }

        self.builder.free_register(elem_value);
        self.builder.free_register(result_reg);
        self.builder.free_register(iter_reg);

        Ok(())
    }
}
