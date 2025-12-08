//! String built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_function, create_object, ExoticObject, JsFunction, JsObjectRef, JsString, JsValue, NativeFunction, PropertyKey};
use super::regexp::build_regex;

/// Create String.prototype with all string methods
pub fn create_string_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        let charat_fn = create_function(JsFunction::Native(NativeFunction {
            name: "charAt".to_string(),
            func: string_char_at,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("charAt"), JsValue::Object(charat_fn));

        let indexof_fn = create_function(JsFunction::Native(NativeFunction {
            name: "indexOf".to_string(),
            func: string_index_of,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("indexOf"), JsValue::Object(indexof_fn));

        let lastindexof_fn = create_function(JsFunction::Native(NativeFunction {
            name: "lastIndexOf".to_string(),
            func: string_last_index_of,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("lastIndexOf"), JsValue::Object(lastindexof_fn));

        let at_fn = create_function(JsFunction::Native(NativeFunction {
            name: "at".to_string(),
            func: string_at,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("at"), JsValue::Object(at_fn));

        let includes_fn = create_function(JsFunction::Native(NativeFunction {
            name: "includes".to_string(),
            func: string_includes,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("includes"), JsValue::Object(includes_fn));

        let startswith_fn = create_function(JsFunction::Native(NativeFunction {
            name: "startsWith".to_string(),
            func: string_starts_with,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("startsWith"), JsValue::Object(startswith_fn));

        let endswith_fn = create_function(JsFunction::Native(NativeFunction {
            name: "endsWith".to_string(),
            func: string_ends_with,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("endsWith"), JsValue::Object(endswith_fn));

        let slice_fn = create_function(JsFunction::Native(NativeFunction {
            name: "slice".to_string(),
            func: string_slice,
            arity: 2,
        }));
        p.set_property(PropertyKey::from("slice"), JsValue::Object(slice_fn));

        let substring_fn = create_function(JsFunction::Native(NativeFunction {
            name: "substring".to_string(),
            func: string_substring,
            arity: 2,
        }));
        p.set_property(PropertyKey::from("substring"), JsValue::Object(substring_fn));

        let tolower_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toLowerCase".to_string(),
            func: string_to_lower_case,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("toLowerCase"), JsValue::Object(tolower_fn));

        let toupper_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toUpperCase".to_string(),
            func: string_to_upper_case,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("toUpperCase"), JsValue::Object(toupper_fn));

        let trim_fn = create_function(JsFunction::Native(NativeFunction {
            name: "trim".to_string(),
            func: string_trim,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("trim"), JsValue::Object(trim_fn));

        let trimstart_fn = create_function(JsFunction::Native(NativeFunction {
            name: "trimStart".to_string(),
            func: string_trim_start,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("trimStart"), JsValue::Object(trimstart_fn));

        let trimend_fn = create_function(JsFunction::Native(NativeFunction {
            name: "trimEnd".to_string(),
            func: string_trim_end,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("trimEnd"), JsValue::Object(trimend_fn));

        let split_fn = create_function(JsFunction::Native(NativeFunction {
            name: "split".to_string(),
            func: string_split,
            arity: 2,
        }));
        p.set_property(PropertyKey::from("split"), JsValue::Object(split_fn));

        let repeat_fn = create_function(JsFunction::Native(NativeFunction {
            name: "repeat".to_string(),
            func: string_repeat,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("repeat"), JsValue::Object(repeat_fn));

        let replace_fn = create_function(JsFunction::Native(NativeFunction {
            name: "replace".to_string(),
            func: string_replace,
            arity: 2,
        }));
        p.set_property(PropertyKey::from("replace"), JsValue::Object(replace_fn));

        let replaceall_fn = create_function(JsFunction::Native(NativeFunction {
            name: "replaceAll".to_string(),
            func: string_replace_all,
            arity: 2,
        }));
        p.set_property(PropertyKey::from("replaceAll"), JsValue::Object(replaceall_fn));

        let padstart_fn = create_function(JsFunction::Native(NativeFunction {
            name: "padStart".to_string(),
            func: string_pad_start,
            arity: 2,
        }));
        p.set_property(PropertyKey::from("padStart"), JsValue::Object(padstart_fn));

        let padend_fn = create_function(JsFunction::Native(NativeFunction {
            name: "padEnd".to_string(),
            func: string_pad_end,
            arity: 2,
        }));
        p.set_property(PropertyKey::from("padEnd"), JsValue::Object(padend_fn));

        let concat_fn = create_function(JsFunction::Native(NativeFunction {
            name: "concat".to_string(),
            func: string_concat,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("concat"), JsValue::Object(concat_fn));

        let charcodeat_fn = create_function(JsFunction::Native(NativeFunction {
            name: "charCodeAt".to_string(),
            func: string_char_code_at,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("charCodeAt"), JsValue::Object(charcodeat_fn));

        let codepointat_fn = create_function(JsFunction::Native(NativeFunction {
            name: "codePointAt".to_string(),
            func: string_code_point_at,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("codePointAt"), JsValue::Object(codepointat_fn));

        let match_fn = create_function(JsFunction::Native(NativeFunction {
            name: "match".to_string(),
            func: string_match,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("match"), JsValue::Object(match_fn));

        let matchall_fn = create_function(JsFunction::Native(NativeFunction {
            name: "matchAll".to_string(),
            func: string_match_all,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("matchAll"), JsValue::Object(matchall_fn));

        let search_fn = create_function(JsFunction::Native(NativeFunction {
            name: "search".to_string(),
            func: string_search,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("search"), JsValue::Object(search_fn));
    }
    proto
}

/// Create String constructor with static methods (fromCharCode, fromCodePoint)
pub fn create_string_constructor(string_prototype: &JsObjectRef) -> JsObjectRef {
    let constructor = create_object();
    {
        let mut str_obj = constructor.borrow_mut();

        let fromcharcode_fn = create_function(JsFunction::Native(NativeFunction {
            name: "fromCharCode".to_string(),
            func: string_from_char_code,
            arity: 1,
        }));
        str_obj.set_property(PropertyKey::from("fromCharCode"), JsValue::Object(fromcharcode_fn));

        let fromcodepoint_fn = create_function(JsFunction::Native(NativeFunction {
            name: "fromCodePoint".to_string(),
            func: string_from_code_point,
            arity: 1,
        }));
        str_obj.set_property(PropertyKey::from("fromCodePoint"), JsValue::Object(fromcodepoint_fn));

        str_obj.set_property(PropertyKey::from("prototype"), JsValue::Object(string_prototype.clone()));
    }
    constructor
}

pub fn string_char_at(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number() as usize).unwrap_or(0);

    if let Some(ch) = s.as_str().chars().nth(index) {
        Ok(JsValue::String(JsString::from(ch.to_string())))
    } else {
        Ok(JsValue::String(JsString::from("")))
    }
}

pub fn string_index_of(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(JsValue::Number(-1.0));
    }

    match s.as_str()[from_index..].find(&search) {
        Some(pos) => Ok(JsValue::Number((from_index + pos) as f64)),
        None => Ok(JsValue::Number(-1.0)),
    }
}

pub fn string_last_index_of(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let len = s.len();

    // Default from_index is length of string
    let from_index = if args.len() > 1 {
        let n = args[1].to_number();
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
    match s.as_str()[..search_end].rfind(&search) {
        Some(pos) => Ok(JsValue::Number(pos as f64)),
        None => Ok(JsValue::Number(-1.0)),
    }
}

pub fn string_at(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let len = s.len() as isize;
    let index = args.first().map(|v| v.to_number() as isize).unwrap_or(0);

    // Handle negative indices
    let actual_index = if index < 0 {
        len + index
    } else {
        index
    };

    if actual_index < 0 || actual_index >= len {
        return Ok(JsValue::Undefined);
    }

    let char_at = s.as_str().chars().nth(actual_index as usize);
    match char_at {
        Some(c) => Ok(JsValue::String(JsString::from(c.to_string()))),
        None => Ok(JsValue::Undefined),
    }
}

pub fn string_includes(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(JsValue::Boolean(search.is_empty()));
    }

    Ok(JsValue::Boolean(s.as_str()[from_index..].contains(&search)))
}

pub fn string_starts_with(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let position = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if position >= s.len() {
        return Ok(JsValue::Boolean(search.is_empty()));
    }

    Ok(JsValue::Boolean(s.as_str()[position..].starts_with(&search)))
}

pub fn string_ends_with(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let end_position = args.get(1).map(|v| v.to_number() as usize).unwrap_or(s.len());

    let end = end_position.min(s.len());
    Ok(JsValue::Boolean(s.as_str()[..end].ends_with(&search)))
}

pub fn string_slice(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let len = s.len() as i64;

    let start_arg = args.first().map(|v| v.to_number() as i64).unwrap_or(0);
    let end_arg = args.get(1).map(|v| v.to_number() as i64).unwrap_or(len);

    let start = if start_arg < 0 { (len + start_arg).max(0) } else { start_arg.min(len) } as usize;
    let end = if end_arg < 0 { (len + end_arg).max(0) } else { end_arg.min(len) } as usize;

    if start >= end {
        return Ok(JsValue::String(JsString::from("")));
    }

    // Need to handle UTF-8 properly - slice by characters, not bytes
    let chars: Vec<char> = s.as_str().chars().collect();
    let result: String = chars[start.min(chars.len())..end.min(chars.len())].iter().collect();
    Ok(JsValue::String(JsString::from(result)))
}

pub fn string_substring(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let len = s.len();

    let start = args.first().map(|v| {
        let n = v.to_number();
        if n.is_nan() { 0 } else { (n as usize).min(len) }
    }).unwrap_or(0);

    let end = args.get(1).map(|v| {
        let n = v.to_number();
        if n.is_nan() { 0 } else { (n as usize).min(len) }
    }).unwrap_or(len);

    let (start, end) = if start > end { (end, start) } else { (start, end) };

    let chars: Vec<char> = s.as_str().chars().collect();
    let result: String = chars[start.min(chars.len())..end.min(chars.len())].iter().collect();
    Ok(JsValue::String(JsString::from(result)))
}

pub fn string_to_lower_case(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().to_lowercase())))
}

pub fn string_to_upper_case(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().to_uppercase())))
}

pub fn string_trim(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim())))
}

pub fn string_trim_start(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim_start())))
}

pub fn string_trim_end(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim_end())))
}

pub fn string_split(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let separator = args.first().map(|v| v.to_js_string().to_string());
    let limit = args.get(1).map(|v| v.to_number() as usize);

    let parts: Vec<JsValue> = match separator {
        Some(sep) if !sep.is_empty() => {
            let split: Vec<&str> = match limit {
                Some(l) => s.as_str().splitn(l, &sep).collect(),
                None => s.as_str().split(&sep).collect(),
            };
            split.into_iter().map(|p| JsValue::String(JsString::from(p))).collect()
        }
        Some(_) => {
            // Empty separator - split into characters
            let chars: Vec<JsValue> = s.as_str().chars()
                .map(|c| JsValue::String(JsString::from(c.to_string())))
                .collect();
            match limit {
                Some(l) => chars.into_iter().take(l).collect(),
                None => chars,
            }
        }
        None => vec![JsValue::String(JsString::from(s.to_string()))],
    };

    Ok(JsValue::Object(interp.create_array(parts)))
}

pub fn string_repeat(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let count = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(JsValue::String(JsString::from(s.as_str().repeat(count))))
}

pub fn string_replace(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let replacement = args.get(1).map(|v| v.to_js_string().to_string()).unwrap_or_default();

    // Only replace first occurrence (like JS)
    Ok(JsValue::String(JsString::from(s.as_str().replacen(&search, &replacement, 1))))
}

pub fn string_replace_all(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let replacement = args.get(1).map(|v| v.to_js_string().to_string()).unwrap_or_default();

    // Replace all occurrences
    Ok(JsValue::String(JsString::from(s.as_str().replace(&search, &replacement))))
}

pub fn string_pad_start(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = args.get(1).map(|v| v.to_js_string().to_string()).unwrap_or_else(|| " ".to_string());

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

    Ok(JsValue::String(JsString::from(format!("{}{}", padding, s.as_str()))))
}

pub fn string_pad_end(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = args.get(1).map(|v| v.to_js_string().to_string()).unwrap_or_else(|| " ".to_string());

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

    Ok(JsValue::String(JsString::from(format!("{}{}", s.as_str(), padding))))
}

pub fn string_concat(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let mut result = this.to_js_string().to_string();
    for arg in args {
        result.push_str(&arg.to_js_string().to_string());
    }
    Ok(JsValue::String(JsString::from(result)))
}

pub fn string_char_code_at(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number() as usize).unwrap_or(0);

    if let Some(ch) = s.as_str().chars().nth(index) {
        Ok(JsValue::Number(ch as u32 as f64))
    } else {
        Ok(JsValue::Number(f64::NAN))
    }
}

pub fn string_from_char_code(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
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
pub fn string_from_code_point(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let mut result = String::new();
    for arg in args {
        let code_point = arg.to_number();

        // Check if code point is valid
        if code_point.is_nan() || code_point < 0.0 || code_point > 0x10FFFF as f64 || code_point.fract() != 0.0 {
            return Err(JsError::range_error(&format!(
                "Invalid code point {}",
                code_point
            )));
        }

        let code_point = code_point as u32;
        match char::from_u32(code_point) {
            Some(c) => result.push(c),
            None => {
                return Err(JsError::range_error(&format!(
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
pub fn string_code_point_at(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number()).unwrap_or(0.0);

    // Check for negative or non-integer index
    if index < 0.0 || index.fract() != 0.0 {
        return Ok(JsValue::Undefined);
    }

    let index = index as usize;
    let chars: Vec<char> = s.as_str().chars().collect();

    if index >= chars.len() {
        return Ok(JsValue::Undefined);
    }

    let code_point = chars[index] as u32;
    Ok(JsValue::Number(code_point as f64))
}

/// String.prototype.match(regexp)
/// Returns an array of matches or null if no match
pub fn string_match(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string().to_string();

    let regexp_arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Get pattern and flags from the regexp
    let (pattern, flags) = match &regexp_arg {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let ExoticObject::RegExp { ref pattern, ref flags } = obj_ref.exotic {
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
        let matches: Vec<JsValue> = re.find_iter(&s)
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
                    arr.borrow_mut().set_property(PropertyKey::from("index"), JsValue::Number(m.start() as f64));
                }
                arr.borrow_mut().set_property(PropertyKey::from("input"), JsValue::String(JsString::from(s)));
                Ok(JsValue::Object(arr))
            }
            None => Ok(JsValue::Null),
        }
    }
}

/// String.prototype.matchAll(regexp)
/// Returns an iterator of all matches (we return an array for simplicity)
pub fn string_match_all(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string().to_string();

    let regexp_arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Get pattern and flags from the regexp
    let (pattern, flags) = match &regexp_arg {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let ExoticObject::RegExp { ref pattern, ref flags } = obj_ref.exotic {
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
        return Err(JsError::type_error("matchAll must be called with a global RegExp"));
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
            arr.borrow_mut().set_property(PropertyKey::from("index"), JsValue::Number(m.start() as f64));
        }
        arr.borrow_mut().set_property(PropertyKey::from("input"), JsValue::String(JsString::from(s.clone())));
        all_matches.push(JsValue::Object(arr));
    }

    Ok(JsValue::Object(interp.create_array(all_matches)))
}

/// String.prototype.search(regexp)
/// Returns the index of the first match, or -1 if not found
pub fn string_search(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string().to_string();

    let regexp_arg = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Get pattern and flags from the regexp
    let (pattern, flags) = match &regexp_arg {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            if let ExoticObject::RegExp { ref pattern, ref flags } = obj_ref.exotic {
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
