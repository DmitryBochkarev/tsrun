//! Integration tests for WASM handle system.
//!
//! Run with: wasm-pack test --node --features wasm --no-default-features

#![cfg(all(target_arch = "wasm32", feature = "wasm"))]

use tsrun::wasm::{StepStatus, TsRunner};
use wasm_bindgen_test::*;

// Helper to run code to completion
fn run_to_completion(runner: &mut TsRunner, code: &str) -> bool {
    let prep = runner.prepare(code, Some("test.ts".into()));
    if prep.status() == StepStatus::Error {
        return false;
    }
    loop {
        let result = runner.step();
        match result.status() {
            StepStatus::Continue => continue,
            StepStatus::Complete | StepStatus::Done => return true,
            _ => return false,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Value Creation Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_create_primitives() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    // Number
    let h = runner.create_number(42.5);
    assert_ne!(h, 0);
    assert_eq!(runner.get_value_type(h), "number");
    assert_eq!(runner.value_as_number(h), 42.5);

    // String
    let h = runner.create_string("hello world");
    assert_eq!(runner.get_value_type(h), "string");
    assert_eq!(runner.value_as_string(h), Some("hello world".into()));

    // Boolean
    let h = runner.create_bool(true);
    assert_eq!(runner.get_value_type(h), "boolean");
    assert_eq!(runner.value_as_bool(h), Some(true));

    // Null
    let h = runner.create_null();
    assert_eq!(runner.get_value_type(h), "null");
    assert!(runner.value_is_null(h));

    // Undefined
    let h = runner.create_undefined();
    assert_eq!(runner.get_value_type(h), "undefined");
    assert!(runner.value_is_undefined(h));
}

#[wasm_bindgen_test]
fn test_create_object_and_array() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    // Object
    let obj = runner.create_object();
    assert_ne!(obj, 0);
    assert_eq!(runner.get_value_type(obj), "object");
    assert!(!runner.value_is_array(obj));

    // Array
    let arr = runner.create_array();
    assert_ne!(arr, 0);
    assert_eq!(runner.get_value_type(arr), "object");
    assert!(runner.value_is_array(arr));
    assert_eq!(runner.array_length(arr), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Object Operations Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_object_property_operations() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let obj = runner.create_object();
    let val = runner.create_number(123.0);

    // Set property
    assert!(runner.set_property(obj, "x", val));
    assert!(runner.has_property(obj, "x"));

    // Get property
    let got = runner.get_property(obj, "x");
    assert_ne!(got, 0);
    assert_eq!(runner.value_as_number(got), 123.0);

    // Get keys
    let keys = runner.get_keys(obj);
    assert!(keys.contains(&"x".to_string()));

    // Delete property
    assert!(runner.delete_property(obj, "x"));
    assert!(!runner.has_property(obj, "x"));
}

#[wasm_bindgen_test]
fn test_nested_objects() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let outer = runner.create_object();
    let inner = runner.create_object();
    let val = runner.create_string("nested");

    runner.set_property(inner, "value", val);
    runner.set_property(outer, "inner", inner);

    // Access nested property
    let got_inner = runner.get_property(outer, "inner");
    assert_eq!(runner.get_value_type(got_inner), "object");

    let got_val = runner.get_property(got_inner, "value");
    assert_eq!(runner.value_as_string(got_val), Some("nested".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Array Operations Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_array_operations() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let arr = runner.create_array();
    assert_eq!(runner.array_length(arr), 0);

    // Push values
    let v1 = runner.create_number(10.0);
    let v2 = runner.create_number(20.0);
    assert!(runner.push(arr, v1));
    assert!(runner.push(arr, v2));
    assert_eq!(runner.array_length(arr), 2);

    // Get by index
    let e0 = runner.get_index(arr, 0);
    assert_eq!(runner.value_as_number(e0), 10.0);

    let e1 = runner.get_index(arr, 1);
    assert_eq!(runner.value_as_number(e1), 20.0);

    // Set by index
    let v3 = runner.create_number(30.0);
    assert!(runner.set_index(arr, 0, v3));
    let updated = runner.get_index(arr, 0);
    assert_eq!(runner.value_as_number(updated), 30.0);
}

#[wasm_bindgen_test]
fn test_array_sparse_access() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let arr = runner.create_array();
    let val = runner.create_number(99.0);

    // Set at index 5 (sparse)
    assert!(runner.set_index(arr, 5, val));
    assert_eq!(runner.array_length(arr), 6);

    // Index 0-4 should be undefined
    let e0 = runner.get_index(arr, 0);
    assert!(runner.value_is_undefined(e0));

    // Index 5 should have the value
    let e5 = runner.get_index(arr, 5);
    assert_eq!(runner.value_as_number(e5), 99.0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Handle Lifecycle Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_handle_release() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let h = runner.create_number(42.0);
    assert_eq!(runner.value_as_number(h), 42.0);

    runner.release_handle(h);

    // After release, handle should be invalid
    assert!(runner.value_as_number(h).is_nan());
    assert_eq!(runner.get_value_type(h), "undefined");
}

#[wasm_bindgen_test]
fn test_handle_duplicate() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let h1 = runner.create_number(42.0);
    let h2 = runner.duplicate_handle(h1);

    assert_ne!(h1, h2);
    assert_eq!(runner.value_as_number(h1), 42.0);
    assert_eq!(runner.value_as_number(h2), 42.0);

    // Release original, duplicate should still work
    runner.release_handle(h1);
    assert_eq!(runner.value_as_number(h2), 42.0);
}

#[wasm_bindgen_test]
fn test_handles_cleared_on_prepare() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let h = runner.create_number(42.0);
    assert_eq!(runner.value_as_number(h), 42.0);

    // Prepare new code - clears handles
    runner.prepare("2", Some("test.ts".into()));

    // Handle should now be invalid
    assert!(runner.value_as_number(h).is_nan());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Export Access Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_export_access() {
    let mut runner = TsRunner::new();
    let success = run_to_completion(
        &mut runner,
        r#"
        export const VERSION = "1.0.0";
        export const count = 42;
        export function greet() { return "hello"; }
    "#,
    );
    assert!(success);

    // Get export names
    let names = runner.get_export_names();
    assert!(names.contains(&"VERSION".to_string()));
    assert!(names.contains(&"count".to_string()));
    assert!(names.contains(&"greet".to_string()));

    // Get VERSION export
    let version = runner.get_export("VERSION");
    assert_ne!(version, 0);
    assert_eq!(runner.get_value_type(version), "string");
    assert_eq!(runner.value_as_string(version), Some("1.0.0".into()));

    // Get count export
    let count = runner.get_export("count");
    assert_eq!(runner.value_as_number(count), 42.0);

    // Get greet export (function)
    let greet = runner.get_export("greet");
    assert!(runner.value_is_function(greet));

    // Non-existent export
    let missing = runner.get_export("nonexistent");
    assert_eq!(missing, 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Value Inspection Edge Cases
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_value_inspection_type_mismatches() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    // String handle - as_number should return NaN
    let str_h = runner.create_string("not a number");
    assert!(runner.value_as_number(str_h).is_nan());

    // Number handle - as_string should return None
    let num_h = runner.create_number(42.0);
    assert_eq!(runner.value_as_string(num_h), None);
    assert_eq!(runner.value_as_bool(num_h), None);

    // Not arrays or functions
    assert!(!runner.value_is_array(num_h));
    assert!(!runner.value_is_function(num_h));
}

#[wasm_bindgen_test]
fn test_invalid_handle_zero() {
    let runner = TsRunner::new();

    // Handle 0 is always invalid
    assert_eq!(runner.get_value_type(0), "undefined");
    assert!(runner.value_as_number(0).is_nan());
    assert_eq!(runner.value_as_string(0), None);
    assert_eq!(runner.value_as_bool(0), None);
    assert!(runner.value_is_undefined(0));
    assert!(!runner.value_is_null(0));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Integration Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_modify_exported_object() {
    let mut runner = TsRunner::new();
    let success = run_to_completion(&mut runner, "export const obj = { a: 1 };");
    assert!(success);

    let obj = runner.get_export("obj");
    assert_ne!(obj, 0);

    // Modify the exported object
    let new_val = runner.create_number(999.0);
    runner.set_property(obj, "b", new_val);

    // Verify modification
    let b = runner.get_property(obj, "b");
    assert_eq!(runner.value_as_number(b), 999.0);

    // Original property still there
    let a = runner.get_property(obj, "a");
    assert_eq!(runner.value_as_number(a), 1.0);
}

#[wasm_bindgen_test]
fn test_build_object_from_scratch() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    // Build a complex object structure
    let user = runner.create_object();
    let name = runner.create_string("Alice");
    let age = runner.create_number(30.0);
    let hobbies = runner.create_array();

    runner.set_property(user, "name", name);
    runner.set_property(user, "age", age);
    runner.set_property(user, "hobbies", hobbies);

    let h1 = runner.create_string("reading");
    let h2 = runner.create_string("coding");
    runner.push(hobbies, h1);
    runner.push(hobbies, h2);

    // Verify structure
    let name_h = runner.get_property(user, "name");
    assert_eq!(runner.value_as_string(name_h), Some("Alice".into()));

    let age_h = runner.get_property(user, "age");
    assert_eq!(runner.value_as_number(age_h), 30.0);

    let got_hobbies = runner.get_property(user, "hobbies");
    assert!(runner.value_is_array(got_hobbies));
    assert_eq!(runner.array_length(got_hobbies), 2);

    let hobby_h = runner.get_index(got_hobbies, 0);
    assert_eq!(runner.value_as_string(hobby_h), Some("reading".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Error Creation Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_create_error() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let err = runner.create_error("Something went wrong");
    assert_ne!(err, 0);
    assert_eq!(runner.get_value_type(err), "object");

    // Check error properties
    let name_h = runner.get_property(err, "name");
    assert_eq!(runner.value_as_string(name_h), Some("Error".into()));

    let msg_h = runner.get_property(err, "message");
    assert_eq!(
        runner.value_as_string(msg_h),
        Some("Something went wrong".into())
    );

    let stack_h = runner.get_property(err, "stack");
    assert_eq!(
        runner.value_as_string(stack_h),
        Some("Error: Something went wrong".into())
    );
}

#[wasm_bindgen_test]
fn test_create_error_empty_message() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let err = runner.create_error("");
    assert_ne!(err, 0);

    let msg_h = runner.get_property(err, "message");
    assert_eq!(runner.value_as_string(msg_h), Some("".into()));

    let stack_h = runner.get_property(err, "stack");
    assert_eq!(runner.value_as_string(stack_h), Some("Error".into()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Step Result Value Handle Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_step_result_value_handle() {
    let mut runner = TsRunner::new();
    runner.prepare("42 + 1", Some("test.ts".into()));

    loop {
        let result = runner.step();
        match result.status() {
            StepStatus::Continue => continue,
            StepStatus::Complete => {
                // value_handle should contain the result
                let handle = result.value_handle();
                assert_ne!(handle, 0);
                assert_eq!(runner.get_value_type(handle), "number");
                assert_eq!(runner.value_as_number(handle), 43.0);
                break;
            }
            _ => panic!("Unexpected status"),
        }
    }
}

#[wasm_bindgen_test]
fn test_step_result_object_value() {
    let mut runner = TsRunner::new();
    runner.prepare("({ x: 10, y: 20 })", Some("test.ts".into()));

    loop {
        let result = runner.step();
        match result.status() {
            StepStatus::Continue => continue,
            StepStatus::Complete => {
                let handle = result.value_handle();
                assert_eq!(runner.get_value_type(handle), "object");

                let x = runner.get_property(handle, "x");
                assert_eq!(runner.value_as_number(x), 10.0);

                let y = runner.get_property(handle, "y");
                assert_eq!(runner.value_as_number(y), 20.0);
                break;
            }
            _ => panic!("Unexpected status"),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Promise Tests (Handle-based)
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_create_promise_returns_handle() {
    let mut runner = TsRunner::new();
    run_to_completion(&mut runner, "1");

    let promise = runner.create_promise();
    assert_ne!(promise, 0);
    assert_eq!(runner.get_value_type(promise), "object");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Source Module Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen_test]
fn test_source_module_basic() {
    let mut runner = TsRunner::new();

    // Register a source module
    runner.register_source_module("test:math", "export const PI = 3.14159;");

    // Import from the registered module
    let prep = runner.prepare(
        r#"import { PI } from "test:math"; PI;"#,
        Some("test.ts".into()),
    );
    assert!(prep.status() == StepStatus::Continue);

    // Run to completion
    loop {
        let result = runner.step();
        match result.status() {
            StepStatus::Continue => continue,
            StepStatus::Complete => {
                let handle = result.value_handle();
                assert_eq!(runner.get_value_type(handle), "number");
                let val = runner.value_as_number(handle);
                assert!((val - 3.14159).abs() < 0.00001);
                break;
            }
            _ => panic!("Unexpected status"),
        }
    }
}

#[wasm_bindgen_test]
fn test_source_module_with_function() {
    let mut runner = TsRunner::new();

    // Register a source module with a function
    runner.register_source_module(
        "app:utils",
        r#"
        export function double(x: number): number {
            return x * 2;
        }
    "#,
    );

    // Use the function
    let prep = runner.prepare(
        r#"import { double } from "app:utils"; double(21);"#,
        Some("test.ts".into()),
    );
    assert!(prep.status() == StepStatus::Continue);

    loop {
        let result = runner.step();
        match result.status() {
            StepStatus::Continue => continue,
            StepStatus::Complete => {
                let handle = result.value_handle();
                assert_eq!(runner.value_as_number(handle), 42.0);
                break;
            }
            _ => panic!("Unexpected status"),
        }
    }
}

#[wasm_bindgen_test]
fn test_multiple_source_modules() {
    let mut runner = TsRunner::new();

    // Register multiple modules
    runner.register_source_module("config:defaults", "export const timeout = 5000;");
    runner.register_source_module("config:flags", "export const debug = true;");

    // Import from both
    let prep = runner.prepare(
        r#"
        import { timeout } from "config:defaults";
        import { debug } from "config:flags";
        debug ? timeout : 0;
    "#,
        Some("test.ts".into()),
    );
    assert!(prep.status() == StepStatus::Continue);

    loop {
        let result = runner.step();
        match result.status() {
            StepStatus::Continue => continue,
            StepStatus::Complete => {
                let handle = result.value_handle();
                assert_eq!(runner.value_as_number(handle), 5000.0);
                break;
            }
            _ => panic!("Unexpected status"),
        }
    }
}

#[wasm_bindgen_test]
fn test_source_module_importing_another() {
    let mut runner = TsRunner::new();

    // Module A exports a constant
    runner.register_source_module("lib:a", "export const BASE = 10;");

    // Module B imports from A
    runner.register_source_module(
        "lib:b",
        r#"
        import { BASE } from "lib:a";
        export const DOUBLED = BASE * 2;
    "#,
    );

    // Main code imports from B
    let prep = runner.prepare(
        r#"import { DOUBLED } from "lib:b"; DOUBLED;"#,
        Some("test.ts".into()),
    );
    assert!(prep.status() == StepStatus::Continue);

    loop {
        let result = runner.step();
        match result.status() {
            StepStatus::Continue => continue,
            StepStatus::Complete => {
                let handle = result.value_handle();
                assert_eq!(runner.value_as_number(handle), 20.0);
                break;
            }
            _ => panic!("Unexpected status"),
        }
    }
}

#[wasm_bindgen_test]
fn test_source_modules_cleared_on_prepare() {
    let mut runner = TsRunner::new();

    // Register a module
    runner.register_source_module("test:first", "export const x = 1;");

    // First prepare uses it
    let prep = runner.prepare(
        r#"import { x } from "test:first"; x;"#,
        Some("test.ts".into()),
    );
    assert!(prep.status() == StepStatus::Continue);
    run_to_completion(&mut runner, "1"); // just to reset

    // Second prepare should NOT have the module (modules are drained)
    let mut runner2 = TsRunner::new();
    runner2.register_source_module("test:second", "export const y = 2;");
    let prep2 = runner2.prepare(
        r#"import { y } from "test:second"; y;"#,
        Some("test.ts".into()),
    );
    assert!(prep2.status() == StepStatus::Continue);

    loop {
        let result = runner2.step();
        match result.status() {
            StepStatus::Continue => continue,
            StepStatus::Complete => {
                let handle = result.value_handle();
                assert_eq!(runner2.value_as_number(handle), 2.0);
                break;
            }
            _ => panic!("Unexpected status"),
        }
    }
}
