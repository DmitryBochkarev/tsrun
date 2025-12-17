//! Lexer benchmarks
//!
//! Run with: cargo bench --bench lexer
//! Profile with: cargo flamegraph --bench lexer -- --bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use typescript_eval::lexer::{Lexer, TokenKind};
use typescript_eval::string_dict::StringDict;

/// Simple expression
const SIMPLE_EXPR: &str = "1 + 2 * 3 - 4 / 5";

/// Variable declarations
const VARIABLES: &str = r#"
let x = 1;
const y = 2;
var z = 3;
let a = x + y + z;
const b = a * 2;
"#;

/// String literals with escapes
const STRINGS: &str = r#"
const hello = "Hello, World!";
const escaped = "Line1\nLine2\tTabbed";
const unicode = "\u{1F600} emoji \u0041";
const template = `Hello ${name}!`;
"#;

/// Operators stress test
const OPERATORS: &str = r#"
a + b - c * d / e % f ** g
x === y !== z == w != v
a && b || c ?? d
a & b | c ^ d ~ e
a << 2 >> 3 >>> 4
a += b -= c *= d /= e %= f **= g
a &&= b ||= c ??= d
a < b <= c > d >= e
++x --y x++ y--
a?.b a?.() a ?? b
...rest
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

/// Array operations
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

/// Numbers in various formats
const NUMBERS: &str = r#"
const decimal = 42;
const float = 3.14159;
const negative = -273.15;
const scientific = 6.022e23;
const hex = 0xFF;
const octal = 0o755;
const binary = 0b1010;
const bigint = 9007199254740991n;
const underscore = 1_000_000;
"#;

/// Comments stress test
const COMMENTS: &str = r#"
// Single line comment
const a = 1; // inline comment

/* Multi-line
   comment
   spanning
   multiple lines */
const b = 2;

/**
 * JSDoc style comment
 * @param x The first parameter
 * @param y The second parameter
 * @returns The sum of x and y
 */
function add(x: number, y: number): number {
    return x + y;
}

/* nested /* comments */ are tricky */
const c = 3;
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
    ];

    let mut i = 0;
    while source.len() < size {
        source.push_str(patterns[i % patterns.len()]);
        source.push_str("\n\n");
        i += 1;
    }
    source
}

fn bench_lexer_individual(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer/individual");

    let cases = [
        ("simple_expr", SIMPLE_EXPR),
        ("variables", VARIABLES),
        ("strings", STRINGS),
        ("operators", OPERATORS),
        ("class_def", CLASS_DEF),
        ("functions", FUNCTIONS),
        ("control_flow", CONTROL_FLOW),
        ("objects", OBJECTS),
        ("arrays", ARRAYS),
        ("types", TYPES),
        ("numbers", NUMBERS),
        ("comments", COMMENTS),
    ];

    for (name, source) in cases {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(BenchmarkId::new("bytes", name), source, |b, s| {
            let mut dict = StringDict::new();
            b.iter(|| {
                let mut lexer = Lexer::new(black_box(s), &mut dict);
                loop {
                    let token = lexer.next_token();
                    if token.kind == TokenKind::Eof {
                        break;
                    }
                    black_box(&token);
                }
            });
        });
    }

    group.finish();
}

fn bench_lexer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer/throughput");

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
                let mut dict = StringDict::new();
                b.iter(|| {
                    let mut lexer = Lexer::new(black_box(s), &mut dict);
                    loop {
                        let token = lexer.next_token();
                        if token.kind == TokenKind::Eof {
                            break;
                        }
                        black_box(&token);
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_lexer_token_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer/token_types");

    // Identifiers and keywords
    let identifiers = "foo bar baz qux let const var function class interface type async await yield";
    group.bench_function("identifiers_keywords", |b| {
        let mut dict = StringDict::new();
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(identifiers), &mut dict);
            loop {
                let token = lexer.next_token();
                if token.kind == TokenKind::Eof {
                    break;
                }
                black_box(&token);
            }
        });
    });

    // Numbers only
    let numbers = "1 2 3 42 3.14 1e10 0xFF 0o755 0b1010 123n 1_000_000";
    group.bench_function("numbers", |b| {
        let mut dict = StringDict::new();
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(numbers), &mut dict);
            loop {
                let token = lexer.next_token();
                if token.kind == TokenKind::Eof {
                    break;
                }
                black_box(&token);
            }
        });
    });

    // Strings only
    let strings = r#""hello" 'world' "escaped\n\t" "unicode\u0041" `template`"#;
    group.bench_function("strings", |b| {
        let mut dict = StringDict::new();
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(strings), &mut dict);
            loop {
                let token = lexer.next_token();
                if token.kind == TokenKind::Eof {
                    break;
                }
                black_box(&token);
            }
        });
    });

    // Operators only
    let operators = "+ - * / % ** ++ -- = == === != !== < <= > >= << >> >>> & && | || ^ ~ ! ? ?? ?. => ...";
    group.bench_function("operators", |b| {
        let mut dict = StringDict::new();
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(operators), &mut dict);
            loop {
                let token = lexer.next_token();
                if token.kind == TokenKind::Eof {
                    break;
                }
                black_box(&token);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_lexer_individual,
    bench_lexer_throughput,
    bench_lexer_token_types,
);
criterion_main!(benches);
