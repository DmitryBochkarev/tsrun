//! Public API for interacting with JavaScript values from Rust.
//!
//! All methods that create or return GC-managed objects accept a `&Guard<JsObject>`
//! parameter. The caller is responsible for creating and managing guards to keep
//! objects alive.
//!
//! # Example
//!
//! ```
//! use tsrun::{Interpreter, api, JsValue};
//!
//! let mut interp = Interpreter::new();
//! let guard = api::create_guard(&interp);
//!
//! // Create values - objects are guarded by the provided guard
//! let obj = api::create_object(&mut interp, &guard).unwrap();
//! api::set_property(&obj, "name", JsValue::from("Alice")).unwrap();
//!
//! // Read values
//! let name = api::get_property(&obj, "name").unwrap();
//! assert_eq!(name.as_str(), Some("Alice"));
//!
//! // Call methods - results are guarded
//! let arr = api::create_from_json(&mut interp, &guard, &serde_json::json!([3, 1, 2])).unwrap();
//! api::call_method(&mut interp, &guard, &arr, "sort", &[]).unwrap();
//! let joined = api::call_method(&mut interp, &guard, &arr, "join", &[JsValue::from("-")]).unwrap();
//! assert_eq!(joined.as_str(), Some("1-2-3"));
//! ```

use crate::JsString;
use crate::error::JsError;
use crate::gc::{Gc, Guard};
use crate::interpreter::{self, Interpreter};
use crate::prelude::*;
use crate::value::{self, CheapClone, JsObject, JsValue};

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
/// ```
/// use tsrun::{Interpreter, api};
///
/// let interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// // All objects created with this guard stay alive until guard is dropped
/// ```
pub fn create_guard(interp: &Interpreter) -> Guard<JsObject> {
    interp.heap.create_guard()
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
/// ```
/// use tsrun::api;
///
/// let num = api::create_value(42);
/// let text = api::create_value("hello");
/// let flag = api::create_value(true);
///
/// assert_eq!(num.as_number(), Some(42.0));
/// assert_eq!(text.as_str(), Some("hello"));
/// assert_eq!(flag.as_bool(), Some(true));
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
/// ```
/// use tsrun::{Interpreter, api};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
///
/// let user = api::create_from_json(&mut interp, &guard, &json!({
///     "name": "Alice",
///     "age": 30
/// })).unwrap();
///
/// let items = api::create_from_json(&mut interp, &guard, &json!([1, 2, 3])).unwrap();
/// assert_eq!(api::len(&items), Some(3));
/// ```
pub fn create_from_json(
    interp: &mut Interpreter,
    guard: &Guard<JsObject>,
    json: &serde_json::Value,
) -> Result<JsValue, JsError> {
    interpreter::builtins::json::json_to_js_value_with_guard(interp, json, guard)
}

/// Create an empty object.
///
/// # Example
/// ```
/// use tsrun::{Interpreter, api, JsValue};
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let obj = api::create_object(&mut interp, &guard).unwrap();
/// api::set_property(&obj, "x", JsValue::from(42)).unwrap();
/// ```
pub fn create_object(
    interp: &mut Interpreter,
    guard: &Guard<JsObject>,
) -> Result<JsValue, JsError> {
    create_from_json(interp, guard, &serde_json::json!({}))
}

/// Create an empty array.
///
/// # Example
/// ```
/// use tsrun::{Interpreter, api, JsValue};
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let arr = api::create_array(&mut interp, &guard).unwrap();
/// api::push(&arr, JsValue::from(1)).unwrap();
/// assert_eq!(api::len(&arr), Some(1));
/// ```
pub fn create_array(interp: &mut Interpreter, guard: &Guard<JsObject>) -> Result<JsValue, JsError> {
    create_from_json(interp, guard, &serde_json::json!([]))
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
/// ```
/// use tsrun::{Interpreter, api};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let user = api::create_from_json(&mut interp, &guard, &json!({"name": "Alice", "age": 30})).unwrap();
/// let name = api::get_property(&user, "name").unwrap();
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
/// ```
/// use tsrun::{Interpreter, api};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let arr = api::create_from_json(&mut interp, &guard, &json!([10, 20, 30])).unwrap();
/// let first = api::get_index(&arr, 0).unwrap();
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
/// ```
/// use tsrun::{Interpreter, api};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let arr = api::create_from_json(&mut interp, &guard, &json!([1, 2, 3])).unwrap();
/// let elements = api::get_elements(&arr).unwrap();
/// assert_eq!(elements.len(), 3);
/// assert_eq!(elements[0].as_number(), Some(1.0));
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
/// ```
/// use tsrun::{Interpreter, api, JsValue};
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let obj = api::create_object(&mut interp, &guard).unwrap();
/// api::set_property(&obj, "name", JsValue::from("Alice")).unwrap();
/// api::set_property(&obj, "age", JsValue::from(30)).unwrap();
///
/// assert_eq!(api::get_property(&obj, "name").unwrap().as_str(), Some("Alice"));
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
/// ```
/// use tsrun::{Interpreter, api, JsValue};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let arr = api::create_from_json(&mut interp, &guard, &json!([1, 2, 3])).unwrap();
/// api::set_index(&arr, 1, JsValue::from(20)).unwrap();  // [1, 20, 3]
/// assert_eq!(api::get_index(&arr, 1).unwrap().as_number(), Some(20.0));
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
/// ```
/// use tsrun::{Interpreter, api, JsValue};
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let arr = api::create_array(&mut interp, &guard).unwrap();
/// api::push(&arr, JsValue::from(1)).unwrap();
/// api::push(&arr, JsValue::from(2)).unwrap();
/// // arr is now [1, 2]
/// assert_eq!(api::len(&arr), Some(2));
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
/// ```
/// use tsrun::{Interpreter, api, JsValue};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let arr = api::create_from_json(&mut interp, &guard, &json!([3, 1, 2])).unwrap();
/// api::call_method(&mut interp, &guard, &arr, "sort", &[]).unwrap();
/// // arr is now [1, 2, 3]
///
/// let result = api::call_method(&mut interp, &guard, &arr, "join", &[JsValue::from("-")]).unwrap();
/// assert_eq!(result.as_str(), Some("1-2-3"));
/// ```
pub fn call_method(
    interp: &mut Interpreter,
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

    let result = interp.call_function(method, JsValue::Object(object.cheap_clone()), args)?;

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
/// ```
/// use tsrun::{Interpreter, StepResult, api, JsValue};
///
/// let mut interp = Interpreter::new();
/// interp.prepare("function add(a, b) { return a + b; } add", None).unwrap();
///
/// // Run to get the function
/// let add_fn = loop {
///     match interp.step().unwrap() {
///         StepResult::Continue => continue,
///         StepResult::Complete(v) => break v,
///         _ => panic!("Unexpected"),
///     }
/// };
///
/// // Call the function
/// let guard = api::create_guard(&interp);
/// let sum = api::call_function(
///     &mut interp,
///     &guard,
///     add_fn.value(),
///     None,
///     &[JsValue::from(10), JsValue::from(20)]
/// ).unwrap();
/// assert_eq!(sum.as_number(), Some(30.0));
/// ```
pub fn call_function(
    interp: &mut Interpreter,
    guard: &Guard<JsObject>,
    func: &JsValue,
    this: Option<&JsValue>,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    if !func.is_callable() {
        return Err(JsError::type_error("Value is not a function"));
    }

    let this_value = this.cloned().unwrap_or(JsValue::Undefined);

    let result = interp.call_function(func.clone(), this_value, args)?;

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
/// ```
/// use tsrun::{Interpreter, api};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let obj = api::create_from_json(&mut interp, &guard, &json!({"nested": {"x": 1}})).unwrap();
/// let nested = api::get_property(&obj, "nested").unwrap();
/// api::guard_value(&guard, &nested);  // nested object kept alive by guard
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
/// ```
/// use tsrun::{Interpreter, api};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let arr = api::create_from_json(&mut interp, &guard, &json!([1, 2, 3, 4, 5])).unwrap();
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
/// ```
/// use tsrun::{Interpreter, api};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let arr = api::create_from_json(&mut interp, &guard, &json!([1, 2, 3])).unwrap();
/// let obj = api::create_from_json(&mut interp, &guard, &json!({"x": 1})).unwrap();
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
/// ```
/// use tsrun::{Interpreter, api};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let guard = api::create_guard(&interp);
/// let obj = api::create_from_json(&mut interp, &guard, &json!({"a": 1, "b": 2})).unwrap();
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

// ═══════════════════════════════════════════════════════════════════════════════
// Module Export Access
// ═══════════════════════════════════════════════════════════════════════════════

/// Get an exported value from the main module by name.
///
/// This resolves the export through the module namespace object, handling
/// live bindings correctly. Returns `None` if no main module has been evaluated
/// or if the export doesn't exist.
///
/// Note: If the export is an object, you should guard it to prevent GC collection.
///
/// # Example
/// ```
/// use tsrun::{Interpreter, StepResult, api};
///
/// let mut interp = Interpreter::new();
/// interp.prepare(r#"export const VERSION = "1.0.0";"#, Some("/main.ts".into())).unwrap();
///
/// // Run to completion
/// loop {
///     match interp.step().unwrap() {
///         StepResult::Continue => continue,
///         StepResult::Complete(_) => break,
///         _ => break,
///     }
/// }
///
/// // Get export
/// let version = api::get_export(&interp, "VERSION").unwrap();
/// assert_eq!(version.as_str(), Some("1.0.0"));
/// ```
pub fn get_export(interp: &Interpreter, name: &str) -> Option<JsValue> {
    interp.get_export(name)
}

/// Get all export names from the main module.
///
/// Returns an empty vector if no main module has been evaluated.
///
/// # Example
/// ```
/// use tsrun::{Interpreter, StepResult, api};
///
/// let mut interp = Interpreter::new();
/// interp.prepare("export const a = 1; export const b = 2;", Some("/m.ts".into())).unwrap();
///
/// // Run to completion
/// loop {
///     match interp.step().unwrap() {
///         StepResult::Continue => continue,
///         StepResult::Complete(_) => break,
///         _ => break,
///     }
/// }
///
/// let exports = api::get_export_names(&interp);
/// assert!(exports.contains(&"a".to_string()));
/// assert!(exports.contains(&"b".to_string()));
/// ```
pub fn get_export_names(interp: &Interpreter) -> Vec<String> {
    interp.get_export_names()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Promise Creation and Resolution
// ═══════════════════════════════════════════════════════════════════════════════

use crate::{OrderId, RuntimeValue};

/// Create a RuntimeValue from a serde_json value for use in OrderResponse.
///
/// This is the recommended way to create object responses for orders.
/// The returned RuntimeValue keeps the object alive until it is consumed.
///
/// # Example
/// ```
/// use tsrun::{Interpreter, api, RuntimeValue};
/// use serde_json::json;
///
/// let mut interp = Interpreter::new();
/// let response_value = api::create_response_object(&mut interp, &json!({
///     "status": "ok",
///     "data": [1, 2, 3]
/// })).unwrap();
///
/// // response_value can be used in OrderResponse::result
/// assert!(response_value.is_object());
/// ```
pub fn create_response_object(
    interp: &mut Interpreter,
    json: &serde_json::Value,
) -> Result<RuntimeValue, JsError> {
    let guard = interp.heap.create_guard();
    let value = interpreter::builtins::json::json_to_js_value_with_guard(interp, json, &guard)?;
    Ok(RuntimeValue::with_guard(value, guard))
}

/// Create an unresolved Promise that can be resolved or rejected later.
///
/// This is useful when the host wants to return a Promise from `fulfill_orders`
/// that will be resolved asynchronously (e.g., when a network request completes).
///
/// The returned `RuntimeValue` contains the Promise and keeps it alive.
/// Store it and later call `resolve_promise` or `reject_promise` to settle it.
///
/// # Example
/// ```
/// use tsrun::{Interpreter, api};
///
/// let mut interp = Interpreter::new();
/// let promise = api::create_promise(&mut interp);
///
/// // The promise is pending until resolved
/// assert!(promise.is_object());
///
/// // Later: api::resolve_promise(&mut interp, &promise, result_value)
/// ```
pub fn create_promise(interp: &mut Interpreter) -> RuntimeValue {
    let guard = interp.heap.create_guard();
    let promise = interpreter::builtins::promise::create_promise(interp, &guard);
    RuntimeValue::with_guard(JsValue::Object(promise), guard)
}

/// Create an unresolved Promise linked to an order for cancellation tracking.
///
/// Similar to `create_promise()`, but the Promise is associated with the given
/// order ID. When this Promise "loses" in a `Promise.race()`, the order ID will
/// be included in the `cancelled` list of `StepResult::Suspended`.
///
/// Use this when returning a Promise as an order response to enable automatic
/// cancellation notification when the Promise is no longer needed.
///
/// # Example
/// ```
/// use tsrun::{Interpreter, api, OrderId};
///
/// let mut interp = Interpreter::new();
/// let order_id = OrderId(42);
/// let promise = api::create_order_promise(&mut interp, order_id);
///
/// // Promise is linked to order_id for cancellation tracking
/// assert!(promise.is_object());
/// ```
pub fn create_order_promise(interp: &mut Interpreter, order_id: OrderId) -> RuntimeValue {
    let guard = interp.heap.create_guard();
    let promise = interpreter::builtins::promise::create_order_promise(interp, &guard, order_id);
    RuntimeValue::with_guard(JsValue::Object(promise), guard)
}

/// Resolve a Promise that was created with `create_promise`.
///
/// This will fulfill the Promise with the given value and queue any
/// `.then()` handlers. Call `interp.step()` afterwards to
/// execute the queued handlers.
///
/// # Errors
/// Returns an error if the value is not a Promise.
pub fn resolve_promise(
    interp: &mut Interpreter,
    promise: &RuntimeValue,
    value: RuntimeValue,
) -> Result<(), JsError> {
    let JsValue::Object(promise_obj) = promise.value() else {
        return Err(JsError::type_error("Expected a Promise object"));
    };

    interpreter::builtins::promise::resolve_promise_value(
        interp,
        promise_obj,
        value.value().clone(),
    )?;

    // Check if any waiting contexts are now ready
    interp.check_resolved_promises_public();

    Ok(())
}

/// Reject a Promise that was created with `create_promise`.
///
/// This will reject the Promise with the given reason and queue any
/// `.catch()` or rejection handlers. Call `interp.step()` afterwards
/// to execute the queued handlers.
///
/// # Errors
/// Returns an error if the value is not a Promise.
pub fn reject_promise(
    interp: &mut Interpreter,
    promise: &RuntimeValue,
    reason: RuntimeValue,
) -> Result<(), JsError> {
    let JsValue::Object(promise_obj) = promise.value() else {
        return Err(JsError::type_error("Expected a Promise object"));
    };

    interpreter::builtins::promise::reject_promise_value(
        interp,
        promise_obj,
        reason.value().clone(),
    )?;

    // Check if any waiting contexts are now ready
    interp.check_resolved_promises_public();

    Ok(())
}
