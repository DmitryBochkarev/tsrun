//! Math built-in methods

use crate::error::JsError;
use crate::gc::Gc;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsObject, JsValue};

/// Initialize Math object and bind it to global scope.
/// Returns the Math object for rooting.
pub fn init_math(interp: &mut Interpreter) -> Gc<JsObject> {
    // Use root_guard for permanent global objects
    let math_obj = interp.root_guard.alloc();
    math_obj.borrow_mut().prototype = Some(interp.object_prototype.clone());

    // Constants
    let pi_key = interp.key("PI");
    let e_key = interp.key("E");
    let ln2_key = interp.key("LN2");
    let ln10_key = interp.key("LN10");
    let log2e_key = interp.key("LOG2E");
    let log10e_key = interp.key("LOG10E");
    let sqrt2_key = interp.key("SQRT2");
    let sqrt1_2_key = interp.key("SQRT1_2");

    {
        let mut math = math_obj.borrow_mut();
        math.set_property(pi_key, JsValue::Number(std::f64::consts::PI));
        math.set_property(e_key, JsValue::Number(std::f64::consts::E));
        math.set_property(ln2_key, JsValue::Number(std::f64::consts::LN_2));
        math.set_property(ln10_key, JsValue::Number(std::f64::consts::LN_10));
        math.set_property(log2e_key, JsValue::Number(std::f64::consts::LOG2_E));
        math.set_property(log10e_key, JsValue::Number(std::f64::consts::LOG10_E));
        math.set_property(sqrt2_key, JsValue::Number(std::f64::consts::SQRT_2));
        math.set_property(
            sqrt1_2_key,
            JsValue::Number(std::f64::consts::FRAC_1_SQRT_2),
        );
    }

    // Rounding methods
    interp.register_method(&math_obj, "abs", math_abs, 1);
    interp.register_method(&math_obj, "floor", math_floor, 1);
    interp.register_method(&math_obj, "ceil", math_ceil, 1);
    interp.register_method(&math_obj, "round", math_round, 1);
    interp.register_method(&math_obj, "trunc", math_trunc, 1);
    interp.register_method(&math_obj, "sign", math_sign, 1);

    // Min/max
    interp.register_method(&math_obj, "min", math_min, 2);
    interp.register_method(&math_obj, "max", math_max, 2);

    // Power and root functions
    interp.register_method(&math_obj, "pow", math_pow, 2);
    interp.register_method(&math_obj, "sqrt", math_sqrt, 1);
    interp.register_method(&math_obj, "cbrt", math_cbrt, 1);
    interp.register_method(&math_obj, "hypot", math_hypot, 2);

    // Logarithmic and exponential
    interp.register_method(&math_obj, "log", math_log, 1);
    interp.register_method(&math_obj, "log10", math_log10, 1);
    interp.register_method(&math_obj, "log2", math_log2, 1);
    interp.register_method(&math_obj, "log1p", math_log1p, 1);
    interp.register_method(&math_obj, "exp", math_exp, 1);
    interp.register_method(&math_obj, "expm1", math_expm1, 1);

    // Trigonometric
    interp.register_method(&math_obj, "sin", math_sin, 1);
    interp.register_method(&math_obj, "cos", math_cos, 1);
    interp.register_method(&math_obj, "tan", math_tan, 1);
    interp.register_method(&math_obj, "asin", math_asin, 1);
    interp.register_method(&math_obj, "acos", math_acos, 1);
    interp.register_method(&math_obj, "atan", math_atan, 1);
    interp.register_method(&math_obj, "atan2", math_atan2, 2);

    // Hyperbolic
    interp.register_method(&math_obj, "sinh", math_sinh, 1);
    interp.register_method(&math_obj, "cosh", math_cosh, 1);
    interp.register_method(&math_obj, "tanh", math_tanh, 1);
    interp.register_method(&math_obj, "asinh", math_asinh, 1);
    interp.register_method(&math_obj, "acosh", math_acosh, 1);
    interp.register_method(&math_obj, "atanh", math_atanh, 1);

    // Random
    interp.register_method(&math_obj, "random", math_random, 0);

    // Root Math object and bind to global
    interp.root_guard.guard(math_obj.clone());
    let math_key = interp.key("Math");
    let result = math_obj.clone();
    interp
        .global
        .borrow_mut()
        .set_property(math_key, JsValue::Object(math_obj));

    result
}

pub fn math_abs(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.abs())))
}

pub fn math_floor(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.floor())))
}

pub fn math_ceil(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.ceil())))
}

pub fn math_round(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.round())))
}

pub fn math_trunc(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.trunc())))
}

pub fn math_sign(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
    Ok(Guarded::unguarded(JsValue::Number(result)))
}

pub fn math_min(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    if args.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::INFINITY)));
    }
    let mut min = f64::INFINITY;
    for arg in args {
        let n = arg.to_number();
        if n.is_nan() {
            return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
        }
        if n < min {
            min = n;
        }
    }
    Ok(Guarded::unguarded(JsValue::Number(min)))
}

pub fn math_max(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    if args.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NEG_INFINITY)));
    }
    let mut max = f64::NEG_INFINITY;
    for arg in args {
        let n = arg.to_number();
        if n.is_nan() {
            return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
        }
        if n > max {
            max = n;
        }
    }
    Ok(Guarded::unguarded(JsValue::Number(max)))
}

pub fn math_pow(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let base = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let exp = args.get(1).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(base.powf(exp))))
}

pub fn math_sqrt(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.sqrt())))
}

pub fn math_log(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.ln())))
}

pub fn math_exp(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.exp())))
}

pub fn math_random(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple pseudo-random using system time (not cryptographically secure)
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as f64)
        .unwrap_or(0.0);
    let random = (seed / 1_000_000_000.0) % 1.0;
    Ok(Guarded::unguarded(JsValue::Number(random)))
}

pub fn math_sin(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.sin())))
}

pub fn math_cos(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.cos())))
}

pub fn math_tan(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.tan())))
}

pub fn math_asin(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.asin())))
}

pub fn math_acos(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.acos())))
}

pub fn math_atan(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.atan())))
}

pub fn math_atan2(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let y = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let x = args.get(1).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(y.atan2(x))))
}

pub fn math_sinh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.sinh())))
}

pub fn math_cosh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.cosh())))
}

pub fn math_tanh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.tanh())))
}

pub fn math_asinh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.asinh())))
}

pub fn math_acosh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.acosh())))
}

pub fn math_atanh(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.atanh())))
}

pub fn math_cbrt(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.cbrt())))
}

pub fn math_hypot(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    if args.is_empty() {
        return Ok(Guarded::unguarded(JsValue::Number(0.0)));
    }
    let sum_sq: f64 = args
        .iter()
        .map(|v| {
            let n = v.to_number();
            n * n
        })
        .sum();
    Ok(Guarded::unguarded(JsValue::Number(sum_sq.sqrt())))
}

pub fn math_log10(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.log10())))
}

pub fn math_log2(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.log2())))
}

pub fn math_log1p(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.ln_1p())))
}

pub fn math_expm1(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(Guarded::unguarded(JsValue::Number(n.exp_m1())))
}
