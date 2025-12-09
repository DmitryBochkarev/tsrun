//! Math built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, JsFunction, JsObjectRef, JsValue, NativeFunction, PropertyKey,
};

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

        // Methods
        let abs_fn = create_function(JsFunction::Native(NativeFunction {
            name: "abs".to_string(),
            func: math_abs,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("abs"), JsValue::Object(abs_fn));

        let floor_fn = create_function(JsFunction::Native(NativeFunction {
            name: "floor".to_string(),
            func: math_floor,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("floor"), JsValue::Object(floor_fn));

        let ceil_fn = create_function(JsFunction::Native(NativeFunction {
            name: "ceil".to_string(),
            func: math_ceil,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("ceil"), JsValue::Object(ceil_fn));

        let round_fn = create_function(JsFunction::Native(NativeFunction {
            name: "round".to_string(),
            func: math_round,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("round"), JsValue::Object(round_fn));

        let trunc_fn = create_function(JsFunction::Native(NativeFunction {
            name: "trunc".to_string(),
            func: math_trunc,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("trunc"), JsValue::Object(trunc_fn));

        let sign_fn = create_function(JsFunction::Native(NativeFunction {
            name: "sign".to_string(),
            func: math_sign,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("sign"), JsValue::Object(sign_fn));

        let min_fn = create_function(JsFunction::Native(NativeFunction {
            name: "min".to_string(),
            func: math_min,
            arity: 2,
        }));
        math.set_property(PropertyKey::from("min"), JsValue::Object(min_fn));

        let max_fn = create_function(JsFunction::Native(NativeFunction {
            name: "max".to_string(),
            func: math_max,
            arity: 2,
        }));
        math.set_property(PropertyKey::from("max"), JsValue::Object(max_fn));

        let pow_fn = create_function(JsFunction::Native(NativeFunction {
            name: "pow".to_string(),
            func: math_pow,
            arity: 2,
        }));
        math.set_property(PropertyKey::from("pow"), JsValue::Object(pow_fn));

        let sqrt_fn = create_function(JsFunction::Native(NativeFunction {
            name: "sqrt".to_string(),
            func: math_sqrt,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("sqrt"), JsValue::Object(sqrt_fn));

        let log_fn = create_function(JsFunction::Native(NativeFunction {
            name: "log".to_string(),
            func: math_log,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("log"), JsValue::Object(log_fn));

        let exp_fn = create_function(JsFunction::Native(NativeFunction {
            name: "exp".to_string(),
            func: math_exp,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("exp"), JsValue::Object(exp_fn));

        let random_fn = create_function(JsFunction::Native(NativeFunction {
            name: "random".to_string(),
            func: math_random,
            arity: 0,
        }));
        math.set_property(PropertyKey::from("random"), JsValue::Object(random_fn));

        let sin_fn = create_function(JsFunction::Native(NativeFunction {
            name: "sin".to_string(),
            func: math_sin,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("sin"), JsValue::Object(sin_fn));

        let cos_fn = create_function(JsFunction::Native(NativeFunction {
            name: "cos".to_string(),
            func: math_cos,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("cos"), JsValue::Object(cos_fn));

        let tan_fn = create_function(JsFunction::Native(NativeFunction {
            name: "tan".to_string(),
            func: math_tan,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("tan"), JsValue::Object(tan_fn));

        let asin_fn = create_function(JsFunction::Native(NativeFunction {
            name: "asin".to_string(),
            func: math_asin,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("asin"), JsValue::Object(asin_fn));

        let acos_fn = create_function(JsFunction::Native(NativeFunction {
            name: "acos".to_string(),
            func: math_acos,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("acos"), JsValue::Object(acos_fn));

        let atan_fn = create_function(JsFunction::Native(NativeFunction {
            name: "atan".to_string(),
            func: math_atan,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("atan"), JsValue::Object(atan_fn));

        let atan2_fn = create_function(JsFunction::Native(NativeFunction {
            name: "atan2".to_string(),
            func: math_atan2,
            arity: 2,
        }));
        math.set_property(PropertyKey::from("atan2"), JsValue::Object(atan2_fn));

        let sinh_fn = create_function(JsFunction::Native(NativeFunction {
            name: "sinh".to_string(),
            func: math_sinh,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("sinh"), JsValue::Object(sinh_fn));

        let cosh_fn = create_function(JsFunction::Native(NativeFunction {
            name: "cosh".to_string(),
            func: math_cosh,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("cosh"), JsValue::Object(cosh_fn));

        let tanh_fn = create_function(JsFunction::Native(NativeFunction {
            name: "tanh".to_string(),
            func: math_tanh,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("tanh"), JsValue::Object(tanh_fn));

        let asinh_fn = create_function(JsFunction::Native(NativeFunction {
            name: "asinh".to_string(),
            func: math_asinh,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("asinh"), JsValue::Object(asinh_fn));

        let acosh_fn = create_function(JsFunction::Native(NativeFunction {
            name: "acosh".to_string(),
            func: math_acosh,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("acosh"), JsValue::Object(acosh_fn));

        let atanh_fn = create_function(JsFunction::Native(NativeFunction {
            name: "atanh".to_string(),
            func: math_atanh,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("atanh"), JsValue::Object(atanh_fn));

        let cbrt_fn = create_function(JsFunction::Native(NativeFunction {
            name: "cbrt".to_string(),
            func: math_cbrt,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("cbrt"), JsValue::Object(cbrt_fn));

        let hypot_fn = create_function(JsFunction::Native(NativeFunction {
            name: "hypot".to_string(),
            func: math_hypot,
            arity: 2,
        }));
        math.set_property(PropertyKey::from("hypot"), JsValue::Object(hypot_fn));

        let log10_fn = create_function(JsFunction::Native(NativeFunction {
            name: "log10".to_string(),
            func: math_log10,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("log10"), JsValue::Object(log10_fn));

        let log2_fn = create_function(JsFunction::Native(NativeFunction {
            name: "log2".to_string(),
            func: math_log2,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("log2"), JsValue::Object(log2_fn));

        let log1p_fn = create_function(JsFunction::Native(NativeFunction {
            name: "log1p".to_string(),
            func: math_log1p,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("log1p"), JsValue::Object(log1p_fn));

        let expm1_fn = create_function(JsFunction::Native(NativeFunction {
            name: "expm1".to_string(),
            func: math_expm1,
            arity: 1,
        }));
        math.set_property(PropertyKey::from("expm1"), JsValue::Object(expm1_fn));
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
