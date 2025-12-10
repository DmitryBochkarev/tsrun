//! Number built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_object, register_method, JsObjectRef, JsString, JsValue, PropertyKey};

use super::global::{global_parse_float, global_parse_int};

/// Create Number.prototype with toFixed, toString, toPrecision, toExponential
pub fn create_number_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        register_method(&mut p, "toFixed", number_to_fixed, 1);
        register_method(&mut p, "toString", number_to_string, 1);
        register_method(&mut p, "toPrecision", number_to_precision, 1);
        register_method(&mut p, "toExponential", number_to_exponential, 1);
    }
    proto
}

/// Create Number constructor with static methods and constants
pub fn create_number_constructor(number_prototype: &JsObjectRef) -> JsObjectRef {
    let constructor = create_object();
    {
        let mut num = constructor.borrow_mut();

        // Static methods
        register_method(&mut num, "isNaN", number_is_nan, 1);
        register_method(&mut num, "isFinite", number_is_finite, 1);
        register_method(&mut num, "isInteger", number_is_integer, 1);
        register_method(&mut num, "isSafeInteger", number_is_safe_integer, 1);
        register_method(&mut num, "parseInt", global_parse_int, 2);
        register_method(&mut num, "parseFloat", global_parse_float, 1);

        // Constants
        num.set_property(
            PropertyKey::from("POSITIVE_INFINITY"),
            JsValue::Number(f64::INFINITY),
        );
        num.set_property(
            PropertyKey::from("NEGATIVE_INFINITY"),
            JsValue::Number(f64::NEG_INFINITY),
        );
        num.set_property(PropertyKey::from("MAX_VALUE"), JsValue::Number(f64::MAX));
        num.set_property(
            PropertyKey::from("MIN_VALUE"),
            JsValue::Number(f64::MIN_POSITIVE),
        );
        num.set_property(
            PropertyKey::from("MAX_SAFE_INTEGER"),
            JsValue::Number(9007199254740991.0),
        );
        num.set_property(
            PropertyKey::from("MIN_SAFE_INTEGER"),
            JsValue::Number(-9007199254740991.0),
        );
        num.set_property(PropertyKey::from("EPSILON"), JsValue::Number(f64::EPSILON));
        num.set_property(PropertyKey::from("NaN"), JsValue::Number(f64::NAN));

        num.set_property(
            PropertyKey::from("prototype"),
            JsValue::Object(number_prototype.clone()),
        );
    }
    constructor
}

// Number.isNaN - stricter, no type coercion
pub fn number_is_nan(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => Ok(JsValue::Boolean(n.is_nan())),
        _ => Ok(JsValue::Boolean(false)),
    }
}

// Number.isFinite - stricter, no type coercion
pub fn number_is_finite(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => Ok(JsValue::Boolean(n.is_finite())),
        _ => Ok(JsValue::Boolean(false)),
    }
}

// Number.isInteger
pub fn number_is_integer(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => {
            let is_int = n.is_finite() && n.trunc() == *n;
            Ok(JsValue::Boolean(is_int))
        }
        _ => Ok(JsValue::Boolean(false)),
    }
}

// Number.isSafeInteger
pub fn number_is_safe_integer(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    const MAX_SAFE: f64 = 9007199254740991.0;
    match args.first() {
        Some(JsValue::Number(n)) => {
            let is_safe = n.is_finite() && n.trunc() == *n && n.abs() <= MAX_SAFE;
            Ok(JsValue::Boolean(is_safe))
        }
        _ => Ok(JsValue::Boolean(false)),
    }
}

// Number.prototype.toFixed
pub fn number_to_fixed(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = this.to_number();
    let digits = args.first().map(|v| v.to_number() as i32).unwrap_or(0);

    if !(0..=100).contains(&digits) {
        return Err(JsError::range_error(
            "toFixed() digits argument must be between 0 and 100",
        ));
    }

    let result = format!("{:.prec$}", n, prec = digits as usize);
    Ok(JsValue::String(JsString::from(result)))
}

// Number.prototype.toString
pub fn number_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = this.to_number();
    let radix = args.first().map(|v| v.to_number() as i32).unwrap_or(10);

    if !(2..=36).contains(&radix) {
        return Err(JsError::range_error(
            "toString() radix must be between 2 and 36",
        ));
    }

    if radix == 10 {
        return Ok(JsValue::String(JsString::from(format!("{}", n))));
    }

    // For other radixes, we need integer conversion
    if !n.is_finite() || n.fract() != 0.0 {
        return Ok(JsValue::String(JsString::from(format!("{}", n))));
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

    Ok(JsValue::String(JsString::from(result)))
}

// Number.prototype.toPrecision
pub fn number_to_precision(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = this.to_number();

    if args.is_empty() || matches!(args.first(), Some(JsValue::Undefined)) {
        return Ok(JsValue::String(JsString::from(format!("{}", n))));
    }

    let precision = args.first().map(|v| v.to_number() as i32).unwrap_or(1);

    if !(1..=100).contains(&precision) {
        return Err(JsError::range_error(
            "toPrecision() argument must be between 1 and 100",
        ));
    }

    if !n.is_finite() {
        return Ok(JsValue::String(JsString::from(format!("{}", n))));
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
                return Ok(JsValue::String(JsString::from(format!(
                    "{:.prec$}",
                    n,
                    prec = decimals as usize
                ))));
            }
        } else if (-4..0).contains(&exp) {
            // For small numbers, use fixed notation
            let decimals = precision - 1 - exp;
            if (0..=100).contains(&decimals) {
                return Ok(JsValue::String(JsString::from(format!(
                    "{:.prec$}",
                    n,
                    prec = decimals as usize
                ))));
            }
        }

        // Use exponential notation
        let exp_sign = if exp >= 0 { "+" } else { "" };
        return Ok(JsValue::String(JsString::from(format!(
            "{}e{}{}",
            mantissa, exp_sign, exp
        ))));
    }

    Ok(JsValue::String(JsString::from(format!("{}", n))))
}

// Number.prototype.toExponential
pub fn number_to_exponential(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = this.to_number();

    if !n.is_finite() {
        return Ok(JsValue::String(JsString::from(format!("{}", n))));
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
    Ok(JsValue::String(JsString::from(result)))
}
