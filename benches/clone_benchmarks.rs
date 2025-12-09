//! Benchmarks for clone-heavy operations
//!
//! These benchmarks measure performance of operations that involve expensive clones.
//! Run with: cargo bench
//! Results saved to: target/criterion/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use typescript_eval::{JsValue, Runtime};

/// Helper to evaluate code and return result
fn eval(source: &str) -> JsValue {
    let mut runtime = Runtime::new();
    runtime.eval_simple(source).expect("eval failed")
}

/// Helper to just run code (ignore result)
fn run(source: &str) {
    let mut runtime = Runtime::new();
    let _ = runtime.eval_simple(source);
}

// ============================================================================
// ENVIRONMENT CLONE BENCHMARKS
// These measure the cost of scope chain cloning (Environment.clone())
// ============================================================================

/// Benchmark: Nested function definitions
/// Each function definition clones the environment for its closure
fn bench_nested_closures(c: &mut Criterion) {
    let mut group = c.benchmark_group("environment_clones");

    // Test different nesting depths
    for depth in [1, 3, 5, 10, 20] {
        let code = generate_nested_closures(depth);

        group.throughput(Throughput::Elements(depth as u64));
        group.bench_with_input(
            BenchmarkId::new("nested_closures", depth),
            &code,
            |b, code| {
                b.iter(|| run(black_box(code)));
            },
        );
    }

    group.finish();
}

/// Generate nested closure code with given depth
fn generate_nested_closures(depth: usize) -> String {
    let mut code = String::new();

    // Build nested functions
    for i in 0..depth {
        code.push_str(&format!("function f{}() {{ let x{} = {}; ", i, i, i));
    }

    // Return sum of all captured variables
    code.push_str("return ");
    for i in 0..depth {
        if i > 0 {
            code.push_str(" + ");
        }
        code.push_str(&format!("x{}", i));
    }
    code.push(';');

    // Close all functions and call the outermost
    for _ in 0..depth {
        code.push_str(" }");
    }
    code.push_str(" f0()");

    code
}

/// Benchmark: Closure capture in loop
/// Tests environment cloning when creating closures in a loop
fn bench_closure_in_loop(c: &mut Criterion) {
    let mut group = c.benchmark_group("environment_clones");

    for count in [10, 50, 100, 500] {
        let code = format!(
            r#"
            let fns = [];
            for (let i = 0; i < {}; i++) {{
                fns.push(() => i);
            }}
            fns[0]()
            "#,
            count
        );

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("closure_in_loop", count),
            &code,
            |b, code| {
                b.iter(|| run(black_box(code)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Try-catch blocks (create new scope for catch)
fn bench_try_catch_scopes(c: &mut Criterion) {
    let mut group = c.benchmark_group("environment_clones");

    for iterations in [10, 50, 100] {
        let code = format!(
            r#"
            let result = 0;
            for (let i = 0; i < {}; i++) {{
                try {{
                    result += i;
                }} catch (e) {{
                    result -= 1;
                }}
            }}
            result
            "#,
            iterations
        );

        group.throughput(Throughput::Elements(iterations as u64));
        group.bench_with_input(
            BenchmarkId::new("try_catch_in_loop", iterations),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

// ============================================================================
// AST CLONE BENCHMARKS
// These measure the cost of cloning function bodies and parameters
// ============================================================================

/// Benchmark: Function with large body
/// Tests cost of cloning BlockStatement
fn bench_large_function_body(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_clones");

    for statements in [10, 50, 100, 200] {
        let mut body = String::new();
        for i in 0..statements {
            body.push_str(&format!("let x{} = {}; ", i, i));
        }

        let code = format!(
            r#"
            function largeFunc() {{
                {}
                return x0;
            }}
            largeFunc()
            "#,
            body
        );

        group.throughput(Throughput::Elements(statements as u64));
        group.bench_with_input(
            BenchmarkId::new("large_function_body", statements),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Function with many parameters
/// Tests cost of cloning Vec<FunctionParam>
fn bench_many_parameters(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_clones");

    for param_count in [5, 10, 20, 50] {
        let params: Vec<String> = (0..param_count).map(|i| format!("p{}", i)).collect();
        let sum: Vec<String> = (0..param_count).map(|i| format!("p{}", i)).collect();
        let args: Vec<String> = (0..param_count).map(|i| i.to_string()).collect();

        let code = format!(
            r#"
            function manyParams({}) {{
                return {};
            }}
            manyParams({})
            "#,
            params.join(", "),
            sum.join(" + "),
            args.join(", ")
        );

        group.throughput(Throughput::Elements(param_count as u64));
        group.bench_with_input(
            BenchmarkId::new("many_parameters", param_count),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Generator functions
/// Generators clone body/params on each resume
fn bench_generator_iterations(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_clones");

    for yields in [5, 10, 25, 50] {
        let mut yield_stmts = String::new();
        for i in 0..yields {
            yield_stmts.push_str(&format!("yield {}; ", i));
        }

        let code = format!(
            r#"
            function* gen() {{
                {}
            }}
            let sum = 0;
            for (let v of gen()) {{
                sum += v;
            }}
            sum
            "#,
            yield_stmts
        );

        group.throughput(Throughput::Elements(yields as u64));
        group.bench_with_input(
            BenchmarkId::new("generator_yields", yields),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Class with many methods
/// Each method creates an InterpretedFunction with cloned body
fn bench_class_many_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_clones");

    for method_count in [5, 10, 20, 50] {
        let mut methods = String::new();
        for i in 0..method_count {
            methods.push_str(&format!(
                "method{}() {{ return {}; }} ",
                i, i
            ));
        }

        let code = format!(
            r#"
            class MyClass {{
                {}
            }}
            let obj = new MyClass();
            obj.method0()
            "#,
            methods
        );

        group.throughput(Throughput::Elements(method_count as u64));
        group.bench_with_input(
            BenchmarkId::new("class_methods", method_count),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

// ============================================================================
// CALLBACK/ARRAY BENCHMARKS
// These measure the cost of Vec<JsValue> allocation in callbacks
// ============================================================================

/// Benchmark: Array.map with callback
/// Creates new Vec for each callback invocation
fn bench_array_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("callback_allocs");

    for size in [10, 100, 500, 1000] {
        let code = format!(
            r#"
            let arr = [];
            for (let i = 0; i < {}; i++) arr.push(i);
            arr.map((x, i, a) => x * 2)
            "#,
            size
        );

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("array_map", size), &code, |b, code| {
            b.iter(|| run(black_box(code)));
        });
    }

    group.finish();
}

/// Benchmark: Array.filter
fn bench_array_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("callback_allocs");

    for size in [10, 100, 500, 1000] {
        let code = format!(
            r#"
            let arr = [];
            for (let i = 0; i < {}; i++) arr.push(i);
            arr.filter((x, i, a) => x % 2 === 0)
            "#,
            size
        );

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("array_filter", size), &code, |b, code| {
            b.iter(|| run(black_box(code)));
        });
    }

    group.finish();
}

/// Benchmark: Array.forEach
fn bench_array_foreach(c: &mut Criterion) {
    let mut group = c.benchmark_group("callback_allocs");

    for size in [10, 100, 500, 1000] {
        let code = format!(
            r#"
            let arr = [];
            for (let i = 0; i < {}; i++) arr.push(i);
            let sum = 0;
            arr.forEach((x, i, a) => {{ sum += x; }});
            sum
            "#,
            size
        );

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("array_foreach", size),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Array.reduce
fn bench_array_reduce(c: &mut Criterion) {
    let mut group = c.benchmark_group("callback_allocs");

    for size in [10, 100, 500, 1000] {
        let code = format!(
            r#"
            let arr = [];
            for (let i = 0; i < {}; i++) arr.push(i);
            arr.reduce((acc, x, i, a) => acc + x, 0)
            "#,
            size
        );

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("array_reduce", size), &code, |b, code| {
            b.iter(|| eval(black_box(code)));
        });
    }

    group.finish();
}

/// Benchmark: Chained array methods
fn bench_array_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("callback_allocs");

    for size in [10, 100, 500] {
        let code = format!(
            r#"
            let arr = [];
            for (let i = 0; i < {}; i++) arr.push(i);
            arr.filter(x => x % 2 === 0)
               .map(x => x * 2)
               .reduce((a, b) => a + b, 0)
            "#,
            size
        );

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("array_chain", size), &code, |b, code| {
            b.iter(|| eval(black_box(code)));
        });
    }

    group.finish();
}

// ============================================================================
// STRING CLONE BENCHMARKS
// These measure the cost of String cloning in identifiers
// ============================================================================

/// Benchmark: Many variable definitions
/// Each definition clones the identifier name
fn bench_many_variables(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_clones");

    for count in [10, 50, 100, 200] {
        let mut code = String::new();
        for i in 0..count {
            code.push_str(&format!("let variable_with_long_name_{} = {}; ", i, i));
        }
        code.push_str("variable_with_long_name_0");

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("variable_definitions", count),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Many property accesses
fn bench_property_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_clones");

    for count in [10, 50, 100, 200] {
        let mut setup = String::from("let obj = { ");
        for i in 0..count {
            setup.push_str(&format!("property_name_{}: {}, ", i, i));
        }
        setup.push_str("}; ");

        let mut accesses = String::new();
        for i in 0..count {
            accesses.push_str(&format!("obj.property_name_{} + ", i));
        }
        accesses.push('0');

        let code = format!("{}{}", setup, accesses);

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("property_access", count),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

// ============================================================================
// COMBINED/REALISTIC BENCHMARKS
// These simulate real-world usage patterns
// ============================================================================

/// Benchmark: Recursive fibonacci (deep call stack, many env clones)
fn bench_fibonacci(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic");

    for n in [10, 15, 20] {
        let code = format!(
            r#"
            function fib(n) {{
                if (n <= 1) return n;
                return fib(n - 1) + fib(n - 2);
            }}
            fib({})
            "#,
            n
        );

        group.bench_with_input(BenchmarkId::new("fibonacci", n), &code, |b, code| {
            b.iter(|| eval(black_box(code)));
        });
    }

    group.finish();
}

/// Benchmark: Object-heavy code (many property clones)
fn bench_object_manipulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic");

    for count in [10, 50, 100] {
        let code = format!(
            r#"
            let objects = [];
            for (let i = 0; i < {}; i++) {{
                objects.push({{
                    id: i,
                    name: "item" + i,
                    value: i * 10,
                    nested: {{
                        a: i,
                        b: i + 1
                    }}
                }});
            }}
            objects.map(o => o.value).reduce((a, b) => a + b, 0)
            "#,
            count
        );

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("object_manipulation", count),
            &code,
            |b, code| {
                b.iter(|| eval(black_box(code)));
            },
        );
    }

    group.finish();
}

/// Benchmark: Simulated config generation (realistic use case)
fn bench_config_generation(c: &mut Criterion) {
    let code = r#"
        function generateConfig(env) {
            const base = {
                name: "my-app",
                version: "1.0.0",
                debug: env === "development"
            };

            const endpoints = {
                development: "http://localhost:3000",
                staging: "https://staging.example.com",
                production: "https://example.com"
            };

            return {
                ...base,
                apiUrl: endpoints[env] || endpoints.production,
                features: {
                    analytics: env === "production",
                    logging: env !== "production",
                    cache: true
                },
                limits: {
                    maxRequests: env === "production" ? 1000 : 100,
                    timeout: 30000
                }
            };
        }

        let configs = ["development", "staging", "production"].map(env => generateConfig(env));
        configs.length
    "#;

    c.bench_function("config_generation", |b| {
        b.iter(|| eval(black_box(code)));
    });
}

// ============================================================================
// BASELINE BENCHMARKS
// Simple operations for comparison
// ============================================================================

/// Benchmark: Simple arithmetic (baseline, minimal cloning)
fn bench_arithmetic_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline");

    group.bench_function("simple_arithmetic", |b| {
        b.iter(|| eval(black_box("1 + 2 * 3 - 4 / 2")));
    });

    group.bench_function("arithmetic_loop", |b| {
        b.iter(|| {
            eval(black_box(
                r#"
                let sum = 0;
                for (let i = 0; i < 100; i++) {
                    sum += i * 2;
                }
                sum
                "#,
            ))
        });
    });

    group.finish();
}

/// Benchmark: Runtime creation (baseline overhead)
fn bench_runtime_creation(c: &mut Criterion) {
    c.bench_function("runtime_creation", |b| {
        b.iter(|| {
            let runtime = Runtime::new();
            black_box(runtime);
        });
    });
}

// ============================================================================
// CRITERION CONFIGURATION
// ============================================================================

criterion_group!(
    benches,
    // Environment clones
    bench_nested_closures,
    bench_closure_in_loop,
    bench_try_catch_scopes,
    // AST clones
    bench_large_function_body,
    bench_many_parameters,
    bench_generator_iterations,
    bench_class_many_methods,
    // Callback allocations
    bench_array_map,
    bench_array_filter,
    bench_array_foreach,
    bench_array_reduce,
    bench_array_chain,
    // String clones
    bench_many_variables,
    bench_property_access,
    // Realistic
    bench_fibonacci,
    bench_object_manipulation,
    bench_config_generation,
    // Baseline
    bench_arithmetic_baseline,
    bench_runtime_creation,
);

criterion_main!(benches);
