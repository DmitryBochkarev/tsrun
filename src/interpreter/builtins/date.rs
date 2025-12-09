//! Date built-in methods

use chrono::{Datelike, TimeZone, Timelike, Utc};

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, CheapClone, ExoticObject, JsFunction, JsObjectRef, JsString,
    JsValue, NativeFunction, PropertyKey,
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

        let get_time_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getTime".to_string(),
            func: date_get_time,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("getTime"), JsValue::Object(get_time_fn));

        let get_full_year_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getFullYear".to_string(),
            func: date_get_full_year,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getFullYear"),
            JsValue::Object(get_full_year_fn),
        );

        let get_month_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getMonth".to_string(),
            func: date_get_month,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("getMonth"), JsValue::Object(get_month_fn));

        let get_date_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getDate".to_string(),
            func: date_get_date,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("getDate"), JsValue::Object(get_date_fn));

        let get_day_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getDay".to_string(),
            func: date_get_day,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("getDay"), JsValue::Object(get_day_fn));

        let get_hours_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getHours".to_string(),
            func: date_get_hours,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("getHours"), JsValue::Object(get_hours_fn));

        let get_minutes_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getMinutes".to_string(),
            func: date_get_minutes,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getMinutes"),
            JsValue::Object(get_minutes_fn),
        );

        let get_seconds_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getSeconds".to_string(),
            func: date_get_seconds,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getSeconds"),
            JsValue::Object(get_seconds_fn),
        );

        let get_milliseconds_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getMilliseconds".to_string(),
            func: date_get_milliseconds,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getMilliseconds"),
            JsValue::Object(get_milliseconds_fn),
        );

        // UTC getter methods (same as regular getters since we store UTC internally)
        let get_utc_full_year_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getUTCFullYear".to_string(),
            func: date_get_full_year,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getUTCFullYear"),
            JsValue::Object(get_utc_full_year_fn),
        );

        let get_utc_month_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getUTCMonth".to_string(),
            func: date_get_month,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getUTCMonth"),
            JsValue::Object(get_utc_month_fn),
        );

        let get_utc_date_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getUTCDate".to_string(),
            func: date_get_date,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getUTCDate"),
            JsValue::Object(get_utc_date_fn),
        );

        let get_utc_day_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getUTCDay".to_string(),
            func: date_get_day,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getUTCDay"),
            JsValue::Object(get_utc_day_fn),
        );

        let get_utc_hours_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getUTCHours".to_string(),
            func: date_get_hours,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getUTCHours"),
            JsValue::Object(get_utc_hours_fn),
        );

        let get_utc_minutes_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getUTCMinutes".to_string(),
            func: date_get_minutes,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getUTCMinutes"),
            JsValue::Object(get_utc_minutes_fn),
        );

        let get_utc_seconds_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getUTCSeconds".to_string(),
            func: date_get_seconds,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getUTCSeconds"),
            JsValue::Object(get_utc_seconds_fn),
        );

        let get_utc_milliseconds_fn = create_function(JsFunction::Native(NativeFunction {
            name: "getUTCMilliseconds".to_string(),
            func: date_get_milliseconds,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("getUTCMilliseconds"),
            JsValue::Object(get_utc_milliseconds_fn),
        );

        let to_iso_string_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toISOString".to_string(),
            func: date_to_iso_string,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("toISOString"),
            JsValue::Object(to_iso_string_fn),
        );

        let to_json_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toJSON".to_string(),
            func: date_to_iso_string, // toJSON returns the same as toISOString
            arity: 0,
        }));
        p.set_property(PropertyKey::from("toJSON"), JsValue::Object(to_json_fn));

        let value_of_fn = create_function(JsFunction::Native(NativeFunction {
            name: "valueOf".to_string(),
            func: date_get_time, // valueOf returns the same as getTime
            arity: 0,
        }));
        p.set_property(PropertyKey::from("valueOf"), JsValue::Object(value_of_fn));

        // Setter methods
        let set_time_fn = create_function(JsFunction::Native(NativeFunction {
            name: "setTime".to_string(),
            func: date_set_time,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("setTime"), JsValue::Object(set_time_fn));

        let set_full_year_fn = create_function(JsFunction::Native(NativeFunction {
            name: "setFullYear".to_string(),
            func: date_set_full_year,
            arity: 3,
        }));
        p.set_property(
            PropertyKey::from("setFullYear"),
            JsValue::Object(set_full_year_fn),
        );

        let set_month_fn = create_function(JsFunction::Native(NativeFunction {
            name: "setMonth".to_string(),
            func: date_set_month,
            arity: 2,
        }));
        p.set_property(PropertyKey::from("setMonth"), JsValue::Object(set_month_fn));

        let set_date_fn = create_function(JsFunction::Native(NativeFunction {
            name: "setDate".to_string(),
            func: date_set_date,
            arity: 1,
        }));
        p.set_property(PropertyKey::from("setDate"), JsValue::Object(set_date_fn));

        let set_hours_fn = create_function(JsFunction::Native(NativeFunction {
            name: "setHours".to_string(),
            func: date_set_hours,
            arity: 4,
        }));
        p.set_property(PropertyKey::from("setHours"), JsValue::Object(set_hours_fn));

        let set_minutes_fn = create_function(JsFunction::Native(NativeFunction {
            name: "setMinutes".to_string(),
            func: date_set_minutes,
            arity: 3,
        }));
        p.set_property(
            PropertyKey::from("setMinutes"),
            JsValue::Object(set_minutes_fn),
        );

        let set_seconds_fn = create_function(JsFunction::Native(NativeFunction {
            name: "setSeconds".to_string(),
            func: date_set_seconds,
            arity: 2,
        }));
        p.set_property(
            PropertyKey::from("setSeconds"),
            JsValue::Object(set_seconds_fn),
        );

        let set_milliseconds_fn = create_function(JsFunction::Native(NativeFunction {
            name: "setMilliseconds".to_string(),
            func: date_set_milliseconds,
            arity: 1,
        }));
        p.set_property(
            PropertyKey::from("setMilliseconds"),
            JsValue::Object(set_milliseconds_fn),
        );

        // toString methods
        let to_string_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toString".to_string(),
            func: date_to_string,
            arity: 0,
        }));
        p.set_property(PropertyKey::from("toString"), JsValue::Object(to_string_fn));

        let to_date_string_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toDateString".to_string(),
            func: date_to_date_string,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("toDateString"),
            JsValue::Object(to_date_string_fn),
        );

        let to_time_string_fn = create_function(JsFunction::Native(NativeFunction {
            name: "toTimeString".to_string(),
            func: date_to_time_string,
            arity: 0,
        }));
        p.set_property(
            PropertyKey::from("toTimeString"),
            JsValue::Object(to_time_string_fn),
        );
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

        let now_fn = create_function(JsFunction::Native(NativeFunction {
            name: "now".to_string(),
            func: date_now,
            arity: 0,
        }));
        date.set_property(PropertyKey::from("now"), JsValue::Object(now_fn));

        let utc_fn = create_function(JsFunction::Native(NativeFunction {
            name: "UTC".to_string(),
            func: date_utc,
            arity: 7,
        }));
        date.set_property(PropertyKey::from("UTC"), JsValue::Object(utc_fn));

        let parse_fn = create_function(JsFunction::Native(NativeFunction {
            name: "parse".to_string(),
            func: date_parse,
            arity: 1,
        }));
        date.set_property(PropertyKey::from("parse"), JsValue::Object(parse_fn));

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
