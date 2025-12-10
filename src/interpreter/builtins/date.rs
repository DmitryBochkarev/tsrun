//! Date built-in methods

use chrono::{Datelike, TimeZone, Timelike, Utc};

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, register_method, CheapClone, ExoticObject, JsFunction,
    JsObjectRef, JsString, JsValue, NativeFunction, PropertyKey,
};

/// Create a date from components, handling JavaScript-style overflow
/// (e.g., month 12 becomes January of next year, day 0 becomes last day of previous month)
fn make_date_from_components(
    year: i32,
    month: i32,
    day: i32,
    hours: u32,
    minutes: u32,
    seconds: u32,
    ms: u32,
) -> f64 {
    use chrono::Duration;

    // Handle 2-digit years (0-99 map to 1900-1999)
    let year = if (0..100).contains(&year) {
        year + 1900
    } else {
        year
    };

    // Normalize month (0-indexed, can overflow)
    let total_months = year * 12 + month;
    let norm_year = total_months.div_euclid(12);
    let norm_month = (total_months.rem_euclid(12) + 1) as u32; // 1-indexed for chrono

    // Create base date with day=1, then add (day-1) days to handle day overflow
    let Some(base_date) = Utc
        .with_ymd_and_hms(norm_year, norm_month, 1, hours, minutes, seconds)
        .single()
    else {
        return f64::NAN;
    };

    // Add (day - 1) days to handle overflow
    let adjusted_date = base_date + Duration::days((day - 1) as i64);
    adjusted_date.timestamp_millis() as f64 + ms as f64
}

/// Parse a date string in various formats, returning timestamp in milliseconds
fn parse_date_string(s: &str) -> f64 {
    // Try RFC3339 format (with timezone, e.g., "2024-12-25T10:30:00Z")
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp_millis() as f64;
    }

    // Try ISO 8601 without timezone (e.g., "2024-12-25T10:30:00") - treat as UTC
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return dt.and_utc().timestamp_millis() as f64;
    }

    // Try ISO 8601 with milliseconds but no timezone (e.g., "2024-12-25T10:30:00.000")
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return dt.and_utc().timestamp_millis() as f64;
    }

    // Try date-only format (e.g., "2024-12-25") - treat as midnight UTC
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return dt
            .and_hms_opt(0, 0, 0)
            .map(|d| d.and_utc().timestamp_millis() as f64)
            .unwrap_or(f64::NAN);
    }

    f64::NAN
}

/// Create Date.prototype with getTime, getFullYear, getMonth, etc.
pub fn create_date_prototype() -> JsObjectRef {
    let proto = create_object();
    {
        let mut p = proto.borrow_mut();

        // Getter methods (local time = UTC in our implementation)
        register_method(&mut p, "getTime", date_get_time, 0);
        register_method(&mut p, "getFullYear", date_get_full_year, 0);
        register_method(&mut p, "getMonth", date_get_month, 0);
        register_method(&mut p, "getDate", date_get_date, 0);
        register_method(&mut p, "getDay", date_get_day, 0);
        register_method(&mut p, "getHours", date_get_hours, 0);
        register_method(&mut p, "getMinutes", date_get_minutes, 0);
        register_method(&mut p, "getSeconds", date_get_seconds, 0);
        register_method(&mut p, "getMilliseconds", date_get_milliseconds, 0);

        // UTC getter methods (same as regular getters since we store UTC internally)
        register_method(&mut p, "getUTCFullYear", date_get_full_year, 0);
        register_method(&mut p, "getUTCMonth", date_get_month, 0);
        register_method(&mut p, "getUTCDate", date_get_date, 0);
        register_method(&mut p, "getUTCDay", date_get_day, 0);
        register_method(&mut p, "getUTCHours", date_get_hours, 0);
        register_method(&mut p, "getUTCMinutes", date_get_minutes, 0);
        register_method(&mut p, "getUTCSeconds", date_get_seconds, 0);
        register_method(&mut p, "getUTCMilliseconds", date_get_milliseconds, 0);

        // Setter methods
        register_method(&mut p, "setTime", date_set_time, 1);
        register_method(&mut p, "setFullYear", date_set_full_year, 3);
        register_method(&mut p, "setMonth", date_set_month, 2);
        register_method(&mut p, "setDate", date_set_date, 1);
        register_method(&mut p, "setHours", date_set_hours, 4);
        register_method(&mut p, "setMinutes", date_set_minutes, 3);
        register_method(&mut p, "setSeconds", date_set_seconds, 2);
        register_method(&mut p, "setMilliseconds", date_set_milliseconds, 1);

        // Conversion methods
        register_method(&mut p, "toISOString", date_to_iso_string, 0);
        register_method(&mut p, "toJSON", date_to_iso_string, 0); // toJSON = toISOString
        register_method(&mut p, "valueOf", date_get_time, 0); // valueOf = getTime
        register_method(&mut p, "toString", date_to_string, 0);
        register_method(&mut p, "toDateString", date_to_date_string, 0);
        register_method(&mut p, "toTimeString", date_to_time_string, 0);
    }
    proto
}

/// Create Date constructor with static methods (now, UTC, parse)
pub fn create_date_constructor(date_prototype: &JsObjectRef) -> JsObjectRef {
    let constructor = create_function(JsFunction::Native(NativeFunction {
        name: "Date".to_string(),
        func: date_constructor,
        arity: 0,
    }));
    {
        let mut date = constructor.borrow_mut();

        register_method(&mut date, "now", date_now, 0);
        register_method(&mut date, "UTC", date_utc, 7);
        register_method(&mut date, "parse", date_parse, 1);

        date.set_property(
            PropertyKey::from("prototype"),
            JsValue::Object(date_prototype.cheap_clone()),
        );
    }
    constructor
}

pub fn date_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let timestamp = if args.is_empty() {
        // new Date() - current time
        Utc::now().timestamp_millis() as f64
    } else if args.len() == 1 {
        match args.first() {
            Some(JsValue::Number(n)) => *n,
            Some(JsValue::String(s)) => {
                // Parse date string - try multiple formats
                parse_date_string(s.as_ref())
            }
            _ => f64::NAN,
        }
    } else {
        // new Date(year, month, day?, hours?, minutes?, seconds?, ms?)
        let year = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN) as i32;
        let month = args.get(1).map(|v| v.to_number()).unwrap_or(0.0) as i32;
        let day = args.get(2).map(|v| v.to_number()).unwrap_or(1.0) as i32;
        let hours = args.get(3).map(|v| v.to_number()).unwrap_or(0.0) as u32;
        let minutes = args.get(4).map(|v| v.to_number()).unwrap_or(0.0) as u32;
        let seconds = args.get(5).map(|v| v.to_number()).unwrap_or(0.0) as u32;
        let ms = args.get(6).map(|v| v.to_number()).unwrap_or(0.0) as u32;

        make_date_from_components(year, month, day, hours, minutes, seconds, ms)
    };

    let date_obj = create_object();
    {
        let mut obj = date_obj.borrow_mut();
        obj.exotic = ExoticObject::Date { timestamp };
        obj.prototype = Some(interp.date_prototype.cheap_clone());
    }
    Ok(JsValue::Object(date_obj))
}

pub fn date_now(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    Ok(JsValue::Number(Utc::now().timestamp_millis() as f64))
}

pub fn date_utc(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let year = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN) as i32;
    let month = args.get(1).map(|v| v.to_number()).unwrap_or(0.0) as i32;
    let day = args.get(2).map(|v| v.to_number()).unwrap_or(1.0) as i32;
    let hours = args.get(3).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let minutes = args.get(4).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let seconds = args.get(5).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let ms = args.get(6).map(|v| v.to_number()).unwrap_or(0.0) as u32;

    let timestamp = make_date_from_components(year, month, day, hours, minutes, seconds, ms);
    Ok(JsValue::Number(timestamp))
}

pub fn date_parse(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let s = args
        .first()
        .cloned()
        .unwrap_or(JsValue::Undefined)
        .to_js_string()
        .to_string();
    let timestamp = parse_date_string(&s);
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

pub fn date_get_time(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    Ok(JsValue::Number(get_date_timestamp(&this)?))
}

pub fn date_get_full_year(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.year() as f64))
}

pub fn date_get_month(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number((dt.month() - 1) as f64)) // 0-indexed
}

pub fn date_get_date(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.day() as f64))
}

pub fn date_get_day(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.weekday().num_days_from_sunday() as f64))
}

pub fn date_get_hours(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.hour() as f64))
}

pub fn date_get_minutes(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.minute() as f64))
}

pub fn date_get_seconds(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::Number(dt.second() as f64))
}

pub fn date_get_milliseconds(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }
    Ok(JsValue::Number((ts as i64 % 1000) as f64))
}

pub fn date_to_iso_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Err(JsError::range_error("Invalid Date"));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(JsValue::String(JsString::from(
        dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
    )))
}

// Setter methods - they modify the date and return the new timestamp

fn set_date_timestamp(this: &JsValue, new_ts: f64) -> Result<f64, JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error("this is not a Date"));
    };
    let mut obj_ref = obj.borrow_mut();
    if let ExoticObject::Date { ref mut timestamp } = obj_ref.exotic {
        *timestamp = new_ts;
        Ok(new_ts)
    } else {
        Err(JsError::type_error("this is not a Date"))
    }
}

pub fn date_set_time(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let new_time = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let ts = set_date_timestamp(&this, new_time)?;
    Ok(JsValue::Number(ts))
}

pub fn date_set_full_year(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }

    let dt = chrono::DateTime::from_timestamp_millis(current_ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);

    let new_year = args
        .first()
        .map(|v| v.to_number() as i32)
        .unwrap_or(dt.year());
    let new_month = args
        .get(1)
        .map(|v| v.to_number() as u32 + 1)
        .unwrap_or(dt.month());
    let new_day = args
        .get(2)
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.day());

    let new_dt = Utc
        .with_ymd_and_hms(
            new_year,
            new_month,
            new_day,
            dt.hour(),
            dt.minute(),
            dt.second(),
        )
        .single()
        .map(|d| d.timestamp_millis() as f64 + (current_ts as i64 % 1000) as f64)
        .unwrap_or(f64::NAN);

    let ts = set_date_timestamp(&this, new_dt)?;
    Ok(JsValue::Number(ts))
}

pub fn date_set_month(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }

    let dt = chrono::DateTime::from_timestamp_millis(current_ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);

    let new_month = args
        .first()
        .map(|v| v.to_number() as u32 + 1)
        .unwrap_or(dt.month());
    let new_day = args
        .get(1)
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.day());

    let new_dt = Utc
        .with_ymd_and_hms(
            dt.year(),
            new_month,
            new_day,
            dt.hour(),
            dt.minute(),
            dt.second(),
        )
        .single()
        .map(|d| d.timestamp_millis() as f64 + (current_ts as i64 % 1000) as f64)
        .unwrap_or(f64::NAN);

    let ts = set_date_timestamp(&this, new_dt)?;
    Ok(JsValue::Number(ts))
}

pub fn date_set_date(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }

    let dt = chrono::DateTime::from_timestamp_millis(current_ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);

    let new_day = args
        .first()
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.day());

    let new_dt = Utc
        .with_ymd_and_hms(
            dt.year(),
            dt.month(),
            new_day,
            dt.hour(),
            dt.minute(),
            dt.second(),
        )
        .single()
        .map(|d| d.timestamp_millis() as f64 + (current_ts as i64 % 1000) as f64)
        .unwrap_or(f64::NAN);

    let ts = set_date_timestamp(&this, new_dt)?;
    Ok(JsValue::Number(ts))
}

pub fn date_set_hours(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }

    let dt = chrono::DateTime::from_timestamp_millis(current_ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);

    let new_hour = args
        .first()
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.hour());
    let new_min = args
        .get(1)
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.minute());
    let new_sec = args
        .get(2)
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.second());
    let new_ms = args
        .get(3)
        .map(|v| v.to_number() as i64)
        .unwrap_or((current_ts as i64) % 1000);

    let new_dt = Utc
        .with_ymd_and_hms(dt.year(), dt.month(), dt.day(), new_hour, new_min, new_sec)
        .single()
        .map(|d| d.timestamp_millis() as f64 + new_ms as f64)
        .unwrap_or(f64::NAN);

    let ts = set_date_timestamp(&this, new_dt)?;
    Ok(JsValue::Number(ts))
}

pub fn date_set_minutes(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }

    let dt = chrono::DateTime::from_timestamp_millis(current_ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);

    let new_min = args
        .first()
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.minute());
    let new_sec = args
        .get(1)
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.second());
    let new_ms = args
        .get(2)
        .map(|v| v.to_number() as i64)
        .unwrap_or((current_ts as i64) % 1000);

    let new_dt = Utc
        .with_ymd_and_hms(dt.year(), dt.month(), dt.day(), dt.hour(), new_min, new_sec)
        .single()
        .map(|d| d.timestamp_millis() as f64 + new_ms as f64)
        .unwrap_or(f64::NAN);

    let ts = set_date_timestamp(&this, new_dt)?;
    Ok(JsValue::Number(ts))
}

pub fn date_set_seconds(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }

    let dt = chrono::DateTime::from_timestamp_millis(current_ts as i64)
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);

    let new_sec = args
        .first()
        .map(|v| v.to_number() as u32)
        .unwrap_or(dt.second());
    let new_ms = args
        .get(1)
        .map(|v| v.to_number() as i64)
        .unwrap_or((current_ts as i64) % 1000);

    let new_dt = Utc
        .with_ymd_and_hms(
            dt.year(),
            dt.month(),
            dt.day(),
            dt.hour(),
            dt.minute(),
            new_sec,
        )
        .single()
        .map(|d| d.timestamp_millis() as f64 + new_ms as f64)
        .unwrap_or(f64::NAN);

    let ts = set_date_timestamp(&this, new_dt)?;
    Ok(JsValue::Number(ts))
}

pub fn date_set_milliseconds(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(JsValue::Number(f64::NAN));
    }

    let new_ms = args
        .first()
        .map(|v| v.to_number() as i64)
        .unwrap_or((current_ts as i64) % 1000);

    // Keep the same time, just change milliseconds
    let base_ts = (current_ts as i64 / 1000) * 1000; // Round down to seconds
    let new_ts = base_ts as f64 + new_ms as f64;

    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(JsValue::Number(ts))
}

/// Date.prototype.toString()
/// Returns a string like "Thu Jan 01 1970 00:00:00 GMT+0000 (UTC)"
pub fn date_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::String(JsString::from("Invalid Date")));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    // Format like "Thu Jan 01 1970 00:00:00 GMT+0000 (UTC)"
    let formatted = dt.format("%a %b %d %Y %H:%M:%S GMT+0000 (UTC)").to_string();
    Ok(JsValue::String(JsString::from(formatted)))
}

/// Date.prototype.toDateString()
/// Returns the date part like "Thu Jan 01 1970"
pub fn date_to_date_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::String(JsString::from("Invalid Date")));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    // Format like "Thu Jan 01 1970"
    let formatted = dt.format("%a %b %d %Y").to_string();
    Ok(JsValue::String(JsString::from(formatted)))
}

/// Date.prototype.toTimeString()
/// Returns the time part like "00:00:00 GMT+0000 (UTC)"
pub fn date_to_time_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(JsValue::String(JsString::from("Invalid Date")));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    // Format like "00:00:00 GMT+0000 (UTC)"
    let formatted = dt.format("%H:%M:%S GMT+0000 (UTC)").to_string();
    Ok(JsValue::String(JsString::from(formatted)))
}
