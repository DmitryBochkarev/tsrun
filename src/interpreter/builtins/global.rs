//! Global built-in functions (parseInt, parseFloat, URI functions, etc.)

use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::interpreter::Interpreter;
use crate::parser::Parser;
use crate::value::{
    CheapClone, ExoticObject, Guarded, JsObject, JsString, JsValue, Property, PropertyKey,
};

/// Register global functions (parseInt, parseFloat, isNaN, isFinite, URI functions)
pub fn init_global_functions(interp: &mut Interpreter) {
    // globalThis - reference to the global object itself
    let global_clone = interp.global.clone();
    interp.root_guard.guard(global_clone.clone());
    let key = PropertyKey::String(interp.intern("globalThis"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(global_clone));

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

    // structuredClone (deep clone)
    let structured_clone_fn =
        interp.create_native_function("structuredClone", global_structured_clone, 1);
    interp.root_guard.guard(structured_clone_fn.clone());
    let key = PropertyKey::String(interp.intern("structuredClone"));
    interp
        .global
        .borrow_mut()
        .set_property(key, JsValue::Object(structured_clone_fn));

    // eval - dynamic code execution
    // Note: This creates the indirect eval function. Direct eval calls are handled
    // specially in the interpreter to preserve the calling scope.
    let eval_fn = create_eval_function(interp);
    interp.root_guard.guard(eval_fn.clone());

    // Set eval on global with proper property descriptor:
    // { writable: true, enumerable: false, configurable: true }
    let key = PropertyKey::String(interp.intern("eval"));
    interp.global.borrow_mut().properties.insert(
        key,
        Property::with_attributes(JsValue::Object(eval_fn), true, false, true),
    );
}

/// Create the global eval function with proper name and length properties.
///
/// This function implements **indirect eval** - it executes code in the global scope.
/// Direct eval (where `eval(...)` is called directly) is handled specially by the
/// interpreter to preserve the calling scope.
fn create_eval_function(interp: &mut Interpreter) -> Gc<JsObject> {
    let func = interp.create_native_function("eval", global_eval, 1);

    // Set name property with correct descriptor:
    // { value: "eval", writable: false, enumerable: false, configurable: true }
    let name_key = PropertyKey::String(interp.intern("name"));
    func.borrow_mut().properties.insert(
        name_key,
        Property::with_attributes(JsValue::String(JsString::from("eval")), false, false, true),
    );

    // Set length property with correct descriptor:
    // { value: 1, writable: false, enumerable: false, configurable: true }
    let length_key = PropertyKey::String(interp.intern("length"));
    func.borrow_mut().properties.insert(
        length_key,
        Property::with_attributes(JsValue::Number(1.0), false, false, true),
    );

    func
}

/// The eval function (implements indirect eval - uses global scope).
///
/// For direct eval calls (`eval(...)`), the interpreter handles this specially
/// to preserve the calling scope. This native function is used when eval is
/// called indirectly (e.g., `(1, eval)(...)` or `var e = eval; e(...)`).
pub fn global_eval(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // If no argument, return undefined
    let arg = match args.first() {
        None => return Ok(Guarded::unguarded(JsValue::Undefined)),
        Some(v) => v,
    };

    // If argument is not a string, return it directly
    let code = match arg {
        JsValue::String(s) => s.to_string(),
        _ => return Ok(Guarded::unguarded(arg.clone())),
    };

    // Execute the code in global scope (indirect eval behavior)
    eval_code_in_scope(interp, &code, true)
}

/// Execute eval code in the specified scope.
///
/// If `use_global_scope` is true, executes in global scope (indirect eval).
/// If false, executes in the current scope (direct eval).
pub fn eval_code_in_scope(
    interp: &mut Interpreter,
    code: &str,
    use_global_scope: bool,
) -> Result<Guarded, JsError> {
    // For indirect eval, use global `this`
    let this_value = JsValue::Object(interp.global.cheap_clone());
    eval_code_in_scope_with_this(interp, code, use_global_scope, this_value)
}

/// Execute eval code in the specified scope with a specific `this` value.
/// Used by direct eval to preserve the calling context's `this`.
pub fn eval_code_in_scope_with_this(
    interp: &mut Interpreter,
    code: &str,
    use_global_scope: bool,
    this_value: JsValue,
) -> Result<Guarded, JsError> {
    // Empty or whitespace-only code returns undefined
    if code.trim().is_empty() {
        return Ok(Guarded::unguarded(JsValue::Undefined));
    }

    // Parse the code
    let mut parser = Parser::new(code, &mut interp.string_dict);
    let program = parser
        .parse_program()
        .map_err(|e| JsError::syntax_error_simple(format!("eval: {}", e)))?;

    // Save current environment
    let saved_env = interp.env.clone();

    // If using global scope (indirect eval), switch to global environment
    if use_global_scope {
        interp.env = interp.global_env.clone();
    }

    // In strict mode, eval code runs in its own lexical environment
    // This ensures let/const declarations don't leak to the outer scope
    // but can still read from it via the scope chain
    let eval_scope = interp.push_scope();

    // Execute the program with completion value tracking for proper eval semantics
    // Note: Var hoisting is handled by the bytecode compiler in compile_program_for_eval
    let result = interp.execute_program_for_eval_with_this(&program, this_value);

    // Pop the eval scope
    interp.pop_scope(eval_scope);

    // Restore original environment (needed for indirect eval case)
    interp.env = saved_env;

    match result {
        Ok(value) => {
            // Create a guard for the result if it's an object
            let guard = interp.guard_value(&value);
            Ok(Guarded { value, guard })
        }
        Err(e) => Err(e),
    }
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

    if !chars.len().is_multiple_of(4) {
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

/// structuredClone - Deep clone a value
/// Creates a deep copy of the given value using the structured clone algorithm
pub fn global_structured_clone(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let guard = interp.heap.create_guard();

    // Clone the value
    let cloned = structured_clone_internal(interp, &guard, &value)?;

    Ok(Guarded::with_guard(cloned, guard))
}

/// Internal recursive cloning function
fn structured_clone_internal(
    interp: &mut Interpreter,
    guard: &Guard<JsObject>,
    value: &JsValue,
) -> Result<JsValue, JsError> {
    match value {
        // Primitives are returned as-is (they're value types)
        JsValue::Undefined => Ok(JsValue::Undefined),
        JsValue::Null => Ok(JsValue::Null),
        JsValue::Boolean(b) => Ok(JsValue::Boolean(*b)),
        JsValue::Number(n) => Ok(JsValue::Number(*n)),
        JsValue::String(s) => Ok(JsValue::String(s.clone())),

        // Symbols cannot be cloned
        JsValue::Symbol(_) => Err(JsError::type_error(
            "Symbol cannot be cloned with structuredClone",
        )),

        // Objects require deep cloning
        JsValue::Object(obj) => clone_object(interp, guard, obj),
    }
}

/// Clone an object (handles arrays, maps, sets, dates, regexps, errors, and plain objects)
fn clone_object(
    interp: &mut Interpreter,
    guard: &Guard<JsObject>,
    obj: &Gc<JsObject>,
) -> Result<JsValue, JsError> {
    let obj_ref = obj.borrow();

    // Check the exotic type
    match &obj_ref.exotic {
        // Functions cannot be cloned
        ExoticObject::Function(_) => Err(JsError::type_error(
            "Function cannot be cloned with structuredClone",
        )),

        // Generators cannot be cloned
        ExoticObject::Generator(_) | ExoticObject::BytecodeGenerator(_) => Err(
            JsError::type_error("Generator cannot be cloned with structuredClone"),
        ),

        // Promises cannot be cloned
        ExoticObject::Promise(_) => Err(JsError::type_error(
            "Promise cannot be cloned with structuredClone",
        )),

        // Environments cannot be cloned
        ExoticObject::Environment(_) => Err(JsError::type_error(
            "Environment cannot be cloned with structuredClone",
        )),

        // Arrays - clone elements recursively
        ExoticObject::Array { elements } => {
            // Clone elements recursively first, then release borrow
            let elements_to_clone: Vec<JsValue> = elements.clone();
            drop(obj_ref); // Release borrow before recursive calls

            let mut cloned_elements = Vec::with_capacity(elements_to_clone.len());
            for elem in &elements_to_clone {
                cloned_elements.push(structured_clone_internal(interp, guard, elem)?);
            }

            let arr = interp.create_array_from(guard, cloned_elements);
            Ok(JsValue::Object(arr))
        }

        // Maps - clone entries recursively
        ExoticObject::Map { entries } => {
            let entries_to_clone: Vec<(JsValue, JsValue)> = entries.clone();
            drop(obj_ref);

            let mut cloned_entries = Vec::with_capacity(entries_to_clone.len());
            for (key, val) in &entries_to_clone {
                let cloned_key = structured_clone_internal(interp, guard, key)?;
                let cloned_val = structured_clone_internal(interp, guard, val)?;
                cloned_entries.push((cloned_key, cloned_val));
            }

            let map_obj = interp.create_object(guard);
            {
                let mut map_ref = map_obj.borrow_mut();
                map_ref.prototype = Some(interp.map_prototype.clone());
                map_ref.exotic = ExoticObject::Map {
                    entries: cloned_entries,
                };
            }
            Ok(JsValue::Object(map_obj))
        }

        // Sets - clone entries recursively
        ExoticObject::Set { entries } => {
            let entries_to_clone: Vec<JsValue> = entries.clone();
            drop(obj_ref);

            let mut cloned_entries = Vec::with_capacity(entries_to_clone.len());
            for entry in &entries_to_clone {
                cloned_entries.push(structured_clone_internal(interp, guard, entry)?);
            }

            let set_obj = interp.create_object(guard);
            {
                let mut set_ref = set_obj.borrow_mut();
                set_ref.prototype = Some(interp.set_prototype.clone());
                set_ref.exotic = ExoticObject::Set {
                    entries: cloned_entries,
                };
            }
            Ok(JsValue::Object(set_obj))
        }

        // Dates - clone the timestamp
        ExoticObject::Date { timestamp } => {
            let ts = *timestamp;
            drop(obj_ref);

            let date_obj = interp.create_object(guard);
            {
                let mut date_ref = date_obj.borrow_mut();
                date_ref.prototype = Some(interp.date_prototype.clone());
                date_ref.exotic = ExoticObject::Date { timestamp: ts };
            }
            Ok(JsValue::Object(date_obj))
        }

        // RegExps - clone pattern and flags
        ExoticObject::RegExp { pattern, flags } => {
            let pattern_clone = pattern.clone();
            let flags_clone = flags.clone();
            drop(obj_ref);

            let regexp_obj = interp.create_object(guard);
            {
                let mut regexp_ref = regexp_obj.borrow_mut();
                regexp_ref.prototype = Some(interp.regexp_prototype.clone());
                regexp_ref.exotic = ExoticObject::RegExp {
                    pattern: pattern_clone.clone(),
                    flags: flags_clone.clone(),
                };

                // Set properties like source, flags, etc.
                regexp_ref.set_property(
                    PropertyKey::from("source"),
                    JsValue::String(JsString::from(pattern_clone.as_str())),
                );
                regexp_ref.set_property(
                    PropertyKey::from("flags"),
                    JsValue::String(JsString::from(flags_clone.as_str())),
                );
                regexp_ref.set_property(
                    PropertyKey::from("global"),
                    JsValue::Boolean(flags_clone.contains('g')),
                );
                regexp_ref.set_property(
                    PropertyKey::from("ignoreCase"),
                    JsValue::Boolean(flags_clone.contains('i')),
                );
                regexp_ref.set_property(
                    PropertyKey::from("multiline"),
                    JsValue::Boolean(flags_clone.contains('m')),
                );
                regexp_ref.set_property(
                    PropertyKey::from("dotAll"),
                    JsValue::Boolean(flags_clone.contains('s')),
                );
                regexp_ref.set_property(
                    PropertyKey::from("unicode"),
                    JsValue::Boolean(flags_clone.contains('u')),
                );
                regexp_ref.set_property(
                    PropertyKey::from("sticky"),
                    JsValue::Boolean(flags_clone.contains('y')),
                );
                regexp_ref.set_property(PropertyKey::from("lastIndex"), JsValue::Number(0.0));
            }
            Ok(JsValue::Object(regexp_obj))
        }

        // Booleans - clone the boolean value
        ExoticObject::Boolean(b) => {
            let bool_val = *b;
            drop(obj_ref);

            let bool_obj = interp.create_object(guard);
            {
                let mut bool_ref = bool_obj.borrow_mut();
                bool_ref.prototype = Some(interp.boolean_prototype.clone());
                bool_ref.exotic = ExoticObject::Boolean(bool_val);
            }
            Ok(JsValue::Object(bool_obj))
        }

        // Numbers - clone the number value
        ExoticObject::Number(n) => {
            let num_val = *n;
            drop(obj_ref);

            let num_obj = interp.create_object(guard);
            {
                let mut num_ref = num_obj.borrow_mut();
                num_ref.prototype = Some(interp.number_prototype.clone());
                num_ref.exotic = ExoticObject::Number(num_val);
            }
            Ok(JsValue::Object(num_obj))
        }

        // Strings - clone the string value
        ExoticObject::StringObj(s) => {
            let str_val = s.cheap_clone();
            drop(obj_ref);

            let str_obj = interp.create_object(guard);
            {
                let mut str_ref = str_obj.borrow_mut();
                str_ref.prototype = Some(interp.string_prototype.clone());
                str_ref.exotic = ExoticObject::StringObj(str_val.cheap_clone());
                // Also set length property
                let length_key = PropertyKey::String(interp.intern("length"));
                str_ref.set_property(
                    length_key,
                    JsValue::Number(str_val.as_str().chars().count() as f64),
                );
            }
            Ok(JsValue::Object(str_obj))
        }

        // Enums - clone the enum data
        ExoticObject::Enum(data) => {
            let data_clone = data.clone();
            drop(obj_ref);

            let enum_obj = interp.create_object(guard);
            {
                let mut enum_ref = enum_obj.borrow_mut();
                enum_ref.exotic = ExoticObject::Enum(data_clone);
            }
            Ok(JsValue::Object(enum_obj))
        }

        // Ordinary objects - clone properties recursively
        ExoticObject::Ordinary => {
            // Collect properties to clone (extract values from Property wrapper)
            let props_to_clone: Vec<(PropertyKey, JsValue)> = obj_ref
                .properties
                .iter()
                .map(|(k, prop)| (k.clone(), prop.value.clone()))
                .collect();

            // Check if this is an Error object by looking at prototype chain
            let is_error = is_error_object(&obj_ref, interp);

            drop(obj_ref);

            let cloned_obj = interp.create_object(guard);

            // Clone each property
            for (key, value) in &props_to_clone {
                let cloned_value = structured_clone_internal(interp, guard, value)?;
                cloned_obj
                    .borrow_mut()
                    .set_property(key.clone(), cloned_value);
            }

            // For error objects, the properties are already cloned so we don't need
            // to set a special prototype - object_prototype is sufficient
            let _ = is_error; // silence unused warning

            Ok(JsValue::Object(cloned_obj))
        }

        // Proxies cannot be cloned with structuredClone
        ExoticObject::Proxy(_) => Err(JsError::type_error(
            "Proxy cannot be cloned with structuredClone",
        )),

        // RawJSON - clone the raw JSON string
        ExoticObject::RawJSON(raw) => {
            let raw_clone = raw.cheap_clone();
            drop(obj_ref);

            let raw_obj = interp.create_object(guard);
            {
                let mut raw_ref = raw_obj.borrow_mut();
                raw_ref.exotic = ExoticObject::RawJSON(raw_clone);
                raw_ref.prototype = None;
                raw_ref.null_prototype = true;
            }
            Ok(JsValue::Object(raw_obj))
        }

        // Symbol wrapper objects - clone with the same symbol value
        ExoticObject::Symbol(sym) => {
            let sym_clone = sym.clone();
            drop(obj_ref);

            let sym_obj = interp.create_object(guard);
            {
                let mut sym_ref = sym_obj.borrow_mut();
                sym_ref.exotic = ExoticObject::Symbol(sym_clone);
                sym_ref.prototype = Some(interp.symbol_prototype.cheap_clone());
            }
            Ok(JsValue::Object(sym_obj))
        }
    }
}

/// Check if an object is an Error object by looking for error-like properties
fn is_error_object(obj_ref: &std::cell::Ref<JsObject>, _interp: &Interpreter) -> bool {
    // Check if it has name, message, and stack properties typical of Error objects
    let has_name = obj_ref.properties.contains_key(&PropertyKey::from("name"));
    let has_message = obj_ref
        .properties
        .contains_key(&PropertyKey::from("message"));
    let has_stack = obj_ref.properties.contains_key(&PropertyKey::from("stack"));

    // If it has all three error-like properties, consider it an error
    has_name && has_message && has_stack
}
