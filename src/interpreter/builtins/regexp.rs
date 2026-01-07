//! RegExp built-in methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::platform::CompiledRegex;
use crate::prelude::{Rc, String, ToString, Vec, format};
use crate::value::{ExoticObject, Guarded, JsObject, JsString, JsValue, PropertyKey};

// ═══════════════════════════════════════════════════════════════════════════════
// Backward Compatibility: build_regex for string methods
// ═══════════════════════════════════════════════════════════════════════════════
//
// The string methods (match, replace, split, search, matchAll) currently use
// fancy_regex methods directly (captures_iter, split, etc.) which aren't abstracted
// by the CompiledRegex trait yet. For backward compatibility, we keep build_regex
// here which directly uses fancy_regex when the feature is enabled.
//
// TODO: In a future iteration, update string methods to use CompiledRegex trait
// methods so they can benefit from custom providers (like WasmRegExpProvider).

#[cfg(feature = "regex")]
mod regex_compat {
    use super::*;

    /// Convert a JavaScript regex pattern to a Rust regex pattern.
    /// Handles differences between JS and Rust regex syntax.
    fn js_regex_to_rust(pattern: &str) -> String {
        let mut result = String::with_capacity(pattern.len() + 16);
        let chars: Vec<char> = pattern.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut in_char_class = false;
        let mut char_class_start = false;

        while i < len {
            let Some(c) = chars.get(i).copied() else {
                break;
            };

            if c == '\\' {
                if let Some(next) = chars.get(i + 1).copied() {
                    result.push(c);
                    result.push(next);
                    i += 2;
                    char_class_start = false;
                    continue;
                }
            }

            if !in_char_class {
                if c == '[' {
                    in_char_class = true;
                    char_class_start = true;
                    result.push(c);
                } else {
                    result.push(c);
                }
            } else {
                if char_class_start {
                    if c == '^' {
                        result.push(c);
                    } else if c == ']' {
                        result.push(c);
                        char_class_start = false;
                    } else if c == '[' {
                        result.push('\\');
                        result.push('[');
                        char_class_start = false;
                    } else {
                        result.push(c);
                        char_class_start = false;
                    }
                } else if c == ']' {
                    in_char_class = false;
                    result.push(c);
                } else if c == '[' {
                    result.push('\\');
                    result.push('[');
                } else {
                    result.push(c);
                }
            }
            i += 1;
        }

        result
    }

    /// Build a fancy_regex::Regex from pattern and flags.
    /// Used by string methods that need direct access to fancy_regex features.
    pub fn build_regex(pattern: &str, flags: &str) -> Result<fancy_regex::Regex, JsError> {
        let mut regex_pattern = js_regex_to_rust(pattern);
        let mut prefix = String::new();

        if flags.contains('i') {
            prefix.push('i');
        }
        if flags.contains('m') {
            prefix.push('m');
        }
        if flags.contains('s') {
            prefix.push('s');
        }

        if !prefix.is_empty() {
            regex_pattern = format!("(?{}){}", prefix, regex_pattern);
        }

        fancy_regex::Regex::new(&regex_pattern)
            .map_err(|e| JsError::syntax_error(format!("Invalid regular expression: {}", e), 0, 0))
    }
}

#[cfg(feature = "regex")]
pub use regex_compat::build_regex;

/// Initialize RegExp.prototype with test and exec methods
pub fn init_regexp_prototype(interp: &mut Interpreter) {
    let proto = interp.regexp_prototype.clone();

    interp.register_method(&proto, "test", regexp_test, 1);
    interp.register_method(&proto, "exec", regexp_exec, 1);
}

/// Create RegExp constructor
pub fn create_regexp_constructor(interp: &mut Interpreter) -> Gc<JsObject> {
    let constructor = interp.create_native_function("RegExp", regexp_constructor, 2);

    // Set constructor.prototype = RegExp.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.regexp_prototype.clone()));

    // Set RegExp.prototype.constructor = RegExp
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .regexp_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(constructor.clone()));

    // Add Symbol.species getter
    interp.register_species_getter(&constructor);

    constructor
}

pub fn regexp_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let empty = interp.intern("");
    let pattern_arg = args
        .first()
        .cloned()
        .unwrap_or(JsValue::String(empty.clone()));
    let pattern = interp.to_js_string(&pattern_arg).to_string();
    let flags_arg = args.get(1).cloned().unwrap_or(JsValue::String(empty));
    let flags = interp.to_js_string(&flags_arg).to_string();

    // Compile the regex to validate it (also caches for later use)
    let compiled = interp.compile_regexp(&pattern, &flags)?;

    // Pre-intern all property keys
    let source_key = PropertyKey::String(interp.intern("source"));
    let flags_key = PropertyKey::String(interp.intern("flags"));
    let global_key = PropertyKey::String(interp.intern("global"));
    let ignore_case_key = PropertyKey::String(interp.intern("ignoreCase"));
    let multiline_key = PropertyKey::String(interp.intern("multiline"));
    let dot_all_key = PropertyKey::String(interp.intern("dotAll"));
    let unicode_key = PropertyKey::String(interp.intern("unicode"));
    let sticky_key = PropertyKey::String(interp.intern("sticky"));
    let last_index_key = PropertyKey::String(interp.intern("lastIndex"));

    let guard = interp.heap.create_guard();
    let regexp_obj = interp.create_object(&guard);
    {
        let mut obj = regexp_obj.borrow_mut();
        obj.exotic = ExoticObject::RegExp {
            pattern: pattern.clone(),
            flags: flags.clone(),
            compiled: Some(compiled),
        };
        obj.prototype = Some(interp.regexp_prototype.clone());
        obj.set_property(source_key, JsValue::String(JsString::from(pattern)));
        obj.set_property(flags_key, JsValue::String(JsString::from(flags.clone())));
        obj.set_property(global_key, JsValue::Boolean(flags.contains('g')));
        obj.set_property(ignore_case_key, JsValue::Boolean(flags.contains('i')));
        obj.set_property(multiline_key, JsValue::Boolean(flags.contains('m')));
        obj.set_property(dot_all_key, JsValue::Boolean(flags.contains('s')));
        obj.set_property(unicode_key, JsValue::Boolean(flags.contains('u')));
        obj.set_property(sticky_key, JsValue::Boolean(flags.contains('y')));
        // Initialize lastIndex to 0
        obj.set_property(last_index_key, JsValue::Number(0.0));
    }
    Ok(Guarded::with_guard(JsValue::Object(regexp_obj), guard))
}

/// Get pattern and flags from a RegExp object
pub fn get_regexp_data(this: &JsValue) -> Result<(String, String), JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error("this is not a RegExp"));
    };
    let obj_ref = obj.borrow();
    if let ExoticObject::RegExp {
        ref pattern,
        ref flags,
        ..
    } = obj_ref.exotic
    {
        Ok((pattern.clone(), flags.clone()))
    } else {
        Err(JsError::type_error("this is not a RegExp"))
    }
}

/// Get the compiled regex from a RegExp object, compiling if needed
pub fn get_compiled_regexp(
    interp: &Interpreter,
    obj: &Gc<JsObject>,
) -> Result<Rc<dyn CompiledRegex>, JsError> {
    let obj_ref = obj.borrow();
    if let ExoticObject::RegExp {
        ref pattern,
        ref flags,
        ref compiled,
        ..
    } = obj_ref.exotic
    {
        if let Some(compiled) = compiled {
            return Ok(compiled.clone());
        }
        // Not cached - compile now
        let pattern = pattern.clone();
        let flags = flags.clone();
        drop(obj_ref);
        interp.compile_regexp(&pattern, &flags)
    } else {
        Err(JsError::type_error("this is not a RegExp"))
    }
}

pub fn regexp_test(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(ref obj) = this else {
        return Err(JsError::type_error("this is not a RegExp"));
    };

    let re = get_compiled_regexp(interp, obj)?;

    // Use ToString abstract operation (calls object's toString if needed)
    let input_arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let input = interp.coerce_to_string(&input_arg)?.to_string();

    let is_match = re
        .is_match(&input)
        .map_err(|e| JsError::syntax_error(e, 0, 0))?;
    Ok(Guarded::unguarded(JsValue::Boolean(is_match)))
}

pub fn regexp_exec(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let JsValue::Object(ref obj) = this else {
        return Err(JsError::type_error("this is not a RegExp"));
    };

    let (_, flags) = get_regexp_data(&this)?;
    let re = get_compiled_regexp(interp, obj)?;

    // Use ToString abstract operation (calls object's toString if needed)
    let input_arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let input = interp.coerce_to_string(&input_arg)?.to_string();

    let is_global = flags.contains('g');
    let is_sticky = flags.contains('y');

    // Pre-intern property keys
    let last_index_key = PropertyKey::String(interp.intern("lastIndex"));
    let index_key = PropertyKey::String(interp.intern("index"));
    let input_key = PropertyKey::String(interp.intern("input"));

    // Get lastIndex for global/sticky regexes
    let last_index = if is_global || is_sticky {
        let li = obj
            .borrow()
            .get_property(&last_index_key)
            .unwrap_or(JsValue::Number(0.0));
        match li {
            JsValue::Number(n) => n as usize,
            _ => 0,
        }
    } else {
        0
    };

    // Check if lastIndex is past end of string
    if last_index > input.len() {
        if is_global || is_sticky {
            obj.borrow_mut()
                .set_property(last_index_key, JsValue::Number(0.0));
        }
        return Ok(Guarded::unguarded(JsValue::Null));
    }

    // Use the provider's find method which handles start position
    let match_result = re
        .find(&input, last_index)
        .map_err(|e| JsError::syntax_error(e, 0, 0))?;

    match match_result {
        Some(regex_match) => {
            // Build result array from captures
            let mut result = Vec::new();
            for capture in &regex_match.captures {
                match capture {
                    Some((start, end)) => {
                        let s = input.get(*start..*end).unwrap_or("");
                        result.push(JsValue::String(JsString::from(s)));
                    }
                    None => result.push(JsValue::Undefined),
                }
            }

            let guard = interp.heap.create_guard();
            let arr = interp.create_array_from(&guard, result);

            // Set index property (match start position)
            arr.borrow_mut()
                .set_property(index_key, JsValue::Number(regex_match.start as f64));
            arr.borrow_mut()
                .set_property(input_key, JsValue::String(JsString::from(input.clone())));

            // Update lastIndex for global/sticky regexes
            if is_global || is_sticky {
                obj.borrow_mut()
                    .set_property(last_index_key, JsValue::Number(regex_match.end as f64));
            }

            Ok(Guarded::with_guard(JsValue::Object(arr), guard))
        }
        None => {
            // Reset lastIndex to 0 on no match for global/sticky
            if is_global || is_sticky {
                obj.borrow_mut()
                    .set_property(last_index_key, JsValue::Number(0.0));
            }
            Ok(Guarded::unguarded(JsValue::Null))
        }
    }
}
