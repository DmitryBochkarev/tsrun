//! Number built-in methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsObject, JsString, JsValue};

/// Initialize Number.prototype with toFixed, toString, toPrecision, toExponential
pub fn init_number_prototype(interp: &mut Interpreter) {
    let proto = interp.number_prototype.clone();

    interp.register_method(&proto, "toFixed", number_to_fixed, 1);
    interp.register_method(&proto, "toString", number_to_string, 1);
    interp.register_method(&proto, "toPrecision", number_to_precision, 1);
    interp.register_method(&proto, "toExponential", number_to_exponential, 1);
}

/// Number constructor function - Number(value) converts value to number
pub fn number_constructor_fn(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Number() with no arguments returns 0
    if args.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Number(0.0)));
    }
    // Number(value) converts value to number
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    Ok(Guarded::unguarded(JsValue::Number(value.to_number())))
}

/// Create Number constructor with static methods and constants
pub fn create_number_constructor(interp: &mut Interpreter) -> Gc<JsObject> {
    let constructor = interp.create_native_function("Number", number_constructor_fn, 1);

    // Static methods
    interp.register_method(&constructor, "isNaN", number_is_nan, 1);
    interp.register_method(&constructor, "isFinite", number_is_finite, 1);
    interp.register_method(&constructor, "isInteger", number_is_integer, 1);
    interp.register_method(&constructor, "isSafeInteger", number_is_safe_integer, 1);
    interp.register_method(&constructor, "parseFloat", number_parse_float, 1);
    interp.register_method(&constructor, "parseInt", number_parse_int, 2);

    // Constants
    let max_value_key = interp.key("MAX_VALUE");
    let min_value_key = interp.key("MIN_VALUE");
    let max_safe_key = interp.key("MAX_SAFE_INTEGER");
    let min_safe_key = interp.key("MIN_SAFE_INTEGER");
    let nan_key = interp.key("NaN");
    let pos_inf_key = interp.key("POSITIVE_INFINITY");
    let neg_inf_key = interp.key("NEGATIVE_INFINITY");
    let epsilon_key = interp.key("EPSILON");

    {
        let mut c = constructor.borrow_mut();
        c.set_property(max_value_key, JsValue::Number(f64::MAX));
        c.set_property(min_value_key, JsValue::Number(f64::MIN_POSITIVE));
        c.set_property(max_safe_key, JsValue::Number(9007199254740991.0));
        c.set_property(min_safe_key, JsValue::Number(-9007199254740991.0));
        c.set_property(nan_key, JsValue::Number(f64::NAN));
        c.set_property(pos_inf_key, JsValue::Number(f64::INFINITY));
        c.set_property(neg_inf_key, JsValue::Number(f64::NEG_INFINITY));
        c.set_property(epsilon_key, JsValue::Number(f64::EPSILON));
    }

    let proto_key = interp.key("prototype");
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.number_prototype.clone()));

    constructor
}

/// Number.parseFloat - same as global parseFloat
pub fn number_parse_float(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = args
        .first()
        .cloned()
        .unwrap_or(JsValue::Undefined)
        .to_js_string()
        .to_string();

    let trimmed = s.trim_start();
    let result = trimmed.parse::<f64>().unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(result)))
}

/// Number.parseInt - same as global parseInt
pub fn number_parse_int(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let s = args
        .first()
        .cloned()
        .unwrap_or(JsValue::Undefined)
        .to_js_string()
        .to_string();
    let radix = args.get(1).map(|v| v.to_number() as i32).unwrap_or(10);

    let trimmed = s.trim_start();

    // Handle radix
    let radix = if radix == 0 {
        10
    } else if !(2..=36).contains(&radix) {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    } else {
        radix
    };

    let result = i64::from_str_radix(trimmed, radix as u32)
        .map(|n| n as f64)
        .unwrap_or(f64::NAN);

    Ok(Guarded::unguarded(JsValue::Number(result)))
}

// Number.isNaN - stricter, no type coercion
pub fn number_is_nan(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => Ok(Guarded::unguarded(JsValue::Boolean(n.is_nan()))),
        _ => Ok(Guarded::unguarded(JsValue::Boolean(false))),
    }
}

// Number.isFinite - stricter, no type coercion
pub fn number_is_finite(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => Ok(Guarded::unguarded(JsValue::Boolean(n.is_finite()))),
        _ => Ok(Guarded::unguarded(JsValue::Boolean(false))),
    }
}

// Number.isInteger
pub fn number_is_integer(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => {
            let is_int = n.is_finite() && n.trunc() == *n;
            Ok(Guarded::unguarded(JsValue::Boolean(is_int)))
        }
        _ => Ok(Guarded::unguarded(JsValue::Boolean(false))),
    }
}

// Number.isSafeInteger
pub fn number_is_safe_integer(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    const MAX_SAFE: f64 = 9007199254740991.0;
    match args.first() {
        Some(JsValue::Number(n)) => {
            let is_safe = n.is_finite() && n.trunc() == *n && n.abs() <= MAX_SAFE;
            Ok(Guarded::unguarded(JsValue::Boolean(is_safe)))
        }
        _ => Ok(Guarded::unguarded(JsValue::Boolean(false))),
    }
}

// Number.prototype.toFixed
pub fn number_to_fixed(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = this.to_number();
    let digits = args.first().map(|v| v.to_number() as i32).unwrap_or(0);

    if !(0..=100).contains(&digits) {
        return Err(JsError::range_error(
            "toFixed() digits argument must be between 0 and 100",
        ));
    }

    let result = format!("{:.prec$}", n, prec = digits as usize);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

// Number.prototype.toString
pub fn number_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = this.to_number();
    let radix = args.first().map(|v| v.to_number() as i32).unwrap_or(10);

    if !(2..=36).contains(&radix) {
        return Err(JsError::range_error(
            "toString() radix must be between 2 and 36",
        ));
    }

    if radix == 10 {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            format!("{}", n),
        ))));
    }

    // For other radixes, we need integer conversion
    if !n.is_finite() || n.fract() != 0.0 {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            format!("{}", n),
        ))));
    }

    let int_val = n as i64;
    let result = match radix {
        2 => format!("{:b}", int_val.abs()),
        8 => format!("{:o}", int_val.abs()),
        16 => format!("{:x}", int_val.abs()),
        _ => {
            // Generic radix conversion
            const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
            let mut num = int_val.abs();
            let mut result = String::new();
            while num > 0 {
                let digit_idx = (num % radix as i64) as usize;
                // radix is validated to be 2-36, so digit_idx is always 0-35
                if let Some(&ch) = DIGITS.get(digit_idx) {
                    result.insert(0, ch as char);
                }
                num /= radix as i64;
            }
            if result.is_empty() {
                result = "0".to_string();
            }
            result
        }
    };

    let result = if int_val < 0 {
        format!("-{}", result)
    } else {
        result
    };

    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}

// Number.prototype.toPrecision
pub fn number_to_precision(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = this.to_number();

    if args.is_empty() || matches!(args.first(), Some(JsValue::Undefined)) {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            format!("{}", n),
        ))));
    }

    let precision = args.first().map(|v| v.to_number() as i32).unwrap_or(1);

    if !(1..=100).contains(&precision) {
        return Err(JsError::range_error(
            "toPrecision() argument must be between 1 and 100",
        ));
    }

    if !n.is_finite() {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            format!("{}", n),
        ))));
    }

    let result = format!("{:.prec$e}", n, prec = (precision - 1) as usize);
    // Parse and reformat to match JS behavior
    let parts: Vec<&str> = result.split('e').collect();
    if let [mantissa_str, exp_str] = parts.as_slice() {
        let mantissa = mantissa_str.parse::<f64>().unwrap_or(0.0);
        let exp: i32 = exp_str.parse().unwrap_or(0);

        // If exponent is small enough, use fixed notation
        if exp >= 0 && exp < precision {
            let decimals = precision - 1 - exp;
            if decimals >= 0 {
                return Ok(Guarded::unguarded(JsValue::String(JsString::from(
                    format!("{:.prec$}", n, prec = decimals as usize),
                ))));
            }
        } else if (-4..0).contains(&exp) {
            // For small numbers, use fixed notation
            let decimals = precision - 1 - exp;
            if (0..=100).contains(&decimals) {
                return Ok(Guarded::unguarded(JsValue::String(JsString::from(
                    format!("{:.prec$}", n, prec = decimals as usize),
                ))));
            }
        }

        // Use exponential notation
        let exp_sign = if exp >= 0 { "+" } else { "" };
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            format!("{}e{}{}", mantissa, exp_sign, exp),
        ))));
    }

    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        format!("{}", n),
    ))))
}

// Number.prototype.toExponential
pub fn number_to_exponential(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = this.to_number();

    if !n.is_finite() {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            format!("{}", n),
        ))));
    }

    let digits = args.first().map(|v| v.to_number() as i32).unwrap_or(6);

    if !(0..=100).contains(&digits) {
        return Err(JsError::range_error(
            "toExponential() argument must be between 0 and 100",
        ));
    }

    let result = format!("{:.prec$e}", n, prec = digits as usize);
    // Convert Rust's "e" notation to JS format (e.g., "1.23e2" -> "1.23e+2")
    let result = result.replace("e", "e+").replace("e+-", "e-");
    Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
}
