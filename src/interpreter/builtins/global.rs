//! Global built-in functions (parseInt, parseFloat, URI functions, etc.)

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_function, JsFunction, JsString, JsValue, NativeFunction};

/// Register global functions (parseInt, parseFloat, isNaN, isFinite, URI functions) into environment
pub fn register_global_functions(interp: &mut Interpreter) {
    let name = interp.intern("parseInt");
    let parseint_fn = create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: global_parse_int,
            arity: 2,
        }),
    );
    interp.env_arena.define(
        interp.env,
        "parseInt".to_string(),
        JsValue::Object(parseint_fn),
        false,
    );

    let name = interp.intern("parseFloat");
    let parsefloat_fn = create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: global_parse_float,
            arity: 1,
        }),
    );
    interp.env_arena.define(
        interp.env,
        "parseFloat".to_string(),
        JsValue::Object(parsefloat_fn),
        false,
    );

    let name = interp.intern("isNaN");
    let isnan_fn = create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: global_is_nan,
            arity: 1,
        }),
    );
    interp.env_arena.define(
        interp.env,
        "isNaN".to_string(),
        JsValue::Object(isnan_fn),
        false,
    );

    let name = interp.intern("isFinite");
    let isfinite_fn = create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: global_is_finite,
            arity: 1,
        }),
    );
    interp.env_arena.define(
        interp.env,
        "isFinite".to_string(),
        JsValue::Object(isfinite_fn),
        false,
    );

    let name = interp.intern("encodeURI");
    let encodeuri_fn = create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: global_encode_uri,
            arity: 1,
        }),
    );
    interp.env_arena.define(
        interp.env,
        "encodeURI".to_string(),
        JsValue::Object(encodeuri_fn),
        false,
    );

    let name = interp.intern("decodeURI");
    let decodeuri_fn = create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: global_decode_uri,
            arity: 1,
        }),
    );
    interp.env_arena.define(
        interp.env,
        "decodeURI".to_string(),
        JsValue::Object(decodeuri_fn),
        false,
    );

    let name = interp.intern("encodeURIComponent");
    let encodeuricomponent_fn = create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: global_encode_uri_component,
            arity: 1,
        }),
    );
    interp.env_arena.define(
        interp.env,
        "encodeURIComponent".to_string(),
        JsValue::Object(encodeuricomponent_fn),
        false,
    );

    let name = interp.intern("decodeURIComponent");
    let decodeuricomponent_fn = create_function(
        &mut interp.gc_space,
        &mut interp.string_dict,
        JsFunction::Native(NativeFunction {
            name,
            func: global_decode_uri_component,
            arity: 1,
        }),
    );
    interp.env_arena.define(
        interp.env,
        "decodeURIComponent".to_string(),
        JsValue::Object(decodeuricomponent_fn),
        false,
    );
}

pub fn global_parse_int(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let string = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
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
        return Ok(JsValue::Number(f64::NAN));
    }

    let result = if negative { -result } else { result };
    Ok(JsValue::Number(result as f64))
}

pub fn global_parse_float(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let string = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
    let string = string.as_str().to_string();
    let s = string.trim();

    if s.is_empty() {
        return Ok(JsValue::Number(f64::NAN));
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
        Ok(n) => Ok(JsValue::Number(n)),
        Err(_) => Ok(JsValue::Number(f64::NAN)),
    }
}

// Global isNaN - converts argument to number first
pub fn global_is_nan(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Boolean(n.is_nan()))
}

// Global isFinite - converts argument to number first
pub fn global_is_finite(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Boolean(n.is_finite()))
}

// Characters that encodeURI should NOT encode (RFC 3986 + extra URI chars)
const URI_UNESCAPED: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
const URI_RESERVED: &str = ";/?:@&=+$,#";

pub fn global_encode_uri(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
    Ok(JsValue::String(JsString::from(result)))
}

pub fn global_decode_uri(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
    let result = percent_decode(s.as_str(), true);
    Ok(JsValue::String(JsString::from(result)))
}

pub fn global_encode_uri_component(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
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
    Ok(JsValue::String(JsString::from(result)))
}

pub fn global_decode_uri_component(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));
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
