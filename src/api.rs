//! Public API for interacting with JavaScript values from Rust.
//!
//! All methods that create or return GC-managed objects accept a `&Guard<JsObject>`
//! parameter. The caller is responsible for creating and managing guards to keep
//! objects alive.
//!
//! # Example
//!
//! ```ignore
//! use tsrun::{Runtime, api, JsValue};
//!
//! let mut runtime = Runtime::new();
//! let guard = api::create_guard(&runtime);
//!
//! // Create values - objects are guarded by the provided guard
//! let obj = api::create_object(&mut runtime, &guard)?;
//! api::set_property(&obj, "name", JsValue::from("Alice"))?;
//!
//! // Read values
//! let name = api::get_property(&obj, "name")?;
//! assert_eq!(name.as_str(), Some("Alice"));
//!
//! // Call methods - results are guarded
//! let arr = api::create_from_json(&mut runtime, &guard, &serde_json::json!([3, 1, 2]))?;
//! api::call_method(&mut runtime, &guard, &arr, "sort", &[])?;
//! let joined = api::call_method(&mut runtime, &guard, &arr, "join", &[JsValue::from("-")])?;
//! assert_eq!(joined.as_str(), Some("1-2-3"));
//! ```

use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::interpreter;
use crate::value::{self, CheapClone, JsObject, JsValue};
use crate::{JsString, Runtime};

// ═══════════════════════════════════════════════════════════════════════════════
// Guard Creation
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a guard for keeping GC-managed objects alive.
///
/// Guards are used to protect objects from garbage collection. Objects
/// added to a guard (either via allocation or `guard.guard(obj)`) will
/// remain alive as long as the guard exists.
///
/// # Example
/// ```ignore
/// let guard = api::create_guard(&runtime);
/// let obj = api::create_object(&mut runtime, &guard)?;
/// // obj is kept alive by guard
/// // When guard is dropped, obj may be collected
/// ```
pub fn create_guard(runtime: &Runtime) -> Guard<JsObject> {
    runtime.interpreter.heap.create_guard()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Value Creation
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a JsValue from any type that implements Into<JsValue>.
///
/// Works for primitive types that don't need GC protection:
/// - `bool` → JsValue::Boolean
/// - `f64`, `i32`, `i64`, `u32`, `u64`, `usize` → JsValue::Number
/// - `&str`, `String` → JsValue::String
/// - `()` → JsValue::Undefined
///
/// For complex types (objects, arrays), use `create_from_json()` instead.
///
/// # Example
/// ```ignore
/// let num = api::create_value(42);
/// let text = api::create_value("hello");
/// let flag = api::create_value(true);
/// ```
pub fn create_value<T: Into<JsValue>>(value: T) -> JsValue {
    value.into()
}

/// Create a JsValue containing undefined.
pub fn create_undefined() -> JsValue {
    JsValue::Undefined
}

/// Create a JsValue containing null.
pub fn create_null() -> JsValue {
    JsValue::Null
}

/// Create a JsValue from a JSON value.
///
/// The created objects are added to the provided guard to keep them alive.
///
/// # Example
/// ```ignore
/// use serde_json::json;
///
/// let guard = runtime.create_guard();
/// let user = api::create_from_json(&mut runtime, &guard, &json!({
///     "name": "Alice",
///     "age": 30
/// }))?;
///
/// let items = api::create_from_json(&mut runtime, &guard, &json!([1, 2, 3]))?;
/// ```
pub fn create_from_json(
    runtime: &mut Runtime,
    guard: &Guard<JsObject>,
    json: &serde_json::Value,
) -> Result<JsValue, JsError> {
    interpreter::builtins::json::json_to_js_value_with_guard(&mut runtime.interpreter, json, guard)
}

/// Create an empty object.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let obj = api::create_object(&mut runtime, &guard)?;
/// ```
pub fn create_object(runtime: &mut Runtime, guard: &Guard<JsObject>) -> Result<JsValue, JsError> {
    create_from_json(runtime, guard, &serde_json::json!({}))
}

/// Create an empty array.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let arr = api::create_array(&mut runtime, &guard)?;
/// ```
pub fn create_array(runtime: &mut Runtime, guard: &Guard<JsObject>) -> Result<JsValue, JsError> {
    create_from_json(runtime, guard, &serde_json::json!([]))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Property Access (Read)
// ═══════════════════════════════════════════════════════════════════════════════

/// Get a property value by key.
///
/// Returns the property value. If the value is an object, the caller must
/// ensure it's guarded if needed for GC safety.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let user = api::create_from_json(&mut runtime, &guard, &json!({"name": "Alice", "age": 30}))?;
/// let name = api::get_property(&user, "name")?;
/// assert_eq!(name.as_str(), Some("Alice"));
/// ```
pub fn get_property(obj: &JsValue, key: &str) -> Result<JsValue, JsError> {
    let object = obj
        .as_object()
        .ok_or_else(|| JsError::type_error("Cannot get property of non-object"))?;

    let prop_key = value::PropertyKey::String(JsString::from(key));
    let value = {
        let borrowed = object.borrow();
        borrowed.get_property(&prop_key)
    };

    Ok(value.unwrap_or(JsValue::Undefined))
}

/// Get an array element by index.
///
/// Returns undefined if the index is out of bounds.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let arr = api::create_from_json(&mut runtime, &guard, &json!([10, 20, 30]))?;
/// let first = api::get_index(&arr, 0)?;
/// assert_eq!(first.as_number(), Some(10.0));
/// ```
pub fn get_index(arr: &JsValue, index: usize) -> Result<JsValue, JsError> {
    let object = arr
        .as_object()
        .ok_or_else(|| JsError::type_error("Cannot get index of non-object"))?;

    let value = {
        let borrowed = object.borrow();
        borrowed
            .array_elements()
            .and_then(|elements| elements.get(index).cloned())
    };

    Ok(value.unwrap_or(JsValue::Undefined))
}

/// Get all elements of an array as JsValues.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let arr = api::create_from_json(&mut runtime, &guard, &json!([1, 2, 3]))?;
/// let elements = api::get_elements(&arr)?;
/// for elem in elements {
///     println!("{}", elem);
/// }
/// ```
pub fn get_elements(arr: &JsValue) -> Result<Vec<JsValue>, JsError> {
    let object = arr
        .as_object()
        .ok_or_else(|| JsError::type_error("Cannot get elements of non-object"))?;

    let elements = {
        let borrowed = object.borrow();
        borrowed
            .array_elements()
            .map(|e| e.to_vec())
            .unwrap_or_default()
    };

    Ok(elements)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Property Mutation
// ═══════════════════════════════════════════════════════════════════════════════

/// Set a property on an object.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let obj = api::create_object(&mut runtime, &guard)?;
/// api::set_property(&obj, "name", JsValue::from("Alice"))?;
/// api::set_property(&obj, "age", JsValue::from(30))?;
/// ```
pub fn set_property(obj: &JsValue, key: &str, value: JsValue) -> Result<(), JsError> {
    let object = obj
        .as_object()
        .ok_or_else(|| JsError::type_error("Cannot set property on non-object"))?;

    let prop_key = value::PropertyKey::String(JsString::from(key));
    object.borrow_mut().set_property(prop_key, value);
    Ok(())
}

/// Set an array element by index.
///
/// If the index is beyond the current length, the array will be extended
/// with undefined values.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let arr = api::create_from_json(&mut runtime, &guard, &json!([1, 2, 3]))?;
/// api::set_index(&arr, 1, JsValue::from(20))?;  // [1, 20, 3]
/// ```
pub fn set_index(arr: &JsValue, index: usize, value: JsValue) -> Result<(), JsError> {
    let object = arr
        .as_object()
        .ok_or_else(|| JsError::type_error("Cannot set index on non-object"))?;

    let mut borrowed = object.borrow_mut();

    match &mut borrowed.exotic {
        value::ExoticObject::Array { elements } => {
            // Extend array if needed
            while elements.len() <= index {
                elements.push(JsValue::Undefined);
            }
            if let Some(elem) = elements.get_mut(index) {
                *elem = value;
            }
            let new_len = elements.len();
            drop(borrowed);
            object.borrow_mut().set_property(
                value::PropertyKey::String(JsString::from("length")),
                JsValue::Number(new_len as f64),
            );
            Ok(())
        }
        _ => Err(JsError::type_error("Cannot set index on non-array")),
    }
}

/// Push a value onto an array.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let arr = api::create_array(&mut runtime, &guard)?;
/// api::push(&arr, JsValue::from(1))?;
/// api::push(&arr, JsValue::from(2))?;
/// // arr is now [1, 2]
/// ```
pub fn push(arr: &JsValue, value: JsValue) -> Result<(), JsError> {
    let object = arr
        .as_object()
        .ok_or_else(|| JsError::type_error("Cannot push to non-object"))?;

    let mut borrowed = object.borrow_mut();

    match &mut borrowed.exotic {
        value::ExoticObject::Array { elements } => {
            elements.push(value);
            let new_len = elements.len();
            drop(borrowed);
            object.borrow_mut().set_property(
                value::PropertyKey::String(JsString::from("length")),
                JsValue::Number(new_len as f64),
            );
            Ok(())
        }
        _ => Err(JsError::type_error("Cannot push to non-array")),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Function and Method Calls
// ═══════════════════════════════════════════════════════════════════════════════

/// Call a method on an object.
///
/// The result (if an object) is added to the provided guard.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let arr = api::create_from_json(&mut runtime, &guard, &json!([3, 1, 2]))?;
/// api::call_method(&mut runtime, &guard, &arr, "sort", &[])?;
/// // arr is now [1, 2, 3]
///
/// let result = api::call_method(&mut runtime, &guard, &arr, "join", &[JsValue::from("-")])?;
/// assert_eq!(result.as_str(), Some("1-2-3"));
/// ```
pub fn call_method(
    runtime: &mut Runtime,
    guard: &Guard<JsObject>,
    obj: &JsValue,
    method_name: &str,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let object = obj
        .as_object()
        .ok_or_else(|| JsError::type_error("Cannot call method on non-object"))?;

    // Look up the method from the object's properties and prototype chain
    let prop_key = value::PropertyKey::String(JsString::from(method_name));
    let method = {
        let borrowed = object.borrow();
        borrowed.get_property(&prop_key)
    };

    let method =
        method.ok_or_else(|| JsError::type_error(format!("{} is not a function", method_name)))?;

    if !method.is_callable() {
        return Err(JsError::type_error(format!(
            "{} is not a function",
            method_name
        )));
    }

    let result =
        runtime
            .interpreter
            .call_function(method, JsValue::Object(object.cheap_clone()), args)?;

    // Guard the result if it's an object
    if let Some(obj) = result.value.as_object() {
        guard.guard(obj.cheap_clone());
    }

    Ok(result.value)
}

/// Call a function with optional `this` binding.
///
/// The result (if an object) is added to the provided guard.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let result = runtime.eval("function add(a, b) { return a + b; } add")?;
/// if let RuntimeResult::Complete(add_fn) = result {
///     let sum = api::call_function(
///         &mut runtime,
///         &guard,
///         add_fn.value(),
///         None,
///         &[JsValue::from(10), JsValue::from(20)]
///     )?;
///     assert_eq!(sum.as_number(), Some(30.0));
/// }
/// ```
pub fn call_function(
    runtime: &mut Runtime,
    guard: &Guard<JsObject>,
    func: &JsValue,
    this: Option<&JsValue>,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    if !func.is_callable() {
        return Err(JsError::type_error("Value is not a function"));
    }

    let this_value = this.cloned().unwrap_or(JsValue::Undefined);

    let result = runtime
        .interpreter
        .call_function(func.clone(), this_value, args)?;

    // Guard the result if it's an object
    if let Some(obj) = result.value.as_object() {
        guard.guard(obj.cheap_clone());
    }

    Ok(result.value)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Utility Functions
// ═══════════════════════════════════════════════════════════════════════════════

/// Guard a JsValue if it contains an object.
///
/// This is a convenience function to ensure an object is kept alive by a guard.
/// Does nothing for primitive values.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let value = api::get_property(&obj, "nested")?;
/// api::guard_value(&guard, &value);  // Ensure nested object is kept alive
/// ```
pub fn guard_value(guard: &Guard<JsObject>, value: &JsValue) {
    if let Some(obj) = value.as_object() {
        guard.guard(obj.cheap_clone());
    }
}

/// Get the length of an array.
///
/// Returns `None` if this is not an array.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let arr = api::create_from_json(&mut runtime, &guard, &json!([1, 2, 3, 4, 5]))?;
/// assert_eq!(api::len(&arr), Some(5));
/// ```
pub fn len(arr: &JsValue) -> Option<usize> {
    let obj = arr.as_object()?;
    let borrowed = obj.borrow();
    borrowed.array_length().map(|l| l as usize)
}

/// Check if the array is empty.
///
/// Returns `None` if this is not an array.
pub fn is_empty(arr: &JsValue) -> Option<bool> {
    len(arr).map(|l| l == 0)
}

/// Check if this value is an array.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let arr = api::create_from_json(&mut runtime, &guard, &json!([1, 2, 3]))?;
/// let obj = api::create_from_json(&mut runtime, &guard, &json!({"x": 1}))?;
/// assert!(api::is_array(&arr));
/// assert!(!api::is_array(&obj));
/// ```
pub fn is_array(value: &JsValue) -> bool {
    if let Some(obj) = value.as_object() {
        let borrowed = obj.borrow();
        borrowed.array_length().is_some()
    } else {
        false
    }
}

/// Get all property keys of an object.
///
/// Returns an empty vector if this is not an object.
///
/// # Example
/// ```ignore
/// let guard = runtime.create_guard();
/// let obj = api::create_from_json(&mut runtime, &guard, &json!({"a": 1, "b": 2}))?;
/// let keys = api::keys(&obj);
/// assert!(keys.contains(&"a".to_string()));
/// assert!(keys.contains(&"b".to_string()));
/// ```
pub fn keys(obj: &JsValue) -> Vec<String> {
    if let Some(object) = obj.as_object() {
        let borrowed = object.borrow();
        borrowed
            .properties
            .keys()
            .filter_map(|k| match k {
                value::PropertyKey::String(s) => Some(s.to_string()),
                value::PropertyKey::Index(i) => Some(i.to_string()),
                value::PropertyKey::Symbol(_) => None,
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Get the Gc<JsObject> from a JsValue if it's an object.
///
/// This is useful when you need direct access to the object pointer.
pub fn as_object(value: &JsValue) -> Option<Gc<JsObject>> {
    value.as_object().map(|o| o.cheap_clone())
}
