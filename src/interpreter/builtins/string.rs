//! String built-in methods

use unicode_normalization::UnicodeNormalization;

use super::regexp::build_regex;
use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, register_method, ExoticObject, JsFunction, JsObjectRef,
    JsString, JsValue, NativeFunction, PropertyKey,
};

/// Create String.prototype with all string methods
pub fn create_string_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        // Character access
        register_method(&mut p, "charAt", string_char_at, 1);
        register_method(&mut p, "charCodeAt", string_char_code_at, 1);
        register_method(&mut p, "codePointAt", string_code_point_at, 1);
        register_method(&mut p, "at", string_at, 1);

        // Search methods
        register_method(&mut p, "indexOf", string_index_of, 1);
        register_method(&mut p, "lastIndexOf", string_last_index_of, 1);
        register_method(&mut p, "includes", string_includes, 1);
        register_method(&mut p, "startsWith", string_starts_with, 1);
        register_method(&mut p, "endsWith", string_ends_with, 1);
        register_method(&mut p, "search", string_search, 1);

        // Extraction methods
        register_method(&mut p, "slice", string_slice, 2);
        register_method(&mut p, "substring", string_substring, 2);
        register_method(&mut p, "substr", string_substr, 2);

        // Case conversion
        register_method(&mut p, "toLowerCase", string_to_lower_case, 0);
        register_method(&mut p, "toUpperCase", string_to_upper_case, 0);

        // Whitespace handling
        register_method(&mut p, "trim", string_trim, 0);
        register_method(&mut p, "trimStart", string_trim_start, 0);
        register_method(&mut p, "trimEnd", string_trim_end, 0);

        // Transformation methods
        register_method(&mut p, "split", string_split, 2);
        register_method(&mut p, "repeat", string_repeat, 1);
        register_method(&mut p, "replace", string_replace, 2);
        register_method(&mut p, "replaceAll", string_replace_all, 2);
        register_method(&mut p, "padStart", string_pad_start, 2);
        register_method(&mut p, "padEnd", string_pad_end, 2);
        register_method(&mut p, "concat", string_concat, 1);
        register_method(&mut p, "normalize", string_normalize, 1);

        // RegExp methods
        register_method(&mut p, "match", string_match, 1);
        register_method(&mut p, "matchAll", string_match_all, 1);

        // Comparison
        register_method(&mut p, "localeCompare", string_locale_compare, 1);
    }
    proto
}

/// String constructor function - String(value) converts value to string
pub fn string_constructor_fn(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    // String() with no arguments returns ""
    if args.is_empty() {
        return Ok(JsValue::String(JsString::from("")));
    }
    // String(value) converts value to string
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(JsValue::String(value.to_js_string()))
}

/// Create String constructor with static methods (fromCharCode, fromCodePoint)
pub fn create_string_constructor(string_prototype: &JsObjectRef) -> JsObjectRef {
    let constructor = create_function(JsFunction::Native(NativeFunction {
        name: "String".to_string(),
        func: string_constructor_fn,
        arity: 1,
    }));
    {
        let mut str_obj = constructor.borrow_mut();

        register_method(&mut str_obj, "fromCharCode", string_from_char_code, 1);
        register_method(&mut str_obj, "fromCodePoint", string_from_code_point, 1);

        str_obj.set_property(
            PropertyKey::from("prototype"),
            JsValue::Object(string_prototype.clone()),
        );
    }
    constructor
}

pub fn string_char_at(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number() as usize).unwrap_or(0);

    if let Some(ch) = s.as_str().chars().nth(index) {
        Ok(JsValue::String(JsString::from(ch.to_string())))
    } else {
        Ok(JsValue::String(JsString::from("")))
    }
}

pub fn string_index_of(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(JsValue::Number(-1.0));
    }

    // Use get() for safe slicing - from_index is validated above to be < len
    match s
        .as_str()
        .get(from_index..)
        .and_then(|slice| slice.find(&search))
    {
        Some(pos) => Ok(JsValue::Number((from_index + pos) as f64)),
        None => Ok(JsValue::Number(-1.0)),
    }
}

pub fn string_last_index_of(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
        return Ok(JsValue::Number(from_index.min(len) as f64));
    }

    // Search backwards from from_index
    let search_end = (from_index + search.len()).min(len);
    match s
        .as_str()
        .get(..search_end)
        .and_then(|slice| slice.rfind(&search))
    {
        Some(pos) => Ok(JsValue::Number(pos as f64)),
        None => Ok(JsValue::Number(-1.0)),
    }
}

pub fn string_at(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let len = s.len() as isize;
    let index = args.first().map(|v| v.to_number() as isize).unwrap_or(0);

    // Handle negative indices
    let actual_index = if index < 0 { len + index } else { index };

    if actual_index < 0 || actual_index >= len {
        return Ok(JsValue::Undefined);
    }

    let char_at = s.as_str().chars().nth(actual_index as usize);
    match char_at {
        Some(c) => Ok(JsValue::String(JsString::from(c.to_string()))),
        None => Ok(JsValue::Undefined),
    }
}

pub fn string_includes(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(JsValue::Boolean(search.is_empty()));
    }

    Ok(JsValue::Boolean(
        s.as_str()
            .get(from_index..)
            .map(|slice| slice.contains(&search))
            .unwrap_or(false),
    ))
}

pub fn string_starts_with(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_default();
    let position = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if position >= s.len() {
        return Ok(JsValue::Boolean(search.is_empty()));
    }

    Ok(JsValue::Boolean(
        s.as_str()
            .get(position..)
            .map(|slice| slice.starts_with(&search))
            .unwrap_or(false),
    ))
}

pub fn string_ends_with(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
    Ok(JsValue::Boolean(
        s.as_str()
            .get(..end)
            .map(|slice| slice.ends_with(&search))
            .unwrap_or(false),
    ))
}

pub fn string_slice(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
        return Ok(JsValue::String(JsString::from("")));
    }

    // Need to handle UTF-8 properly - slice by characters, not bytes
    let chars: Vec<char> = s.as_str().chars().collect();
    let start_clamped = start.min(chars.len());
    let end_clamped = end.min(chars.len());
    let result: String = chars
        .get(start_clamped..end_clamped)
        .map(|slice| slice.iter().collect())
        .unwrap_or_default();
    Ok(JsValue::String(JsString::from(result)))
}

pub fn string_substring(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
    Ok(JsValue::String(JsString::from(result)))
}

/// String.prototype.substr(start, length?) - deprecated but still supported
pub fn string_substr(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
        return Ok(JsValue::String(JsString::from("")));
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
    Ok(JsValue::String(JsString::from(result)))
}

pub fn string_to_lower_case(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().to_lowercase())))
}

pub fn string_to_upper_case(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().to_uppercase())))
}

pub fn string_trim(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim())))
}

pub fn string_trim_start(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim_start())))
}

pub fn string_trim_end(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim_end())))
}

pub fn string_split(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    use crate::interpreter::builtins::regexp::{build_regex, get_regexp_data};

    let s = this.to_js_string();
    let separator_arg = args.first().cloned();
    let limit = args.get(1).map(|v| v.to_number() as usize);

    let parts: Vec<JsValue> = match separator_arg {
        Some(sep) => {
            // Check if separator is a RegExp
            if let Ok((pattern, flags)) = get_regexp_data(&sep) {
                // Split using RegExp
                let re = build_regex(&pattern, &flags)?;
                let split: Vec<&str> = re.split(s.as_str()).collect();
                let result: Vec<JsValue> = split
                    .into_iter()
                    .map(|p| JsValue::String(JsString::from(p)))
                    .collect();
                // Apply limit after collecting all parts
                match limit {
                    Some(l) => result.into_iter().take(l).collect(),
                    None => result,
                }
            } else {
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
        }
        None => vec![JsValue::String(JsString::from(s.to_string()))],
    };

    Ok(JsValue::Object(interp.create_array(parts)))
}

pub fn string_repeat(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let count = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(JsValue::String(JsString::from(s.as_str().repeat(count))))
}

pub fn string_replace(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    use crate::interpreter::builtins::regexp::{build_regex, get_regexp_data};

    let s = this.to_js_string();
    let search_arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let replacement_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Check if search is a RegExp
    if let Ok((pattern, flags)) = get_regexp_data(&search_arg) {
        let is_global = flags.contains('g');
        let re = build_regex(&pattern, &flags)?;

        // Check if replacement is a function (callable)
        if replacement_arg.is_callable() {
            // Replace with callback function
            let mut result = String::new();
            let mut last_end = 0;

            // Helper to build callback args: (match, p1, p2, ..., offset, string)
            let build_callback_args = |caps: &regex::Captures, s: &str| -> Vec<JsValue> {
                let mut args = Vec::new();

                // First arg is the full match
                if let Some(m) = caps.get(0) {
                    args.push(JsValue::String(JsString::from(m.as_str())));

                    // Add capture groups (p1, p2, ...)
                    for i in 1..caps.len() {
                        match caps.get(i) {
                            Some(g) => args.push(JsValue::String(JsString::from(g.as_str()))),
                            None => args.push(JsValue::Undefined),
                        }
                    }

                    // offset
                    args.push(JsValue::Number(m.start() as f64));
                    // original string
                    args.push(JsValue::String(JsString::from(s)));
                }

                args
            };

            if is_global {
                // Global: replace all matches
                for caps in re.captures_iter(s.as_str()) {
                    let m = caps
                        .get(0)
                        .ok_or_else(|| JsError::internal_error("regex match failed"))?;

                    // Add text before match
                    result.push_str(s.as_str().get(last_end..m.start()).unwrap_or(""));

                    // Call the replacement function with (match, p1, p2, ..., offset, string)
                    let call_args = build_callback_args(&caps, s.as_str());

                    let replaced = interp.call_function(
                        replacement_arg.clone(),
                        JsValue::Undefined,
                        &call_args,
                    )?;
                    result.push_str(replaced.to_js_string().as_ref());

                    last_end = m.end();
                }
                // Add remaining text
                result.push_str(s.as_str().get(last_end..).unwrap_or(""));
            } else {
                // Non-global: replace first match only
                if let Some(caps) = re.captures(s.as_str()) {
                    let m = caps
                        .get(0)
                        .ok_or_else(|| JsError::internal_error("regex match failed"))?;

                    // Text before match
                    result.push_str(s.as_str().get(..m.start()).unwrap_or(""));

                    // Call callback with (match, p1, p2, ..., offset, string)
                    let call_args = build_callback_args(&caps, s.as_str());

                    let replaced = interp.call_function(
                        replacement_arg.clone(),
                        JsValue::Undefined,
                        &call_args,
                    )?;
                    result.push_str(replaced.to_js_string().as_ref());

                    // Text after match
                    result.push_str(s.as_str().get(m.end()..).unwrap_or(""));
                } else {
                    // No match, return original string
                    return Ok(JsValue::String(s));
                }
            }

            Ok(JsValue::String(JsString::from(result)))
        } else {
            // Replace with string
            let replacement = replacement_arg.to_js_string().to_string();

            if is_global {
                // Global: replace all matches
                Ok(JsValue::String(JsString::from(
                    re.replace_all(s.as_str(), replacement.as_str()).to_string(),
                )))
            } else {
                // Non-global: replace first match only
                Ok(JsValue::String(JsString::from(
                    re.replace(s.as_str(), replacement.as_str()).to_string(),
                )))
            }
        }
    } else {
        // String search - only replace first occurrence
        let search = search_arg.to_js_string().to_string();
        let replacement = replacement_arg.to_js_string().to_string();

        Ok(JsValue::String(JsString::from(s.as_str().replacen(
            &search,
            &replacement,
            1,
        ))))
    }
}

pub fn string_replace_all(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
    Ok(JsValue::String(JsString::from(
        s.as_str().replace(&search, &replacement),
    )))
}

pub fn string_pad_start(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = args
        .get(1)
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| " ".to_string());

    let current_len = s.as_str().chars().count();
    if current_len >= target_length || pad_string.is_empty() {
        return Ok(JsValue::String(s));
    }

    let pad_len = target_length - current_len;
    let mut padding = String::new();
    while padding.len() < pad_len {
        padding.push_str(&pad_string);
    }
    padding.truncate(pad_len);

    Ok(JsValue::String(JsString::from(format!(
        "{}{}",
        padding,
        s.as_str()
    ))))
}

pub fn string_pad_end(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = args
        .get(1)
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| " ".to_string());

    let current_len = s.as_str().chars().count();
    if current_len >= target_length || pad_string.is_empty() {
        return Ok(JsValue::String(s));
    }

    let pad_len = target_length - current_len;
    let mut padding = String::new();
    while padding.len() < pad_len {
        padding.push_str(&pad_string);
    }
    padding.truncate(pad_len);

    Ok(JsValue::String(JsString::from(format!(
        "{}{}",
        s.as_str(),
        padding
    ))))
}

pub fn string_concat(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let mut result = this.to_js_string().to_string();
    for arg in args {
        result.push_str(arg.to_js_string().as_ref());
    }
    Ok(JsValue::String(JsString::from(result)))
}

pub fn string_char_code_at(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number() as usize).unwrap_or(0);

    if let Some(ch) = s.as_str().chars().nth(index) {
        Ok(JsValue::Number(ch as u32 as f64))
    } else {
        Ok(JsValue::Number(f64::NAN))
    }
}

pub fn string_from_char_code(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let chars: String = args
        .iter()
        .map(|v| {
            let code = v.to_number() as u32;
            char::from_u32(code).unwrap_or('\u{FFFD}')
        })
        .collect();
    Ok(JsValue::String(JsString::from(chars)))
}

/// String.fromCodePoint(...codePoints)
/// Creates a string from Unicode code points (supports full range including supplementary characters)
pub fn string_from_code_point(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
    Ok(JsValue::String(JsString::from(result)))
}

/// String.prototype.codePointAt(index)
/// Returns the Unicode code point value at the given index
pub fn string_code_point_at(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number()).unwrap_or(0.0);

    // Check for negative or non-integer index
    if index < 0.0 || index.fract() != 0.0 {
        return Ok(JsValue::Undefined);
    }

    let index = index as usize;
    let chars: Vec<char> = s.as_str().chars().collect();

    match chars.get(index) {
        Some(&ch) => Ok(JsValue::Number(ch as u32 as f64)),
        None => Ok(JsValue::Undefined),
    }
}

/// String.prototype.match(regexp)
/// Returns an array of matches or null if no match
pub fn string_match(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string().to_string();

    let regexp_arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Get pattern and flags from the regexp
    let (pattern, flags) = match &regexp_arg {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let ExoticObject::RegExp {
                ref pattern,
                ref flags,
            } = obj_ref.exotic
            {
                (pattern.clone(), flags.clone())
            } else {
                // Convert to string and use as pattern
                (regexp_arg.to_js_string().to_string(), String::new())
            }
        }
        _ => {
            // Convert to string and use as pattern
            (regexp_arg.to_js_string().to_string(), String::new())
        }
    };

    let re = build_regex(&pattern, &flags)?;

    if flags.contains('g') {
        // Global flag: return array of all matches
        let matches: Vec<JsValue> = re
            .find_iter(&s)
            .map(|m| JsValue::String(JsString::from(m.as_str())))
            .collect();

        if matches.is_empty() {
            Ok(JsValue::Null)
        } else {
            Ok(JsValue::Object(interp.create_array(matches)))
        }
    } else {
        // Non-global: return first match with groups (like exec)
        match re.captures(&s) {
            Some(caps) => {
                let mut result = Vec::new();
                for cap in caps.iter() {
                    match cap {
                        Some(m) => result.push(JsValue::String(JsString::from(m.as_str()))),
                        None => result.push(JsValue::Undefined),
                    }
                }
                let arr = interp.create_array(result);
                // Add index property
                if let Some(m) = caps.get(0) {
                    arr.borrow_mut().set_property(
                        PropertyKey::from("index"),
                        JsValue::Number(m.start() as f64),
                    );
                }
                arr.borrow_mut().set_property(
                    PropertyKey::from("input"),
                    JsValue::String(JsString::from(s)),
                );
                Ok(JsValue::Object(arr))
            }
            None => Ok(JsValue::Null),
        }
    }
}

/// String.prototype.matchAll(regexp)
/// Returns an iterator of all matches (we return an array for simplicity)
pub fn string_match_all(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string().to_string();

    let regexp_arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Get pattern and flags from the regexp
    let (pattern, flags) = match &regexp_arg {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let ExoticObject::RegExp {
                ref pattern,
                ref flags,
            } = obj_ref.exotic
            {
                (pattern.clone(), flags.clone())
            } else {
                return Err(JsError::type_error("matchAll requires a global RegExp"));
            }
        }
        _ => {
            return Err(JsError::type_error("matchAll requires a RegExp argument"));
        }
    };

    // matchAll requires global flag
    if !flags.contains('g') {
        return Err(JsError::type_error(
            "matchAll must be called with a global RegExp",
        ));
    }

    let re = build_regex(&pattern, &flags)?;

    // Collect all matches with their capture groups
    let mut all_matches = Vec::new();
    for caps in re.captures_iter(&s) {
        let mut result = Vec::new();
        for cap in caps.iter() {
            match cap {
                Some(m) => result.push(JsValue::String(JsString::from(m.as_str()))),
                None => result.push(JsValue::Undefined),
            }
        }
        let arr = interp.create_array(result);
        // Add index property
        if let Some(m) = caps.get(0) {
            arr.borrow_mut().set_property(
                PropertyKey::from("index"),
                JsValue::Number(m.start() as f64),
            );
        }
        arr.borrow_mut().set_property(
            PropertyKey::from("input"),
            JsValue::String(JsString::from(s.clone())),
        );
        all_matches.push(JsValue::Object(arr));
    }

    Ok(JsValue::Object(interp.create_array(all_matches)))
}

/// String.prototype.search(regexp)
/// Returns the index of the first match, or -1 if not found
pub fn string_search(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = this.to_js_string().to_string();

    let regexp_arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Get pattern and flags from the regexp
    let (pattern, flags) = match &regexp_arg {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let ExoticObject::RegExp {
                ref pattern,
                ref flags,
            } = obj_ref.exotic
            {
                (pattern.clone(), flags.clone())
            } else {
                // Convert to string and use as pattern
                (regexp_arg.to_js_string().to_string(), String::new())
            }
        }
        _ => {
            // Convert to string and use as pattern
            (regexp_arg.to_js_string().to_string(), String::new())
        }
    };

    let re = build_regex(&pattern, &flags)?;

    match re.find(&s) {
        Some(m) => Ok(JsValue::Number(m.start() as f64)),
        None => Ok(JsValue::Number(-1.0)),
    }
}

/// String.prototype.normalize(form?)
/// Returns the Unicode Normalization Form of the string
/// Forms: "NFC" (default), "NFD", "NFKC", "NFKD"
pub fn string_normalize(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    Ok(JsValue::String(JsString::from(normalized)))
}

/// String.prototype.localeCompare(compareString)
/// Compares two strings in the current locale
/// Returns: -1 if string comes before, 0 if equal, 1 if string comes after
pub fn string_locale_compare(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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

    Ok(JsValue::Number(result))
}
