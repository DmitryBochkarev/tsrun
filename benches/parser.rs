//! Parser benchmarks
//!
//! Run with: cargo bench --bench parser
//! Profile with: cargo flamegraph --bench parser -- --bench

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use tsrun::parser::Parser;
use tsrun::string_dict::StringDict;

/// Simple expressions
const SIMPLE_EXPR: &str = "1 + 2 * 3 - 4 / 5";

/// Binary expression tree (deep nesting)
fn generate_binary_expr(depth: usize) -> String {
    if depth == 0 {
        "x".to_string()
    } else {
        format!(
            "({} + {})",
            generate_binary_expr(depth - 1),
            generate_binary_expr(depth - 1)
        )
    }
}

/// Variable declarations
const VARIABLES: &str = r#"
let x = 1;
const y = 2;
var z = 3;
let a = x + y + z;
const b = a * 2;
let { foo, bar: baz } = obj;
const [first, second, ...rest] = arr;
"#;

/// Class definition (TypeScript)
const CLASS_DEF: &str = r#"
class Counter extends Base implements ICounter {
    #count: number = 0;
    private name: string;
    public readonly id: number;
    static instances: number = 0;

    constructor(name: string, initialValue: number = 0) {
        super();
        this.name = name;
        this.#count = initialValue;
        Counter.instances++;
    }

    get value(): number {
        return this.#count;
    }

    set value(n: number) {
        if (n >= 0) {
            this.#count = n;
        }
    }

    increment(): this {
        this.#count++;
        return this;
    }

    decrement(): this {
        this.#count--;
        return this;
    }

    reset(): void {
        this.#count = 0;
    }

    static create(name: string): Counter {
        return new Counter(name);
    }
}
"#;

/// Function with various parameter patterns
const FUNCTIONS: &str = r#"
function simple(a, b) { return a + b; }
function typed(a: number, b: string): boolean { return true; }
function defaultParams(x = 1, y = 2) { return x + y; }
function restParams(...args: number[]) { return args.reduce((a, b) => a + b, 0); }
function destructured({ x, y }: Point, [a, b]: [number, number]) { return x + y + a + b; }
const arrow = (x: number) => x * 2;
const arrowBlock = (x: number): number => { return x * 2; };
async function asyncFn() { return await Promise.resolve(42); }
function* generator() { yield 1; yield 2; yield 3; }
"#;

/// Control flow
const CONTROL_FLOW: &str = r#"
if (condition) {
    doSomething();
} else if (otherCondition) {
    doSomethingElse();
} else {
    doDefault();
}

for (let i = 0; i < 10; i++) {
    console.log(i);
}

for (const item of items) {
    process(item);
}

for (const key in object) {
    if (object.hasOwnProperty(key)) {
        console.log(key, object[key]);
    }
}

while (running) {
    tick();
}

do {
    attempt();
} while (shouldRetry);

switch (value) {
    case 1:
        handleOne();
        break;
    case 2:
    case 3:
        handleTwoOrThree();
        break;
    default:
        handleDefault();
}

try {
    riskyOperation();
} catch (error) {
    handleError(error);
} finally {
    cleanup();
}

throw new Error("Something went wrong");
"#;

/// JSON-like object literals
const OBJECTS: &str = r#"
const config = {
    name: "MyApp",
    version: "1.0.0",
    settings: {
        debug: true,
        logLevel: "info",
        features: ["auth", "api", "cache"],
    },
    database: {
        host: "localhost",
        port: 5432,
        credentials: {
            user: "admin",
            password: "secret123",
        },
    },
    endpoints: [
        { path: "/api/users", method: "GET" },
        { path: "/api/users", method: "POST" },
        { path: "/api/users/:id", method: "PUT" },
        { path: "/api/users/:id", method: "DELETE" },
    ],
};
"#;

/// Array operations with method chaining
const ARRAYS: &str = r#"
const numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
const doubled = numbers.map(n => n * 2);
const evens = numbers.filter(n => n % 2 === 0);
const sum = numbers.reduce((acc, n) => acc + n, 0);
const found = numbers.find(n => n > 5);
const index = numbers.findIndex(n => n > 5);
const sorted = [...numbers].sort((a, b) => b - a);
const sliced = numbers.slice(2, 5);
const joined = numbers.join(", ");
const [first, second, ...rest] = numbers;
const combined = [...numbers, ...doubled];

// Method chaining
const result = numbers
    .filter(n => n > 2)
    .map(n => n * 2)
    .reduce((acc, n) => acc + n, 0);
"#;

/// Type annotations (TypeScript)
const TYPES: &str = r#"
type Point = { x: number; y: number };
type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };
interface User {
    id: number;
    name: string;
    email?: string;
    readonly createdAt: Date;
    updateProfile(data: Partial<User>): Promise<void>;
}
interface Admin extends User {
    permissions: string[];
}
type StringOrNumber = string | number;
type Keys = keyof User;
type Optional<T> = T | null | undefined;
"#;

/// Template literals with expressions
const TEMPLATES: &str = r#"
const simple = `Hello, World!`;
const interpolated = `Hello, ${name}!`;
const multiline = `
    Line 1
    Line 2
    Line 3
`;
const nested = `outer ${`inner ${value}`} outer`;
const tagged = html`<div class="${className}">${content}</div>`;
const complex = `Result: ${items.map(i => `${i.name}: ${i.value}`).join(', ')}`;
"#;

/// Destructuring patterns
const DESTRUCTURING: &str = r#"
const { a, b, c } = obj;
const { x: newX, y: newY } = point;
const { deep: { nested: { value } } } = complex;
const { prop = defaultValue } = maybeObj;
const [first, second, third] = arr;
const [head, ...tail] = list;
const [, , third] = sparse;
const [{ x }, { y }] = points;
function fn({ a, b }: { a: number; b: string }) {}
const {
    user: { name, email },
    settings: { theme = 'dark' }
} = config;
"#;

/// Async/await patterns
const ASYNC: &str = r#"
async function fetchData() {
    const response = await fetch('/api/data');
    const data = await response.json();
    return data;
}

const asyncArrow = async () => {
    await delay(100);
    return 'done';
};

async function parallel() {
    const [a, b, c] = await Promise.all([
        fetchA(),
        fetchB(),
        fetchC(),
    ]);
    return { a, b, c };
}

async function sequential() {
    const a = await fetchA();
    const b = await fetchB(a);
    const c = await fetchC(b);
    return c;
}

async function errorHandling() {
    try {
        const result = await riskyOperation();
        return result;
    } catch (error) {
        console.error(error);
        throw new Error('Operation failed');
    }
}
"#;

/// Import/export statements
const MODULES: &str = r#"
import { foo, bar } from './module';
import defaultExport from './default';
import * as namespace from './namespace';
import { original as renamed } from './renamed';
import type { TypeOnly } from './types';

export const value = 42;
export function exportedFn() {}
export class ExportedClass {}
export { foo, bar };
export { original as renamed };
export default class DefaultClass {}
export * from './reexport';
export type { TypeExport };
"#;

/// Large realistic file
fn generate_large_source(size: usize) -> String {
    let mut source = String::with_capacity(size);
    let patterns = [
        CLASS_DEF,
        FUNCTIONS,
        CONTROL_FLOW,
        OBJECTS,
        ARRAYS,
        TYPES,
        TEMPLATES,
        DESTRUCTURING,
        ASYNC,
    ];

    let mut i = 0;
    while source.len() < size {
        if let Some(pattern) = patterns.get(i % patterns.len()) {
            source.push_str(pattern);
            source.push_str("\n\n");
        }
        i += 1;
    }
    source
}

fn bench_parser_individual(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser/individual");

    let cases = [
        ("simple_expr", SIMPLE_EXPR),
        ("variables", VARIABLES),
        ("class_def", CLASS_DEF),
        ("functions", FUNCTIONS),
        ("control_flow", CONTROL_FLOW),
        ("objects", OBJECTS),
        ("arrays", ARRAYS),
        ("types", TYPES),
        ("templates", TEMPLATES),
        ("destructuring", DESTRUCTURING),
        ("async", ASYNC),
        ("modules", MODULES),
    ];

    for (name, source) in cases {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(BenchmarkId::new("bytes", name), source, |b, s| {
            b.iter(|| {
                let mut dict = StringDict::new();
                let mut parser = Parser::new(black_box(s), &mut dict);
                let result = parser.parse_program();
                black_box(result)
            });
        });
    }

    group.finish();
}

fn bench_parser_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser/throughput");

    // Test with different source sizes
    let sizes = [1_000, 10_000, 100_000, 500_000];

    for size in sizes {
        let source = generate_large_source(size);
        let actual_size = source.len();

        group.throughput(Throughput::Bytes(actual_size as u64));
        group.bench_with_input(
            BenchmarkId::new("large_source", format!("{}KB", actual_size / 1024)),
            &source,
            |b, s| {
                b.iter(|| {
                    let mut dict = StringDict::new();
                    let mut parser = Parser::new(black_box(s), &mut dict);
                    let result = parser.parse_program();
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn bench_parser_expression_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser/expression_depth");

    // Test expression parsing at different nesting depths
    for depth in [5, 10, 15, 20] {
        let source = generate_binary_expr(depth);

        group.bench_with_input(
            BenchmarkId::new("binary_tree", format!("depth_{}", depth)),
            &source,
            |b, s| {
                b.iter(|| {
                    let mut dict = StringDict::new();
                    let mut parser = Parser::new(black_box(s), &mut dict);
                    let result = parser.parse_program();
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn bench_parser_statements(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser/statements");

    // Generate many simple statements
    let many_lets: String = (0..1000)
        .map(|i| format!("let x{} = {};\n", i, i))
        .collect();
    let many_fns: String = (0..100)
        .map(|i| format!("function f{}(a, b) {{ return a + b; }}\n", i))
        .collect();
    let many_classes: String = (0..50)
        .map(|i| format!("class C{} {{ constructor() {{ this.x = {}; }} }}\n", i, i))
        .collect();

    group.bench_function("1000_let_statements", |b| {
        b.iter(|| {
            let mut dict = StringDict::new();
            let mut parser = Parser::new(black_box(&many_lets), &mut dict);
            let result = parser.parse_program();
            black_box(result)
        });
    });

    group.bench_function("100_function_declarations", |b| {
        b.iter(|| {
            let mut dict = StringDict::new();
            let mut parser = Parser::new(black_box(&many_fns), &mut dict);
            let result = parser.parse_program();
            black_box(result)
        });
    });

    group.bench_function("50_class_declarations", |b| {
        b.iter(|| {
            let mut dict = StringDict::new();
            let mut parser = Parser::new(black_box(&many_classes), &mut dict);
            let result = parser.parse_program();
            black_box(result)
        });
    });

    group.finish();
}

fn bench_parser_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser/string_interning");

    // Test with many repeated identifiers (benefits from interning)
    let repeated_ids: String = (0..1000)
        .map(|_| "const x = foo + bar + baz + qux;\n")
        .collect();

    // Test with many unique identifiers (stress test for interning)
    let unique_ids: String = (0..1000)
        .map(|i| format!("const x{} = foo{} + bar{} + baz{};\n", i, i, i, i))
        .collect();

    group.bench_function("repeated_identifiers", |b| {
        b.iter(|| {
            let mut dict = StringDict::new();
            let mut parser = Parser::new(black_box(&repeated_ids), &mut dict);
            let result = parser.parse_program();
            black_box(result)
        });
    });

    group.bench_function("unique_identifiers", |b| {
        b.iter(|| {
            let mut dict = StringDict::new();
            let mut parser = Parser::new(black_box(&unique_ids), &mut dict);
            let result = parser.parse_program();
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parser_individual,
    bench_parser_throughput,
    bench_parser_expression_depth,
    bench_parser_statements,
    bench_parser_string_interning,
);
criterion_main!(benches);
