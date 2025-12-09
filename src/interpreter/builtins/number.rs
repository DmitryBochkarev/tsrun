//! Number built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, JsFunction, JsObjectRef, JsString, JsValue, NativeFunction,
    PropertyKey,
};

use super::global::{global_parse_float, global_parse_int};

/// Create Number.prototype with toFixed, toString, toPrecision, toExponential
pub fn create_number_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        let tofixed_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toFixed".to_string(),
            func: number_to_fixed,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("toFixed"), JsValue::Object(tofixed_fn));

        let tostring_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toString".to_string(),
            func: number_to_string,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("toString"), JsValue::Object(tostring_fn));

        let toprecision_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toPrecision".to_string(),
            func: number_to_precision,
            arity: 1,
        }));
        p.set_property(
            PropertyKey::from("toPrecision"),
            JsValue::Object(toprecision_fn),
        );

        let toexponential_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toExponential".to_string(),
            func: number_to_exponential,
            arity: 1,
        }));
        p.set_property(
            PropertyKey::from("toExponential"),
            JsValue::Object(toexponential_fn),
        );
    }
    proto
}

/// Create Number constructor with static methods and constants
pub fn create_number_constructor(number_prototype: &JsObjectRef) -> JsObjectRef {
    let constructor = create_object();
    {
        let mut num = constructor.borrow_mut();

        // Static methods
        let isnan_fn = create_function(JsFunction::Native(NativeFunction {
            name: "isNaN".to_string(),
            func: number_is_nan,
            arity: 1,
        }));
        num.set_property(PropertyKey::from("isNaN"), JsValue::Object(isnan_fn));

        let isfinite_fn = create_function(JsFunction::Native(NativeFunction {
            name: "isFinite".to_string(),
            func: number_is_finite,
            arity: 1,
        }));
        num.set_property(PropertyKey::from("isFinite"), JsValue::Object(isfinite_fn));

        let isinteger_fn = create_function(JsFunction::Native(NativeFunction {
            name: "isInteger".to_string(),
            func: number_is_integer,
            arity: 1,
        }));
        num.set_property(
            PropertyKey::from("isInteger"),
            JsValue::Object(isinteger_fn),
        );

        let issafeinteger_fn = create_function(JsFunction::Native(NativeFunction {
            name: "isSafeInteger".to_string(),
            func: number_is_safe_integer,
            arity: 1,
        }));
        num.set_property(
            PropertyKey::from("isSafeInteger"),
            JsValue::Object(issafeinteger_fn),
        );

        let parseint_fn = create_function(JsFunction::Native(NativeFunction {
            name: "parseInt".to_string(),
            func: global_parse_int,
            arity: 2,
        }));
        num.set_property(PropertyKey::from("parseInt"), JsValue::Object(parseint_fn));

        let parsefloat_fn = create_function(JsFunction::Native(NativeFunction {
            name: "parseFloat".to_string(),
            func: global_parse_float,
            arity: 1,
        }));
        num.set_property(
            PropertyKey::from("parseFloat"),
            JsValue::Object(parsefloat_fn),
        );

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
    args: Vec<JsValue>,
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
    args: Vec<JsValue>,
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
    args: Vec<JsValue>,
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
    args: Vec<JsValue>,
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
    args: Vec<JsValue>,
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
    args: Vec<JsValue>,
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
            let chars: Vec<char> = "0123456789abcdefghijklmnopqrstuvwxyz".chars().collect();
            let mut num = int_val.abs();
            let mut result = String::new();
            while num > 0 {
                result.insert(0, chars[(num % radix as i64) as usize]);
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
    args: Vec<JsValue>,
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
    if parts.len() == 2 {
        let mantissa = parts[0].parse::<f64>().unwrap_or(0.0);
        let exp: i32 = parts[1].parse().unwrap_or(0);

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
    args: Vec<JsValue>,
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
