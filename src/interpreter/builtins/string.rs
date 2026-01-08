//! String built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::prelude::{String, ToString, Vec, format, math, vec};
use crate::value::{CheapClone, Guarded, JsObjectRef, JsString, JsValue, PropertyKey};

/// Initialize String.prototype with all string methods.
/// The prototype object must already exist in `interp.string_prototype`.
pub fn init_string_prototype(interp: &mut Interpreter) {
    let proto = interp.string_prototype.clone();

    // Character access
    interp.register_method(&proto, "charAt", string_char_at, 1);
    interp.register_method(&proto, "charCodeAt", string_char_code_at, 1);
    interp.register_method(&proto, "codePointAt", string_code_point_at, 1);
    interp.register_method(&proto, "at", string_at, 1);

    // Search methods
    interp.register_method(&proto, "indexOf", string_index_of, 1);
    interp.register_method(&proto, "lastIndexOf", string_last_index_of, 1);
    interp.register_method(&proto, "includes", string_includes, 1);
    interp.register_method(&proto, "startsWith", string_starts_with, 1);
    interp.register_method(&proto, "endsWith", string_ends_with, 1);
    interp.register_method(&proto, "search", string_search, 1);

    // Extraction methods
    interp.register_method(&proto, "slice", string_slice, 2);
    interp.register_method(&proto, "substring", string_substring, 2);
    interp.register_method(&proto, "substr", string_substr, 2);

    // Case conversion
    interp.register_method(&proto, "toLowerCase", string_to_lower_case, 0);
    interp.register_method(&proto, "toUpperCase", string_to_upper_case, 0);

    // Whitespace handling
    interp.register_method(&proto, "trim", string_trim, 0);
    interp.register_method(&proto, "trimStart", string_trim_start, 0);
    interp.register_method(&proto, "trimEnd", string_trim_end, 0);

    // Transformation methods
    interp.register_method(&proto, "split", string_split, 2);
    interp.register_method(&proto, "repeat", string_repeat, 1);
    interp.register_method(&proto, "replace", string_replace, 2);
    interp.register_method(&proto, "replaceAll", string_replace_all, 2);
    interp.register_method(&proto, "padStart", string_pad_start, 2);
    interp.register_method(&proto, "padEnd", string_pad_end, 2);
    interp.register_method(&proto, "concat", string_concat, 1);
    interp.register_method(&proto, "normalize", string_normalize, 1);
    interp.register_method(&proto, "match", string_match, 1);
    interp.register_method(&proto, "matchAll", string_match_all, 1);

    // Comparison
    interp.register_method(&proto, "localeCompare", string_locale_compare, 1);

    // Primitive conversion
    interp.register_method(&proto, "valueOf", string_value_of, 0);
    interp.register_method(&proto, "toString", string_to_string, 0);
}

/// String constructor function - String(value) converts value to string
/// When called without `new`, returns a primitive string
/// When called with `new`, returns a String wrapper object
pub fn string_constructor_fn(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use crate::value::ExoticObject;

    // Get the string value from argument - use coerce_to_string for ToPrimitive handling
    let str_val = if args.is_empty() {
        interp.intern("")
    } else {
        let arg = args.first().cloned().unwrap_or(JsValue::Undefined);
        interp.coerce_to_string(&arg)?
    };

    // Check if called with `new` (this will be a fresh object with String.prototype)
    if let JsValue::Object(obj) = &this {
        // Check if this object was created by the `new` operator
        // by checking if it has string_prototype as its prototype
        let is_new_call = {
            let borrowed = obj.borrow();
            if let Some(ref proto) = borrowed.prototype {
                core::ptr::eq(
                    &*proto.borrow() as *const _,
                    &*interp.string_prototype.borrow() as *const _,
                )
            } else {
                false
            }
        };

        if is_new_call {
            // Called with `new` - set the internal string value to make it a String wrapper
            obj.borrow_mut().exotic = ExoticObject::StringObj(str_val.cheap_clone());
            // Also set the length property (String objects have a read-only length)
            let length_key = PropertyKey::String(interp.intern("length"));
            obj.borrow_mut().set_property(
                length_key,
                JsValue::Number(str_val.as_str().chars().count() as f64),
            );
            return Ok(Guarded::unguarded(this));
        }
    }

    // Called as function - return primitive string
    Ok(Guarded::unguarded(JsValue::String(str_val)))
}

/// Create String constructor with static methods (fromCharCode, fromCodePoint)
pub fn create_string_constructor(interp: &mut Interpreter) -> JsObjectRef {
    let constructor = interp.create_native_function("String", string_constructor_fn, 1);

    interp.register_method(&constructor, "fromCharCode", string_from_char_code, 1);
    interp.register_method(&constructor, "fromCodePoint", string_from_code_point, 1);

    // Set constructor.prototype = String.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.string_prototype.clone()));

    // Set String.prototype.constructor = String
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .string_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(constructor.clone()));

    constructor
}

/// String.prototype.valueOf()
/// Returns the primitive string value
pub fn string_value_of(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let str_val = get_string_value(&this)?;
    Ok(Guarded::unguarded(JsValue::String(str_val)))
}

/// String.prototype.toString()
/// Returns the primitive string value (same as valueOf for String)
pub fn string_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let str_val = get_string_value(&this)?;
    Ok(Guarded::unguarded(JsValue::String(str_val)))
}

/// Helper to extract string value from `this`
/// Works for both primitive strings and String wrapper objects
fn get_string_value(this: &JsValue) -> Result<JsString, JsError> {
    use crate::value::ExoticObject;

    match this {
        JsValue::String(s) => Ok(s.cheap_clone()),
        JsValue::Object(obj) => {
            let borrowed = obj.borrow();
            match &borrowed.exotic {
                ExoticObject::StringObj(s) => Ok(s.cheap_clone()),
                _ => Err(JsError::type_error(
                    "String.prototype method called on incompatible receiver",
                )),
            }
        }
        _ => Err(JsError::type_error(
            "String.prototype method called on incompatible receiver",
        )),
    }
}

pub fn string_char_at(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    // ToInteger: convert to number (with ToPrimitive for objects), then truncate towards zero
    let index_num = if let Some(v) = args.first() {
        interp.coerce_to_number(v)?
    } else {
        0.0
    };

    // Handle NaN -> 0, otherwise truncate
    let index = if index_num.is_nan() {
        0i64
    } else {
        math::trunc(index_num) as i64
    };

    // Negative or out of bounds -> empty string
    if index < 0 || index as usize >= s.as_str().chars().count() {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(""))));
    }

    if let Some(ch) = s.as_str().chars().nth(index as usize) {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(
            ch.to_string(),
        ))))
    } else {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(""))))
    }
}

pub fn string_index_of(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let search = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(""),
    };
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(Guarded::unguarded(JsValue::Number(-1.0)));
    }

    // Use get() for safe slicing - from_index is validated above to be < len
    match s
        .as_str()
        .get(from_index..)
        .and_then(|slice| slice.find(search.as_str()))
    {
        Some(pos) => Ok(Guarded::unguarded(JsValue::Number(
            (from_index + pos) as f64,
        ))),
        None => Ok(Guarded::unguarded(JsValue::Number(-1.0))),
    }
}

pub fn string_last_index_of(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let search = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(""),
    };
    let len = s.len();

    // Default from_index is length of string
    let from_index = if let Some(arg) = args.get(1) {
        let n = arg.to_number();
        if n.is_nan() {
            len
        } else {
            (n as isize).max(0) as usize
        }
    } else {
        len
    };

    // Empty search string returns from_index clamped to length
    if search.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Number(
            from_index.min(len) as f64
        )));
    }

    // Search backwards from from_index
    let search_end = (from_index + search.len()).min(len);
    match s
        .as_str()
        .get(..search_end)
        .and_then(|slice| slice.rfind(search.as_str()))
    {
        Some(pos) => Ok(Guarded::unguarded(JsValue::Number(pos as f64))),
        None => Ok(Guarded::unguarded(JsValue::Number(-1.0))),
    }
}

pub fn string_at(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let len = s.len() as isize;
    let index = if let Some(v) = args.first() {
        interp.coerce_to_number(v)? as isize
    } else {
        0
    };

    // Handle negative indices
    let actual_index = if index < 0 { len + index } else { index };

    if actual_index < 0 || actual_index >= len {
        return Ok(Guarded::unguarded(JsValue::Undefined));
    }

    let char_at = s.as_str().chars().nth(actual_index as usize);
    match char_at {
        Some(c) => Ok(Guarded::unguarded(JsValue::String(JsString::from(
            c.to_string(),
        )))),
        None => Ok(Guarded::unguarded(JsValue::Undefined)),
    }
}

pub fn string_includes(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let search = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(""),
    };
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(Guarded::unguarded(JsValue::Boolean(search.is_empty())));
    }

    Ok(Guarded::unguarded(JsValue::Boolean(
        s.as_str()
            .get(from_index..)
            .map(|slice| slice.contains(search.as_str()))
            .unwrap_or(false),
    )))
}

pub fn string_starts_with(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let search = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(""),
    };
    let position = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if position >= s.len() {
        return Ok(Guarded::unguarded(JsValue::Boolean(search.is_empty())));
    }

    Ok(Guarded::unguarded(JsValue::Boolean(
        s.as_str()
            .get(position..)
            .map(|slice| slice.starts_with(search.as_str()))
            .unwrap_or(false),
    )))
}

pub fn string_ends_with(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let search = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(""),
    };
    let end_position = args
        .get(1)
        .map(|v| v.to_number() as usize)
        .unwrap_or(s.len());

    let end = end_position.min(s.len());
    Ok(Guarded::unguarded(JsValue::Boolean(
        s.as_str()
            .get(..end)
            .map(|slice| slice.ends_with(search.as_str()))
            .unwrap_or(false),
    )))
}

pub fn string_slice(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let len = s.len() as i64;

    let start_arg = args.first().map(|v| v.to_number() as i64).unwrap_or(0);
    let end_arg = args.get(1).map(|v| v.to_number() as i64).unwrap_or(len);

    let start = if start_arg < 0 {
        (len + start_arg).max(0)
    } else {
        start_arg.min(len)
    } as usize;
    let end = if end_arg < 0 {
        (len + end_arg).max(0)
    } else {
        end_arg.min(len)
    } as usize;

    if start >= end {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(""))));
    }

    // Need to handle UTF-8 properly - slice by characters, not bytes
    let chars: Vec<char> = s.as_str().chars().collect();
    let start_clamped = start.min(chars.len());
    let end_clamped = end.min(chars.len());
    let result: String = chars
        .get(start_clamped..end_clamped)
        .map(|slice| slice.iter().collect())
        .unwrap_or_default();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

pub fn string_substring(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let len = s.len();

    let start = args
        .first()
        .map(|v| {
            let n = v.to_number();
            if n.is_nan() { 0 } else { (n as usize).min(len) }
        })
        .unwrap_or(0);

    let end = args
        .get(1)
        .map(|v| {
            let n = v.to_number();
            if n.is_nan() { 0 } else { (n as usize).min(len) }
        })
        .unwrap_or(len);

    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };

    let chars: Vec<char> = s.as_str().chars().collect();
    let start_clamped = start.min(chars.len());
    let end_clamped = end.min(chars.len());
    let result: String = chars
        .get(start_clamped..end_clamped)
        .map(|slice| slice.iter().collect())
        .unwrap_or_default();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

/// String.prototype.substr(start, length?) - deprecated but still supported
pub fn string_substr(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let chars: Vec<char> = s.as_str().chars().collect();
    let len = chars.len() as i64;

    // Get start index
    let start_arg = args.first().map(|v| v.to_number()).unwrap_or(0.0);
    let mut start = if start_arg.is_nan() {
        0
    } else {
        start_arg as i64
    };

    // Negative start counts from end
    if start < 0 {
        start = (len + start).max(0);
    }

    // If start is beyond string length, return empty string
    if start >= len {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(""))));
    }

    // Get length (default: rest of string)
    let length = args
        .get(1)
        .map(|v| {
            let n = v.to_number();
            if n.is_nan() || n < 0.0 { 0 } else { n as usize }
        })
        .unwrap_or((len - start) as usize);

    let start_idx = start as usize;
    let end_idx = (start_idx + length).min(chars.len());

    let result: String = chars
        .get(start_idx..end_idx)
        .map(|slice| slice.iter().collect())
        .unwrap_or_default();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

pub fn string_to_lower_case(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().to_lowercase(),
    ))))
}

pub fn string_to_upper_case(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().to_uppercase(),
    ))))
}

pub fn string_trim(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().trim(),
    ))))
}

pub fn string_trim_start(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().trim_start(),
    ))))
}

pub fn string_trim_end(
    interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().trim_end(),
    ))))
}

pub fn string_split(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use crate::value::ExoticObject;

    let s = interp.to_js_string(&this);
    let separator_arg = args.first().cloned();
    let limit = args.get(1).map(|v| v.to_number() as usize);

    let parts: Vec<JsValue> = match separator_arg {
        // Per ECMAScript spec: if separator is undefined, return array containing original string
        Some(JsValue::Undefined) | None => {
            vec![JsValue::String(JsString::from(s.to_string()))]
        }
        Some(sep) => {
            // Check if separator is a RegExp
            if let JsValue::Object(ref obj) = sep {
                let obj_ref = obj.borrow();
                if let ExoticObject::RegExp {
                    ref pattern,
                    ref flags,
                    ..
                } = obj_ref.exotic
                {
                    let pattern = pattern.clone();
                    let flags = flags.clone();
                    drop(obj_ref);

                    #[cfg(any(feature = "regex", feature = "wasm"))]
                    {
                        let re = interp.compile_regexp(&pattern, &flags)?;
                        let split_result = re.split(s.as_str()).map_err(JsError::type_error)?;
                        let split: Vec<JsValue> = split_result
                            .into_iter()
                            .map(|p| JsValue::String(JsString::from(p)))
                            .collect();
                        return match limit {
                            Some(l) => {
                                let limited: Vec<JsValue> = split.into_iter().take(l).collect();
                                let guard = interp.heap.create_guard();
                                let arr = interp.create_array_from(&guard, limited);
                                Ok(Guarded::with_guard(JsValue::Object(arr), guard))
                            }
                            None => {
                                let guard = interp.heap.create_guard();
                                let arr = interp.create_array_from(&guard, split);
                                Ok(Guarded::with_guard(JsValue::Object(arr), guard))
                            }
                        };
                    }
                    #[cfg(not(any(feature = "regex", feature = "wasm")))]
                    {
                        let _ = (pattern, flags);
                        return Err(JsError::type_error(
                            "RegExp not available (enable 'regex' or 'wasm' feature)",
                        ));
                    }
                }
            }

            // String separator
            let sep_str = interp.to_js_string(&sep);
            if sep_str.is_empty() {
                // Empty separator - split into characters
                let chars: Vec<JsValue> = s
                    .as_str()
                    .chars()
                    .map(|c| JsValue::String(JsString::from(c.to_string())))
                    .collect();
                match limit {
                    Some(l) => chars.into_iter().take(l).collect(),
                    None => chars,
                }
            } else {
                let split: Vec<&str> = s.as_str().split(sep_str.as_str()).collect();
                let result: Vec<JsValue> = split
                    .into_iter()
                    .map(|p| JsValue::String(JsString::from(p)))
                    .collect();
                // Apply limit after collecting all parts
                match limit {
                    Some(l) => result.into_iter().take(l).collect(),
                    None => result,
                }
            }
        }
    };

    let guard = interp.heap.create_guard();
    let arr = interp.create_array_from(&guard, parts);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn string_repeat(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let count = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().repeat(count),
    ))))
}

pub fn string_replace(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use crate::value::ExoticObject;

    let s = interp.to_js_string(&this).to_string();
    let search_arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let replacement_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Check if replacement is a function
    let is_replacement_function = if let JsValue::Object(ref obj) = replacement_arg {
        let obj_ref = obj.borrow();
        obj_ref.is_callable()
    } else {
        false
    };

    // Check if search is a RegExp
    if let JsValue::Object(ref obj) = search_arg {
        let obj_ref = obj.borrow();
        if let ExoticObject::RegExp {
            ref pattern,
            ref flags,
            ..
        } = obj_ref.exotic
        {
            let pattern = pattern.clone();
            let flags = flags.clone();
            drop(obj_ref);

            #[cfg(any(feature = "regex", feature = "wasm"))]
            {
                let re = interp.compile_regexp(&pattern, &flags)?;
                let is_global = flags.contains('g');

                if is_replacement_function {
                    // Function replacement - iterate over matches
                    let mut result = String::new();
                    let mut last_end = 0;

                    let matches = if is_global {
                        re.find_iter(&s).map_err(JsError::type_error)?
                    } else {
                        // Single match
                        match re.find(&s, 0).map_err(JsError::type_error)? {
                            Some(m) => vec![m],
                            None => vec![],
                        }
                    };

                    for m in matches {
                        result.push_str(s.get(last_end..m.start).unwrap_or(""));

                        // Build args: match, capture groups..., index, input
                        let matched_str = s.get(m.start..m.end).unwrap_or("");
                        let mut call_args = vec![JsValue::String(JsString::from(matched_str))];
                        // Add capture groups (skip index 0 which is the full match)
                        for cap in m.captures.iter().skip(1) {
                            match cap {
                                Some((start, end)) => {
                                    let cap_str = s.get(*start..*end).unwrap_or("");
                                    call_args.push(JsValue::String(JsString::from(cap_str)));
                                }
                                None => call_args.push(JsValue::Undefined),
                            }
                        }
                        call_args.push(JsValue::Number(m.start as f64));
                        call_args.push(JsValue::String(JsString::from(s.clone())));

                        let replace_result = interp.call_function(
                            replacement_arg.clone(),
                            JsValue::Undefined,
                            &call_args,
                        )?;
                        result.push_str(interp.to_js_string(&replace_result.value).as_ref());

                        last_end = m.end;
                    }
                    result.push_str(s.get(last_end..).unwrap_or(""));
                    return Ok(Guarded::unguarded(JsValue::String(JsString::from(result))));
                } else {
                    // String replacement - use CompiledRegex replace methods
                    let replacement_template = interp.to_js_string(&replacement_arg).to_string();
                    let result = if is_global {
                        re.replace_all(&s, &replacement_template)
                            .map_err(JsError::type_error)?
                    } else {
                        re.replace(&s, &replacement_template)
                            .map_err(JsError::type_error)?
                    };
                    return Ok(Guarded::unguarded(JsValue::String(JsString::from(result))));
                }
            }
            #[cfg(not(any(feature = "regex", feature = "wasm")))]
            {
                let _ = (pattern, flags);
                return Err(JsError::type_error(
                    "RegExp not available (enable 'regex' or 'wasm' feature)",
                ));
            }
        }
    }

    // String search - only replace first occurrence
    // Use coerce_to_string to properly call toString() on objects
    let search = interp.coerce_to_string(&search_arg)?.to_string();

    if is_replacement_function {
        // Function replacement for string search
        if let Some(start) = s.find(&search) {
            let call_args = vec![
                JsValue::String(JsString::from(search.clone())),
                JsValue::Number(start as f64),
                JsValue::String(JsString::from(s.clone())),
            ];

            let replace_result =
                interp.call_function(replacement_arg, JsValue::Undefined, &call_args)?;
            let replacement = interp.to_js_string(&replace_result.value).to_string();

            let mut result = String::new();
            result.push_str(s.get(..start).unwrap_or(""));
            result.push_str(&replacement);
            result.push_str(s.get(start + search.len()..).unwrap_or(""));
            return Ok(Guarded::unguarded(JsValue::String(JsString::from(result))));
        }
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(s))));
    }

    // String replacement with $ pattern expansion
    let replacement_template = interp.to_js_string(&replacement_arg).to_string();
    if let Some(start) = s.find(&search) {
        let before = s.get(..start).unwrap_or("");
        let after = s.get(start + search.len()..).unwrap_or("");
        let expanded = expand_replacement_pattern(
            &replacement_template,
            &search,
            before,
            after,
            None, // No capture groups for string search
        );
        let mut result = String::new();
        result.push_str(before);
        result.push_str(&expanded);
        result.push_str(after);
        Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
    } else {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(s))))
    }
}

pub fn string_replace_all(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let search = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(""),
    };
    let replacement = match args.get(1) {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(""),
    };

    // Replace all occurrences
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().replace(search.as_str(), replacement.as_str()),
    ))))
}

pub fn string_pad_start(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = match args.get(1) {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(" "),
    };

    let current_len = s.as_str().chars().count();
    if current_len >= target_length || pad_string.is_empty() {
        return Ok(Guarded::unguarded(JsValue::String(s)));
    }

    let pad_len = target_length - current_len;
    let mut padding = String::new();
    while padding.len() < pad_len {
        padding.push_str(pad_string.as_str());
    }
    padding.truncate(pad_len);

    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        format!("{}{}", padding, s.as_str()),
    ))))
}

pub fn string_pad_end(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = match args.get(1) {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(" "),
    };

    let current_len = s.as_str().chars().count();
    if current_len >= target_length || pad_string.is_empty() {
        return Ok(Guarded::unguarded(JsValue::String(s)));
    }

    let pad_len = target_length - current_len;
    let mut padding = String::new();
    while padding.len() < pad_len {
        padding.push_str(pad_string.as_str());
    }
    padding.truncate(pad_len);

    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        format!("{}{}", s.as_str(), padding),
    ))))
}

pub fn string_concat(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let mut result = interp.to_js_string(&this).to_string();
    for arg in args {
        result.push_str(interp.to_js_string(arg).as_ref());
    }
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

pub fn string_char_code_at(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let index = if let Some(v) = args.first() {
        interp.coerce_to_number(v)? as usize
    } else {
        0
    };

    if let Some(ch) = s.as_str().chars().nth(index) {
        Ok(Guarded::unguarded(JsValue::Number(ch as u32 as f64)))
    } else {
        Ok(Guarded::unguarded(JsValue::Number(f64::NAN)))
    }
}

/// ToUint16 abstract operation per ECMAScript spec
/// Converts a number to a 16-bit unsigned integer (0-65535)
fn to_uint16(n: f64) -> u16 {
    // Step 1: Already have the number
    // Step 2: If NaN, +0, -0, +∞, -∞, return 0
    if n.is_nan() || n == 0.0 || n.is_infinite() {
        return 0;
    }
    // Step 3: posInt = sign(n) * floor(abs(n))
    let pos_int = n.signum() * math::floor(n.abs());
    // Step 4: int16bit = posInt modulo 2^16
    // Rust's rem_euclid gives the correct mathematical modulo (always positive)
    let int16bit = math::rem_euclid(pos_int, 65536.0);
    // Step 5: Return int16bit
    int16bit as u16
}

pub fn string_from_char_code(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let chars: String = args
        .iter()
        .map(|v| {
            // Apply ToUint16 conversion as per ECMAScript spec
            let code = to_uint16(v.to_number());
            // u16 is always a valid Unicode code unit (char is UTF-32)
            char::from_u32(code as u32).unwrap_or('\u{FFFD}')
        })
        .collect();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(chars))))
}

/// String.fromCodePoint(...codePoints)
/// Creates a string from Unicode code points (supports full range including supplementary characters)
pub fn string_from_code_point(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let mut result = String::new();
    for arg in args {
        let code_point = arg.to_number();

        // Check if code point is valid
        if code_point.is_nan()
            || code_point < 0.0
            || code_point > 0x10FFFF as f64
            || math::fract(code_point) != 0.0
        {
            return Err(JsError::range_error(format!(
                "Invalid code point {}",
                code_point
            )));
        }

        let code_point = code_point as u32;
        match char::from_u32(code_point) {
            Some(c) => result.push(c),
            None => {
                return Err(JsError::range_error(format!(
                    "Invalid code point {}",
                    code_point
                )));
            }
        }
    }
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

/// String.prototype.codePointAt(index)
/// Returns the Unicode code point value at the given index
pub fn string_code_point_at(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let index = args.first().map(|v| v.to_number()).unwrap_or(0.0);

    // Check for negative or non-integer index
    if index < 0.0 || math::fract(index) != 0.0 {
        return Ok(Guarded::unguarded(JsValue::Undefined));
    }

    let index = index as usize;
    let chars: Vec<char> = s.as_str().chars().collect();

    match chars.get(index) {
        Some(&ch) => Ok(Guarded::unguarded(JsValue::Number(ch as u32 as f64))),
        None => Ok(Guarded::unguarded(JsValue::Undefined)),
    }
}

/// String.prototype.match(regexp)
/// Returns an array of matches or null if no match
#[cfg(not(any(feature = "regex", feature = "wasm")))]
pub fn string_match(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    Err(JsError::type_error(
        "String.prototype.match requires 'regex' or 'wasm' feature",
    ))
}

/// String.prototype.match(regexp)
/// Returns an array of matches or null if no match
#[cfg(any(feature = "regex", feature = "wasm"))]
pub fn string_match(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use crate::value::ExoticObject;

    let s = interp.to_js_string(&this).to_string();
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Check if argument is a RegExp
    let (pattern, flags) = if let JsValue::Object(ref obj) = arg {
        let obj_ref = obj.borrow();
        if let ExoticObject::RegExp {
            ref pattern,
            ref flags,
            ..
        } = obj_ref.exotic
        {
            (pattern.clone(), flags.clone())
        } else {
            // Convert to string and use as pattern
            drop(obj_ref);
            (interp.to_js_string(&arg).to_string(), String::new())
        }
    } else {
        // Convert to string and use as pattern
        (interp.to_js_string(&arg).to_string(), String::new())
    };

    let re = interp.compile_regexp(&pattern, &flags)?;
    let is_global = flags.contains('g');

    if is_global {
        // Global flag: return array of all matches
        let matches_result = re.find_iter(&s).map_err(JsError::type_error)?;
        let matches: Vec<JsValue> = matches_result
            .iter()
            .map(|m| {
                let matched_str = s.get(m.start..m.end).unwrap_or("");
                JsValue::String(JsString::from(matched_str))
            })
            .collect();

        if matches.is_empty() {
            Ok(Guarded::unguarded(JsValue::Null))
        } else {
            let guard = interp.heap.create_guard();
            let arr = interp.create_array_from(&guard, matches);
            Ok(Guarded::with_guard(JsValue::Object(arr), guard))
        }
    } else {
        // Non-global: return first match with capture groups
        let find_result = re.find(&s, 0).map_err(JsError::type_error)?;
        match find_result {
            Some(m) => {
                let mut result = Vec::new();
                // Add full match
                let full_match = s.get(m.start..m.end).unwrap_or("");
                result.push(JsValue::String(JsString::from(full_match)));
                // Add capture groups
                for cap in m.captures.iter().skip(1) {
                    match cap {
                        Some((start, end)) => {
                            let cap_str = s.get(*start..*end).unwrap_or("");
                            result.push(JsValue::String(JsString::from(cap_str)));
                        }
                        None => result.push(JsValue::Undefined),
                    }
                }
                let guard = interp.heap.create_guard();
                let arr = interp.create_array_from(&guard, result);

                // Add index property
                let index_key = PropertyKey::String(interp.intern("index"));
                arr.borrow_mut()
                    .set_property(index_key, JsValue::Number(m.start as f64));

                // Add input property
                let input_key = PropertyKey::String(interp.intern("input"));
                arr.borrow_mut()
                    .set_property(input_key, JsValue::String(JsString::from(s)));

                Ok(Guarded::with_guard(JsValue::Object(arr), guard))
            }
            None => Ok(Guarded::unguarded(JsValue::Null)),
        }
    }
}

/// String.prototype.matchAll(regexp)
/// Returns an iterator of all matches (we return an array for simplicity)
#[cfg(not(feature = "regex"))]
pub fn string_match_all(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    Err(JsError::type_error(
        "String.prototype.matchAll requires 'regex' feature",
    ))
}

/// String.prototype.matchAll(regexp)
/// Returns an iterator of all matches (we return an array for simplicity)
#[cfg(feature = "regex")]
pub fn string_match_all(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use super::regexp::build_regex;
    use crate::value::ExoticObject;

    let s = interp.to_js_string(&this).to_string();
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Check if argument is a RegExp
    let (pattern, flags) = if let JsValue::Object(ref obj) = arg {
        let obj_ref = obj.borrow();
        if let ExoticObject::RegExp {
            ref pattern,
            ref flags,
            ..
        } = obj_ref.exotic
        {
            // matchAll requires global flag
            if !flags.contains('g') {
                drop(obj_ref);
                return Err(JsError::type_error(
                    "String.prototype.matchAll called with a non-global RegExp argument",
                ));
            }
            (pattern.clone(), flags.clone())
        } else {
            drop(obj_ref);
            // Convert to string, treat as global search
            (interp.to_js_string(&arg).to_string(), "g".to_string())
        }
    } else {
        // Convert to string, treat as global search
        (interp.to_js_string(&arg).to_string(), "g".to_string())
    };

    let re = build_regex(&pattern, &flags)?;

    // Collect all matches with capture groups
    // Use single guard for all match arrays
    let guard = interp.heap.create_guard();
    let mut all_matches = Vec::new();
    for caps in re.captures_iter(&s).filter_map(|r| r.ok()) {
        let mut match_result = Vec::new();
        for cap in caps.iter() {
            match cap {
                Some(m) => match_result.push(JsValue::String(JsString::from(m.as_str()))),
                None => match_result.push(JsValue::Undefined),
            }
        }
        let arr = interp.create_array_from(&guard, match_result);

        // Add index property
        let index_key = PropertyKey::String(interp.intern("index"));
        let match_start = caps.get(0).map(|m| m.start()).unwrap_or(0);
        arr.borrow_mut()
            .set_property(index_key, JsValue::Number(match_start as f64));

        // Add input property
        let input_key = PropertyKey::String(interp.intern("input"));
        arr.borrow_mut()
            .set_property(input_key, JsValue::String(JsString::from(s.clone())));

        all_matches.push(JsValue::Object(arr));
    }

    let result_arr = interp.create_array_from(&guard, all_matches);
    Ok(Guarded::with_guard(JsValue::Object(result_arr), guard))
}

/// String.prototype.search(regexp)
/// Returns the index of the first match, or -1 if not found
#[cfg(not(any(feature = "regex", feature = "wasm")))]
pub fn string_search(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    Err(JsError::type_error(
        "String.prototype.search requires 'regex' or 'wasm' feature",
    ))
}

/// String.prototype.search(regexp)
/// Returns the index of the first match, or -1 if not found
#[cfg(any(feature = "regex", feature = "wasm"))]
pub fn string_search(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use crate::value::ExoticObject;

    let s = interp.to_js_string(&this).to_string();
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Check if argument is a RegExp
    let (pattern, flags) = if let JsValue::Object(ref obj) = arg {
        let obj_ref = obj.borrow();
        if let ExoticObject::RegExp {
            ref pattern,
            ref flags,
            ..
        } = obj_ref.exotic
        {
            (pattern.clone(), flags.clone())
        } else {
            drop(obj_ref);
            (interp.to_js_string(&arg).to_string(), String::new())
        }
    } else {
        (interp.to_js_string(&arg).to_string(), String::new())
    };

    let re = interp.compile_regexp(&pattern, &flags)?;

    match re.find(&s, 0).map_err(JsError::type_error)? {
        Some(m) => Ok(Guarded::unguarded(JsValue::Number(m.start as f64))),
        None => Ok(Guarded::unguarded(JsValue::Number(-1.0))),
    }
}

/// String.prototype.normalize(form?)
/// Returns the Unicode Normalization Form of the string
/// Forms: "NFC" (default), "NFD", "NFKC", "NFKD"
/// Note: This is a no-op implementation that returns the string unchanged.
/// Most strings are already NFC-normalized or ASCII.
pub fn string_normalize(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);

    let form = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern("NFC"),
    };

    // Validate the form argument
    match form.as_str() {
        "NFC" | "NFD" | "NFKC" | "NFKD" => {}
        _ => {
            return Err(JsError::range_error(format!(
                "The normalization form should be one of NFC, NFD, NFKC, NFKD. Received: {}",
                form.as_str()
            )));
        }
    };

    // Return string unchanged - most strings are already normalized
    Ok(Guarded::unguarded(JsValue::String(s)))
}

/// Expand JavaScript replacement string $ patterns (no captures version)
/// $$ -> $
/// $& -> matched substring
/// $` -> portion before match
/// $' -> portion after match
#[cfg(not(feature = "regex"))]
fn expand_replacement_pattern(
    replacement: &str,
    matched: &str,
    before_match: &str,
    after_match: &str,
    _captures: Option<()>, // Placeholder for API compatibility
) -> String {
    let mut result = String::with_capacity(replacement.len());
    let chars: Vec<char> = replacement.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars.get(i) == Some(&'$') && i + 1 < chars.len() {
            match chars.get(i + 1) {
                Some('$') => {
                    result.push('$');
                    i += 2;
                }
                Some('&') => {
                    result.push_str(matched);
                    i += 2;
                }
                Some('`') => {
                    result.push_str(before_match);
                    i += 2;
                }
                Some('\'') => {
                    result.push_str(after_match);
                    i += 2;
                }
                Some(c) if c.is_ascii_digit() => {
                    // Without regex, capture groups aren't available - treat as literal
                    result.push('$');
                    i += 1;
                }
                _ => {
                    result.push('$');
                    i += 1;
                }
            }
        } else {
            if let Some(&c) = chars.get(i) {
                result.push(c);
            }
            i += 1;
        }
    }

    result
}

/// Expand JavaScript replacement string $ patterns
/// $$ -> $
/// $& -> matched substring
/// $` -> portion before match
/// $' -> portion after match
/// $n or $nn -> nth capture group (1-99)
#[cfg(feature = "regex")]
fn expand_replacement_pattern(
    replacement: &str,
    matched: &str,
    before_match: &str,
    after_match: &str,
    captures: Option<&fancy_regex::Captures>,
) -> String {
    let mut result = String::with_capacity(replacement.len());
    let chars: Vec<char> = replacement.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars.get(i) == Some(&'$') && i + 1 < chars.len() {
            match chars.get(i + 1) {
                Some('$') => {
                    // $$ -> literal $
                    result.push('$');
                    i += 2;
                }
                Some('&') => {
                    // $& -> matched substring
                    result.push_str(matched);
                    i += 2;
                }
                Some('`') => {
                    // $` -> portion before match
                    result.push_str(before_match);
                    i += 2;
                }
                Some('\'') => {
                    // $' -> portion after match
                    result.push_str(after_match);
                    i += 2;
                }
                Some(c) if c.is_ascii_digit() => {
                    // $n or $nn -> capture group
                    let first_digit = *c;
                    let mut group_num = (first_digit as u32 - '0' as u32) as usize;
                    let mut consumed = 2;

                    // Check for second digit (for $10-$99)
                    if i + 2 < chars.len()
                        && let Some(second) = chars.get(i + 2)
                        && second.is_ascii_digit()
                    {
                        let two_digit = group_num * 10 + (*second as u32 - '0' as u32) as usize;
                        // Only use two-digit if it's a valid group reference
                        if let Some(caps) = captures
                            && two_digit <= caps.len().saturating_sub(1)
                            && two_digit > 0
                        {
                            group_num = two_digit;
                            consumed = 3;
                        }
                    }

                    // Get the capture group (group 0 is the whole match, groups are 1-indexed in JS)
                    if group_num > 0 {
                        if let Some(caps) = captures
                            && let Some(m) = caps.get(group_num)
                        {
                            result.push_str(m.as_str());
                        }
                        // Undefined group -> empty string (nothing to push)
                    } else {
                        // $0 is not valid, treat as literal
                        result.push('$');
                        result.push(first_digit);
                    }
                    i += consumed;
                }
                _ => {
                    // Unknown $ sequence, treat as literal
                    result.push('$');
                    i += 1;
                }
            }
        } else {
            if let Some(&c) = chars.get(i) {
                result.push(c);
            }
            i += 1;
        }
    }

    result
}

/// String.prototype.localeCompare(compareString)
/// Compares two strings in the current locale
/// Returns: -1 if string comes before, 0 if equal, 1 if string comes after
pub fn string_locale_compare(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = interp.to_js_string(&this);
    let compare_string = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern(""),
    };

    // Simple lexicographic comparison (locale-insensitive for now)
    let result = match s.as_str().cmp(compare_string.as_str()) {
        core::cmp::Ordering::Less => -1.0,
        core::cmp::Ordering::Equal => 0.0,
        core::cmp::Ordering::Greater => 1.0,
    };

    Ok(Guarded::unguarded(JsValue::Number(result)))
}
