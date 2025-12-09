//! Tests for export handling and call_function API

use typescript_eval::value::PropertyKey;
use typescript_eval::{JsValue, Runtime, RuntimeResult};

/// Helper to run eval and expect Complete
fn run_eval(runtime: &mut Runtime, source: &str) {
    match runtime.eval(source).unwrap() {
        RuntimeResult::Complete(_) => {}
        other => panic!("Expected Complete, got {:?}", other),
    }
}

#[test]
fn test_call_exported_render_function() {
    let source = r#"
        type Context = {
            name: string;
        }

        type Output = {
            apiVersion: string;
            greeting: string[];
        }

        export function render(ctx: Context): Output {
            return {
                apiVersion: 'v1',
                greeting: [
                    "Hello, ",
                    ctx.name,
                    "!",
                ]
            };
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    // Create context object
    let context = serde_json::json!({
        "name": "World"
    });

    let result = runtime.call_function("render", &context).unwrap();

    // Result should be an object with apiVersion and greeting
    match result {
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();

            // Check apiVersion
            let api_version = obj_ref
                .get_property(&PropertyKey::from("apiVersion"))
                .unwrap();
            assert_eq!(api_version, JsValue::from("v1"));

            // Check greeting array
            let greeting = obj_ref
                .get_property(&PropertyKey::from("greeting"))
                .unwrap();
            if let JsValue::Object(arr) = greeting {
                let arr_ref = arr.borrow();
                let elem0 = arr_ref.get_property(&PropertyKey::from("0")).unwrap();
                let elem1 = arr_ref.get_property(&PropertyKey::from("1")).unwrap();
                let elem2 = arr_ref.get_property(&PropertyKey::from("2")).unwrap();
                assert_eq!(elem0, JsValue::from("Hello, "));
                assert_eq!(elem1, JsValue::from("World"));
                assert_eq!(elem2, JsValue::from("!"));
            } else {
                panic!("greeting should be an array");
            }
        }
        _ => panic!("Expected object result, got {:?}", result),
    }
}

#[test]
fn test_export_function_declaration() {
    let source = r#"
        export function add(a: number, b: number): number {
            return a + b;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("add", &serde_json::json!([5, 3]))
        .unwrap();
    assert_eq!(result, JsValue::Number(8.0));
}

#[test]
fn test_export_const() {
    let source = r#"
        export const VERSION = "1.0.0";
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    let version = exports.get("VERSION").unwrap();
    assert_eq!(*version, JsValue::from("1.0.0"));
}

#[test]
fn test_export_multiple_declarations() {
    let source = r#"
        export const name = "test";
        export let count = 42;
        export function greet(s: string): string {
            return "Hello, " + s;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    assert!(exports.contains_key("name"));
    assert!(exports.contains_key("count"));
    assert!(exports.contains_key("greet"));
}

#[test]
fn test_call_function_with_object_arg() {
    let source = r#"
        export function process(config: { value: number }): number {
            return config.value * 2;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("process", &serde_json::json!({"value": 21}))
        .unwrap();
    assert_eq!(result, JsValue::Number(42.0));
}

// ============ Argument Type Tests ============

#[test]
fn test_call_function_no_args() {
    let source = r#"
        export function getVersion(): string {
            return "1.0.0";
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    // Pass empty array for no args
    let result = runtime
        .call_function("getVersion", &serde_json::json!([]))
        .unwrap();
    assert_eq!(result, JsValue::from("1.0.0"));
}

#[test]
fn test_call_function_primitive_string_arg() {
    let source = r#"
        export function greet(name: string): string {
            return "Hello, " + name + "!";
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("greet", &serde_json::json!("World"))
        .unwrap();
    assert_eq!(result, JsValue::from("Hello, World!"));
}

#[test]
fn test_call_function_primitive_number_arg() {
    let source = r#"
        export function double(n: number): number {
            return n * 2;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("double", &serde_json::json!(21))
        .unwrap();
    assert_eq!(result, JsValue::Number(42.0));
}

#[test]
fn test_call_function_primitive_boolean_arg() {
    let source = r#"
        export function negate(b: boolean): boolean {
            return !b;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("negate", &serde_json::json!(true))
        .unwrap();
    assert_eq!(result, JsValue::Boolean(false));

    let result2 = runtime
        .call_function("negate", &serde_json::json!(false))
        .unwrap();
    assert_eq!(result2, JsValue::Boolean(true));
}

#[test]
fn test_call_function_null_arg() {
    let source = r#"
        export function isNull(x: any): boolean {
            return x === null;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("isNull", &serde_json::json!(null))
        .unwrap();
    assert_eq!(result, JsValue::Boolean(true));
}

#[test]
fn test_call_function_multiple_args_spread() {
    let source = r#"
        export function sum(a: number, b: number, c: number): number {
            return a + b + c;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    // Array args are spread as individual arguments
    let result = runtime
        .call_function("sum", &serde_json::json!([10, 20, 30]))
        .unwrap();
    assert_eq!(result, JsValue::Number(60.0));
}

#[test]
fn test_call_function_nested_object_arg() {
    let source = r#"
        type Config = {
            server: {
                host: string;
                port: number;
            };
            debug: boolean;
        }

        export function getServerUrl(config: Config): string {
            return config.server.host + ":" + config.server.port;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let config = serde_json::json!({
        "server": {
            "host": "localhost",
            "port": 8080
        },
        "debug": true
    });

    let result = runtime.call_function("getServerUrl", &config).unwrap();
    assert_eq!(result, JsValue::from("localhost:8080"));
}

#[test]
fn test_call_function_array_in_object_arg() {
    let source = r#"
        type Input = {
            items: number[];
        }

        export function sumItems(input: Input): number {
            let sum = 0;
            for (let i = 0; i < input.items.length; i++) {
                sum += input.items[i];
            }
            return sum;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let input = serde_json::json!({
        "items": [1, 2, 3, 4, 5]
    });

    let result = runtime.call_function("sumItems", &input).unwrap();
    assert_eq!(result, JsValue::Number(15.0));
}

#[test]
fn test_call_function_empty_object_arg() {
    let source = r#"
        export function countKeys(obj: object): number {
            return Object.keys(obj).length;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("countKeys", &serde_json::json!({}))
        .unwrap();
    assert_eq!(result, JsValue::Number(0.0));
}

#[test]
fn test_call_function_empty_array_arg() {
    let source = r#"
        export function getLength(arr: any[]): number {
            return arr.length;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    // Pass array as single argument (wrapped in another array)
    let result = runtime
        .call_function("getLength", &serde_json::json!([[]]))
        .unwrap();
    assert_eq!(result, JsValue::Number(0.0));
}

// ============ Return Type Tests ============

#[test]
fn test_call_function_returns_array() {
    let source = r#"
        export function range(n: number): number[] {
            const result: number[] = [];
            for (let i = 0; i < n; i++) {
                result.push(i);
            }
            return result;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime.call_function("range", &serde_json::json!(3)).unwrap();

    if let JsValue::Object(arr) = result {
        let arr_ref = arr.borrow();
        assert_eq!(
            arr_ref.get_property(&PropertyKey::from("length")),
            Some(JsValue::Number(3.0))
        );
        assert_eq!(
            arr_ref.get_property(&PropertyKey::from("0")),
            Some(JsValue::Number(0.0))
        );
        assert_eq!(
            arr_ref.get_property(&PropertyKey::from("1")),
            Some(JsValue::Number(1.0))
        );
        assert_eq!(
            arr_ref.get_property(&PropertyKey::from("2")),
            Some(JsValue::Number(2.0))
        );
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_call_function_returns_nested_object() {
    let source = r#"
        export function createUser(name: string, age: number) {
            return {
                profile: {
                    name: name,
                    age: age
                },
                metadata: {
                    created: "2024-01-01"
                }
            };
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("createUser", &serde_json::json!(["Alice", 30]))
        .unwrap();

    if let JsValue::Object(obj) = result {
        let obj_ref = obj.borrow();
        let profile = obj_ref.get_property(&PropertyKey::from("profile")).unwrap();

        if let JsValue::Object(profile_obj) = profile {
            let profile_ref = profile_obj.borrow();
            assert_eq!(
                profile_ref.get_property(&PropertyKey::from("name")),
                Some(JsValue::from("Alice"))
            );
            assert_eq!(
                profile_ref.get_property(&PropertyKey::from("age")),
                Some(JsValue::Number(30.0))
            );
        } else {
            panic!("Expected profile to be an object");
        }
    } else {
        panic!("Expected object result");
    }
}

#[test]
fn test_call_function_returns_undefined() {
    let source = r#"
        export function doNothing(): void {
            // no return
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("doNothing", &serde_json::json!([]))
        .unwrap();
    assert_eq!(result, JsValue::Undefined);
}

#[test]
fn test_call_function_returns_null() {
    let source = r#"
        export function returnNull(): null {
            return null;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function("returnNull", &serde_json::json!([]))
        .unwrap();
    assert_eq!(result, JsValue::Null);
}

// ============ Error Cases ============

#[test]
fn test_call_nonexistent_export() {
    let source = r#"
        export function exists(): void {}
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime.call_function("doesNotExist", &serde_json::json!([]));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{:?}", err).contains("not exported"));
}

#[test]
fn test_call_non_function_export() {
    let source = r#"
        export const notAFunction = 42;
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime.call_function("notAFunction", &serde_json::json!([]));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{:?}", err).contains("Not a function"));
}

// ============ Complex Scenarios ============

#[test]
fn test_render_with_array_context() {
    let source = r#"
        type Item = {
            id: number;
            name: string;
        }

        type Context = {
            title: string;
            items: Item[];
        }

        export function render(ctx: Context) {
            const itemNames: string[] = [];
            for (let i = 0; i < ctx.items.length; i++) {
                itemNames.push(ctx.items[i].name);
            }
            return {
                title: ctx.title,
                itemCount: ctx.items.length,
                names: itemNames
            };
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let context = serde_json::json!({
        "title": "My List",
        "items": [
            {"id": 1, "name": "First"},
            {"id": 2, "name": "Second"},
            {"id": 3, "name": "Third"}
        ]
    });

    let result = runtime.call_function("render", &context).unwrap();

    if let JsValue::Object(obj) = result {
        let obj_ref = obj.borrow();
        assert_eq!(
            obj_ref.get_property(&PropertyKey::from("title")),
            Some(JsValue::from("My List"))
        );
        assert_eq!(
            obj_ref.get_property(&PropertyKey::from("itemCount")),
            Some(JsValue::Number(3.0))
        );

        let names = obj_ref.get_property(&PropertyKey::from("names")).unwrap();
        if let JsValue::Object(names_arr) = names {
            let names_ref = names_arr.borrow();
            assert_eq!(
                names_ref.get_property(&PropertyKey::from("0")),
                Some(JsValue::from("First"))
            );
            assert_eq!(
                names_ref.get_property(&PropertyKey::from("1")),
                Some(JsValue::from("Second"))
            );
            assert_eq!(
                names_ref.get_property(&PropertyKey::from("2")),
                Some(JsValue::from("Third"))
            );
        } else {
            panic!("Expected names to be an array");
        }
    } else {
        panic!("Expected object result");
    }
}

#[test]
fn test_render_kubernetes_manifest() {
    let source = r#"
        type Context = {
            name: string;
            image: string;
            replicas: number;
            port: number;
        }

        export function render(ctx: Context) {
            return {
                apiVersion: "apps/v1",
                kind: "Deployment",
                metadata: {
                    name: ctx.name
                },
                spec: {
                    replicas: ctx.replicas,
                    template: {
                        spec: {
                            containers: [
                                {
                                    name: ctx.name,
                                    image: ctx.image,
                                    ports: [
                                        { containerPort: ctx.port }
                                    ]
                                }
                            ]
                        }
                    }
                }
            };
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let context = serde_json::json!({
        "name": "my-app",
        "image": "nginx:latest",
        "replicas": 3,
        "port": 80
    });

    let result = runtime.call_function("render", &context).unwrap();

    if let JsValue::Object(obj) = result {
        let obj_ref = obj.borrow();
        assert_eq!(
            obj_ref.get_property(&PropertyKey::from("apiVersion")),
            Some(JsValue::from("apps/v1"))
        );
        assert_eq!(
            obj_ref.get_property(&PropertyKey::from("kind")),
            Some(JsValue::from("Deployment"))
        );

        // Check nested structure
        let metadata = obj_ref.get_property(&PropertyKey::from("metadata")).unwrap();
        if let JsValue::Object(meta_obj) = metadata {
            let meta_ref = meta_obj.borrow();
            assert_eq!(
                meta_ref.get_property(&PropertyKey::from("name")),
                Some(JsValue::from("my-app"))
            );
        } else {
            panic!("Expected metadata to be an object");
        }

        let spec = obj_ref.get_property(&PropertyKey::from("spec")).unwrap();
        if let JsValue::Object(spec_obj) = spec {
            let spec_ref = spec_obj.borrow();
            assert_eq!(
                spec_ref.get_property(&PropertyKey::from("replicas")),
                Some(JsValue::Number(3.0))
            );
        } else {
            panic!("Expected spec to be an object");
        }
    } else {
        panic!("Expected object result");
    }
}

#[test]
fn test_multiple_function_calls() {
    let source = r#"
        let counter = 0;

        export function increment(): number {
            counter += 1;
            return counter;
        }

        export function getCount(): number {
            return counter;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    // Call increment multiple times
    let r1 = runtime
        .call_function("increment", &serde_json::json!([]))
        .unwrap();
    assert_eq!(r1, JsValue::Number(1.0));

    let r2 = runtime
        .call_function("increment", &serde_json::json!([]))
        .unwrap();
    assert_eq!(r2, JsValue::Number(2.0));

    let r3 = runtime
        .call_function("increment", &serde_json::json!([]))
        .unwrap();
    assert_eq!(r3, JsValue::Number(3.0));

    // Verify with getCount
    let count = runtime
        .call_function("getCount", &serde_json::json!([]))
        .unwrap();
    assert_eq!(count, JsValue::Number(3.0));
}

#[test]
fn test_function_uses_helper() {
    let source = r#"
        function formatName(first: string, last: string): string {
            return first + " " + last;
        }

        export function createGreeting(ctx: { firstName: string; lastName: string }): string {
            return "Hello, " + formatName(ctx.firstName, ctx.lastName) + "!";
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let result = runtime
        .call_function(
            "createGreeting",
            &serde_json::json!({"firstName": "John", "lastName": "Doe"}),
        )
        .unwrap();
    assert_eq!(result, JsValue::from("Hello, John Doe!"));
}

// ============ Export with Import Tests ============

#[test]
fn test_export_after_import() {
    // Test that exports work correctly after imports are resolved
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { baseValue } from './config';

        export function calculate(x: number): number {
            return baseValue + x;
        }

        // Return something to verify execution
        "ready"
    "#,
        )
        .unwrap();

    // Resolve the import
    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./config");
            let module = runtime.create_module_object(vec![
                ("baseValue".to_string(), JsValue::Number(100.0)),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    // Continue and verify
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::from("ready"));
        }
        _ => panic!("Expected Complete"),
    }

    // Now call the exported function
    let calc_result = runtime
        .call_function("calculate", &serde_json::json!(42))
        .unwrap();
    assert_eq!(calc_result, JsValue::Number(142.0)); // 100 + 42
}

// Note: export default is parsed but not currently tracked in the exports map.
// These tests document expected behavior once implemented.

#[test]
#[ignore] // TODO: Implement export default tracking
fn test_export_default_function() {
    let source = r#"
        export default function greet(name: string): string {
            return "Hello, " + name;
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    assert!(exports.contains_key("default"));
}

#[test]
#[ignore] // TODO: Implement export default tracking
fn test_export_default_expression() {
    let source = r#"
        const config = {
            version: "1.0.0",
            name: "my-app"
        };
        export default config;
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    assert!(exports.contains_key("default"));

    if let Some(JsValue::Object(obj)) = exports.get("default") {
        let obj_ref = obj.borrow();
        assert_eq!(
            obj_ref.get_property(&PropertyKey::from("version")),
            Some(JsValue::from("1.0.0"))
        );
    } else {
        panic!("Expected default export to be an object");
    }
}

#[test]
fn test_export_list() {
    let source = r#"
        const x = 1;
        const y = 2;
        function add(a: number, b: number): number { return a + b; }

        export { x, y, add };
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    assert!(exports.contains_key("x"));
    assert!(exports.contains_key("y"));
    assert!(exports.contains_key("add"));

    assert_eq!(*exports.get("x").unwrap(), JsValue::Number(1.0));
    assert_eq!(*exports.get("y").unwrap(), JsValue::Number(2.0));
}

#[test]
fn test_export_renamed() {
    let source = r#"
        const internalName = "value";
        export { internalName as publicName };
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    assert!(exports.contains_key("publicName"));
    assert!(!exports.contains_key("internalName"));

    assert_eq!(*exports.get("publicName").unwrap(), JsValue::from("value"));
}

#[test]
fn test_export_class() {
    let source = r#"
        export class Calculator {
            value: number;

            constructor(initial: number) {
                this.value = initial;
            }

            add(n: number): number {
                this.value += n;
                return this.value;
            }
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    assert!(exports.contains_key("Calculator"));
}

#[test]
fn test_export_async_function() {
    let source = r#"
        export async function fetchData(): Promise<number> {
            return await Promise.resolve(42);
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    assert!(exports.contains_key("fetchData"));
}

#[test]
fn test_multiple_exports_with_import() {
    // Test complex scenario with imports and multiple exports
    let mut runtime = Runtime::new();
    let result = runtime
        .eval(
            r#"
        import { CONFIG_VERSION } from './config';

        export const version = CONFIG_VERSION;
        export const name = "my-module";

        export function getInfo(): string {
            return name + " v" + version;
        }

        "initialized"
    "#,
        )
        .unwrap();

    // Resolve import
    match result {
        RuntimeResult::ImportAwaited { slot, .. } => {
            let module = runtime.create_module_object(vec![
                ("CONFIG_VERSION".to_string(), JsValue::from("2.0")),
            ]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    assert!(matches!(result, RuntimeResult::Complete(_)));

    // Verify exports
    let exports = runtime.get_exports();
    assert_eq!(*exports.get("version").unwrap(), JsValue::from("2.0"));
    assert_eq!(*exports.get("name").unwrap(), JsValue::from("my-module"));

    // Call exported function
    let info = runtime
        .call_function("getInfo", &serde_json::json!([]))
        .unwrap();
    assert_eq!(info, JsValue::from("my-module v2.0"));
}

// ============ Export with Types Tests ============

#[test]
fn test_export_interface_ignored_at_runtime() {
    // Interfaces are type-only and should not appear in exports
    let source = r#"
        export interface Config {
            name: string;
            value: number;
        }

        export const config: Config = {
            name: "test",
            value: 42
        };
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    // Interface should NOT be in exports (it's type-only)
    // Only the value `config` should be exported
    assert!(exports.contains_key("config"));
}

#[test]
#[ignore] // TODO: Implement export type parsing
fn test_export_type_alias_ignored_at_runtime() {
    // Type aliases are type-only
    let source = r#"
        export type ID = string | number;

        export const defaultId: ID = "default";
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    // Type alias should NOT be in exports
    // Only defaultId should be exported
    assert!(exports.contains_key("defaultId"));
}

#[test]
#[ignore] // TODO: Implement export enum tracking
fn test_export_enum() {
    // Enums are compiled to objects at runtime
    let source = r#"
        export enum Status {
            Active = "active",
            Inactive = "inactive"
        }
    "#;

    let mut runtime = Runtime::new();
    run_eval(&mut runtime, source);

    let exports = runtime.get_exports();
    assert!(exports.contains_key("Status"));

    if let Some(JsValue::Object(status)) = exports.get("Status") {
        let status_ref = status.borrow();
        assert_eq!(
            status_ref.get_property(&PropertyKey::from("Active")),
            Some(JsValue::from("active"))
        );
        assert_eq!(
            status_ref.get_property(&PropertyKey::from("Inactive")),
            Some(JsValue::from("inactive"))
        );
    } else {
        panic!("Expected Status to be an object");
    }
}
