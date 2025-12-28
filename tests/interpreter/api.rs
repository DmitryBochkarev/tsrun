//! Tests for the public API ergonomics

use tsrun::{JsValue, Runtime, RuntimeResult, api};

// ═══════════════════════════════════════════════════════════════════════════════
// JsValue Type Check Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_is_undefined() {
    assert!(JsValue::Undefined.is_undefined());
    assert!(!JsValue::Null.is_undefined());
    assert!(!JsValue::Boolean(true).is_undefined());
    assert!(!JsValue::Number(42.0).is_undefined());
    assert!(!JsValue::from("hello").is_undefined());
}

#[test]
fn test_is_null() {
    assert!(JsValue::Null.is_null());
    assert!(!JsValue::Undefined.is_null());
    assert!(!JsValue::Boolean(false).is_null());
    assert!(!JsValue::Number(0.0).is_null());
}

#[test]
fn test_is_nullish() {
    assert!(JsValue::Undefined.is_nullish());
    assert!(JsValue::Null.is_nullish());
    assert!(!JsValue::Boolean(false).is_nullish());
    assert!(!JsValue::Number(0.0).is_nullish());
    assert!(!JsValue::from("").is_nullish());
}

#[test]
fn test_is_boolean() {
    assert!(JsValue::Boolean(true).is_boolean());
    assert!(JsValue::Boolean(false).is_boolean());
    assert!(!JsValue::Undefined.is_boolean());
    assert!(!JsValue::Number(1.0).is_boolean());
}

#[test]
fn test_is_number() {
    assert!(JsValue::Number(42.0).is_number());
    assert!(JsValue::Number(0.0).is_number());
    assert!(JsValue::Number(f64::NAN).is_number());
    assert!(JsValue::Number(f64::INFINITY).is_number());
    assert!(!JsValue::Boolean(true).is_number());
    assert!(!JsValue::from("42").is_number());
}

#[test]
fn test_is_string() {
    assert!(JsValue::from("hello").is_string());
    assert!(JsValue::from("").is_string());
    assert!(!JsValue::Number(42.0).is_string());
    assert!(!JsValue::Undefined.is_string());
}

#[test]
fn test_type_name() {
    assert_eq!(JsValue::Undefined.type_name(), "undefined");
    assert_eq!(JsValue::Null.type_name(), "null");
    assert_eq!(JsValue::Boolean(true).type_name(), "boolean");
    assert_eq!(JsValue::Number(42.0).type_name(), "number");
    assert_eq!(JsValue::from("hello").type_name(), "string");
}

// ═══════════════════════════════════════════════════════════════════════════════
// JsValue Extraction Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_as_bool() {
    assert_eq!(JsValue::Boolean(true).as_bool(), Some(true));
    assert_eq!(JsValue::Boolean(false).as_bool(), Some(false));
    assert_eq!(JsValue::Number(1.0).as_bool(), None);
    assert_eq!(JsValue::from("true").as_bool(), None);
    assert_eq!(JsValue::Undefined.as_bool(), None);
}

#[test]
fn test_as_number() {
    assert_eq!(JsValue::Number(42.0).as_number(), Some(42.0));
    assert_eq!(JsValue::Number(0.0).as_number(), Some(0.0));
    assert_eq!(JsValue::Number(-3.14).as_number(), Some(-3.14));
    assert_eq!(JsValue::Boolean(true).as_number(), None);
    assert_eq!(JsValue::from("42").as_number(), None);

    // NaN is still Some(NaN)
    let nan = JsValue::Number(f64::NAN).as_number();
    assert!(nan.is_some());
    assert!(nan.unwrap().is_nan());
}

#[test]
fn test_as_str() {
    assert_eq!(JsValue::from("hello").as_str(), Some("hello"));
    assert_eq!(JsValue::from("").as_str(), Some(""));
    assert_eq!(JsValue::from("with spaces").as_str(), Some("with spaces"));
    assert_eq!(JsValue::Number(42.0).as_str(), None);
    assert_eq!(JsValue::Boolean(true).as_str(), None);
    assert_eq!(JsValue::Undefined.as_str(), None);
}

#[test]
fn test_as_js_string() {
    let value = JsValue::from("hello");
    assert!(value.as_js_string().is_some());
    assert_eq!(value.as_js_string().unwrap().as_str(), "hello");

    assert!(JsValue::Number(42.0).as_js_string().is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
// From/Into Trait Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_from_bool() {
    assert_eq!(JsValue::from(true), JsValue::Boolean(true));
    assert_eq!(JsValue::from(false), JsValue::Boolean(false));

    // Test Into as well
    let v: JsValue = true.into();
    assert_eq!(v, JsValue::Boolean(true));
}

#[test]
fn test_from_f64() {
    assert_eq!(JsValue::from(42.0f64), JsValue::Number(42.0));
    assert_eq!(JsValue::from(0.0f64), JsValue::Number(0.0));
    assert_eq!(JsValue::from(-3.14f64), JsValue::Number(-3.14));

    let v: JsValue = 42.0f64.into();
    assert_eq!(v, JsValue::Number(42.0));
}

#[test]
fn test_from_i32() {
    assert_eq!(JsValue::from(42i32), JsValue::Number(42.0));
    assert_eq!(JsValue::from(0i32), JsValue::Number(0.0));
    assert_eq!(JsValue::from(-100i32), JsValue::Number(-100.0));
}

#[test]
fn test_from_i64() {
    assert_eq!(JsValue::from(42i64), JsValue::Number(42.0));
    assert_eq!(JsValue::from(1_000_000i64), JsValue::Number(1_000_000.0));
}

#[test]
fn test_from_u32() {
    assert_eq!(JsValue::from(42u32), JsValue::Number(42.0));
    assert_eq!(JsValue::from(0u32), JsValue::Number(0.0));
}

#[test]
fn test_from_u64() {
    assert_eq!(JsValue::from(42u64), JsValue::Number(42.0));
}

#[test]
fn test_from_usize() {
    assert_eq!(JsValue::from(42usize), JsValue::Number(42.0));
}

#[test]
fn test_from_unit() {
    assert_eq!(JsValue::from(()), JsValue::Undefined);

    let v: JsValue = ().into();
    assert_eq!(v, JsValue::Undefined);
}

#[test]
fn test_from_str() {
    let v = JsValue::from("hello");
    assert_eq!(v.as_str(), Some("hello"));

    let v = JsValue::from("");
    assert_eq!(v.as_str(), Some(""));
}

#[test]
fn test_from_string() {
    let v = JsValue::from(String::from("hello"));
    assert_eq!(v.as_str(), Some("hello"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Value Creation Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_create_value_number() {
    let v = api::create_value(42);
    assert!(v.is_number());
    assert_eq!(v.as_number(), Some(42.0));
}

#[test]
fn test_create_value_f64() {
    let v = api::create_value(3.14f64);
    assert!(v.is_number());
    assert_eq!(v.as_number(), Some(3.14));
}

#[test]
fn test_create_value_bool() {
    let v = api::create_value(true);
    assert!(v.is_boolean());
    assert_eq!(v.as_bool(), Some(true));
}

#[test]
fn test_create_value_string() {
    let v = api::create_value("hello");
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("hello"));
}

#[test]
fn test_create_value_string_owned() {
    let v = api::create_value(String::from("world"));
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("world"));
}

#[test]
fn test_create_undefined() {
    let v = api::create_undefined();
    assert!(v.is_undefined());
}

#[test]
fn test_create_null() {
    let v = api::create_null();
    assert!(v.is_null());
}

#[test]
fn test_create_from_json_object() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let v = api::create_from_json(
        &mut runtime,
        &guard,
        &serde_json::json!({
            "name": "Alice",
            "age": 30
        }),
    )
    .unwrap();

    assert!(v.is_object());

    if let Some(obj) = v.as_object() {
        let borrowed = obj.borrow();
        let name_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("name"));
        let age_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("age"));

        assert_eq!(
            borrowed.get_property(&name_key).unwrap().as_str(),
            Some("Alice")
        );
        assert_eq!(
            borrowed.get_property(&age_key).unwrap().as_number(),
            Some(30.0)
        );
    }
}

#[test]
fn test_create_from_json_array() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let v =
        api::create_from_json(&mut runtime, &guard, &serde_json::json!([1, 2, 3, 4, 5])).unwrap();

    assert!(v.is_object());

    if let Some(obj) = v.as_object() {
        let borrowed = obj.borrow();
        if let Some(elements) = borrowed.array_elements() {
            assert_eq!(elements.len(), 5);
            assert_eq!(elements[0].as_number(), Some(1.0));
            assert_eq!(elements[4].as_number(), Some(5.0));
        }
    }
}

#[test]
fn test_create_from_json_nested() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let v = api::create_from_json(
        &mut runtime,
        &guard,
        &serde_json::json!({
            "user": {
                "name": "Bob",
                "scores": [95, 87, 92]
            }
        }),
    )
    .unwrap();

    assert!(v.is_object());
    // The structure is created and guarded properly
}

#[test]
fn test_create_object_empty() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let v = api::create_object(&mut runtime, &guard).unwrap();
    assert!(v.is_object());
}

#[test]
fn test_create_array_empty() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let v = api::create_array(&mut runtime, &guard).unwrap();
    assert!(v.is_object());

    if let Some(obj) = v.as_object() {
        let borrowed = obj.borrow();
        assert_eq!(borrowed.array_length(), Some(0));
    }
}

#[test]
fn test_create_from_json_primitives() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);

    // JSON null becomes JsValue::Null
    let v = api::create_from_json(&mut runtime, &guard, &serde_json::json!(null)).unwrap();
    assert!(v.is_null());

    // JSON boolean
    let v = api::create_from_json(&mut runtime, &guard, &serde_json::json!(true)).unwrap();
    assert!(v.is_boolean());
    assert_eq!(v.as_bool(), Some(true));

    // JSON number
    let v = api::create_from_json(&mut runtime, &guard, &serde_json::json!(42)).unwrap();
    assert!(v.is_number());
    assert_eq!(v.as_number(), Some(42.0));

    // JSON string
    let v = api::create_from_json(&mut runtime, &guard, &serde_json::json!("hello")).unwrap();
    assert!(v.is_string());
    assert_eq!(v.as_str(), Some("hello"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Display Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_display_undefined() {
    assert_eq!(format!("{}", JsValue::Undefined), "undefined");
}

#[test]
fn test_display_null() {
    assert_eq!(format!("{}", JsValue::Null), "null");
}

#[test]
fn test_display_boolean() {
    assert_eq!(format!("{}", JsValue::Boolean(true)), "true");
    assert_eq!(format!("{}", JsValue::Boolean(false)), "false");
}

#[test]
fn test_display_number() {
    assert_eq!(format!("{}", JsValue::Number(42.0)), "42");
    assert_eq!(format!("{}", JsValue::Number(3.14)), "3.14");
    assert_eq!(format!("{}", JsValue::Number(0.0)), "0");
    assert_eq!(format!("{}", JsValue::Number(-1.0)), "-1");
    assert_eq!(format!("{}", JsValue::Number(f64::INFINITY)), "Infinity");
    assert_eq!(
        format!("{}", JsValue::Number(f64::NEG_INFINITY)),
        "-Infinity"
    );
    assert_eq!(format!("{}", JsValue::Number(f64::NAN)), "NaN");
}

#[test]
fn test_display_string() {
    assert_eq!(format!("{}", JsValue::from("hello")), "hello");
    assert_eq!(format!("{}", JsValue::from("")), "");
    assert_eq!(format!("{}", JsValue::from("with spaces")), "with spaces");
}

// ═══════════════════════════════════════════════════════════════════════════════
// RuntimeValue Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_runtime_value_is_number() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("42").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_number());
        assert!(!rv.is_string());
        assert!(!rv.is_undefined());
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_runtime_value_as_number() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("42").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert_eq!(rv.as_number(), Some(42.0));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_runtime_value_is_string() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("'hello'").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_string());
        assert_eq!(rv.as_str(), Some("hello"));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_runtime_value_is_boolean() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("true").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_boolean());
        assert_eq!(rv.as_bool(), Some(true));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_runtime_value_is_undefined() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("undefined").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_undefined());
        assert!(rv.is_nullish());
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_runtime_value_is_null() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("null").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_null());
        assert!(rv.is_nullish());
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_runtime_value_type_name() {
    let mut runtime = Runtime::new();

    let mut check = |code: &str, expected: &str| {
        let result = runtime.eval(code).unwrap();
        if let RuntimeResult::Complete(rv) = result {
            assert_eq!(rv.type_name(), expected, "for code: {}", code);
        } else {
            panic!("Expected Complete for: {}", code);
        }
    };

    check("undefined", "undefined");
    check("null", "null");
    check("true", "boolean");
    check("42", "number");
    check("'hello'", "string");
    check("({})", "object"); // Parentheses force object literal
    check("[]", "object");
}

#[test]
fn test_runtime_value_display() {
    let mut runtime = Runtime::new();

    let mut check = |code: &str, expected: &str| {
        let result = runtime.eval(code).unwrap();
        if let RuntimeResult::Complete(rv) = result {
            assert_eq!(format!("{}", rv), expected, "for code: {}", code);
        } else {
            panic!("Expected Complete for: {}", code);
        }
    };

    check("undefined", "undefined");
    check("null", "null");
    check("true", "true");
    check("false", "false");
    check("42", "42");
    check("3.14", "3.14");
    check("'hello'", "hello");
    check("({})", "[object Object]"); // Parentheses force object literal
    check("[]", "[object Object]");
}

#[test]
fn test_runtime_value_is_object() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("({})").unwrap(); // Parentheses force object literal

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_object());
        assert!(!rv.is_number());
        assert!(!rv.is_string());
    } else {
        panic!("Expected Complete");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Edge Cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_display_large_numbers() {
    // Very large number - should use exponential
    assert_eq!(format!("{}", JsValue::Number(1e21)), "1e+21");

    // Just under the threshold
    assert_eq!(
        format!("{}", JsValue::Number(999999999999999999999.0)),
        "1e+21"
    );
}

#[test]
fn test_display_small_numbers() {
    // Very small number - should use exponential
    assert_eq!(format!("{}", JsValue::Number(1e-7)), "1e-7");
}

#[test]
fn test_from_integer_precision() {
    // Large integers may lose precision when converted to f64
    let large: i64 = 9007199254740993; // 2^53 + 1
    let value = JsValue::from(large);
    // This demonstrates the precision loss inherent in JS numbers
    assert!(value.is_number());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Complex Object Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_object_property_access() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("({ name: 'Alice', age: 30 })").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_object());
        assert_eq!(rv.type_name(), "object");

        // Access the underlying object
        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            // Get 'name' property
            let name_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("name"));
            let name = borrowed.get_property(&name_key);
            assert!(name.is_some());
            assert_eq!(name.unwrap().as_str(), Some("Alice"));

            // Get 'age' property
            let age_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("age"));
            let age = borrowed.get_property(&age_key);
            assert!(age.is_some());
            assert_eq!(age.unwrap().as_number(), Some(30.0));
        } else {
            panic!("Expected object");
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_nested_object() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval("({ user: { name: 'Bob', settings: { theme: 'dark' } } })")
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_object());

        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            // Get 'user' property
            let user_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("user"));
            let user = borrowed.get_property(&user_key);
            assert!(user.is_some());

            // User should be an object
            let user_val = user.unwrap();
            assert!(user_val.is_object());

            if let Some(user_obj) = user_val.as_object() {
                let user_borrowed = user_obj.borrow();

                // Get 'name' from user
                let name_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("name"));
                let name = user_borrowed.get_property(&name_key);
                assert_eq!(name.unwrap().as_str(), Some("Bob"));

                // Get 'settings' from user
                let settings_key =
                    tsrun::value::PropertyKey::String(tsrun::JsString::from("settings"));
                let settings = user_borrowed.get_property(&settings_key);
                assert!(settings.is_some());

                if let Some(settings_obj) = settings.unwrap().as_object() {
                    let settings_borrowed = settings_obj.borrow();
                    let theme_key =
                        tsrun::value::PropertyKey::String(tsrun::JsString::from("theme"));
                    let theme = settings_borrowed.get_property(&theme_key);
                    assert_eq!(theme.unwrap().as_str(), Some("dark"));
                }
            }
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_array_access() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("[1, 2, 3, 4, 5]").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_object()); // Arrays are objects in JS

        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            // Check array length
            assert_eq!(borrowed.array_length(), Some(5));

            // Access elements directly
            if let Some(elements) = borrowed.array_elements() {
                assert_eq!(elements.len(), 5);
                assert_eq!(elements[0].as_number(), Some(1.0));
                assert_eq!(elements[1].as_number(), Some(2.0));
                assert_eq!(elements[2].as_number(), Some(3.0));
                assert_eq!(elements[3].as_number(), Some(4.0));
                assert_eq!(elements[4].as_number(), Some(5.0));
            } else {
                panic!("Expected array elements");
            }
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_array_of_strings() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("['apple', 'banana', 'cherry']").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            if let Some(elements) = borrowed.array_elements() {
                assert_eq!(elements.len(), 3);
                assert_eq!(elements[0].as_str(), Some("apple"));
                assert_eq!(elements[1].as_str(), Some("banana"));
                assert_eq!(elements[2].as_str(), Some("cherry"));
            }
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_array_of_objects() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval("[{ id: 1, name: 'Alice' }, { id: 2, name: 'Bob' }]")
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            if let Some(elements) = borrowed.array_elements() {
                assert_eq!(elements.len(), 2);

                // Check first element
                assert!(elements[0].is_object());
                if let Some(first) = elements[0].as_object() {
                    let first_borrowed = first.borrow();
                    let id_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("id"));
                    let name_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("name"));

                    assert_eq!(
                        first_borrowed.get_property(&id_key).unwrap().as_number(),
                        Some(1.0)
                    );
                    assert_eq!(
                        first_borrowed.get_property(&name_key).unwrap().as_str(),
                        Some("Alice")
                    );
                }

                // Check second element
                if let Some(second) = elements[1].as_object() {
                    let second_borrowed = second.borrow();
                    let id_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("id"));
                    let name_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("name"));

                    assert_eq!(
                        second_borrowed.get_property(&id_key).unwrap().as_number(),
                        Some(2.0)
                    );
                    assert_eq!(
                        second_borrowed.get_property(&name_key).unwrap().as_str(),
                        Some("Bob")
                    );
                }
            }
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_mixed_array() {
    let mut runtime = Runtime::new();
    let result = runtime
        .eval("[1, 'two', true, null, undefined, { x: 3 }]")
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            if let Some(elements) = borrowed.array_elements() {
                assert_eq!(elements.len(), 6);

                assert_eq!(elements[0].as_number(), Some(1.0));
                assert_eq!(elements[1].as_str(), Some("two"));
                assert_eq!(elements[2].as_bool(), Some(true));
                assert!(elements[3].is_null());
                assert!(elements[4].is_undefined());
                assert!(elements[5].is_object());
            }
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_function_result() {
    let mut runtime = Runtime::new();

    // Define and call a function
    let result = runtime
        .eval(
            r#"
        function add(a, b) {
            return a + b;
        }
        add(10, 20)
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_number());
        assert_eq!(rv.as_number(), Some(30.0));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_function_returning_object() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        function createUser(name, age) {
            return { name, age, active: true };
        }
        createUser('Charlie', 25)
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_object());

        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            let name_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("name"));
            let age_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("age"));
            let active_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("active"));

            assert_eq!(
                borrowed.get_property(&name_key).unwrap().as_str(),
                Some("Charlie")
            );
            assert_eq!(
                borrowed.get_property(&age_key).unwrap().as_number(),
                Some(25.0)
            );
            assert_eq!(
                borrowed.get_property(&active_key).unwrap().as_bool(),
                Some(true)
            );
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_class_instance() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        class Person {
            name: string;
            age: number;

            constructor(name: string, age: number) {
                this.name = name;
                this.age = age;
            }

            greet(): string {
                return "Hello, " + this.name;
            }
        }

        new Person('David', 35)
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_object());

        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            let name_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("name"));
            let age_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("age"));

            assert_eq!(
                borrowed.get_property(&name_key).unwrap().as_str(),
                Some("David")
            );
            assert_eq!(
                borrowed.get_property(&age_key).unwrap().as_number(),
                Some(35.0)
            );
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_map_object() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const map = new Map();
        map.set('key1', 'value1');
        map.set('key2', 42);
        map.size
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_number());
        assert_eq!(rv.as_number(), Some(2.0));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_date_object() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const d = new Date(2024, 0, 15); // Jan 15, 2024
        d.getFullYear()
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_number());
        assert_eq!(rv.as_number(), Some(2024.0));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_json_parse_result() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const data = JSON.parse('{"name":"Eve","scores":[95,87,92]}');
        data
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_object());

        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();

            let name_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("name"));
            assert_eq!(
                borrowed.get_property(&name_key).unwrap().as_str(),
                Some("Eve")
            );

            let scores_key = tsrun::value::PropertyKey::String(tsrun::JsString::from("scores"));
            let scores = borrowed.get_property(&scores_key).unwrap();
            assert!(scores.is_object());

            if let Some(scores_arr) = scores.as_object() {
                let scores_borrowed = scores_arr.borrow();
                if let Some(elements) = scores_borrowed.array_elements() {
                    assert_eq!(elements.len(), 3);
                    assert_eq!(elements[0].as_number(), Some(95.0));
                    assert_eq!(elements[1].as_number(), Some(87.0));
                    assert_eq!(elements[2].as_number(), Some(92.0));
                }
            }
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_json_stringify() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(r#"JSON.stringify({ a: 1, b: "hello", c: true })"#)
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_string());
        let s = rv.as_str().unwrap();
        // The order might vary, so just check it contains the expected parts
        assert!(s.contains("\"a\":1"));
        assert!(s.contains("\"b\":\"hello\""));
        assert!(s.contains("\"c\":true"));
    } else {
        panic!("Expected Complete");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helper Function for Property Access
// ═══════════════════════════════════════════════════════════════════════════════

/// Helper to get a property from a JsValue object
fn get_property(value: &JsValue, key: &str) -> Option<JsValue> {
    if let Some(obj) = value.as_object() {
        let borrowed = obj.borrow();
        let prop_key = tsrun::value::PropertyKey::String(tsrun::JsString::from(key));
        borrowed.get_property(&prop_key)
    } else {
        None
    }
}

#[test]
fn test_helper_get_property() {
    let mut runtime = Runtime::new();
    let result = runtime.eval("({ foo: 42, bar: 'baz' })").unwrap();

    if let RuntimeResult::Complete(rv) = result {
        let foo = get_property(rv.value(), "foo");
        assert_eq!(foo.unwrap().as_number(), Some(42.0));

        let bar = get_property(rv.value(), "bar");
        assert_eq!(bar.unwrap().as_str(), Some("baz"));

        let missing = get_property(rv.value(), "missing");
        assert!(missing.is_none());
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_deeply_nested_structure() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        ({
            level1: {
                level2: {
                    level3: {
                        level4: {
                            value: "deep"
                        }
                    }
                }
            }
        })
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        // Navigate down the chain
        let l1 = get_property(rv.value(), "level1").unwrap();
        let l2 = get_property(&l1, "level2").unwrap();
        let l3 = get_property(&l2, "level3").unwrap();
        let l4 = get_property(&l3, "level4").unwrap();
        let value = get_property(&l4, "value").unwrap();

        assert_eq!(value.as_str(), Some("deep"));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_array_with_computed_values() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const arr = [];
        for (let i = 0; i < 5; i++) {
            arr.push(i * i);
        }
        arr
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();
            if let Some(elements) = borrowed.array_elements() {
                assert_eq!(elements.len(), 5);
                assert_eq!(elements[0].as_number(), Some(0.0)); // 0*0
                assert_eq!(elements[1].as_number(), Some(1.0)); // 1*1
                assert_eq!(elements[2].as_number(), Some(4.0)); // 2*2
                assert_eq!(elements[3].as_number(), Some(9.0)); // 3*3
                assert_eq!(elements[4].as_number(), Some(16.0)); // 4*4
            }
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_object_with_methods() {
    let mut runtime = Runtime::new();

    // Create object with method and call it
    let result = runtime
        .eval(
            r#"
        const obj = {
            value: 10,
            double() {
                return this.value * 2;
            }
        };
        obj.double()
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        assert!(rv.is_number());
        assert_eq!(rv.as_number(), Some(20.0));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_spread_operator_result() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const a = [1, 2];
        const b = [3, 4];
        [...a, ...b]
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        if let Some(obj) = rv.as_object() {
            let borrowed = obj.borrow();
            if let Some(elements) = borrowed.array_elements() {
                assert_eq!(elements.len(), 4);
                assert_eq!(elements[0].as_number(), Some(1.0));
                assert_eq!(elements[1].as_number(), Some(2.0));
                assert_eq!(elements[2].as_number(), Some(3.0));
                assert_eq!(elements[3].as_number(), Some(4.0));
            }
        }
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_object_spread() {
    let mut runtime = Runtime::new();

    let result = runtime
        .eval(
            r#"
        const a = { x: 1, y: 2 };
        const b = { y: 3, z: 4 };
        ({ ...a, ...b })
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        // y should be 3 (b overwrites a)
        let x = get_property(rv.value(), "x");
        let y = get_property(rv.value(), "y");
        let z = get_property(rv.value(), "z");

        assert_eq!(x.unwrap().as_number(), Some(1.0));
        assert_eq!(y.unwrap().as_number(), Some(3.0)); // Overwritten by b
        assert_eq!(z.unwrap().as_number(), Some(4.0));
    } else {
        panic!("Expected Complete");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API Property and Array Access Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_api_get_property() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let user = api::create_from_json(
        &mut runtime,
        &guard,
        &serde_json::json!({"name": "Alice", "age": 30}),
    )
    .unwrap();

    let name_val = api::get_property(&user, "name").unwrap();
    assert_eq!(name_val.as_str(), Some("Alice"));
    let age_val = api::get_property(&user, "age").unwrap();
    assert_eq!(age_val.as_number(), Some(30.0));
    let missing = api::get_property(&user, "missing").unwrap();
    assert!(missing.is_undefined());
}

#[test]
fn test_api_get_index() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr =
        api::create_from_json(&mut runtime, &guard, &serde_json::json!([10, 20, 30])).unwrap();

    assert_eq!(api::get_index(&arr, 0).unwrap().as_number(), Some(10.0));
    assert_eq!(api::get_index(&arr, 1).unwrap().as_number(), Some(20.0));
    assert_eq!(api::get_index(&arr, 2).unwrap().as_number(), Some(30.0));
    assert!(api::get_index(&arr, 10).unwrap().is_undefined());
}

#[test]
fn test_api_len() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr =
        api::create_from_json(&mut runtime, &guard, &serde_json::json!([1, 2, 3, 4, 5])).unwrap();

    assert_eq!(api::len(&arr), Some(5));

    // Non-array returns None
    let obj = api::create_from_json(&mut runtime, &guard, &serde_json::json!({"x": 1})).unwrap();
    assert_eq!(api::len(&obj), None);
}

#[test]
fn test_api_is_empty() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);

    let empty = api::create_array(&mut runtime, &guard).unwrap();
    assert_eq!(api::is_empty(&empty), Some(true));

    let non_empty = api::create_from_json(&mut runtime, &guard, &serde_json::json!([1])).unwrap();
    assert_eq!(api::is_empty(&non_empty), Some(false));
}

#[test]
fn test_api_is_array() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);

    let arr = api::create_from_json(&mut runtime, &guard, &serde_json::json!([1, 2, 3])).unwrap();
    assert!(api::is_array(&arr));

    let obj = api::create_from_json(&mut runtime, &guard, &serde_json::json!({"x": 1})).unwrap();
    assert!(!api::is_array(&obj));

    let num = api::create_value(42);
    assert!(!api::is_array(&num));
}

#[test]
fn test_api_keys() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let obj = api::create_from_json(
        &mut runtime,
        &guard,
        &serde_json::json!({"a": 1, "b": 2, "c": 3}),
    )
    .unwrap();

    let keys = api::keys(&obj);
    assert!(keys.contains(&"a".to_string()));
    assert!(keys.contains(&"b".to_string()));
    assert!(keys.contains(&"c".to_string()));
    assert_eq!(keys.len(), 3);
}

#[test]
fn test_api_elements() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr =
        api::create_from_json(&mut runtime, &guard, &serde_json::json!(["a", "b", "c"])).unwrap();

    let elements = api::get_elements(&arr).unwrap();
    assert_eq!(elements.len(), 3);
    assert_eq!(elements[0].as_str(), Some("a"));
    assert_eq!(elements[1].as_str(), Some("b"));
    assert_eq!(elements[2].as_str(), Some("c"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// API Mutation Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_set_property() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let obj = api::create_object(&mut runtime, &guard).unwrap();

    api::set_property(&obj, "name", JsValue::from("Bob")).unwrap();
    api::set_property(&obj, "age", JsValue::from(25)).unwrap();

    let name = api::get_property(&obj, "name").unwrap();
    assert_eq!(name.as_str(), Some("Bob"));
    let age = api::get_property(&obj, "age").unwrap();
    assert_eq!(age.as_number(), Some(25.0));
}

#[test]
fn test_set_index() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr = api::create_from_json(&mut runtime, &guard, &serde_json::json!([1, 2, 3])).unwrap();

    api::set_index(&arr, 1, JsValue::from(20)).unwrap();

    assert_eq!(api::get_index(&arr, 0).unwrap().as_number(), Some(1.0));
    assert_eq!(api::get_index(&arr, 1).unwrap().as_number(), Some(20.0));
    assert_eq!(api::get_index(&arr, 2).unwrap().as_number(), Some(3.0));
}

#[test]
fn test_set_index_extends() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr = api::create_array(&mut runtime, &guard).unwrap();

    // Setting index 2 on empty array should extend it
    api::set_index(&arr, 2, JsValue::from(42)).unwrap();

    assert_eq!(api::len(&arr), Some(3));
    assert!(api::get_index(&arr, 0).unwrap().is_undefined());
    assert!(api::get_index(&arr, 1).unwrap().is_undefined());
    assert_eq!(api::get_index(&arr, 2).unwrap().as_number(), Some(42.0));
}

#[test]
fn test_push() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr = api::create_array(&mut runtime, &guard).unwrap();

    api::push(&arr, JsValue::from(1)).unwrap();
    api::push(&arr, JsValue::from(2)).unwrap();
    api::push(&arr, JsValue::from(3)).unwrap();

    assert_eq!(api::len(&arr), Some(3));
    assert_eq!(api::get_index(&arr, 0).unwrap().as_number(), Some(1.0));
    assert_eq!(api::get_index(&arr, 1).unwrap().as_number(), Some(2.0));
    assert_eq!(api::get_index(&arr, 2).unwrap().as_number(), Some(3.0));
}

#[test]
fn test_push_mixed_types() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr = api::create_array(&mut runtime, &guard).unwrap();

    api::push(&arr, JsValue::from("hello")).unwrap();
    api::push(&arr, JsValue::from(42)).unwrap();
    api::push(&arr, JsValue::from(true)).unwrap();

    let elements = api::get_elements(&arr).unwrap();
    assert_eq!(elements[0].as_str(), Some("hello"));
    assert_eq!(elements[1].as_number(), Some(42.0));
    assert_eq!(elements[2].as_bool(), Some(true));
}

// ═══════════════════════════════════════════════════════════════════════════════
// API Method Call Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_call_method_join() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr =
        api::create_from_json(&mut runtime, &guard, &serde_json::json!(["a", "b", "c"])).unwrap();

    let result =
        api::call_method(&mut runtime, &guard, &arr, "join", &[JsValue::from("-")]).unwrap();

    assert_eq!(result.as_str(), Some("a-b-c"));
}

#[test]
fn test_call_method_push() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr = api::create_from_json(&mut runtime, &guard, &serde_json::json!([1, 2])).unwrap();

    let result = api::call_method(&mut runtime, &guard, &arr, "push", &[JsValue::from(3)]).unwrap();

    // push returns the new length
    assert_eq!(result.as_number(), Some(3.0));
    assert_eq!(api::len(&arr), Some(3));
}

#[test]
fn test_call_method_tostring() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let arr = api::create_from_json(&mut runtime, &guard, &serde_json::json!([1, 2, 3])).unwrap();

    let result = api::call_method(&mut runtime, &guard, &arr, "toString", &[]).unwrap();

    assert_eq!(result.as_str(), Some("1,2,3"));
}

#[test]
fn test_call_method_map() {
    let mut runtime = Runtime::new();

    // First, define a function in JS
    let result = runtime
        .eval(
            r#"
        const arr = [1, 2, 3];
        arr.map(x => x * 2)
    "#,
        )
        .unwrap();

    if let RuntimeResult::Complete(rv) = result {
        let elements = api::get_elements(rv.value()).unwrap();
        assert_eq!(elements[0].as_number(), Some(2.0));
        assert_eq!(elements[1].as_number(), Some(4.0));
        assert_eq!(elements[2].as_number(), Some(6.0));
    }
}

#[test]
fn test_call_function() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);

    // Define a function
    let result = runtime
        .eval("function add(a, b) { return a + b; } add")
        .unwrap();

    if let RuntimeResult::Complete(add_fn) = result {
        let sum = api::call_function(
            &mut runtime,
            &guard,
            add_fn.value(),
            None,
            &[JsValue::from(10), JsValue::from(20)],
        )
        .unwrap();

        assert_eq!(sum.as_number(), Some(30.0));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_call_function_with_this() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);

    // Define a function that uses `this`
    let result = runtime
        .eval("function getX() { return this.x; } getX")
        .unwrap();

    if let RuntimeResult::Complete(get_x) = result {
        let obj =
            api::create_from_json(&mut runtime, &guard, &serde_json::json!({"x": 42})).unwrap();

        let x = api::call_function(&mut runtime, &guard, get_x.value(), Some(&obj), &[]).unwrap();

        assert_eq!(x.as_number(), Some(42.0));
    } else {
        panic!("Expected Complete");
    }
}

#[test]
fn test_call_method_error_on_missing() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let obj = api::create_object(&mut runtime, &guard).unwrap();

    let result = api::call_method(&mut runtime, &guard, &obj, "nonexistent", &[]);
    assert!(result.is_err());
}

#[test]
fn test_call_function_error_on_non_function() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let num = api::create_value(42);

    let result = api::call_function(&mut runtime, &guard, &num, None, &[]);
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
// Guard Utility Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_guard_value() {
    let mut runtime = Runtime::new();
    let guard = api::create_guard(&runtime);
    let obj = api::create_from_json(
        &mut runtime,
        &guard,
        &serde_json::json!({"nested": {"value": 42}}),
    )
    .unwrap();

    // Get nested object
    let nested = api::get_property(&obj, "nested").unwrap();

    // Guard the nested object
    api::guard_value(&guard, &nested);

    // Verify it's accessible
    let value = api::get_property(&nested, "value").unwrap();
    assert_eq!(value.as_number(), Some(42.0));
}
