//! Global built-in functions (parseInt, parseFloat, URI functions, etc.)

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsString, JsValue, PropertyKey};

/// Register global functions (parseInt, parseFloat, isNaN, isFinite, URI functions)
pub fn init_global_functions(interp: &mut Interpreter) {
    // parseInt
    let parseint_fn = interp.create_native_function("parseInt", global_parse_int, 2);
    interp.root_guard.guard(parseint_fn.clone());
    let key = PropertyKey::String(interp.intern("parseInt"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(parseint_fn));

    // parseFloat
    let parsefloat_fn = interp.create_native_function("parseFloat", global_parse_float, 1);
    interp.root_guard.guard(parsefloat_fn.clone());
    let key = PropertyKey::String(interp.intern("parseFloat"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(parsefloat_fn));

    // isNaN
    let isnan_fn = interp.create_native_function("isNaN", global_is_nan, 1);
    interp.root_guard.guard(isnan_fn.clone());
    let key = PropertyKey::String(interp.intern("isNaN"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(isnan_fn));

    // isFinite
    let isfinite_fn = interp.create_native_function("isFinite", global_is_finite, 1);
    interp.root_guard.guard(isfinite_fn.clone());
    let key = PropertyKey::String(interp.intern("isFinite"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(isfinite_fn));

    // encodeURI
    let encodeuri_fn = interp.create_native_function("encodeURI", global_encode_uri, 1);
    interp.root_guard.guard(encodeuri_fn.clone());
    let key = PropertyKey::String(interp.intern("encodeURI"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(encodeuri_fn));

    // decodeURI
    let decodeuri_fn = interp.create_native_function("decodeURI", global_decode_uri, 1);
    interp.root_guard.guard(decodeuri_fn.clone());
    let key = PropertyKey::String(interp.intern("decodeURI"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(decodeuri_fn));

    // encodeURIComponent
    let encodeuricomponent_fn =
        interp.create_native_function("encodeURIComponent", global_encode_uri_component, 1);
    interp.root_guard.guard(encodeuricomponent_fn.clone());
    let key = PropertyKey::String(interp.intern("encodeURIComponent"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(encodeuricomponent_fn));

    // decodeURIComponent
    let decodeuricomponent_fn =
        interp.create_native_function("decodeURIComponent", global_decode_uri_component, 1);
    interp.root_guard.guard(decodeuricomponent_fn.clone());
    let key = PropertyKey::String(interp.intern("decodeURIComponent"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(decodeuricomponent_fn));

    // btoa (base64 encode)
    let btoa_fn = interp.create_native_function("btoa", global_btoa, 1);
    interp.root_guard.guard(btoa_fn.clone());
    let key = PropertyKey::String(interp.intern("btoa"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(btoa_fn));

    // atob (base64 decode)
    let atob_fn = interp.create_native_function("atob", global_atob, 1);
    interp.root_guard.guard(atob_fn.clone());
    let key = PropertyKey::String(interp.intern("atob"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(atob_fn));
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

// Base64 encoding alphabet
const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// btoa - Binary to ASCII (base64 encode)
/// Encodes a string of binary data to a base64 encoded ASCII string
pub fn global_btoa(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));

    // Check that all characters are in the Latin-1 range (0-255)
    for c in s.as_str().chars() {
        if c as u32 > 255 {
            return Err(JsError::type_error(
                "The string to be encoded contains characters outside of the Latin1 range",
            ));
        }
    }

    let bytes: Vec<u8> = s.as_str().chars().map(|c| c as u8).collect();
    let encoded = base64_encode(&bytes);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(encoded))))
}

/// atob - ASCII to Binary (base64 decode)
/// Decodes a base64 encoded string to a string of binary data
pub fn global_atob(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = args
        .first()
        .map(|v| v.to_js_string())
        .unwrap_or_else(|| JsString::from(""));

    // Remove whitespace as per spec
    let cleaned: String = s.as_str().chars().filter(|c| !c.is_whitespace()).collect();

    match base64_decode(&cleaned) {
        Ok(bytes) => {
            // Convert bytes to Latin-1 string
            let decoded: String = bytes.iter().map(|&b| b as char).collect();
            Ok(Guarded::unguarded(JsValue::String(JsString::from(decoded))))
        }
        Err(msg) => Err(JsError::type_error(format!(
            "The string to be decoded is not correctly encoded: {}",
            msg
        ))),
    }
}

fn base64_encode(data: &[u8]) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < data.len() {
        let b0 = data.get(i).copied().unwrap_or(0);
        let b1 = data.get(i + 1).copied();
        let b2 = data.get(i + 2).copied();

        // First character: top 6 bits of b0
        // Index is guaranteed to be 0-63 (6 bits)
        if let Some(&c) = BASE64_ALPHABET.get((b0 >> 2) as usize) {
            result.push(c as char);
        }

        // Second character: bottom 2 bits of b0 + top 4 bits of b1
        let idx1 = ((b0 & 0x03) << 4) | (b1.unwrap_or(0) >> 4);
        if let Some(&c) = BASE64_ALPHABET.get(idx1 as usize) {
            result.push(c as char);
        }

        // Third character: bottom 4 bits of b1 + top 2 bits of b2, or padding
        if b1.is_some() {
            let idx2 = ((b1.unwrap_or(0) & 0x0F) << 2) | (b2.unwrap_or(0) >> 6);
            if let Some(&c) = BASE64_ALPHABET.get(idx2 as usize) {
                result.push(c as char);
            }
        } else {
            result.push('=');
        }

        // Fourth character: bottom 6 bits of b2, or padding
        if b2.is_some() {
            if let Some(&c) = BASE64_ALPHABET.get((b2.unwrap_or(0) & 0x3F) as usize) {
                result.push(c as char);
            }
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

fn base64_decode(data: &str) -> Result<Vec<u8>, &'static str> {
    let mut result = Vec::new();
    let chars: Vec<char> = data.chars().collect();

    if chars.len() % 4 != 0 {
        return Err("invalid length");
    }

    let mut i = 0;
    while i < chars.len() {
        let c0 = chars.get(i).copied().unwrap_or('=');
        let c1 = chars.get(i + 1).copied().unwrap_or('=');
        let c2 = chars.get(i + 2).copied().unwrap_or('=');
        let c3 = chars.get(i + 3).copied().unwrap_or('=');

        let v0 = base64_char_value(c0)?;
        let v1 = base64_char_value(c1)?;

        // First byte
        result.push((v0 << 2) | (v1 >> 4));

        // Second byte (if not padding)
        if c2 != '=' {
            let v2 = base64_char_value(c2)?;
            result.push(((v1 & 0x0F) << 4) | (v2 >> 2));

            // Third byte (if not padding)
            if c3 != '=' {
                let v3 = base64_char_value(c3)?;
                result.push(((v2 & 0x03) << 6) | v3);
            }
        }

        i += 4;
    }

    Ok(result)
}

fn base64_char_value(c: char) -> Result<u8, &'static str> {
    match c {
        'A'..='Z' => Ok(c as u8 - b'A'),
        'a'..='z' => Ok(c as u8 - b'a' + 26),
        '0'..='9' => Ok(c as u8 - b'0' + 52),
        '+' => Ok(62),
        '/' => Ok(63),
        '=' => Ok(0), // Padding, handled separately
        _ => Err("invalid character"),
    }
}
