//! RegExp built-in methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsObject, JsString, JsValue, PropertyKey};

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

pub fn get_regexp_data(this: &JsValue) -> Result<(String, String), JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error("this is not a RegExp"));
    };
    let obj_ref = obj.borrow();
    if let ExoticObject::RegExp {
        ref pattern,
        ref flags,
    } = obj_ref.exotic
    {
        Ok((pattern.clone(), flags.clone()))
    } else {
        Err(JsError::type_error("this is not a RegExp"))
    }
}

/// Convert a JavaScript regex pattern to a Rust regex pattern.
/// Handles differences between JS and Rust regex syntax:
/// - In JS, `[` inside a character class is a literal character
/// - In Rust, `[` inside a character class needs to be escaped as `\[`
fn js_regex_to_rust(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len() + 16);
    let chars: Vec<char> = pattern.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_char_class = false;
    let mut char_class_start = false; // True right after [ or [^

    while i < len {
        let Some(c) = chars.get(i).copied() else {
            break;
        };

        if c == '\\'
            && let Some(next) = chars.get(i + 1).copied()
        {
            // Escaped character - copy both chars and skip
            result.push(c);
            result.push(next);
            i += 2;
            char_class_start = false;
            continue;
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
            // Inside character class
            if char_class_start {
                // First char(s) after [ have special meaning
                if c == '^' {
                    result.push(c);
                    // Still in char_class_start mode - next char could be ]
                } else if c == ']' {
                    // ] right after [ or [^ is a literal ]
                    result.push(c);
                    char_class_start = false;
                } else if c == '[' {
                    // [ at start of class - needs escaping for Rust
                    result.push('\\');
                    result.push('[');
                    char_class_start = false;
                } else {
                    result.push(c);
                    char_class_start = false;
                }
            } else if c == ']' {
                // End of character class
                in_char_class = false;
                result.push(c);
            } else if c == '[' {
                // Unescaped [ inside character class - escape it for Rust
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

pub fn build_regex(pattern: &str, flags: &str) -> Result<fancy_regex::Regex, JsError> {
    // Convert JS regex syntax to Rust regex syntax
    let mut regex_pattern = js_regex_to_rust(pattern);

    // Build flags prefix
    let mut prefix = String::new();

    // Handle case-insensitive flag (i)
    if flags.contains('i') {
        prefix.push('i');
    }

    // Handle multiline flag (m)
    if flags.contains('m') {
        prefix.push('m');
    }

    // Handle dotAll flag (s) - makes . match newlines
    if flags.contains('s') {
        prefix.push('s');
    }

    if !prefix.is_empty() {
        regex_pattern = format!("(?{}){}", prefix, regex_pattern);
    }

    fancy_regex::Regex::new(&regex_pattern)
        .map_err(|e| JsError::syntax_error(format!("Invalid regular expression: {}", e), 0, 0))
}

pub fn regexp_test(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let (pattern, flags) = get_regexp_data(&this)?;

    // Use ToString abstract operation (calls object's toString if needed)
    let input_arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let input = interp.coerce_to_string(&input_arg)?.to_string();

    let re = build_regex(&pattern, &flags)?;
    let is_match = re
        .is_match(&input)
        .map_err(|e| JsError::syntax_error(format!("Regex execution error: {}", e), 0, 0))?;
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

    let (pattern, flags) = get_regexp_data(&this)?;

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

    let re = build_regex(&pattern, &flags)?;

    // For global/sticky, we need to search starting from lastIndex
    let search_str = if last_index > 0 && last_index <= input.len() {
        // Get substring starting from lastIndex (handle UTF-8 properly)
        input
            .char_indices()
            .nth(last_index)
            .and_then(|(byte_idx, _)| input.get(byte_idx..))
            .unwrap_or("")
    } else if last_index > input.len() {
        // lastIndex past end of string - no match
        if is_global || is_sticky {
            obj.borrow_mut()
                .set_property(last_index_key, JsValue::Number(0.0));
        }
        return Ok(Guarded::unguarded(JsValue::Null));
    } else {
        input.as_str()
    };

    let captures_result = re
        .captures(search_str)
        .map_err(|e| JsError::syntax_error(format!("Regex execution error: {}", e), 0, 0))?;

    match captures_result {
        Some(caps) => {
            let mut result = Vec::new();
            for cap in caps.iter() {
                match cap {
                    Some(m) => result.push(JsValue::String(JsString::from(m.as_str()))),
                    None => result.push(JsValue::Undefined),
                }
            }
            let guard = interp.heap.create_guard();
            let arr = interp.create_array_from(&guard, result);

            // Calculate actual index in original string
            let match_start = caps.get(0).map(|m| m.start()).unwrap_or(0);
            let actual_index = last_index + match_start;

            arr.borrow_mut()
                .set_property(index_key, JsValue::Number(actual_index as f64));
            arr.borrow_mut()
                .set_property(input_key, JsValue::String(JsString::from(input.clone())));

            // Update lastIndex for global/sticky regexes
            if is_global || is_sticky {
                let match_end = caps.get(0).map(|m| m.end()).unwrap_or(0);
                let new_last_index = last_index + match_end;
                obj.borrow_mut()
                    .set_property(last_index_key, JsValue::Number(new_last_index as f64));
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
