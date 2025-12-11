//! RegExp built-in methods

use crate::error::JsError;
use crate::gc::Space;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, register_method, ExoticObject, JsFunction, JsObject,
    JsObjectRef, JsString, JsValue, NativeFunction, PropertyKey,
};

/// Create RegExp.prototype with test and exec methods
pub fn create_regexp_prototype(space: &mut Space<JsObject>) -> JsObjectRef {
    let proto = create_object(space);

    register_method(space, &proto, "test", regexp_test, 1);
    register_method(space, &proto, "exec", regexp_exec, 1);

    proto
}

/// Create RegExp constructor
pub fn create_regexp_constructor(
    space: &mut Space<JsObject>,
    regexp_prototype: &JsObjectRef,
) -> JsObjectRef {
    let constructor = create_function(
        space,
        JsFunction::Native(NativeFunction {
            name: "RegExp".to_string(),
            func: regexp_constructor,
            arity: 2,
        }),
    );
    {
        let mut re = constructor.borrow_mut();
        re.set_property(
            PropertyKey::from("prototype"),
            JsValue::Object(regexp_prototype.clone()),
        );
    }
    constructor
}

pub fn regexp_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let pattern = args
        .first()
        .cloned()
        .unwrap_or(JsValue::String(JsString::from("")))
        .to_js_string()
        .to_string();
    let flags = args
        .get(1)
        .cloned()
        .unwrap_or(JsValue::String(JsString::from("")))
        .to_js_string()
        .to_string();

    let regexp_obj = interp.create_object();
    {
        let mut obj = regexp_obj.borrow_mut();
        obj.exotic = ExoticObject::RegExp {
            pattern: pattern.clone(),
            flags: flags.clone(),
        };
        obj.prototype = Some(interp.regexp_prototype.clone());
        obj.set_property(
            PropertyKey::from("source"),
            JsValue::String(JsString::from(pattern)),
        );
        obj.set_property(
            PropertyKey::from("flags"),
            JsValue::String(JsString::from(flags.clone())),
        );
        obj.set_property(
            PropertyKey::from("global"),
            JsValue::Boolean(flags.contains('g')),
        );
        obj.set_property(
            PropertyKey::from("ignoreCase"),
            JsValue::Boolean(flags.contains('i')),
        );
        obj.set_property(
            PropertyKey::from("multiline"),
            JsValue::Boolean(flags.contains('m')),
        );
        obj.set_property(
            PropertyKey::from("dotAll"),
            JsValue::Boolean(flags.contains('s')),
        );
        obj.set_property(
            PropertyKey::from("unicode"),
            JsValue::Boolean(flags.contains('u')),
        );
        obj.set_property(
            PropertyKey::from("sticky"),
            JsValue::Boolean(flags.contains('y')),
        );
        // Initialize lastIndex to 0
        obj.set_property(PropertyKey::from("lastIndex"), JsValue::Number(0.0));
    }
    Ok(JsValue::Object(regexp_obj))
}

pub fn get_regexp_data(this: &JsValue) -> Result<(String, String), JsError> {
    let JsValue::Object(obj) = this else {
        return Err(JsError::type_error("this is not a RegExp"));
    };
    let obj_ref = obj.borrow();
    if let ExoticObject::RegExp {
        ref pattern,
        ref flags,
    } = obj_ref.exotic
    {
        Ok((pattern.clone(), flags.clone()))
    } else {
        Err(JsError::type_error("this is not a RegExp"))
    }
}

pub fn build_regex(pattern: &str, flags: &str) -> Result<regex::Regex, JsError> {
    let mut regex_pattern = pattern.to_string();

    // Build flags prefix
    let mut prefix = String::new();

    // Handle case-insensitive flag (i)
    if flags.contains('i') {
        prefix.push('i');
    }

    // Handle multiline flag (m)
    if flags.contains('m') {
        prefix.push('m');
    }

    // Handle dotAll flag (s) - makes . match newlines
    if flags.contains('s') {
        prefix.push('s');
    }

    if !prefix.is_empty() {
        regex_pattern = format!("(?{}){}", prefix, regex_pattern);
    }

    regex::Regex::new(&regex_pattern)
        .map_err(|e| JsError::syntax_error(format!("Invalid regular expression: {}", e), 0, 0))
}

pub fn regexp_test(
    _interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let (pattern, flags) = get_regexp_data(&this)?;
    let input = args
        .first()
        .cloned()
        .unwrap_or(JsValue::Undefined)
        .to_js_string()
        .to_string();

    let re = build_regex(&pattern, &flags)?;
    Ok(JsValue::Boolean(re.is_match(&input)))
}

pub fn regexp_exec(
    interp: &mut Interpreter,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let JsValue::Object(ref obj) = this else {
        return Err(JsError::type_error("this is not a RegExp"));
    };

    let (pattern, flags) = get_regexp_data(&this)?;
    let input = args
        .first()
        .cloned()
        .unwrap_or(JsValue::Undefined)
        .to_js_string()
        .to_string();

    let is_global = flags.contains('g');
    let is_sticky = flags.contains('y');

    // Get lastIndex for global/sticky regexes
    let last_index = if is_global || is_sticky {
        let li = obj
            .borrow()
            .get_property(&PropertyKey::from("lastIndex"))
            .unwrap_or(JsValue::Number(0.0));
        match li {
            JsValue::Number(n) => n as usize,
            _ => 0,
        }
    } else {
        0
    };

    let re = build_regex(&pattern, &flags)?;

    // For global/sticky, we need to search starting from lastIndex
    let search_str = if last_index > 0 && last_index <= input.len() {
        // Get substring starting from lastIndex (handle UTF-8 properly)
        input
            .char_indices()
            .nth(last_index)
            .and_then(|(byte_idx, _)| input.get(byte_idx..))
            .unwrap_or("")
    } else if last_index > input.len() {
        // lastIndex past end of string - no match
        if is_global || is_sticky {
            obj.borrow_mut()
                .set_property(PropertyKey::from("lastIndex"), JsValue::Number(0.0));
        }
        return Ok(JsValue::Null);
    } else {
        input.as_str()
    };

    match re.captures(search_str) {
        Some(caps) => {
            let mut result = Vec::new();
            for cap in caps.iter() {
                match cap {
                    Some(m) => result.push(JsValue::String(JsString::from(m.as_str()))),
                    None => result.push(JsValue::Undefined),
                }
            }
            let arr = interp.create_array(result);

            // Calculate actual index in original string
            let match_start = caps.get(0).map(|m| m.start()).unwrap_or(0);
            let actual_index = last_index + match_start;

            arr.borrow_mut().set_property(
                PropertyKey::from("index"),
                JsValue::Number(actual_index as f64),
            );
            arr.borrow_mut().set_property(
                PropertyKey::from("input"),
                JsValue::String(JsString::from(input.clone())),
            );

            // Update lastIndex for global/sticky regexes
            if is_global || is_sticky {
                let match_end = caps.get(0).map(|m| m.end()).unwrap_or(0);
                let new_last_index = last_index + match_end;
                obj.borrow_mut().set_property(
                    PropertyKey::from("lastIndex"),
                    JsValue::Number(new_last_index as f64),
                );
            }

            Ok(JsValue::Object(arr))
        }
        None => {
            // Reset lastIndex to 0 on no match for global/sticky
            if is_global || is_sticky {
                obj.borrow_mut()
                    .set_property(PropertyKey::from("lastIndex"), JsValue::Number(0.0));
            }
            Ok(JsValue::Null)
        }
    }
}
