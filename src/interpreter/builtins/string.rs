//! String built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{JsString, JsValue};

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
