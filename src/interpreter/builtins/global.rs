//! Global built-in functions (parseInt, parseFloat, URI functions, etc.)

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{JsString, JsValue};

pub fn global_parse_int(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let string = args.first().map(|v| v.to_js_string()).unwrap_or_else(|| JsString::from(""));
    let string = string.as_str().to_string();
    let radix = args.get(1).map(|v| v.to_number() as i32).unwrap_or(10);

    // Trim whitespace
    let s = string.trim();

    if s.is_empty() {
        return Ok(JsValue::Number(f64::NAN));
    }

    // Handle radix
    let radix = if radix == 0 { 10 } else { radix };
    if !(2..=36).contains(&radix) {
        return Ok(JsValue::Number(f64::NAN));
    }

    // Handle sign
    let (negative, s) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = s.strip_prefix('+') {
        (false, rest)
    } else {
        (false, s)
    };

    // Handle hex prefix for radix 16
    let s = if radix == 16 {
        s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s)
    } else {
        s
    };

    // Parse digits until invalid character
    let mut result: i64 = 0;
    let mut found_digit = false;

    for c in s.chars() {
        let digit = match c.to_digit(radix as u32) {
            Some(d) => d as i64,
            None => break,
        };
        found_digit = true;
        result = result * (radix as i64) + digit;
    }

    if !found_digit {
        return Ok(JsValue::Number(f64::NAN));
    }

    let result = if negative { -result } else { result };
    Ok(JsValue::Number(result as f64))
}

pub fn global_parse_float(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let string = args.first().map(|v| v.to_js_string()).unwrap_or_else(|| JsString::from(""));
    let string = string.as_str().to_string();
    let s = string.trim();

    if s.is_empty() {
        return Ok(JsValue::Number(f64::NAN));
    }

    // Find the longest valid float prefix
    let mut end = 0;
    let mut has_dot = false;
    let mut has_exp = false;
    let mut chars = s.chars().peekable();

    // Handle sign
    if matches!(chars.peek(), Some('-') | Some('+')) {
        end += 1;
        chars.next();
    }

    // Parse digits and decimal point
    while let Some(&c) = chars.peek() {
        match c {
            '0'..='9' => {
                end += 1;
                chars.next();
            }
            '.' if !has_dot && !has_exp => {
                has_dot = true;
                end += 1;
                chars.next();
            }
            'e' | 'E' if !has_exp => {
                has_exp = true;
                end += 1;
                chars.next();
                // Optional sign after exponent
                if matches!(chars.peek(), Some('-') | Some('+')) {
                    end += 1;
                    chars.next();
                }
            }
            _ => break,
        }
    }

    let num_str = &s[..end];
    match num_str.parse::<f64>() {
        Ok(n) => Ok(JsValue::Number(n)),
        Err(_) => Ok(JsValue::Number(f64::NAN)),
    }
}

// Global isNaN - converts argument to number first
pub fn global_is_nan(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Boolean(n.is_nan()))
}

// Global isFinite - converts argument to number first
pub fn global_is_finite(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Boolean(n.is_finite()))
}

// Characters that encodeURI should NOT encode (RFC 3986 + extra URI chars)
const URI_UNESCAPED: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
const URI_RESERVED: &str = ";/?:@&=+$,#";

pub fn global_encode_uri(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = args.first().map(|v| v.to_js_string()).unwrap_or_else(|| JsString::from(""));
    let allowed: Vec<char> = URI_UNESCAPED.chars().chain(URI_RESERVED.chars()).collect();
    let mut result = String::new();
    for c in s.as_str().chars() {
        if allowed.contains(&c) {
            result.push(c);
        } else {
            // Percent-encode the character
            for byte in c.to_string().as_bytes() {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    Ok(JsValue::String(JsString::from(result)))
}

pub fn global_decode_uri(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = args.first().map(|v| v.to_js_string()).unwrap_or_else(|| JsString::from(""));
    let result = percent_decode(s.as_str(), true);
    Ok(JsValue::String(JsString::from(result)))
}

pub fn global_encode_uri_component(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = args.first().map(|v| v.to_js_string()).unwrap_or_else(|| JsString::from(""));
    let allowed: Vec<char> = URI_UNESCAPED.chars().collect();
    let mut result = String::new();
    for c in s.as_str().chars() {
        if allowed.contains(&c) {
            result.push(c);
        } else {
            // Percent-encode the character
            for byte in c.to_string().as_bytes() {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    Ok(JsValue::String(JsString::from(result)))
}

pub fn global_decode_uri_component(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = args.first().map(|v| v.to_js_string()).unwrap_or_else(|| JsString::from(""));
    let result = percent_decode(s.as_str(), false);
    Ok(JsValue::String(JsString::from(result)))
}

fn percent_decode(s: &str, preserve_reserved: bool) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            // Try to read two hex digits
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    let decoded = byte as char;
                    // For decodeURI, don't decode reserved characters
                    if preserve_reserved && URI_RESERVED.contains(decoded) {
                        result.push('%');
                        result.push_str(&hex);
                    } else {
                        result.push(decoded);
                    }
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }
    result
}
