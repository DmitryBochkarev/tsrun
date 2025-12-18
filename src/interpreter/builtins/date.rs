//! Date built-in methods

use chrono::{Datelike, TimeZone, Timelike, Utc};

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{ExoticObject, Guarded, JsString, JsValue};

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

/// Initialize Date.prototype with getTime, getFullYear, getMonth, etc.
pub fn init_date_prototype(interp: &mut Interpreter) {
    let proto = interp.date_prototype.clone();

    // Getter methods (local time = UTC in our implementation)
    interp.register_method(&proto, "getTime", date_get_time, 0);
    interp.register_method(&proto, "getFullYear", date_get_full_year, 0);
    interp.register_method(&proto, "getMonth", date_get_month, 0);
    interp.register_method(&proto, "getDate", date_get_date, 0);
    interp.register_method(&proto, "getDay", date_get_day, 0);
    interp.register_method(&proto, "getHours", date_get_hours, 0);
    interp.register_method(&proto, "getMinutes", date_get_minutes, 0);
    interp.register_method(&proto, "getSeconds", date_get_seconds, 0);
    interp.register_method(&proto, "getMilliseconds", date_get_milliseconds, 0);

    // UTC getter methods (same as regular getters since we store UTC internally)
    interp.register_method(&proto, "getUTCFullYear", date_get_full_year, 0);
    interp.register_method(&proto, "getUTCMonth", date_get_month, 0);
    interp.register_method(&proto, "getUTCDate", date_get_date, 0);
    interp.register_method(&proto, "getUTCDay", date_get_day, 0);
    interp.register_method(&proto, "getUTCHours", date_get_hours, 0);
    interp.register_method(&proto, "getUTCMinutes", date_get_minutes, 0);
    interp.register_method(&proto, "getUTCSeconds", date_get_seconds, 0);
    interp.register_method(&proto, "getUTCMilliseconds", date_get_milliseconds, 0);

    // Setter methods
    interp.register_method(&proto, "setTime", date_set_time, 1);
    interp.register_method(&proto, "setFullYear", date_set_full_year, 3);
    interp.register_method(&proto, "setMonth", date_set_month, 2);
    interp.register_method(&proto, "setDate", date_set_date, 1);
    interp.register_method(&proto, "setHours", date_set_hours, 4);
    interp.register_method(&proto, "setMinutes", date_set_minutes, 3);
    interp.register_method(&proto, "setSeconds", date_set_seconds, 2);
    interp.register_method(&proto, "setMilliseconds", date_set_milliseconds, 1);

    // Conversion methods
    interp.register_method(&proto, "toISOString", date_to_iso_string, 0);
    interp.register_method(&proto, "toJSON", date_to_iso_string, 0); // toJSON = toISOString
    interp.register_method(&proto, "valueOf", date_get_time, 0); // valueOf = getTime
    interp.register_method(&proto, "toString", date_to_string, 0);
    interp.register_method(&proto, "toDateString", date_to_date_string, 0);
    interp.register_method(&proto, "toTimeString", date_to_time_string, 0);
}

/// Create Date constructor and register it globally
pub fn init_date(interp: &mut Interpreter) {
    init_date_prototype(interp);

    let constructor = interp.create_native_function("Date", date_constructor, 0);
    interp.root_guard.guard(constructor.clone());

    // Add static methods
    interp.register_method(&constructor, "now", date_now, 0);
    interp.register_method(&constructor, "UTC", date_utc, 7);
    interp.register_method(&constructor, "parse", date_parse, 1);

    // Set prototype property on constructor
    let proto_key = interp.key("prototype");
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.date_prototype.clone()));

    // Register globally
    let date_key = interp.key("Date");
    interp
        .global
        .borrow_mut()
        .set_property(date_key, JsValue::Object(constructor));
}

pub fn date_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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

    let guard = interp.heap.create_guard();
    let date_obj = interp.create_object(&guard);
    {
        let mut obj = date_obj.borrow_mut();
        obj.exotic = ExoticObject::Date { timestamp };
        obj.prototype = Some(interp.date_prototype.clone());
    }

    Ok(Guarded::with_guard(JsValue::Object(date_obj), guard))
}

pub fn date_now(
    _interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    Ok(Guarded::unguarded(JsValue::Number(
        Utc::now().timestamp_millis() as f64,
    )))
}

pub fn date_utc(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let year = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN) as i32;
    let month = args.get(1).map(|v| v.to_number()).unwrap_or(0.0) as i32;
    let day = args.get(2).map(|v| v.to_number()).unwrap_or(1.0) as i32;
    let hours = args.get(3).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let minutes = args.get(4).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let seconds = args.get(5).map(|v| v.to_number()).unwrap_or(0.0) as u32;
    let ms = args.get(6).map(|v| v.to_number()).unwrap_or(0.0) as u32;

    let timestamp = make_date_from_components(year, month, day, hours, minutes, seconds, ms);
    Ok(Guarded::unguarded(JsValue::Number(timestamp)))
}

pub fn date_parse(
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
    let timestamp = parse_date_string(&s);
    Ok(Guarded::unguarded(JsValue::Number(timestamp)))
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
) -> Result<Guarded, JsError> {
    Ok(Guarded::unguarded(JsValue::Number(get_date_timestamp(
        &this,
    )?)))
}

pub fn date_get_full_year(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(Guarded::unguarded(JsValue::Number(dt.year() as f64)))
}

pub fn date_get_month(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(Guarded::unguarded(JsValue::Number((dt.month() - 1) as f64))) // 0-indexed
}

pub fn date_get_date(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(Guarded::unguarded(JsValue::Number(dt.day() as f64)))
}

pub fn date_get_day(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(Guarded::unguarded(JsValue::Number(
        dt.weekday().num_days_from_sunday() as f64,
    )))
}

pub fn date_get_hours(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(Guarded::unguarded(JsValue::Number(dt.hour() as f64)))
}

pub fn date_get_minutes(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(Guarded::unguarded(JsValue::Number(dt.minute() as f64)))
}

pub fn date_get_seconds(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(Guarded::unguarded(JsValue::Number(dt.second() as f64)))
}

pub fn date_get_milliseconds(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }
    Ok(Guarded::unguarded(JsValue::Number(
        (ts as i64 % 1000) as f64,
    )))
}

pub fn date_to_iso_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Err(JsError::range_error("Invalid Date"));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
    ))))
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
) -> Result<Guarded, JsError> {
    let new_time = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let ts = set_date_timestamp(&this, new_time)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_full_year(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
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
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_month(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
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
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_date(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
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
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_hours(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
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
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_minutes(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
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
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_seconds(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
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
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_milliseconds(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    if current_ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    }

    let new_ms = args
        .first()
        .map(|v| v.to_number() as i64)
        .unwrap_or((current_ts as i64) % 1000);

    // Keep the same time, just change milliseconds
    let base_ts = (current_ts as i64 / 1000) * 1000; // Round down to seconds
    let new_ts = base_ts as f64 + new_ms as f64;

    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

/// Date.prototype.toString()
/// Returns a string like "Thu Jan 01 1970 00:00:00 GMT+0000 (UTC)"
pub fn date_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            "Invalid Date",
        ))));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    // Format like "Thu Jan 01 1970 00:00:00 GMT+0000 (UTC)"
    let formatted = dt.format("%a %b %d %Y %H:%M:%S GMT+0000 (UTC)").to_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        formatted,
    ))))
}

/// Date.prototype.toDateString()
/// Returns the date part like "Thu Jan 01 1970"
pub fn date_to_date_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            "Invalid Date",
        ))));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    // Format like "Thu Jan 01 1970"
    let formatted = dt.format("%a %b %d %Y").to_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        formatted,
    ))))
}

/// Date.prototype.toTimeString()
/// Returns the time part like "00:00:00 GMT+0000 (UTC)"
pub fn date_to_time_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    if ts.is_nan() {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            "Invalid Date",
        ))));
    }
    let dt =
        chrono::DateTime::from_timestamp_millis(ts as i64).unwrap_or(chrono::DateTime::UNIX_EPOCH);
    // Format like "00:00:00 GMT+0000 (UTC)"
    let formatted = dt.format("%H:%M:%S GMT+0000 (UTC)").to_string();
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        formatted,
    ))))
}
