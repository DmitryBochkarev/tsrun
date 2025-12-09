// Debug tests for data-pipeline example

use super::eval;
use typescript_eval::JsValue;

#[test]
fn test_simple_filter() {
    let result = eval(
        r#"
        const products = [
            { id: 1, name: "A", category: "X" },
            { id: 2, name: "B", category: "Y" },
            { id: 3, name: "C", category: "X" },
        ];
        products.filter(p => p.category === "X").length
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_arrow_with_typed_param() {
    let result = eval(
        r#"
        const filterByCategory = (items: any[], category: string): any[] =>
            items.filter(p => p.category === category);
        const products = [
            { id: 1, category: "X" },
            { id: 2, category: "Y" },
        ];
        filterByCategory(products, "X").length
    "#,
    );
    assert_eq!(result, JsValue::Number(1.0));
}

#[test]
fn test_flatmap() {
    let result = eval(
        r#"
        const products = [
            { tags: ["a", "b"] },
            { tags: ["c", "d"] },
        ];
        products.flatMap(p => p.tags).length
    "#,
    );
    assert_eq!(result, JsValue::Number(4.0));
}

#[test]
fn test_reduce_with_destructuring() {
    let result = eval(
        r#"
        const products = [
            { price: 10, stock: 5 },
            { price: 20, stock: 3 },
        ];
        products.reduce((total, { price, stock }) => total + price * stock, 0)
    "#,
    );
    assert_eq!(result, JsValue::Number(110.0));
}

#[test]
fn test_object_entries() {
    let result = eval(
        r#"
        const obj = { a: 1, b: 2 };
        Object.entries(obj).length
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_array_destructuring_in_arrow_param() {
    // Test array destructuring pattern in arrow function parameter
    let result = eval(
        r#"
        const arr = [[1, 2], [3, 4]];
        arr.map(([a, b]) => a + b).length
    "#,
    );
    // Should return 2 (length of result array [3, 7])
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_object_entries_content() {
    // Check what Object.entries returns
    let result = eval(
        r#"
        const obj = { a: 1 };
        const entries = Object.entries(obj);
        entries[0][0]
    "#,
    );
    assert_eq!(result, JsValue::String("a".into()));
}

#[test]
fn test_object_entries_with_simple_map() {
    // Object.entries with simple map callback (no destructuring)
    let result = eval(
        r#"
        const obj = { a: 1, b: 2 };
        const result = Object.entries(obj).map(entry => entry[0]);
        result.length
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}

#[test]
fn test_entries_destructuring_step_by_step() {
    // Step by step to find where it fails
    let result = eval(
        r#"
        const obj = { a: 1 };
        const entries = Object.entries(obj);
        const first = entries[0];
        const mapped = entries.map(([key, value]) => key + ":" + value);
        mapped[0]
    "#,
    );
    assert_eq!(result, JsValue::String("a:1".into()));
}

#[test]
fn test_object_entries_map() {
    let result = eval(
        r#"
        const obj = { a: 1, b: 2 };
        const result = Object.entries(obj).map(([key, value]) => key + ":" + value);
        result.join(",")
    "#,
    );
    // Order might vary
    if let JsValue::String(s) = result {
        assert!(s.as_str().contains("a:1") && s.as_str().contains("b:2"));
    } else {
        panic!("Expected String");
    }
}

#[test]
fn test_group_by_category() {
    let result = eval(
        r#"
        const products = [
            { id: 1, category: "X" },
            { id: 2, category: "Y" },
            { id: 3, category: "X" },
        ];
        const grouped = products.reduce((groups, product) => {
            const category = product.category;
            if (!groups[category]) {
                groups[category] = [];
            }
            groups[category].push(product);
            return groups;
        }, {});
        Object.keys(grouped).length
    "#,
    );
    assert_eq!(result, JsValue::Number(2.0));
}
