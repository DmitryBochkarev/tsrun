//! Profiling binary for the parser
//!
//! Build with: cargo build --release --bin profile_parser
//! Profile with perf: perf record --call-graph=dwarf ./target/release/profile_parser
//!                    perf report
//! Or with flamegraph: cargo flamegraph --bin profile_parser

use typescript_eval::parser::Parser;
use typescript_eval::string_dict::StringDict;

/// Large realistic TypeScript source for profiling
fn generate_source(size: usize) -> String {
    let patterns = [
        // Class definition
        r#"
class Counter extends Base {
    #count: number = 0;
    private name: string;

    constructor(name: string, initialValue: number = 0) {
        super();
        this.name = name;
        this.#count = initialValue;
    }

    get value(): number { return this.#count; }
    increment(): Counter { this.#count++; return this; }
}
"#,
        // Functions
        r#"
function process(data: Record<string, unknown>): Promise<Result> {
    const { items, meta } = data;
    return items.map((item: Item) => ({
        ...item,
        processed: true,
        timestamp: Date.now(),
    }));
}
const arrow = async (x: number): Promise<number> => {
    return x * 2;
};
"#,
        // Control flow
        r#"
if (condition && otherCondition) {
    for (let i = 0; i < items.length; i++) {
        const item = items[i];
        switch (item.type) {
            case "a": handleA(item); break;
            case "b": handleB(item); break;
            default: handleDefault(item);
        }
    }
} else {
    try {
        riskyOperation();
    } catch (error) {
        console.error(error);
    }
}
"#,
        // Objects and arrays
        r#"
const config = {
    name: "MyApp",
    version: "1.0.0",
    settings: { debug: true, logLevel: "info" },
    endpoints: [
        { path: "/api/users", method: "GET" },
        { path: "/api/posts", method: "POST" },
    ],
};
const [first, ...rest] = numbers.filter(n => n > 0).map(n => n * 2);
"#,
    ];

    let mut source = String::with_capacity(size);
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

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Default to 1MB, can be overridden with command line arg
    let size: usize = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);

    let iterations: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100);

    eprintln!("Generating {}KB source...", size / 1024);
    let source = generate_source(size);
    eprintln!("Source size: {} bytes", source.len());

    eprintln!("Running {} iterations of parser...", iterations);

    let start = std::time::Instant::now();
    let mut total_statements = 0usize;

    for i in 0..iterations {
        let mut dict = StringDict::new();
        let mut parser = Parser::new(&source, &mut dict);
        match parser.parse_program() {
            Ok(program) => {
                total_statements += program.body.len();
            }
            Err(e) => {
                if i == 0 {
                    eprintln!("Parse error: {}", e);
                }
            }
        }
    }

    let elapsed = start.elapsed();
    let bytes_per_sec = (source.len() * iterations) as f64 / elapsed.as_secs_f64();

    eprintln!("Done in {:?}", elapsed);
    eprintln!("Total statements: {}", total_statements);
    eprintln!("Throughput: {:.2} MB/s", bytes_per_sec / 1_000_000.0);
}
