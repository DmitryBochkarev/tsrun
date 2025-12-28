# tsrun

A TypeScript interpreter written in Rust, designed for embedding in applications.

## Overview

tsrun executes TypeScript code directly without transpilation to JavaScript. It uses a register-based bytecode VM for efficient execution. Type annotations are parsed but stripped at runtime (no type checking).

## Features

- ES modules with import/export
- Async/await and Promises
- Classes with inheritance and static blocks
- Generators and iterators
- Destructuring and spread operators
- Template literals
- Full set of built-in objects: Array, String, Object, Map, Set, Date, RegExp, JSON, Math, and more

## Installation

```bash
cargo install tsrun
```

## Usage

```bash
tsrun script.ts
```

## Library Usage

```rust
use tsrun::Runtime;

fn main() {
    let mut runtime = Runtime::new();
    let result = runtime.execute("const x = 1 + 2; x").unwrap();
    println!("{:?}", result);
}
```

## License

MIT
