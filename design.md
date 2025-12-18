# TypeScript Interpreter Design Document

## Overview

**Project:** `typescript-eval`
**Purpose:** Execute TypeScript for config/manifest generation from Rust
**Status:** Milestone 10 Complete (Decorators, 909 tests passing)

### Requirements

- Full TypeScript syntax support (types stripped, not checked at runtime)
- Static import resolution with host-provided modules
- Order-based async model (host fulfills external effects)
- Internal module system (native Rust or TypeScript source)
- Guard-based garbage collection
- **Zero-panic policy** - no runtime panics in production code

### Execution Model

The interpreter uses an **order-based suspension model**:

1. **Static Imports**: Collected before execution, host provides modules
2. **Orders**: Async operations suspend and return "orders" for host to fulfill
3. **Resumption**: Host fulfills orders, interpreter continues until completion

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         EXECUTION FLOW                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. runtime.eval(source)                                                     │
│     │                                                                        │
│     ├──▶ Internal modules (eval:*) → resolve automatically                   │
│     └──▶ External modules → return NeedImports([...])                        │
│                                                                              │
│  2. Host provides modules via runtime.provide_module(specifier, source)      │
│                                                                              │
│  3. runtime.continue_eval() → begin execution                                │
│                                                                              │
│  4. Code calls __order__() → interpreter suspends                            │
│     │                                                                        │
│     └──▶ return Suspended { pending: [...], cancelled: [...] }               │
│                                                                              │
│  5. Host fulfills orders → runtime.fulfill_orders(responses)                 │
│                                                                              │
│  6. Repeat 4-5 until Complete(value)                                         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Next Implementation Priorities

### Priority 1: Async/Await via Orders

- Async functions return order-based promises
- `await` suspends until host fulfills order
- No internal promise resolution (all async is external)

### Priority 2: Serde Integration

- `JsValue` ↔ `serde_json::Value` conversion
- Direct struct serialization/deserialization

### Completed Priorities
- ~~**Guard-based GC**~~ ✅
- ~~**Core language features**~~ ✅
- ~~**Built-in objects**~~ ✅ (Array, String, Object, Number, Math, JSON, Date, Map, Set, Symbol, RegExp, Error)
- ~~**Classes**~~ ✅
- ~~**Zero-Panic Policy**~~ ✅
- ~~**Order System & Internal Modules**~~ ✅
  - `Order`, `OrderId`, `OrderResponse` types
  - `RuntimeResult` enum: `Complete`, `NeedImports`, `Suspended`
  - Native modules (Rust functions) and Source modules (TypeScript)
  - `eval:internal` with `__order__`, `__cancelOrder__`, `__getOrderId__`
  - Import/export statement execution
  - Static import resolution (internal modules resolve automatically)
- ~~**Decorators**~~ ✅
  - Class, method, getter/setter, field decorators
  - Decorator factories and composition
  - Static and private member decorators
  - Context object with `kind`, `name`, `static`, `private`

---

## Architecture

### Runtime Result Model

```rust
/// Unique identifier for an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(pub u64);

/// An order is a request for an external effect
pub struct Order {
    pub id: OrderId,
    pub payload: Guarded,  // JS value describing what to do
}

/// Response to fulfill an order
pub struct OrderResponse {
    pub id: OrderId,
    pub result: Result<JsValue, JsError>,
}

/// Result of running the interpreter
pub enum RuntimeResult {
    /// Execution completed with a value
    Complete(JsValue),

    /// Need these modules before execution can start
    NeedImports(Vec<String>),

    /// Waiting for orders to be fulfilled
    Suspended {
        pending: Vec<Order>,
        cancelled: Vec<OrderId>,
    },
}
```

### Internal Module System

Internal modules can be **native** (Rust) or **source** (TypeScript):

```rust
/// How an internal module is defined
pub enum InternalModuleKind {
    /// Native module with Rust functions
    Native(Vec<(String, InternalExport)>),
    /// Source module (TypeScript code)
    Source(String),
}

/// Definition of an internal module
pub struct InternalModule {
    pub specifier: String,  // e.g., "eval:internal", "eval:fs"
    pub kind: InternalModuleKind,
}
```

### Runtime Configuration

```rust
/// Configuration for creating a Runtime
pub struct RuntimeConfig {
    /// Internal modules available for import
    pub internal_modules: Vec<InternalModule>,
    /// Timeout in milliseconds (0 = no timeout)
    pub timeout_ms: u64,
}

let config = RuntimeConfig {
    internal_modules: vec![
        // Core order system - native
        InternalModule::native("eval:internal")
            .with_function("__order__", order_syscall, 1)
            .with_function("__cancelOrder__", cancel_order_syscall, 1)
            .build(),

        // FS module - TypeScript source
        InternalModule::source("eval:fs", r#"
            import { __order__ } from "eval:internal";
            export async function readFile(path: string): Promise<string> {
                return __order__({ type: "readFile", path });
            }
        "#),
    ],
    timeout_ms: 5000,
};

let mut runtime = Runtime::with_config(config);
```

### Host Loop Pattern

```rust
fn run_script(source: &str, config: RuntimeConfig) -> Result<JsValue, Error> {
    let mut runtime = Runtime::with_config(config);
    let mut result = runtime.eval(source)?;

    loop {
        match result {
            RuntimeResult::Complete(value) => return Ok(value),

            RuntimeResult::NeedImports(specifiers) => {
                for spec in &specifiers {
                    let module_source = load_module_from_disk(spec)?;
                    runtime.provide_module(spec, &module_source)?;
                }
                result = runtime.continue_eval()?;
            }

            RuntimeResult::Suspended { pending, cancelled } => {
                // Handle cancelled orders
                for id in cancelled {
                    cancel_pending_operation(id);
                }

                // Fulfill pending orders
                let responses: Vec<OrderResponse> = pending
                    .into_iter()
                    .map(|order| fulfill_order(order))
                    .collect();

                result = runtime.fulfill_orders(responses)?;
            }
        }
    }
}
```

---

## Feature Checklist

### JavaScript Language Features

#### Variables & Declarations
- [x] `let` declarations
- [x] `const` declarations
- [x] `var` declarations (function-scoped)
- [x] Variable hoisting (var)
- [x] Temporal Dead Zone (let/const)
- [x] Multiple declarators (`let a = 1, b = 2`)

#### Primitive Types & Literals
- [x] `undefined`
- [x] `null`
- [x] Boolean (`true`, `false`)
- [x] Number (integers, floats, `NaN`, `Infinity`)
- [x] String (single/double quotes)
- [x] Template literals (backticks)
- [x] Template literal interpolation (`${expr}`)
- [x] Tagged template literals
- [x] BigInt literals (`123n`) - parsed, converted to Number at runtime
- [x] Symbol

#### Operators
- [x] Arithmetic (`+`, `-`, `*`, `/`, `%`, `**`)
- [x] Comparison (`<`, `>`, `<=`, `>=`)
- [x] Equality (`==`, `!=`, `===`, `!==`)
- [x] Logical (`&&`, `||`, `!`)
- [x] Nullish coalescing (`??`)
- [x] Bitwise (`&`, `|`, `^`, `~`, `<<`, `>>`, `>>>`)
- [x] Unary (`+`, `-`, `!`, `~`, `typeof`, `void`, `delete`)
- [x] Update (`++`, `--`, prefix and postfix)
- [x] Assignment (`=`, `+=`, `-=`, `*=`, `/=`, etc.)
- [x] Logical assignment (`&&=`, `||=`, `??=`)
- [x] Conditional/ternary (`? :`)
- [x] Comma operator
- [x] `typeof` operator
- [x] `instanceof` operator
- [x] `in` operator

#### Control Flow
- [x] `if` / `else if` / `else`
- [x] `switch` / `case` / `default`
- [x] `for` loop
- [x] `for...in` loop
- [x] `for...of` loop
- [x] `while` loop
- [x] `do...while` loop
- [x] `break` (with optional label)
- [x] `continue` (with optional label)
- [x] Labeled statements

#### Functions
- [x] Function declarations
- [x] Function expressions
- [x] Arrow functions
- [x] Arrow functions with expression body
- [x] Default parameters
- [x] Rest parameters (`...args`)
- [x] Closures
- [x] `return` statement
- [x] Implicit `undefined` return
- [x] Generator functions (`function*`)
- [x] `yield` / `yield*`
- [x] `this` binding
- [x] `arguments` object
- [x] `Function.prototype.call`
- [x] `Function.prototype.apply`
- [x] `Function.prototype.bind`

#### Objects
- [x] Object literals
- [x] Computed property names (`{ [expr]: value }`)
- [x] Shorthand property names (`{ x }` for `{ x: x }`)
- [x] Method shorthand (`{ method() {} }`)
- [x] Getter/setter (`get`/`set`)
- [x] Property access (dot notation)
- [x] Property access (bracket notation)
- [x] Optional chaining (`?.`)
- [x] Spread in object literals (`{ ...obj }`)
- [x] `__proto__` property
- [x] Prototype chain lookup

#### Arrays
- [x] Array literals
- [x] Array element access
- [x] Spread in arrays (`[...arr]`)
- [x] Array holes (`[1, , 3]`)
- [x] `length` property

#### Destructuring
- [x] Object destructuring in declarations
- [x] Array destructuring in declarations
- [x] Nested destructuring
- [x] Default values in destructuring
- [x] Rest in destructuring (`{ a, ...rest }`)
- [x] Destructuring in function parameters
- [x] Destructuring in assignment expressions

#### Classes
- [x] Class declarations
- [x] Class expressions
- [x] Constructor
- [x] Instance methods
- [x] Static methods
- [x] Instance fields
- [x] Static fields
- [x] `extends` (inheritance)
- [x] `super` calls
- [x] `super` property access
- [x] Private fields (`#field`)
- [x] Private methods (`#method()`)
- [x] Static initialization blocks

#### Error Handling
- [x] `try` / `catch` / `finally`
- [x] `throw` statement
- [x] Error stack traces
- [x] Custom error types (Error, TypeError, ReferenceError, SyntaxError, RangeError)

#### Modules (ES Modules)
- [x] `import` declarations (parsing)
- [x] `export` declarations (parsing + runtime tracking)
- [x] Named imports/exports (parsing)
- [x] Default imports/exports (parsing)
- [x] Namespace imports (`import * as`) (parsing)
- [x] Re-exports (`export { x } from`) (parsing)
- [x] Static import resolution (via `RuntimeResult::NeedImports`)

#### Async/Await
- [x] `async function` declarations and expressions
- [x] `await` expression (suspends, host fulfills)
- [x] Top-level await in modules
- [x] Async arrow functions

### TypeScript Features

#### Type Annotations (Parse & Ignore)
- [x] Variable type annotations (`: type`)
- [x] Function parameter types
- [x] Function return types
- [x] Optional parameters (`param?`)
- [x] Type assertions (`x as T`)
- [x] Angle bracket assertions (`<T>x`)
- [x] Non-null assertions (`x!`)
- [x] `readonly` modifier

#### Type Declarations (Parse & Ignore)
- [x] `type` aliases
- [x] `interface` declarations
- [x] Generic type parameters (`<T>`)
- [x] Union types (`A | B`)
- [x] Intersection types (`A & B`)
- [x] Tuple types (`[A, B]`)
- [x] Array types (`T[]`, `Array<T>`)
- [x] Object types (`{ x: number }`)
- [x] Function types (`(x: T) => R`)
- [x] Literal types (`"hello"`, `42`)
- [x] `keyof` operator
- [x] `typeof` in types
- [x] Conditional types (`T extends U ? X : Y`)
- [x] Mapped types
- [x] Index access types (`T[K]`)

#### TypeScript-Specific
- [x] `enum` declarations → compile to objects
- [x] `const enum` → inline values
- [x] `namespace` / `module` declarations
- [x] Declaration merging (namespace)
- [x] Accessibility modifiers (`public`, `private`, `protected`) - parsed, ignored
- [x] `abstract` classes - parsed, ignored
- [x] `implements` clause - parsed, ignored

### Built-in Objects & Methods

#### Global Functions
- [x] `parseInt(string, radix?)`
- [x] `parseFloat(string)`
- [x] `isNaN(value)`
- [x] `isFinite(value)`
- [x] `encodeURI(uri)`
- [x] `decodeURI(uri)`
- [x] `encodeURIComponent(str)`
- [x] `decodeURIComponent(str)`

#### Object
- [x] `Object.keys(obj)`
- [x] `Object.values(obj)`
- [x] `Object.entries(obj)`
- [x] `Object.assign(target, ...sources)`
- [x] `Object.freeze(obj)`
- [x] `Object.seal(obj)`
- [x] `Object.isFrozen(obj)`
- [x] `Object.isSealed(obj)`
- [x] `Object.getOwnPropertyNames(obj)`
- [x] `Object.getOwnPropertySymbols(obj)`
- [x] `Object.getOwnPropertyDescriptor(obj, prop)`
- [x] `Object.defineProperty(obj, prop, descriptor)`
- [x] `Object.defineProperties(obj, props)`
- [x] `Object.getPrototypeOf(obj)`
- [x] `Object.setPrototypeOf(obj, proto)`
- [x] `Object.create(proto, props?)`
- [x] `Object.fromEntries(iterable)`
- [x] `Object.hasOwn(obj, prop)`
- [x] `Object.prototype.hasOwnProperty(prop)`
- [x] `Object.prototype.toString()`
- [x] `Object.prototype.valueOf()`

#### Array
- [x] `Array.isArray(value)`
- [x] `Array.from(arrayLike, mapFn?)`
- [x] `Array.of(...items)`
- [x] `Array.prototype.push(...items)`
- [x] `Array.prototype.pop()`
- [x] `Array.prototype.shift()`
- [x] `Array.prototype.unshift(...items)`
- [x] `Array.prototype.slice(start?, end?)`
- [x] `Array.prototype.splice(start, deleteCount?, ...items)`
- [x] `Array.prototype.concat(...items)`
- [x] `Array.prototype.join(separator?)`
- [x] `Array.prototype.reverse()`
- [x] `Array.prototype.sort(compareFn?)`
- [x] `Array.prototype.indexOf(item, fromIndex?)`
- [x] `Array.prototype.lastIndexOf(item, fromIndex?)`
- [x] `Array.prototype.includes(item, fromIndex?)`
- [x] `Array.prototype.find(predicate)`
- [x] `Array.prototype.findIndex(predicate)`
- [x] `Array.prototype.findLast(predicate)`
- [x] `Array.prototype.findLastIndex(predicate)`
- [x] `Array.prototype.filter(predicate)`
- [x] `Array.prototype.map(callback)`
- [x] `Array.prototype.forEach(callback)`
- [x] `Array.prototype.reduce(callback, initial?)`
- [x] `Array.prototype.reduceRight(callback, initial?)`
- [x] `Array.prototype.every(predicate)`
- [x] `Array.prototype.some(predicate)`
- [x] `Array.prototype.flat(depth?)`
- [x] `Array.prototype.flatMap(callback)`
- [x] `Array.prototype.fill(value, start?, end?)`
- [x] `Array.prototype.copyWithin(target, start?, end?)`
- [x] `Array.prototype.entries()`
- [x] `Array.prototype.keys()`
- [x] `Array.prototype.values()`
- [x] `Array.prototype.at(index)`
- [x] `Array.prototype.toReversed()`
- [x] `Array.prototype.toSorted(compareFn?)`
- [x] `Array.prototype.toSpliced(start, deleteCount?, ...items)`
- [x] `Array.prototype.with(index, value)`

#### String
- [x] `String.fromCharCode(...codes)`
- [x] `String.fromCodePoint(...codePoints)`
- [x] `String.prototype.charAt(index)`
- [x] `String.prototype.charCodeAt(index)`
- [x] `String.prototype.codePointAt(index)`
- [x] `String.prototype.concat(...strings)`
- [x] `String.prototype.includes(search, position?)`
- [x] `String.prototype.startsWith(search, position?)`
- [x] `String.prototype.endsWith(search, length?)`
- [x] `String.prototype.indexOf(search, position?)`
- [x] `String.prototype.lastIndexOf(search, position?)`
- [x] `String.prototype.slice(start?, end?)`
- [x] `String.prototype.substring(start, end?)`
- [x] `String.prototype.substr(start, length?)` (deprecated)
- [x] `String.prototype.split(separator?, limit?)`
- [x] `String.prototype.toLowerCase()`
- [x] `String.prototype.toUpperCase()`
- [x] `String.prototype.trim()`
- [x] `String.prototype.trimStart()`
- [x] `String.prototype.trimEnd()`
- [x] `String.prototype.padStart(length, padString?)`
- [x] `String.prototype.padEnd(length, padString?)`
- [x] `String.prototype.repeat(count)`
- [x] `String.prototype.replace(search, replacement)`
- [x] `String.prototype.replaceAll(search, replacement)`
- [x] `String.prototype.match(regexp)`
- [x] `String.prototype.matchAll(regexp)`
- [x] `String.prototype.search(regexp)`
- [x] `String.prototype.at(index)`
- [x] `String.prototype.normalize(form?)`
- [x] `String.prototype.localeCompare(other)`

#### Number
- [x] `Number.isNaN(value)`
- [x] `Number.isFinite(value)`
- [x] `Number.isInteger(value)`
- [x] `Number.isSafeInteger(value)`
- [x] `Number.parseInt(string, radix?)`
- [x] `Number.parseFloat(string)`
- [x] `Number.prototype.toFixed(digits?)`
- [x] `Number.prototype.toPrecision(precision?)`
- [x] `Number.prototype.toExponential(digits?)`
- [x] `Number.prototype.toString(radix?)`
- [x] `Number.POSITIVE_INFINITY`
- [x] `Number.NEGATIVE_INFINITY`
- [x] `Number.MAX_VALUE`
- [x] `Number.MIN_VALUE`
- [x] `Number.MAX_SAFE_INTEGER`
- [x] `Number.MIN_SAFE_INTEGER`
- [x] `Number.EPSILON`
- [x] `Number.NaN`

#### Math
- [x] `Math.abs(x)`
- [x] `Math.ceil(x)`
- [x] `Math.floor(x)`
- [x] `Math.round(x)`
- [x] `Math.trunc(x)`
- [x] `Math.sign(x)`
- [x] `Math.max(...values)`
- [x] `Math.min(...values)`
- [x] `Math.pow(base, exp)`
- [x] `Math.sqrt(x)`
- [x] `Math.cbrt(x)`
- [x] `Math.hypot(...values)`
- [x] `Math.log(x)`
- [x] `Math.log10(x)`
- [x] `Math.log2(x)`
- [x] `Math.log1p(x)`
- [x] `Math.exp(x)`
- [x] `Math.expm1(x)`
- [x] `Math.sin(x)`, `Math.cos(x)`, `Math.tan(x)`
- [x] `Math.asin(x)`, `Math.acos(x)`, `Math.atan(x)`
- [x] `Math.sinh(x)`, `Math.cosh(x)`, `Math.tanh(x)`
- [x] `Math.asinh(x)`, `Math.acosh(x)`, `Math.atanh(x)`
- [x] `Math.atan2(y, x)`
- [x] `Math.random()`
- [x] `Math.PI`, `Math.E`, `Math.LN2`, `Math.LN10`, etc.

#### JSON
- [x] `JSON.parse(text, reviver?)`
- [x] `JSON.stringify(value, replacer?, space?)`

#### Symbol
- [x] `Symbol(description?)`
- [x] `Symbol.for(key)`
- [x] `Symbol.keyFor(sym)`
- [x] `Symbol.prototype.toString()`
- [x] `Symbol.prototype.valueOf()`
- [x] `Symbol.prototype.description`
- [x] Well-known symbols (iterator, toStringTag, hasInstance, etc.)

#### Error
- [x] `new Error(message?)`
- [x] `new TypeError(message?)`
- [x] `new ReferenceError(message?)`
- [x] `new SyntaxError(message?)`
- [x] `new RangeError(message?)`
- [x] `new URIError(message?)`
- [x] `new EvalError(message?)`
- [x] `Error.prototype.stack`
- [x] `Error.prototype.toString()`

#### Map
- [x] `new Map(iterable?)`
- [x] `Map.prototype.get(key)`
- [x] `Map.prototype.set(key, value)`
- [x] `Map.prototype.has(key)`
- [x] `Map.prototype.delete(key)`
- [x] `Map.prototype.clear()`
- [x] `Map.prototype.size`
- [x] `Map.prototype.keys()`
- [x] `Map.prototype.values()`
- [x] `Map.prototype.entries()`
- [x] `Map.prototype.forEach(callback)`

#### Set
- [x] `new Set(iterable?)`
- [x] `Set.prototype.add(value)`
- [x] `Set.prototype.has(value)`
- [x] `Set.prototype.delete(value)`
- [x] `Set.prototype.clear()`
- [x] `Set.prototype.size`
- [x] `Set.prototype.keys()`
- [x] `Set.prototype.values()`
- [x] `Set.prototype.entries()`
- [x] `Set.prototype.forEach(callback)`

#### Date
- [x] `new Date()`
- [x] `new Date(timestamp)`
- [x] `new Date(dateString)`
- [x] `new Date(year, month, day?, ...)`
- [x] `Date.now()`
- [x] `Date.parse(dateString)`
- [x] `Date.UTC(year, month, day?, ...)`
- [x] `Date.prototype.getTime()`
- [x] `Date.prototype.getFullYear()`, `getMonth()`, `getDate()`, etc.
- [x] `Date.prototype.setFullYear()`, `setMonth()`, `setDate()`, etc.
- [x] `Date.prototype.toISOString()`
- [x] `Date.prototype.toJSON()`
- [x] `Date.prototype.toString()`
- [x] `Date.prototype.toDateString()`
- [x] `Date.prototype.toTimeString()`

#### RegExp
- [x] RegExp literals (`/pattern/flags`)
- [x] `new RegExp(pattern, flags?)`
- [x] `RegExp.prototype.test(string)`
- [x] `RegExp.prototype.exec(string)`
- [x] `RegExp.prototype.source`
- [x] `RegExp.prototype.flags`
- [x] `RegExp.prototype.global`
- [x] `RegExp.prototype.ignoreCase`
- [x] `RegExp.prototype.multiline`
- [x] `RegExp.prototype.dotAll`
- [x] `RegExp.prototype.unicode`
- [x] `RegExp.prototype.sticky`

#### Console
- [x] `console.log(...args)`
- [x] `console.error(...args)`
- [x] `console.warn(...args)`
- [x] `console.info(...args)`
- [x] `console.debug(...args)`
- [x] `console.table(data)`
- [x] `console.dir(obj)`
- [x] `console.time(label)`
- [x] `console.timeEnd(label)`
- [x] `console.count(label)`
- [x] `console.countReset(label)`
- [x] `console.clear()`
- [x] `console.group(label)`
- [x] `console.groupEnd()`

#### Promise
- [x] `new Promise(executor)`
- [x] `Promise.prototype.then(onFulfilled, onRejected)`
- [x] `Promise.prototype.catch(onRejected)`
- [x] `Promise.prototype.finally(onFinally)`
- [x] `Promise.resolve(value)`
- [x] `Promise.reject(reason)`
- [x] `Promise.all(iterable)`
- [x] `Promise.race(iterable)`
- [x] `Promise.allSettled(iterable)`
- [x] `Promise.any(iterable)`

**Note:** Promise uses order-based semantics - async operations create orders that the host fulfills.

---

## Not Yet Implemented

This section documents features that are NOT implemented for a complete TypeScript/JavaScript interpreter.

### Parsed But Not Evaluated

These features have AST support but are ignored or partially handled at runtime:

#### Decorators
- [x] Class decorators (`@decorator class Foo {}`)
- [x] Method decorators (`@decorator method() {}`)
- [x] Property/field decorators (`@decorator prop = value`)
- [x] Parameter decorators (`method(@decorator param) {}`)
- [x] Decorator factories (`@decorator() class Foo {}`)
- [x] Decorator composition (multiple decorators)
- [x] Static member decorators
- [x] Private member decorators
- [x] Getter/setter decorators
- [x] Decorator context object (`kind`, `name`, `static`, `private`, `function`, `index`)
- [x] `context.addInitializer()`
- [x] Auto-accessor decorators (`accessor` keyword)

**Status:** Fully implemented for all decorator types including auto-accessors. Context provides `addInitializer()` for class decorators.

#### Dynamic Import
- [ ] `import("./module")` expression at runtime
- [ ] Dynamic module specifiers (`import(variable)`)
- [ ] `import.meta` object
- [ ] `import.meta.url`

**Status:** Parsed as call expression, but no runtime module loading. Only static imports via host are supported.

#### Async Iteration
- [x] `for await...of` loops with arrays of promises
- [x] `for await...of` with custom async iterables (Symbol.asyncIterator)
- [x] `Symbol.asyncIterator` on generator prototype
- [x] Async generator functions (`async function*`)
- [x] `yield` in async generators
- [x] `yield*` with async generators
- [ ] Nested async generators in for-await-of (partial)
- [ ] `generator.throw()` with async generators

**Status:** Core async iteration is implemented. `for await...of` works with arrays of promises, custom async iterables via `Symbol.asyncIterator`, and async generators. Nested async generator patterns have some edge cases.

### Missing Built-in Objects

#### WeakMap
- [ ] `new WeakMap()`
- [ ] `WeakMap.prototype.get(key)`
- [ ] `WeakMap.prototype.set(key, value)`
- [ ] `WeakMap.prototype.has(key)`
- [ ] `WeakMap.prototype.delete(key)`

#### WeakSet
- [ ] `new WeakSet()`
- [ ] `WeakSet.prototype.add(value)`
- [ ] `WeakSet.prototype.has(value)`
- [ ] `WeakSet.prototype.delete(value)`

#### WeakRef & FinalizationRegistry
- [ ] `new WeakRef(target)`
- [ ] `WeakRef.prototype.deref()`
- [ ] `new FinalizationRegistry(callback)`
- [ ] `FinalizationRegistry.prototype.register()`
- [ ] `FinalizationRegistry.prototype.unregister()`

#### Proxy & Reflect
- [ ] `new Proxy(target, handler)`
- [ ] All proxy traps (get, set, has, deleteProperty, etc.)
- [ ] `Reflect.get()`, `Reflect.set()`, `Reflect.has()`, etc.
- [ ] Revocable proxies (`Proxy.revocable()`)

#### ArrayBuffer & TypedArrays
- [ ] `ArrayBuffer`
- [ ] `SharedArrayBuffer`
- [ ] `DataView`
- [ ] `Int8Array`, `Uint8Array`, `Uint8ClampedArray`
- [ ] `Int16Array`, `Uint16Array`
- [ ] `Int32Array`, `Uint32Array`
- [ ] `Float32Array`, `Float64Array`
- [ ] `BigInt64Array`, `BigUint64Array`

#### BigInt (Full Support)
- [ ] Arbitrary precision arithmetic
- [ ] `BigInt()` constructor
- [ ] `BigInt.asIntN()`, `BigInt.asUintN()`
- [ ] BigInt operators with proper semantics

**Current:** BigInt literals (`123n`) are parsed but converted to `Number` at runtime, losing precision.

#### Intl API
- [ ] `Intl.Collator`
- [ ] `Intl.DateTimeFormat`
- [ ] `Intl.NumberFormat`
- [ ] `Intl.PluralRules`
- [ ] `Intl.RelativeTimeFormat`
- [ ] `Intl.ListFormat`
- [ ] `Intl.Segmenter`
- [ ] `Intl.DisplayNames`
- [ ] `Intl.Locale`

#### Atomics
- [ ] `Atomics.add()`, `Atomics.sub()`, etc.
- [ ] `Atomics.wait()`, `Atomics.notify()`
- [ ] `Atomics.load()`, `Atomics.store()`

### Missing Object Methods

#### Object
- [x] `Object.is(value1, value2)`
- [x] `Object.preventExtensions(obj)`
- [x] `Object.isExtensible(obj)`
- [x] `Object.getOwnPropertyDescriptors(obj)`

#### Array
- [ ] `Array.fromAsync(asyncIterable)`

#### String
- [ ] `String.prototype.isWellFormed()`
- [ ] `String.prototype.toWellFormed()`

### Missing Global Functions

- [ ] `eval(code)` - **Intentionally excluded for security**
- [ ] `setTimeout(callback, delay)` - Host-dependent, use orders
- [ ] `setInterval(callback, delay)` - Host-dependent, use orders
- [ ] `clearTimeout(id)` - Host-dependent
- [ ] `clearInterval(id)` - Host-dependent
- [ ] `setImmediate(callback)` - Host-dependent
- [ ] `queueMicrotask(callback)` - Requires microtask queue
- [x] `structuredClone(value)` - Deep cloning
- [x] `atob(encoded)` - Base64 decode
- [x] `btoa(string)` - Base64 encode

**Note:** Timer functions are intentionally not built-in. They should be implemented via the order system by the host.

### Language Features Not Implemented

#### with Statement
- [ ] `with (obj) { ... }`

**Status:** Intentionally not implemented (deprecated, strict mode forbidden).

#### Direct eval
- [ ] `eval("code")` with access to local scope

**Status:** Intentionally excluded for security reasons.

#### Legacy Features
- [ ] `__proto__` property access in literals (`{ __proto__: proto }`) - partially supported
- [ ] `Object.prototype.__defineGetter__()` / `__defineSetter__()`
- [ ] `Object.prototype.__lookupGetter__()` / `__lookupSetter__()`
- [ ] `escape()` / `unescape()` (deprecated)

### TypeScript Features Not Enforced

These are parsed but not enforced at runtime (by design - matches TypeScript behavior):

- `readonly` modifier - properties can be mutated
- `private` / `protected` - no access control at runtime
- `abstract` classes - can be instantiated
- Type assertions - no runtime checking
- Generic constraints - no runtime checking

### Module System Gaps

- [ ] Circular dependency detection/handling
- [ ] Live bindings for exports
- [ ] `export * as ns from "module"`
- [ ] Dynamic import with variable specifiers

### Potential Future Additions

#### High Priority (Common Use Cases)
1. Full BigInt support

#### Medium Priority (Improved Compatibility)
1. WeakMap / WeakSet
2. Async iterators (`for await...of`)
3. `import.meta`

#### Low Priority (Specialized Use)
1. Proxy & Reflect
2. TypedArrays / ArrayBuffer
3. Intl API
4. Atomics / SharedArrayBuffer

---

### Rust Integration

#### Public API
- [x] `Runtime::new()` - Create runtime instance
- [x] `Runtime::with_config(config)` - Create with configuration
- [x] `Runtime::eval(source)` - Evaluate source, returns `RuntimeResult`
- [x] `Runtime::provide_module(specifier, source)` - Provide external module
- [x] `Runtime::continue_eval()` - Continue after providing modules
- [x] `Runtime::fulfill_orders(responses)` - Fulfill pending orders
- [x] `Runtime::get_exports()` - Get all exported values

#### Configuration
- [x] `RuntimeConfig::internal_modules` - Register internal modules
- [x] `RuntimeConfig::timeout_ms` - Execution timeout

#### Serde Bridge
- [x] `JsValue` → `serde_json::Value` (via `js_value_to_json`)
- [x] `serde_json::Value` → `JsValue` (via `json_to_js_value_with_interp`)
- [ ] `JsValue` → Rust struct (via Deserialize)
- [ ] Rust struct → `JsValue` (via Serialize)

---

## Guard-Based Garbage Collection

The interpreter uses a guard-based GC model for memory management.

### Core Types

```rust
/// Memory arena managing all allocations
pub struct Heap<T> {
    inner: Rc<RefCell<Space<T>>>,
}

/// Root anchor that keeps objects alive
pub struct Guard<T: Default + Reset> {
    id: usize,
    space: Weak<RefCell<Space<T>>>,
}

/// Smart pointer to GC-managed object
pub struct Gc<T> {
    id: usize,
    index: usize,
    ptr: NonNull<GcBox<T>>,
}
```

### The Guarded Pattern

Expression evaluation returns `Guarded` to pair values with guards:

```rust
pub struct Guarded {
    pub value: JsValue,
    pub guard: Option<Guard<JsObject>>,
}
```

This keeps newly created objects alive until ownership is transferred:

```rust
// Object stays alive via guard until env.own() establishes ownership
let Guarded { value, guard: _g } = self.evaluate_expression(expr)?;
self.env_define(name, value, mutable);  // ownership transferred
// _g dropped AFTER ownership established - safe!
```

### Ownership Rules

| Situation | Action |
|-----------|--------|
| Define variable with object | `env.own(&obj, &heap)` |
| Set property to object | `parent.own(&obj, &heap)` |
| Array element is object | `array.own(&obj, &heap)` |
| Function captures closure | `func.own(&closure_env, &heap)` |
| Prototype chain | `child.own(&prototype, &heap)` |

### GC Timing Model

The GC uses a **pre-allocation trigger** model:

```
┌─────────────────────────────────────────────────────────────────┐
│                    ALLOCATION SEQUENCE                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. alloc_internal() called                                     │
│     │                                                           │
│  2. net_allocs += 1                                             │
│     │                                                           │
│  3. if net_allocs >= gc_threshold:                              │
│     │   └── collect() ◀── GC runs HERE, before allocation      │
│     │                                                           │
│  4. Allocate new object (from pool or fresh)                    │
│     │                                                           │
│  5. Return Gc<T> to caller                                      │
│     │                                                           │
│  6. Caller adds to guard roots                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Key insight:** GC runs BEFORE the new object exists, so input values to a function may be collected if not guarded. This is why you must guard input values before allocating.

### GC Root Model

Guards track roots explicitly. Objects are alive if reachable from any guard:

```
┌─────────────────────────────────────────────────────────────────┐
│                      ROOT TRACKING                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Guard A              Guard B              Guard C              │
│  ┌─────────┐          ┌─────────┐          ┌─────────┐          │
│  │ roots:  │          │ roots:  │          │ roots:  │          │
│  │  - obj1 │          │  - obj3 │          │  (empty)│          │
│  │  - obj2 │          └─────────┘          └─────────┘          │
│  └─────────┘                                                    │
│       │                    │                                    │
│       ▼                   ▼                                    │
│   ┌──────┐  refs      ┌──────┐                                  │
│   │ obj1 │───────┐    │ obj3 │   ◀── alive via Guard B          │
│   └──────┘       │    └──────┘                                  │
│       │          │    ┌──────┐                                  │
│       ▼          └──▶│ obj4 │◀── alive via obj1               │
│   ┌──────┐            └──────┘                                  │
│   │ obj2 │           ┌──────┐                                   │
│   └──────┘           │ obj5 │   ◀── UNREACHABLE, will be swept  │
│                      └──────┘                                   │
│                                                                 │
│                                                                 │
│                                                                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Generation-Based Slot Reuse

When objects are collected, their slots are reused. Generation counters prevent stale `Gc<T>` pointers from affecting ref_counts:

```rust
struct GcBox<T> {
    generation: Cell<u32>,  // Incremented on reuse
    // ...
}

struct Gc<T> {
    generation: u32,  // Captured at creation time
    // ...
}

// In Gc::clone() and Gc::drop():
if gc_box.generation.get() != self.generation {
    return;  // Stale pointer, ignore
}
```

### Common GC Bug Patterns (Lessons Learned)

| Bug Pattern | Symptom | Fix |
|-------------|---------|-----|
| Guard dropped before use | "X is not a function" | Use destructuring: `let Guarded { value, guard: _g } = ...` |
| Allocate before guarding input | Random values corrupted | Guard inputs BEFORE calling `create_*_with_guard()` |
| Loop drops guards early | Array has wrong elements | Collect guards in `Vec`, drop after container created |
| Promise result not guarded | Async returns undefined | Guard result immediately after extracting from promise |
| Returning JsValue instead of Guarded | Intermittent failures | Functions creating objects should return `Guarded` |

### Stress Testing

The `GC_THRESHOLD` environment variable controls collection frequency:

```bash
GC_THRESHOLD=1 cargo test    # GC on every allocation (stress test)
GC_THRESHOLD=0 cargo test    # Disable automatic GC
cargo test                    # Default: GC every 100 allocations
```

Setting `GC_THRESHOLD=1` is essential for catching GC bugs that only manifest under memory pressure.

---

## Zero-Panic Policy

The codebase enforces a strict **zero-panic policy** via Clippy lints.

### Enforced Lints

```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
indexing_slicing = "deny"
panic = "deny"
unreachable = "deny"
todo = "deny"
unimplemented = "deny"
string_slice = "deny"
```

### Safe Alternatives

| Panic Pattern | Safe Alternative |
|---------------|------------------|
| `vec[i]` | `vec.get(i).ok_or_else(\|\| JsError::...)` |
| `opt.unwrap()` | `opt.ok_or_else(\|\| JsError::...)` |
| `str[start..end]` | `str.get(start..end).unwrap_or("")` |
| `args[0]` | `args.first().cloned().unwrap_or(JsValue::Undefined)` |
| `unreachable!()` | `return Err(JsError::internal_error(...))` |

---

## Module Structure

```
src/
├── lib.rs              # Public API: Runtime, RuntimeResult, RuntimeConfig
├── error.rs            # Error types: JsError, SourceLocation
├── lexer.rs            # Tokenizer: Lexer, Token, TokenKind, Span
├── ast.rs              # AST nodes: Statement, Expression, Pattern
├── parser.rs           # Parser: recursive descent + Pratt parsing
├── value.rs            # Runtime values: JsValue, JsObject, Guarded
├── gc.rs               # Guard-based garbage collector
├── string_dict.rs      # String interning
└── interpreter/
    ├── mod.rs          # Core interpreter with Guarded pattern
    └── builtins/       # Built-in implementations
        ├── array.rs
        ├── string.rs
        ├── number.rs
        ├── object.rs
        ├── function.rs
        ├── math.rs
        ├── json.rs
        ├── date.rs
        ├── map.rs
        ├── set.rs
        ├── symbol.rs
        ├── regexp.rs
        ├── error.rs
        ├── console.rs
        └── global.rs
```

---

## Implementation Milestones

### Milestone 1-7: Core Implementation ✅

- [x] Lexer, Parser, AST
- [x] Core interpreter with Guarded pattern
- [x] Guard-based GC
- [x] All built-in objects
- [x] Classes with inheritance
- [x] Zero-panic policy

### Milestone 8: Order System ✅

- [x] `Order`, `OrderId`, `OrderResponse` types
- [x] `RuntimeResult` with `NeedImports`, `Suspended`
- [x] Internal module system (native + source)
- [x] `eval:internal` with `__order__`, `__cancelOrder__`
- [x] Import collection and resolution

### Milestone 9: Async/Await ✅

- [x] Async functions via order system
- [x] `await` suspends execution
- [x] Generator functions (function*, yield, yield*)

### Milestone 10: Serde Integration

- [ ] `JsValue` ↔ `serde_json::Value`
- [ ] Direct struct serialization

---

## Testing

### Current Status: 909 tests passing

```bash
cargo test                     # Run all tests
cargo test --test interpreter  # Run interpreter tests
cargo test symbol::            # Run symbol tests
cargo test -- --nocapture      # Show output
```

### Known Failures (0)

All tests passing.

---

## Target Use Case

```rust
// Configure runtime with internal modules
let config = RuntimeConfig {
    internal_modules: vec![
        create_eval_internal_module(),
        create_eval_fs_module(),
    ],
    timeout_ms: 5000,
};

let mut runtime = Runtime::with_config(config);

// Evaluate with import/order handling
let mut result = runtime.eval(source)?;
loop {
    match result {
        RuntimeResult::Complete(value) => {
            let manifest: K8sDeployment = serde_json::from_value(value.to_json())?;
            return Ok(manifest);
        }
        RuntimeResult::NeedImports(specs) => {
            for spec in specs {
                runtime.provide_module(&spec, &load_module(&spec)?)?;
            }
            result = runtime.continue_eval()?;
        }
        RuntimeResult::Suspended { pending, .. } => {
            let responses = fulfill_orders(pending);
            result = runtime.fulfill_orders(responses)?;
        }
    }
}
```
