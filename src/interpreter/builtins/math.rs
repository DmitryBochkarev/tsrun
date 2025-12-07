//! Math built-in methods

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::JsValue;

pub fn math_abs(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.abs()))
}

pub fn math_floor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.floor()))
}

pub fn math_ceil(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.ceil()))
}

pub fn math_round(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.round()))
}

pub fn math_trunc(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.trunc()))
}

pub fn math_sign(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
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

pub fn math_min(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
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

pub fn math_max(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
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

pub fn math_pow(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let base = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let exp = args.get(1).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(base.powf(exp)))
}

pub fn math_sqrt(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sqrt()))
}

pub fn math_log(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.ln()))
}

pub fn math_exp(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.exp()))
}

pub fn math_random(_interp: &mut Interpreter, _this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple pseudo-random using system time (not cryptographically secure)
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as f64;
    let random = (seed / 1_000_000_000.0) % 1.0;
    Ok(JsValue::Number(random))
}

pub fn math_sin(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sin()))
}

pub fn math_cos(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.cos()))
}

pub fn math_tan(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.tan()))
}

pub fn math_asin(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.asin()))
}

pub fn math_acos(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.acos()))
}

pub fn math_atan(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.atan()))
}

pub fn math_atan2(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let y = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let x = args.get(1).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(y.atan2(x)))
}

pub fn math_sinh(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sinh()))
}

pub fn math_cosh(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.cosh()))
}

pub fn math_tanh(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.tanh()))
}

pub fn math_asinh(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.asinh()))
}

pub fn math_acosh(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.acosh()))
}

pub fn math_atanh(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.atanh()))
}

pub fn math_cbrt(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.cbrt()))
}

pub fn math_hypot(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    if args.is_empty() {
        return Ok(JsValue::Number(0.0));
    }
    let sum_sq: f64 = args.iter().map(|v| {
        let n = v.to_number();
        n * n
    }).sum();
    Ok(JsValue::Number(sum_sq.sqrt()))
}

pub fn math_log10(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.log10()))
}

pub fn math_log2(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.log2()))
}

pub fn math_log1p(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.ln_1p()))
}

pub fn math_expm1(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.exp_m1()))
}
