//! Global built-in functions (parseInt, parseFloat, URI functions, etc.)

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsString, JsValue};

/// Register global functions (parseInt, parseFloat, isNaN, isFinite, URI functions)
pub fn init_global_functions(interp: &mut Interpreter) {
    // parseInt
    let parseint_fn = interp.create_native_function("parseInt", global_parse_int, 2);
    interp.root_guard.guard(&parseint_fn);
    let key = interp.key("parseInt");
    interp.global.own(&parseint_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(parseint_fn));

    // parseFloat
    let parsefloat_fn = interp.create_native_function("parseFloat", global_parse_float, 1);
    interp.root_guard.guard(&parsefloat_fn);
    let key = interp.key("parseFloat");
    interp.global.own(&parsefloat_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(parsefloat_fn));

    // isNaN
    let isnan_fn = interp.create_native_function("isNaN", global_is_nan, 1);
    interp.root_guard.guard(&isnan_fn);
    let key = interp.key("isNaN");
    interp.global.own(&isnan_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(isnan_fn));

    // isFinite
    let isfinite_fn = interp.create_native_function("isFinite", global_is_finite, 1);
    interp.root_guard.guard(&isfinite_fn);
    let key = interp.key("isFinite");
    interp.global.own(&isfinite_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(isfinite_fn));

    // encodeURI
    let encodeuri_fn = interp.create_native_function("encodeURI", global_encode_uri, 1);
    interp.root_guard.guard(&encodeuri_fn);
    let key = interp.key("encodeURI");
    interp.global.own(&encodeuri_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(encodeuri_fn));

    // decodeURI
    let decodeuri_fn = interp.create_native_function("decodeURI", global_decode_uri, 1);
    interp.root_guard.guard(&decodeuri_fn);
    let key = interp.key("decodeURI");
    interp.global.own(&decodeuri_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(decodeuri_fn));

    // encodeURIComponent
    let encodeuricomponent_fn =
        interp.create_native_function("encodeURIComponent", global_encode_uri_component, 1);
    interp.root_guard.guard(&encodeuricomponent_fn);
    let key = interp.key("encodeURIComponent");
    interp.global.own(&encodeuricomponent_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(encodeuricomponent_fn));

    // decodeURIComponent
    let decodeuricomponent_fn =
        interp.create_native_function("decodeURIComponent", global_decode_uri_component, 1);
    interp.root_guard.guard(&decodeuricomponent_fn);
    let key = interp.key("decodeURIComponent");
    interp.global.own(&decodeuricomponent_fn, &interp.heap);
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(decodeuricomponent_fn));
}

pub fn global_parse_int(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let string = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
    let string = string.as_str().to_string();
    let radix = args.get(1).map(|v| v.to_number() as i32).unwrap_or(10);

    // Trim whitespace
    let s = string.trim();

    if s.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }

    // Handle radix
    let radix = if radix == 0 { 10 } else { radix };
    if !(2..=36).contains(&radix) {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
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
        s.strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(s)
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
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }

    let result = if negative { -result } else { result };
    Ok(Guarded::unguarded(JsValue::Number(result as f64)))
}

pub fn global_parse_float(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let string = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
    let string = string.as_str().to_string();
    let s = string.trim();

    if s.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }

    // Find the longest valid float prefix
    let mut num_str = String::new();
    let mut has_dot = false;
    let mut has_exp = false;
    let mut chars = s.chars().peekable();

    // Handle sign
    if matches!(chars.peek(), Some('-') | Some('+')) {
        if let Some(c) = chars.next() {
            num_str.push(c);
        }
    }

    // Parse digits and decimal point
    while let Some(&c) = chars.peek() {
        match c {
            '0'..='9' => {
                num_str.push(c);
                chars.next();
            }
            '.' if !has_dot && !has_exp => {
                has_dot = true;
                num_str.push(c);
                chars.next();
            }
            'e' | 'E' if !has_exp => {
                has_exp = true;
                num_str.push(c);
                chars.next();
                // Optional sign after exponent
                if matches!(chars.peek(), Some('-') | Some('+')) {
                    if let Some(sign) = chars.next() {
                        num_str.push(sign);
                    }
                }
            }
            _ => break,
        }
    }
    match num_str.parse::<f64>() {
        Ok(n) => Ok(Guarded::unguarded(JsValue::Number(n))),
        Err(_) => Ok(Guarded::unguarded(JsValue::Number(f64::NAN))),
    }
}

// Global isNaN - converts argument to number first
pub fn global_is_nan(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Boolean(n.is_nan())))
}

// Global isFinite - converts argument to number first
pub fn global_is_finite(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Boolean(n.is_finite())))
}

// Characters that encodeURI should NOT encode (RFC 3986 + extra URI chars)
const URI_UNESCAPED: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
const URI_RESERVED: &str = ";/?:@&=+$,#";

pub fn global_encode_uri(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
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
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

pub fn global_decode_uri(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
    let result = percent_decode(s.as_str(), true);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

pub fn global_encode_uri_component(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
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
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

pub fn global_decode_uri_component(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
    let result = percent_decode(s.as_str(), false);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
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
