//! Math built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_object, register_method, JsObjectRef, JsValue, PropertyKey};

/// Create Math object with all math methods and constants
pub fn create_math_object() -> JsObjectRef {
    let math_obj = create_object();
    {
        let mut math = math_obj.borrow_mut();

        // Constants
        math.set_property(
            PropertyKey::from("PI"),
            JsValue::Number(std::f64::consts::PI),
        );
        math.set_property(PropertyKey::from("E"), JsValue::Number(std::f64::consts::E));
        math.set_property(
            PropertyKey::from("LN2"),
            JsValue::Number(std::f64::consts::LN_2),
        );
        math.set_property(
            PropertyKey::from("LN10"),
            JsValue::Number(std::f64::consts::LN_10),
        );
        math.set_property(
            PropertyKey::from("LOG2E"),
            JsValue::Number(std::f64::consts::LOG2_E),
        );
        math.set_property(
            PropertyKey::from("LOG10E"),
            JsValue::Number(std::f64::consts::LOG10_E),
        );
        math.set_property(
            PropertyKey::from("SQRT2"),
            JsValue::Number(std::f64::consts::SQRT_2),
        );
        math.set_property(
            PropertyKey::from("SQRT1_2"),
            JsValue::Number(std::f64::consts::FRAC_1_SQRT_2),
        );

        // Rounding methods
        register_method(&mut math, "abs", math_abs, 1);
        register_method(&mut math, "floor", math_floor, 1);
        register_method(&mut math, "ceil", math_ceil, 1);
        register_method(&mut math, "round", math_round, 1);
        register_method(&mut math, "trunc", math_trunc, 1);
        register_method(&mut math, "sign", math_sign, 1);

        // Min/max
        register_method(&mut math, "min", math_min, 2);
        register_method(&mut math, "max", math_max, 2);

        // Power and root functions
        register_method(&mut math, "pow", math_pow, 2);
        register_method(&mut math, "sqrt", math_sqrt, 1);
        register_method(&mut math, "cbrt", math_cbrt, 1);
        register_method(&mut math, "hypot", math_hypot, 2);

        // Logarithmic and exponential
        register_method(&mut math, "log", math_log, 1);
        register_method(&mut math, "log10", math_log10, 1);
        register_method(&mut math, "log2", math_log2, 1);
        register_method(&mut math, "log1p", math_log1p, 1);
        register_method(&mut math, "exp", math_exp, 1);
        register_method(&mut math, "expm1", math_expm1, 1);

        // Trigonometric
        register_method(&mut math, "sin", math_sin, 1);
        register_method(&mut math, "cos", math_cos, 1);
        register_method(&mut math, "tan", math_tan, 1);
        register_method(&mut math, "asin", math_asin, 1);
        register_method(&mut math, "acos", math_acos, 1);
        register_method(&mut math, "atan", math_atan, 1);
        register_method(&mut math, "atan2", math_atan2, 2);

        // Hyperbolic
        register_method(&mut math, "sinh", math_sinh, 1);
        register_method(&mut math, "cosh", math_cosh, 1);
        register_method(&mut math, "tanh", math_tanh, 1);
        register_method(&mut math, "asinh", math_asinh, 1);
        register_method(&mut math, "acosh", math_acosh, 1);
        register_method(&mut math, "atanh", math_atanh, 1);

        // Random
        register_method(&mut math, "random", math_random, 0);
    }
    math_obj
}

pub fn math_abs(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.abs()))
}

pub fn math_floor(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.floor()))
}

pub fn math_ceil(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.ceil()))
}

pub fn math_round(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.round()))
}

pub fn math_trunc(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.trunc()))
}

pub fn math_sign(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let result = if n.is_nan() {
        f64::NAN
    } else if n > 0.0 {
        1.0
    } else if n < 0.0 {
        -1.0
    } else {
        0.0
    };
    Ok(JsValue::Number(result))
}

pub fn math_min(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::INFINITY));
    }
    let mut min = f64::INFINITY;
    for arg in args {
        let n = arg.to_number();
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        if n < min {
            min = n;
        }
    }
    Ok(JsValue::Number(min))
}

pub fn math_max(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::NEG_INFINITY));
    }
    let mut max = f64::NEG_INFINITY;
    for arg in args {
        let n = arg.to_number();
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        if n > max {
            max = n;
        }
    }
    Ok(JsValue::Number(max))
}

pub fn math_pow(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let base = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let exp = args.get(1).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(base.powf(exp)))
}

pub fn math_sqrt(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sqrt()))
}

pub fn math_log(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.ln()))
}

pub fn math_exp(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.exp()))
}

pub fn math_random(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple pseudo-random using system time (not cryptographically secure)
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as f64)
        .unwrap_or(0.0);
    let random = (seed / 1_000_000_000.0) % 1.0;
    Ok(JsValue::Number(random))
}

pub fn math_sin(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sin()))
}

pub fn math_cos(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.cos()))
}

pub fn math_tan(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.tan()))
}

pub fn math_asin(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.asin()))
}

pub fn math_acos(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.acos()))
}

pub fn math_atan(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.atan()))
}

pub fn math_atan2(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let y = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let x = args.get(1).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(y.atan2(x)))
}

pub fn math_sinh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sinh()))
}

pub fn math_cosh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.cosh()))
}

pub fn math_tanh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.tanh()))
}

pub fn math_asinh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.asinh()))
}

pub fn math_acosh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.acosh()))
}

pub fn math_atanh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.atanh()))
}

pub fn math_cbrt(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.cbrt()))
}

pub fn math_hypot(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    if args.is_empty() {
        return Ok(JsValue::Number(0.0));
    }
    let sum_sq: f64 = args
        .iter()
        .map(|v| {
            let n = v.to_number();
            n * n
        })
        .sum();
    Ok(JsValue::Number(sum_sq.sqrt()))
}

pub fn math_log10(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.log10()))
}

pub fn math_log2(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.log2()))
}

pub fn math_log1p(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.ln_1p()))
}

pub fn math_expm1(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.exp_m1()))
}
