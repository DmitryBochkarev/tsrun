//! Date built-in methods
//!
//! Implements JavaScript Date using platform-provided time and manual calendar calculations.

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::prelude::*;
use crate::value::{ExoticObject, Guarded, JsString, JsValue, PropertyKey};

const MS_PER_SECOND: i64 = 1000;
const MS_PER_MINUTE: i64 = 60 * MS_PER_SECOND;
const MS_PER_HOUR: i64 = 60 * MS_PER_MINUTE;
const MS_PER_DAY: i64 = 24 * MS_PER_HOUR;

/// Convert days since Unix epoch to (year, month, day)
/// Uses the algorithm from Howard Hinnant's date library
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

/// Convert (year, month, day) to days since Unix epoch
fn ymd_to_days(year: i32, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let m = if month <= 2 { month + 12 } else { month };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (m - 3) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

/// Get weekday from days since Unix epoch (0 = Sunday)
fn days_to_weekday(days: i64) -> u32 {
    // Jan 1, 1970 was Thursday (day 4)
    ((days % 7 + 4 + 7) % 7) as u32
}

/// Check if year is a leap year
fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Get days in a month
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Date components extracted from a timestamp
struct DateComponents {
    year: i32,
    month: u32, // 1-12
    day: u32,   // 1-31
    hour: u32,
    minute: u32,
    second: u32,
    ms: u32,
    weekday: u32, // 0 = Sunday
}

/// Convert timestamp (ms since epoch) to date components
fn ts_to_components(ts: f64) -> Option<DateComponents> {
    if ts.is_nan() || ts.is_infinite() {
        return None;
    }
    let ts = ts as i64;
    let days = ts.div_euclid(MS_PER_DAY);
    let time_of_day = ts.rem_euclid(MS_PER_DAY);

    let (year, month, day) = days_to_ymd(days);
    let hour = (time_of_day / MS_PER_HOUR) as u32;
    let minute = ((time_of_day % MS_PER_HOUR) / MS_PER_MINUTE) as u32;
    let second = ((time_of_day % MS_PER_MINUTE) / MS_PER_SECOND) as u32;
    let ms = (time_of_day % MS_PER_SECOND) as u32;
    let weekday = days_to_weekday(days);

    Some(DateComponents {
        year,
        month,
        day,
        hour,
        minute,
        second,
        ms,
        weekday,
    })
}

/// Convert date components to timestamp (ms since epoch)
fn components_to_ts(
    year: i32,
    month: i32,
    day: i32,
    hour: u32,
    minute: u32,
    second: u32,
    ms: u32,
) -> f64 {
    // Handle 2-digit years (0-99 map to 1900-1999)
    let year = if (0..100).contains(&year) {
        year + 1900
    } else {
        year
    };

    // Normalize month (0-indexed from JS, can overflow)
    let total_months = year as i64 * 12 + month as i64;
    let norm_year = total_months.div_euclid(12) as i32;
    let norm_month = (total_months.rem_euclid(12) + 1) as u32;

    // Calculate days, allowing day overflow
    let base_days = ymd_to_days(norm_year, norm_month, 1);
    let total_days = base_days + (day - 1) as i64;

    let time_ms = hour as i64 * MS_PER_HOUR
        + minute as i64 * MS_PER_MINUTE
        + second as i64 * MS_PER_SECOND
        + ms as i64;

    (total_days * MS_PER_DAY + time_ms) as f64
}

const WEEKDAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Format a timestamp for Date.prototype.toString()
fn format_date_for_tostring(ts: f64) -> String {
    let Some(c) = ts_to_components(ts) else {
        return "Invalid Date".to_string();
    };
    format!(
        "{} {} {:02} {:04} {:02}:{:02}:{:02} GMT+0000 (UTC)",
        WEEKDAY_NAMES
            .get(c.weekday as usize)
            .copied()
            .unwrap_or("???"),
        MONTH_NAMES
            .get((c.month - 1) as usize)
            .copied()
            .unwrap_or("???"),
        c.day,
        c.year,
        c.hour,
        c.minute,
        c.second
    )
}

/// Parse a date string in various formats, returning timestamp in milliseconds
fn parse_date_string(s: &str) -> f64 {
    let s = s.trim();

    // Try ISO 8601 formats: "2024-12-25T10:30:00Z", "2024-12-25T10:30:00", "2024-12-25"
    if let Some(ts) = parse_iso8601(s) {
        return ts;
    }

    f64::NAN
}

/// Parse ISO 8601 date format
fn parse_iso8601(s: &str) -> Option<f64> {
    // Split date and time parts
    let (date_part, time_part) = if let Some(t_pos) = s.find('T') {
        (
            s.get(..t_pos)?,
            Some(s.get(t_pos + 1..)?.trim_end_matches('Z')),
        )
    } else {
        (s, None)
    };

    // Parse date: YYYY-MM-DD or YYYY-MM or YYYY
    let date_parts: Vec<&str> = date_part.split('-').collect();
    let year: i32 = date_parts.first()?.parse().ok()?;
    let month: u32 = date_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let day: u32 = date_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

    // Validate date
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }

    let (hour, minute, second, ms) = if let Some(time) = time_part {
        parse_time(time)?
    } else {
        (0, 0, 0, 0)
    };

    Some(components_to_ts(
        year,
        (month - 1) as i32,
        day as i32,
        hour,
        minute,
        second,
        ms,
    ))
}

/// Parse time part: HH:MM:SS.mmm or HH:MM:SS or HH:MM
fn parse_time(s: &str) -> Option<(u32, u32, u32, u32)> {
    let (time_str, ms) = if let Some(dot_pos) = s.find('.') {
        let ms_str = s.get(dot_pos + 1..)?;
        let ms: u32 = ms_str.parse().ok()?;
        // Handle different precision (1, 2, or 3 digits)
        let ms = match ms_str.len() {
            1 => ms * 100,
            2 => ms * 10,
            3 => ms,
            _ => return None,
        };
        (s.get(..dot_pos)?, ms)
    } else {
        (s, 0)
    };

    let parts: Vec<&str> = time_str.split(':').collect();
    let hour: u32 = parts.first()?.parse().ok()?;
    let minute: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let second: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }

    Some((hour, minute, second, ms))
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
    interp.register_method(&proto, "toJSON", date_to_iso_string, 0);
    interp.register_method(&proto, "valueOf", date_get_time, 0);
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

    // Set constructor.prototype = Date.prototype
    let proto_key = PropertyKey::String(interp.intern("prototype"));
    constructor
        .borrow_mut()
        .set_property(proto_key, JsValue::Object(interp.date_prototype.clone()));

    // Set Date.prototype.constructor = Date
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .date_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(constructor.clone()));

    // Register globally
    let date_key = PropertyKey::String(interp.intern("Date"));
    interp
        .global
        .borrow_mut()
        .set_property(date_key, JsValue::Object(constructor));
}

pub fn date_constructor(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    // Check if called with `new`
    let is_new_call = if let JsValue::Object(obj) = &this {
        let borrowed = obj.borrow();
        if let Some(ref proto) = borrowed.prototype {
            core::ptr::eq(
                &*proto.borrow() as *const _,
                &*interp.date_prototype.borrow() as *const _,
            )
        } else {
            false
        }
    } else {
        false
    };

    // When called as a function (without `new`), return the current date/time as a string
    if !is_new_call {
        let date_string = format_date_for_tostring(interp.now_millis() as f64);
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            date_string,
        ))));
    }

    // Called with `new` - create a Date object
    let timestamp = if args.is_empty() {
        interp.now_millis() as f64
    } else if args.len() == 1 {
        match args.first() {
            Some(JsValue::Number(n)) => *n,
            Some(JsValue::String(s)) => parse_date_string(s.as_ref()),
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

        components_to_ts(year, month, day, hours, minutes, seconds, ms)
    };

    // Set the exotic Date object on the this object
    if let JsValue::Object(obj) = &this {
        obj.borrow_mut().exotic = ExoticObject::Date { timestamp };
    }

    Ok(Guarded::unguarded(this))
}

pub fn date_now(
    interp: &mut Interpreter,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    Ok(Guarded::unguarded(JsValue::Number(interp.now_millis() as f64)))
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

    let timestamp = components_to_ts(year, month, day, hours, minutes, seconds, ms);
    Ok(Guarded::unguarded(JsValue::Number(timestamp)))
}

pub fn date_parse(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let arg = args.first().cloned().unwrap_or(JsValue::Undefined);
    let s = interp.to_js_string(&arg).to_string();
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
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };
    Ok(Guarded::unguarded(JsValue::Number(c.year as f64)))
}

pub fn date_get_month(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };
    Ok(Guarded::unguarded(JsValue::Number((c.month - 1) as f64)))
}

pub fn date_get_date(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };
    Ok(Guarded::unguarded(JsValue::Number(c.day as f64)))
}

pub fn date_get_day(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };
    Ok(Guarded::unguarded(JsValue::Number(c.weekday as f64)))
}

pub fn date_get_hours(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };
    Ok(Guarded::unguarded(JsValue::Number(c.hour as f64)))
}

pub fn date_get_minutes(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };
    Ok(Guarded::unguarded(JsValue::Number(c.minute as f64)))
}

pub fn date_get_seconds(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };
    Ok(Guarded::unguarded(JsValue::Number(c.second as f64)))
}

pub fn date_get_milliseconds(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };
    Ok(Guarded::unguarded(JsValue::Number(c.ms as f64)))
}

pub fn date_to_iso_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Err(JsError::range_error("Invalid Date"));
    };
    let iso = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        c.year, c.month, c.day, c.hour, c.minute, c.second, c.ms
    );
    Ok(Guarded::unguarded(JsValue::String(JsString::from(iso))))
}

// Setter methods

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
    let Some(c) = ts_to_components(current_ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };

    let new_year = args.first().map(|v| v.to_number() as i32).unwrap_or(c.year);
    let new_month = args
        .get(1)
        .map(|v| v.to_number() as i32)
        .unwrap_or((c.month - 1) as i32);
    let new_day = args
        .get(2)
        .map(|v| v.to_number() as i32)
        .unwrap_or(c.day as i32);

    let new_ts = components_to_ts(
        new_year, new_month, new_day, c.hour, c.minute, c.second, c.ms,
    );
    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_month(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(current_ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };

    let new_month = args
        .first()
        .map(|v| v.to_number() as i32)
        .unwrap_or((c.month - 1) as i32);
    let new_day = args
        .get(1)
        .map(|v| v.to_number() as i32)
        .unwrap_or(c.day as i32);

    let new_ts = components_to_ts(c.year, new_month, new_day, c.hour, c.minute, c.second, c.ms);
    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_date(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(current_ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };

    let new_day = args
        .first()
        .map(|v| v.to_number() as i32)
        .unwrap_or(c.day as i32);

    let new_ts = components_to_ts(
        c.year,
        (c.month - 1) as i32,
        new_day,
        c.hour,
        c.minute,
        c.second,
        c.ms,
    );
    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_hours(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(current_ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };

    let new_hour = args.first().map(|v| v.to_number() as u32).unwrap_or(c.hour);
    let new_min = args
        .get(1)
        .map(|v| v.to_number() as u32)
        .unwrap_or(c.minute);
    let new_sec = args
        .get(2)
        .map(|v| v.to_number() as u32)
        .unwrap_or(c.second);
    let new_ms = args.get(3).map(|v| v.to_number() as u32).unwrap_or(c.ms);

    let new_ts = components_to_ts(
        c.year,
        (c.month - 1) as i32,
        c.day as i32,
        new_hour,
        new_min,
        new_sec,
        new_ms,
    );
    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_minutes(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(current_ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };

    let new_min = args
        .first()
        .map(|v| v.to_number() as u32)
        .unwrap_or(c.minute);
    let new_sec = args
        .get(1)
        .map(|v| v.to_number() as u32)
        .unwrap_or(c.second);
    let new_ms = args.get(2).map(|v| v.to_number() as u32).unwrap_or(c.ms);

    let new_ts = components_to_ts(
        c.year,
        (c.month - 1) as i32,
        c.day as i32,
        c.hour,
        new_min,
        new_sec,
        new_ms,
    );
    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_seconds(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(current_ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };

    let new_sec = args
        .first()
        .map(|v| v.to_number() as u32)
        .unwrap_or(c.second);
    let new_ms = args.get(1).map(|v| v.to_number() as u32).unwrap_or(c.ms);

    let new_ts = components_to_ts(
        c.year,
        (c.month - 1) as i32,
        c.day as i32,
        c.hour,
        c.minute,
        new_sec,
        new_ms,
    );
    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

pub fn date_set_milliseconds(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let current_ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(current_ts) else {
        return Ok(Guarded::unguarded(JsValue::Number(f64::NAN)));
    };

    let new_ms = args.first().map(|v| v.to_number() as u32).unwrap_or(c.ms);

    let new_ts = components_to_ts(
        c.year,
        (c.month - 1) as i32,
        c.day as i32,
        c.hour,
        c.minute,
        c.second,
        new_ms,
    );
    let ts = set_date_timestamp(&this, new_ts)?;
    Ok(Guarded::unguarded(JsValue::Number(ts)))
}

/// Date.prototype.toString()
pub fn date_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let formatted = format_date_for_tostring(ts);
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        formatted,
    ))))
}

/// Date.prototype.toDateString()
pub fn date_to_date_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            "Invalid Date",
        ))));
    };
    let formatted = format!(
        "{} {} {:02} {:04}",
        WEEKDAY_NAMES
            .get(c.weekday as usize)
            .copied()
            .unwrap_or("???"),
        MONTH_NAMES
            .get((c.month - 1) as usize)
            .copied()
            .unwrap_or("???"),
        c.day,
        c.year
    );
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        formatted,
    ))))
}

/// Date.prototype.toTimeString()
pub fn date_to_time_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    let ts = get_date_timestamp(&this)?;
    let Some(c) = ts_to_components(ts) else {
        return Ok(Guarded::unguarded(JsValue::String(JsString::from(
            "Invalid Date",
        ))));
    };
    let formatted = format!(
        "{:02}:{:02}:{:02} GMT+0000 (UTC)",
        c.hour, c.minute, c.second
    );
    Ok(Guarded::unguarded(JsValue::String(JsString::from(
        formatted,
    ))))
}
