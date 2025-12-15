//! String built-in methods

use unicode_normalization::UnicodeNormalization;

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsObjectRef, JsString, JsValue};

/// Initialize String.prototype with all string methods.
/// The prototype object must already exist in `interp.string_prototype`.
pub fn init_string_prototype(interp: &mut Interpreter) {
    let proto = interp.string_prototype;

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
}

/// String constructor function - String(value) converts value to string
pub fn string_constructor_fn(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // String() with no arguments returns ""
    if args.is_empty() {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(""))));
    }
    // String(value) converts value to string
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(Guarded::unguarded(JsValue::String(value.to_js_string())))
}

/// Create String constructor with static methods (fromCharCode, fromCodePoint)
pub fn create_string_constructor(interp: &mut Interpreter) -> JsObjectRef {
    let constructor = interp.create_native_function("String", string_constructor_fn, 1);

    interp.register_method(&constructor, "fromCharCode", string_from_char_code, 1);
    interp.register_method(&constructor, "fromCodePoint", string_from_code_point, 1);

    let proto_key = interp.key("prototype");
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.string_prototype));

    constructor
}

pub fn string_char_at(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number() as usize).unwrap_or(0);

    if let Some(ch) = s.as_str().chars().nth(index) {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(
            ch.to_string(),
        ))))
    } else {
        Ok(Guarded::unguarded(JsValue::String(JsString::from(""))))
    }
}

pub fn string_index_of(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(Guarded::unguarded(JsValue::Number(-1.0)));
    }

    // Use get() for safe slicing - from_index is validated above to be < len
    match s
        .as_str()
        .get(from_index..)
        .and_then(|slice| slice.find(&search))
    {
        Some(pos) => Ok(Guarded::unguarded(JsValue::Number(
            (from_index + pos) as f64,
        ))),
        None => Ok(Guarded::unguarded(JsValue::Number(-1.0))),
    }
}

pub fn string_last_index_of(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
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
        .and_then(|slice| slice.rfind(&search))
    {
        Some(pos) => Ok(Guarded::unguarded(JsValue::Number(pos as f64))),
        None => Ok(Guarded::unguarded(JsValue::Number(-1.0))),
    }
}

pub fn string_at(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let len = s.len() as isize;
    let index = args.first().map(|v| v.to_number() as isize).unwrap_or(0);

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
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(Guarded::unguarded(JsValue::Boolean(search.is_empty())));
    }

    Ok(Guarded::unguarded(JsValue::Boolean(
        s.as_str()
            .get(from_index..)
            .map(|slice| slice.contains(&search))
            .unwrap_or(false),
    )))
}

pub fn string_starts_with(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let position = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if position >= s.len() {
        return Ok(Guarded::unguarded(JsValue::Boolean(search.is_empty())));
    }

    Ok(Guarded::unguarded(JsValue::Boolean(
        s.as_str()
            .get(position..)
            .map(|slice| slice.starts_with(&search))
            .unwrap_or(false),
    )))
}

pub fn string_ends_with(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let end_position = args
        .get(1)
        .map(|v| v.to_number() as usize)
        .unwrap_or(s.len());

    let end = end_position.min(s.len());
    Ok(Guarded::unguarded(JsValue::Boolean(
        s.as_str()
            .get(..end)
            .map(|slice| slice.ends_with(&search))
            .unwrap_or(false),
    )))
}

pub fn string_slice(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
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
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let len = s.len();

    let start = args
        .first()
        .map(|v| {
            let n = v.to_number();
            if n.is_nan() {
                0
            } else {
                (n as usize).min(len)
            }
        })
        .unwrap_or(0);

    let end = args
        .get(1)
        .map(|v| {
            let n = v.to_number();
            if n.is_nan() {
                0
            } else {
                (n as usize).min(len)
            }
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
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
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
            if n.is_nan() || n < 0.0 {
                0
            } else {
                n as usize
            }
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
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().to_lowercase(),
    ))))
}

pub fn string_to_upper_case(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().to_uppercase(),
    ))))
}

pub fn string_trim(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().trim(),
    ))))
}

pub fn string_trim_start(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().trim_start(),
    ))))
}

pub fn string_trim_end(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().trim_end(),
    ))))
}

pub fn string_split(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use super::regexp::build_regex;
    use crate::value::ExoticObject;

    let s = this.to_js_string();
    let separator_arg = args.first().cloned();
    let limit = args.get(1).map(|v| v.to_number() as usize);

    let parts: Vec<JsValue> = match separator_arg {
        Some(sep) => {
            // Check if separator is a RegExp
            if let JsValue::Object(ref obj) = sep {
                let obj_ref = obj.borrow();
                if let ExoticObject::RegExp {
                    ref pattern,
                    ref flags,
                } = obj_ref.exotic
                {
                    let pattern = pattern.clone();
                    let flags = flags.clone();
                    drop(obj_ref);

                    let re = build_regex(&pattern, &flags)?;
                    let split: Vec<JsValue> = re
                        .split(s.as_str())
                        .map(|p| JsValue::String(JsString::from(p)))
                        .collect();
                    return match limit {
                        Some(l) => {
                            let limited: Vec<JsValue> = split.into_iter().take(l).collect();
                            let (arr, guard) = interp.create_array(limited);
                            Ok(Guarded::with_guard(JsValue::Object(arr), guard))
                        }
                        None => {
                            let (arr, guard) = interp.create_array(split);
                            Ok(Guarded::with_guard(JsValue::Object(arr), guard))
                        }
                    };
                }
            }

            // String separator
            let sep_str = sep.to_js_string().to_string();
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
                let split: Vec<&str> = s.as_str().split(&sep_str).collect();
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
        None => vec![JsValue::String(JsString::from(s.to_string()))],
    };

    let (arr, guard) = interp.create_array(parts);
    Ok(Guarded::with_guard(JsValue::Object(arr), guard))
}

pub fn string_repeat(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
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
    use super::regexp::build_regex;
    use crate::value::ExoticObject;

    let s = this.to_js_string().to_string();
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
        } = obj_ref.exotic
        {
            let pattern = pattern.clone();
            let flags = flags.clone();
            drop(obj_ref);

            let re = build_regex(&pattern, &flags)?;
            let is_global = flags.contains('g');

            if is_replacement_function {
                // Function replacement
                let mut result = String::new();
                let mut last_end = 0;

                let captures_iter: Vec<_> = if is_global {
                    re.captures_iter(&s).collect()
                } else {
                    re.captures(&s).into_iter().collect()
                };

                for caps in captures_iter {
                    let m = caps.get(0).ok_or_else(|| {
                        JsError::internal_error("regex match missing capture group 0")
                    })?;
                    result.push_str(s.get(last_end..m.start()).unwrap_or(""));

                    // Build args: match, capture groups..., index, input
                    let mut call_args = vec![JsValue::String(JsString::from(m.as_str()))];
                    for i in 1..caps.len() {
                        match caps.get(i) {
                            Some(c) => call_args.push(JsValue::String(JsString::from(c.as_str()))),
                            None => call_args.push(JsValue::Undefined),
                        }
                    }
                    call_args.push(JsValue::Number(m.start() as f64));
                    call_args.push(JsValue::String(JsString::from(s.clone())));

                    let replace_result = interp.call_function(
                        replacement_arg.clone(),
                        JsValue::Undefined,
                        &call_args,
                    )?;
                    result.push_str(&replace_result.value.to_js_string().to_string());

                    last_end = m.end();
                }
                result.push_str(s.get(last_end..).unwrap_or(""));
                return Ok(Guarded::unguarded(JsValue::String(JsString::from(result))));
            } else {
                // String replacement
                let replacement = replacement_arg.to_js_string().to_string();
                let result = if is_global {
                    re.replace_all(&s, replacement.as_str()).into_owned()
                } else {
                    re.replace(&s, replacement.as_str()).into_owned()
                };
                return Ok(Guarded::unguarded(JsValue::String(JsString::from(result))));
            }
        }
    }

    // String search - only replace first occurrence
    let search = search_arg.to_js_string().to_string();

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
            let replacement = replace_result.value.to_js_string().to_string();

            let mut result = String::new();
            result.push_str(s.get(..start).unwrap_or(""));
            result.push_str(&replacement);
            result.push_str(s.get(start + search.len()..).unwrap_or(""));
            return Ok(Guarded::unguarded(JsValue::String(JsString::from(result))));
        }
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(s))));
    }

    let replacement = replacement_arg.to_js_string().to_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.replacen(&search, &replacement, 1),
    ))))
}

pub fn string_replace_all(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let replacement = args
        .get(1)
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();

    // Replace all occurrences
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        s.as_str().replace(&search, &replacement),
    ))))
}

pub fn string_pad_start(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = args
        .get(1)
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| " ".to_string());

    let current_len = s.as_str().chars().count();
    if current_len >= target_length || pad_string.is_empty() {
        return Ok(Guarded::unguarded(JsValue::String(s)));
    }

    let pad_len = target_length - current_len;
    let mut padding = String::new();
    while padding.len() < pad_len {
        padding.push_str(&pad_string);
    }
    padding.truncate(pad_len);

    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        format!("{}{}", padding, s.as_str()),
    ))))
}

pub fn string_pad_end(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = args
        .get(1)
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| " ".to_string());

    let current_len = s.as_str().chars().count();
    if current_len >= target_length || pad_string.is_empty() {
        return Ok(Guarded::unguarded(JsValue::String(s)));
    }

    let pad_len = target_length - current_len;
    let mut padding = String::new();
    while padding.len() < pad_len {
        padding.push_str(&pad_string);
    }
    padding.truncate(pad_len);

    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        format!("{}{}", s.as_str(), padding),
    ))))
}

pub fn string_concat(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let mut result = this.to_js_string().to_string();
    for arg in args {
        result.push_str(arg.to_js_string().as_ref());
    }
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

pub fn string_char_code_at(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number() as usize).unwrap_or(0);

    if let Some(ch) = s.as_str().chars().nth(index) {
        Ok(Guarded::unguarded(JsValue::Number(ch as u32 as f64)))
    } else {
        Ok(Guarded::unguarded(JsValue::Number(f64::NAN)))
    }
}

pub fn string_from_char_code(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let chars: String = args
        .iter()
        .map(|v| {
            let code = v.to_number() as u32;
            char::from_u32(code).unwrap_or('\u{FFFD}')
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
            || code_point.fract() != 0.0
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
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number()).unwrap_or(0.0);

    // Check for negative or non-integer index
    if index < 0.0 || index.fract() != 0.0 {
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
pub fn string_match(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use super::regexp::build_regex;
    use crate::value::ExoticObject;

    let s = this.to_js_string().to_string();
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Check if argument is a RegExp
    let (pattern, flags) = if let JsValue::Object(ref obj) = arg {
        let obj_ref = obj.borrow();
        if let ExoticObject::RegExp {
            ref pattern,
            ref flags,
        } = obj_ref.exotic
        {
            (pattern.clone(), flags.clone())
        } else {
            // Convert to string and use as pattern
            drop(obj_ref);
            (arg.to_js_string().to_string(), String::new())
        }
    } else {
        // Convert to string and use as pattern
        (arg.to_js_string().to_string(), String::new())
    };

    let re = build_regex(&pattern, &flags)?;
    let is_global = flags.contains('g');

    if is_global {
        // Global flag: return array of all matches
        let matches: Vec<JsValue> = re
            .find_iter(&s)
            .map(|m| JsValue::String(JsString::from(m.as_str())))
            .collect();

        if matches.is_empty() {
            Ok(Guarded::unguarded(JsValue::Null))
        } else {
            let (arr, guard) = interp.create_array(matches);
            Ok(Guarded::with_guard(JsValue::Object(arr), guard))
        }
    } else {
        // Non-global: return first match with capture groups
        match re.captures(&s) {
            Some(caps) => {
                let mut result = Vec::new();
                for cap in caps.iter() {
                    match cap {
                        Some(m) => result.push(JsValue::String(JsString::from(m.as_str()))),
                        None => result.push(JsValue::Undefined),
                    }
                }
                let (arr, guard) = interp.create_array(result);

                // Add index property
                let index_key = interp.key("index");
                let match_start = caps.get(0).map(|m| m.start()).unwrap_or(0);
                arr.borrow_mut()
                    .set_property(index_key, JsValue::Number(match_start as f64));

                // Add input property
                let input_key = interp.key("input");
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
pub fn string_match_all(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use super::regexp::build_regex;
    use crate::value::ExoticObject;

    let s = this.to_js_string().to_string();
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Check if argument is a RegExp
    let (pattern, flags) = if let JsValue::Object(ref obj) = arg {
        let obj_ref = obj.borrow();
        if let ExoticObject::RegExp {
            ref pattern,
            ref flags,
        } = obj_ref.exotic
        {
            // matchAll requires global flag
            if !flags.contains('g') {
                return Err(JsError::type_error(
                    "String.prototype.matchAll called with a non-global RegExp argument",
                ));
            }
            (pattern.clone(), flags.clone())
        } else {
            drop(obj_ref);
            // Convert to string, treat as global search
            (arg.to_js_string().to_string(), "g".to_string())
        }
    } else {
        // Convert to string, treat as global search
        (arg.to_js_string().to_string(), "g".to_string())
    };

    let re = build_regex(&pattern, &flags)?;

    // Collect all matches with capture groups
    // Keep guards alive until we create the outer array
    let mut all_matches = Vec::new();
    let mut _inner_guards = Vec::new();
    for caps in re.captures_iter(&s) {
        let mut match_result = Vec::new();
        for cap in caps.iter() {
            match cap {
                Some(m) => match_result.push(JsValue::String(JsString::from(m.as_str()))),
                None => match_result.push(JsValue::Undefined),
            }
        }
        let (arr, inner_guard) = interp.create_array(match_result);

        // Add index property
        let index_key = interp.key("index");
        let match_start = caps.get(0).map(|m| m.start()).unwrap_or(0);
        arr.borrow_mut()
            .set_property(index_key, JsValue::Number(match_start as f64));

        // Add input property
        let input_key = interp.key("input");
        arr.borrow_mut()
            .set_property(input_key, JsValue::String(JsString::from(s.clone())));

        all_matches.push(JsValue::Object(arr));
        _inner_guards.push(inner_guard);
    }

    let (result_arr, guard) = interp.create_array(all_matches);
    Ok(Guarded::with_guard(JsValue::Object(result_arr), guard))
}

/// String.prototype.search(regexp)
/// Returns the index of the first match, or -1 if not found
pub fn string_search(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    use super::regexp::build_regex;
    use crate::value::ExoticObject;

    let s = this.to_js_string().to_string();
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Check if argument is a RegExp
    let (pattern, flags) = if let JsValue::Object(ref obj) = arg {
        let obj_ref = obj.borrow();
        if let ExoticObject::RegExp {
            ref pattern,
            ref flags,
        } = obj_ref.exotic
        {
            (pattern.clone(), flags.clone())
        } else {
            drop(obj_ref);
            (arg.to_js_string().to_string(), String::new())
        }
    } else {
        (arg.to_js_string().to_string(), String::new())
    };

    let re = build_regex(&pattern, &flags)?;

    match re.find(&s) {
        Some(m) => Ok(Guarded::unguarded(JsValue::Number(m.start() as f64))),
        None => Ok(Guarded::unguarded(JsValue::Number(-1.0))),
    }
}

/// String.prototype.normalize(form?)
/// Returns the Unicode Normalization Form of the string
/// Forms: "NFC" (default), "NFD", "NFKC", "NFKD"
pub fn string_normalize(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string().to_string();

    let form = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "NFC".to_string());

    let normalized = match form.as_str() {
        "NFC" => s.nfc().collect::<String>(),
        "NFD" => s.nfd().collect::<String>(),
        "NFKC" => s.nfkc().collect::<String>(),
        "NFKD" => s.nfkd().collect::<String>(),
        _ => {
            return Err(JsError::range_error(format!(
                "The normalization form should be one of NFC, NFD, NFKC, NFKD. Received: {}",
                form
            )))
        }
    };

    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        normalized,
    ))))
}

/// String.prototype.localeCompare(compareString)
/// Compares two strings in the current locale
/// Returns: -1 if string comes before, 0 if equal, 1 if string comes after
pub fn string_locale_compare(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = this.to_js_string().to_string();
    let compare_string = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();

    // Simple lexicographic comparison (locale-insensitive for now)
    let result = match s.cmp(&compare_string) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    };

    Ok(Guarded::unguarded(JsValue::Number(result)))
}
