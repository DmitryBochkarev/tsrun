//! Date built-in methods

use chrono::{Datelike, Timelike, TimeZone, Utc};

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{create_object, ExoticObject, JsString, JsValue};

pub fn date_constructor(interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let timestamp = if args.is_empty() {
        // new Date() - current time
        Utc::now().timestamp_millis() as f64
    } else if args.len() == 1 {
        match &args[0] {
            JsValue::Number(n) => *n,
            JsValue::String(s) => {
                // Parse date string
                chrono::DateTime::parse_from_rfc3339(&s.to_string())
                    .map(|dt| dt.timestamp_millis() as f64)
                    .unwrap_or(f64::NAN)
            }
            _ => f64::NAN,
        }
    } else {
        // new Date(year, month, day?, hours?, minutes?, seconds?, ms?)
        let year = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN) as i32;
        let month = args.get(1).map(|v| v.to_number()).unwrap_or(0.0) as u32;
        let day = args.get(2).map(|v| v.to_number()).unwrap_or(1.0) as u32;
        let hours = args.get(3).map(|v| v.to_number()).unwrap_or(0.0) as u32;
        let minutes = args.get(4).map(|v| v.to_number()).unwrap_or(0.0) as u32;
        let seconds = args.get(5).map(|v| v.to_number()).unwrap_or(0.0) as u32;
        let ms = args.get(6).map(|v| v.to_number()).unwrap_or(0.0) as u32;

        // Handle 2-digit years (0-99 map to 1900-1999)
        let year = if (0..100).contains(&year) { year + 1900 } else { year };

        Utc.with_ymd_and_hms(year, month + 1, day, hours, minutes, seconds)
            .single()
            .map(|dt| dt.timestamp_millis() as f64 + ms as f64)
            .unwrap_or(f64::NAN)
    };

    let date_obj = create_object();
    {
        let mut obj = date_obj.borrow_mut();
        obj.exotic = ExoticObject::Date { timestamp };
        obj.prototype = Some(interp.date_prototype.clone());
    }
    Ok(JsValue::Object(date_obj))
}

pub fn date_now(_interp: &mut Interpreter, _this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    Ok(JsValue::Number(Utc::now().timestamp_millis() as f64))
}

pub fn date_utc(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let year = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN) as i32;
    let month = args.get(1).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let day = args.get(2).map(|v| v.to_number()).unwrap_or(1.0) as u32;
    let hours = args.get(3).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let minutes = args.get(4).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let seconds = args.get(5).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let ms = args.get(6).map(|v| v.to_number()).unwrap_or(0.0) as u32;

    // Handle 2-digit years (0-99 map to 1900-1999)
    let year = if (0..100).contains(&year) { year + 1900 } else { year };

    let timestamp = Utc.with_ymd_and_hms(year, month + 1, day, hours, minutes, seconds)
        .single()
        .map(|dt| dt.timestamp_millis() as f64 + ms as f64)
        .unwrap_or(f64::NAN);

    Ok(JsValue::Number(timestamp))
}

pub fn date_parse(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = args.first().cloned().unwrap_or(JsValue::Undefined).to_js_string().to_string();
    let timestamp = chrono::DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.timestamp_millis() as f64)
        .unwrap_or(f64::NAN);
    Ok(JsValue::Number(timestamp))
}

fn get_date_timestamp(this: &JsValue) -> Result<f64, JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error("this is not a Date"));
    };
    let obj_ref = obj.borrow();
    if let ExoticObject::Date { timestamp } = obj_ref.exotic {
        Ok(timestamp)
    } else {
        Err(JsError::type_error("this is not a Date"))
    }
}

pub fn date_get_time(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    Ok(JsValue::Number(get_date_timestamp(&this)?))
}

pub fn date_get_full_year(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.year() as f64))
}

pub fn date_get_month(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number((dt.month() - 1) as f64)) // 0-indexed
}

pub fn date_get_date(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.day() as f64))
}

pub fn date_get_day(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.weekday().num_days_from_sunday() as f64))
}

pub fn date_get_hours(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.hour() as f64))
}

pub fn date_get_minutes(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.minute() as f64))
}

pub fn date_get_seconds(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.second() as f64))
}

pub fn date_get_milliseconds(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    Ok(JsValue::Number((ts as i64 % 1000) as f64))
}

pub fn date_to_iso_string(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Err(JsError::range_error("Invalid Date"));
    }
    let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::String(JsString::from(dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())))
}
