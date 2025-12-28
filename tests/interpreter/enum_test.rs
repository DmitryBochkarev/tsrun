//! Tests exploring enum behavior and comparing with TypeScript semantics

use super::eval;
use tsrun::JsValue;

// =============================================================================
// BASIC ENUM FUNCTIONALITY
// =============================================================================

#[test]
fn test_enum_numeric_basic() {
    // Basic numeric enum - auto-incrementing from 0
    assert_eq!(
        eval(
            r#"
            enum Direction {
                Up,
                Down,
                Left,
                Right
            }
            Direction.Up
        "#
        ),
        JsValue::Number(0.0)
    );
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down, Left, Right }
            Direction.Right
        "#
        ),
        JsValue::Number(3.0)
    );
}

#[test]
fn test_enum_numeric_explicit_values() {
    // Numeric enum with explicit values
    assert_eq!(
        eval(
            r#"
            enum Status {
                Pending = 10,
                Active = 20,
                Closed = 30
            }
            Status.Active
        "#
        ),
        JsValue::Number(20.0)
    );
}

#[test]
fn test_enum_string_values() {
    // String enum
    assert_eq!(
        eval(
            r#"
            enum Color {
                Red = "red",
                Green = "green",
                Blue = "blue"
            }
            Color.Green
        "#
        ),
        JsValue::from("green")
    );
}

#[test]
fn test_enum_mixed_values() {
    // Mixed enum (numeric + string) - TypeScript allows this
    assert_eq!(
        eval(
            r#"
            enum Mixed {
                A = 0,
                B = "hello",
                C = 1
            }
            Mixed.B
        "#
        ),
        JsValue::from("hello")
    );
}

// =============================================================================
// REVERSE MAPPING (Numeric enums only in TypeScript)
// =============================================================================

#[test]
fn test_enum_reverse_mapping_numeric() {
    // TypeScript generates reverse mappings for numeric enums
    // Direction[0] === "Up"
    assert_eq!(
        eval(
            r#"
            enum Direction {
                Up,
                Down,
                Left,
                Right
            }
            Direction[0]
        "#
        ),
        JsValue::from("Up")
    );
}

#[test]
fn test_enum_reverse_mapping_all_numeric() {
    // All reverse mappings should work
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down, Left, Right }
            Direction[0] + "," + Direction[1] + "," + Direction[2] + "," + Direction[3]
        "#
        ),
        JsValue::from("Up,Down,Left,Right")
    );
}

#[test]
fn test_enum_no_reverse_mapping_for_strings() {
    // TypeScript does NOT generate reverse mappings for string enums
    // Color["red"] should be undefined
    assert_eq!(
        eval(
            r#"
            enum Color {
                Red = "red",
                Green = "green"
            }
            Color["red"]
        "#
        ),
        JsValue::Undefined
    );
}

// =============================================================================
// Object.keys / Object.values BEHAVIOR
// =============================================================================

#[test]
fn test_enum_object_keys_numeric() {
    // For numeric enums, Object.keys includes both names AND reverse mappings
    // TypeScript compiled output: { "0": "Up", "1": "Down", "Up": 0, "Down": 1 }
    // So Object.keys returns ["0", "1", "Up", "Down"]
    assert_eq!(
        eval(
            r#"
        enum Direction { Up, Down }
        Object.keys(Direction).sort().join(",")
    "#
        ),
        JsValue::from("0,1,Down,Up")
    );
}

#[test]
fn test_enum_object_keys_string() {
    // For string enums, Object.keys should only include the member names
    // No reverse mappings
    assert_eq!(
        eval(
            r#"
        enum Color {
            Red = "red",
            Green = "green"
        }
        Object.keys(Color).sort().join(",")
    "#
        ),
        JsValue::from("Green,Red")
    );
}

#[test]
fn test_enum_object_values_numeric() {
    // Object.values on numeric enum includes both numbers and strings (reverse mappings)
    // Note: sorting mixed types may vary, so we'll just check length
    assert_eq!(
        eval(
            r#"
        enum Direction { Up, Down }
        Object.values(Direction).length
    "#
        ),
        JsValue::Number(4.0) // 0, 1, "Up", "Down"
    );
}

#[test]
fn test_enum_object_values_string() {
    // Object.values on string enum - just the string values
    assert_eq!(
        eval(
            r#"
        enum Color {
            Red = "red",
            Green = "green"
        }
        Object.values(Color).sort().join(",")
    "#
        ),
        JsValue::from("green,red")
    );
}

// =============================================================================
// ENUM MUTABILITY
// =============================================================================

#[test]
fn test_enum_is_mutable_like_object() {
    // In TypeScript compiled JS, enums are plain objects and ARE mutable
    // (though TypeScript types prevent this)
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down }
            Direction.Up = 999;
            Direction.Up
        "#
        ),
        JsValue::Number(999.0)
    );
}

#[test]
fn test_enum_can_add_properties() {
    // Can add new properties to enum objects
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down }
            (Direction as any).NewMember = 100;
            (Direction as any).NewMember
        "#
        ),
        JsValue::Number(100.0)
    );
}

// =============================================================================
// FOR...IN ITERATION
// =============================================================================

#[test]
fn test_enum_for_in_numeric() {
    // for...in on numeric enum iterates over all keys (including reverse mappings)
    assert_eq!(
        eval(
            r#"
        enum Direction { Up, Down }
        let keys: string[] = [];
        for (let k in Direction) {
            keys.push(k);
        }
        keys.sort().join(",")
    "#
        ),
        JsValue::from("0,1,Down,Up")
    );
}

#[test]
fn test_enum_for_in_string() {
    // for...in on string enum - only member names
    assert_eq!(
        eval(
            r#"
        enum Color {
            Red = "red",
            Green = "green"
        }
        let keys: string[] = [];
        for (let k in Color) {
            keys.push(k);
        }
        keys.sort().join(",")
    "#
        ),
        JsValue::from("Green,Red")
    );
}

// =============================================================================
// TYPEOF AND INSTANCEOF
// =============================================================================

#[test]
fn test_enum_typeof() {
    // typeof enum is "object"
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down }
            typeof Direction
        "#
        ),
        JsValue::from("object")
    );
}

#[test]
fn test_enum_member_typeof() {
    // typeof enum member depends on value type
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down }
            typeof Direction.Up
        "#
        ),
        JsValue::from("number")
    );
    assert_eq!(
        eval(
            r#"
            enum Color { Red = "red" }
            typeof Color.Red
        "#
        ),
        JsValue::from("string")
    );
}

// =============================================================================
// CONST ENUMS (TypeScript-specific - should be inlined at compile time)
// =============================================================================

#[test]
fn test_const_enum_basic() {
    // const enums in TypeScript are typically inlined and don't exist at runtime
    // Our interpreter may or may not support this - let's test
    // Note: Most interpreters treat const enum same as regular enum
    assert_eq!(
        eval(
            r#"
            const enum Direction { Up, Down }
            Direction.Up
        "#
        ),
        JsValue::Number(0.0)
    );
}

// =============================================================================
// ENUM IN EXPRESSIONS
// =============================================================================

#[test]
fn test_enum_in_arithmetic() {
    // Numeric enum values can be used in arithmetic
    assert_eq!(
        eval(
            r#"
            enum Bits {
                Read = 1,
                Write = 2,
                Execute = 4
            }
            Bits.Read | Bits.Write | Bits.Execute
        "#
        ),
        JsValue::Number(7.0)
    );
}

#[test]
fn test_enum_in_comparison() {
    // Enum values can be compared
    assert_eq!(
        eval(
            r#"
            enum Status { A = 1, B = 2 }
            Status.A < Status.B
        "#
        ),
        JsValue::Boolean(true)
    );
    assert_eq!(
        eval(
            r#"
            enum Status { A = 1, B = 2 }
            Status.A === 1
        "#
        ),
        JsValue::Boolean(true)
    );
}

// =============================================================================
// ENUM COMPUTED MEMBERS
// =============================================================================

#[test]
fn test_enum_computed_from_prior_members() {
    // Enum members can reference prior members
    assert_eq!(
        eval(
            r#"
            enum FileAccess {
                None = 0,
                Read = 1,
                Write = 2,
                ReadWrite = Read | Write
            }
            FileAccess.ReadWrite
        "#
        ),
        JsValue::Number(3.0)
    );
}

// =============================================================================
// JSON.stringify BEHAVIOR
// =============================================================================

#[test]
fn test_enum_json_stringify() {
    // JSON.stringify on enum - should stringify as object
    let result = eval(
        r#"
        enum Direction { Up, Down }
        let json = JSON.stringify(Direction);
        json.includes("Up") && json.includes("Down")
    "#,
    );
    assert_eq!(result, JsValue::Boolean(true));
}

// =============================================================================
// PROTOTYPE CHAIN
// =============================================================================

#[test]
fn test_enum_has_object_prototype() {
    // Enum objects should have Object.prototype methods
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down }
            Direction.hasOwnProperty("Up")
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_enum_tostring() {
    // toString on enum object
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down }
            Direction.toString()
        "#
        ),
        JsValue::from("[object Object]")
    );
}

// =============================================================================
// NEGATIVE/FLOAT NUMERIC VALUES
// =============================================================================

#[test]
fn test_enum_negative_values() {
    // Enums can have negative values
    assert_eq!(
        eval(
            r#"
            enum Temperature {
                Cold = -10,
                Warm = 20,
                Hot = 40
            }
            Temperature.Cold
        "#
        ),
        JsValue::Number(-10.0)
    );
}

#[test]
fn test_enum_negative_reverse_mapping() {
    // Do negative values get reverse mappings?
    // In TypeScript, they do but with string keys like "-10"
    assert_eq!(
        eval(
            r#"
            enum Temperature {
                Cold = -10,
                Warm = 20
            }
            Temperature[-10]
        "#
        ),
        JsValue::from("Cold")
    );
}

#[test]
fn test_enum_float_values() {
    // Float values in enums
    assert_eq!(
        eval(
            r#"
            enum Ratio {
                Half = 0.5,
                Quarter = 0.25
            }
            Ratio.Half
        "#
        ),
        JsValue::Number(0.5)
    );
}

#[test]
fn test_enum_float_reverse_mapping() {
    // TypeScript generates reverse mappings with string keys like "0.5"
    // Accessing Ratio[0.5] converts 0.5 to "0.5" string key
    assert_eq!(
        eval(
            r#"
            enum Ratio {
                Half = 0.5,
                Quarter = 0.25
            }
            Ratio[0.5]
        "#
        ),
        JsValue::from("Half")
    );
}

// =============================================================================
// AUTO-INCREMENT AFTER EXPLICIT VALUE
// =============================================================================

#[test]
fn test_enum_auto_increment_after_explicit() {
    // After an explicit numeric value, auto-increment continues from there
    assert_eq!(
        eval(
            r#"
            enum Mixed {
                A,        // 0
                B = 10,   // 10
                C,        // 11
                D         // 12
            }
            Mixed.A.toString() + "," + Mixed.B + "," + Mixed.C + "," + Mixed.D
        "#
        ),
        JsValue::from("0,10,11,12")
    );
}

// =============================================================================
// ENUM DECLARATION IN DIFFERENT SCOPES
// =============================================================================

#[test]
fn test_enum_in_function_scope() {
    // Enums declared inside functions
    assert_eq!(
        eval(
            r#"
            function test(): number {
                enum Local { A = 5 }
                return Local.A;
            }
            test()
        "#
        ),
        JsValue::Number(5.0)
    );
}

#[test]
fn test_enum_shadowing() {
    // Enum name shadowing
    assert_eq!(
        eval(
            r#"
            enum X { A = 1 }
            function test(): number {
                enum X { A = 99 }
                return X.A;
            }
            test() + X.A
        "#
        ),
        JsValue::Number(100.0) // 99 + 1
    );
}

// =============================================================================
// ENUM AS OBJECT OPERATIONS
// =============================================================================

#[test]
fn test_enum_object_freeze() {
    // Can we freeze an enum?
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down }
            Object.freeze(Direction);
            Object.isFrozen(Direction)
        "#
        ),
        JsValue::Boolean(true)
    );
}

#[test]
fn test_enum_object_keys_after_freeze() {
    // Object.keys still works after freeze
    assert_eq!(
        eval(
            r#"
            enum Direction { Up, Down }
            Object.freeze(Direction);
            Object.keys(Direction).length
        "#
        ),
        JsValue::Number(4.0) // 0, 1, Up, Down
    );
}

// =============================================================================
// EDGE CASES
// =============================================================================

#[test]
fn test_enum_empty() {
    // Empty enum
    assert_eq!(
        eval(
            r#"
            enum Empty {}
            Object.keys(Empty).length
        "#
        ),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_enum_single_member() {
    // Single member enum
    assert_eq!(
        eval(
            r#"
            enum Single { Only }
            Single.Only
        "#
        ),
        JsValue::Number(0.0)
    );
}

#[test]
fn test_enum_member_named_constructor() {
    // Member named "constructor" - potential issue
    assert_eq!(
        eval(
            r#"
            enum Special { constructor = 1 }
            Special.constructor
        "#
        ),
        JsValue::Number(1.0)
    );
}

#[test]
fn test_enum_member_named_prototype() {
    // Member named "prototype" - potential issue
    assert_eq!(
        eval(
            r#"
            enum Special { prototype = 1 }
            Special.prototype
        "#
        ),
        JsValue::Number(1.0)
    );
}
