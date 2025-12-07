# TypeScript Interpreter Design Document

## Overview

**Project:** `typescript-eval`
**Purpose:** Execute TypeScript for config/manifest generation from Rust
**Status:** Milestone 1 Complete (Basic Expressions)

### Requirements

- Full TypeScript syntax support (types stripped, not checked at runtime)
- ES Modules support
- Synchronous execution only (no async/await)
- Full ES2022+ standard built-ins
- Serde integration for Rust API
- Production-quality implementation

---

## Feature Checklist

### JavaScript Language Features

#### Variables & Declarations
- [x] `let` declarations
- [x] `const` declarations
- [x] `var` declarations (function-scoped)
- [x] Variable hoisting (var)
- [ ] Temporal Dead Zone (let/const)
- [x] Multiple declarators (`let a = 1, b = 2`)

#### Primitive Types & Literals
- [x] `undefined`
- [x] `null`
- [x] Boolean (`true`, `false`)
- [x] Number (integers, floats, `NaN`, `Infinity`)
- [x] String (single/double quotes)
- [x] Template literals (backticks)
- [x] Template literal interpolation (`${expr}`)
- [ ] Tagged template literals
- [ ] BigInt literals (`123n`)
- [ ] Symbol

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
- [ ] Generator functions (`function*`)
- [ ] `yield` / `yield*`
- [x] `this` binding
- [ ] `arguments` object
- [ ] `Function.prototype.call`
- [ ] `Function.prototype.apply`
- [ ] `Function.prototype.bind`

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
- [ ] `__proto__` property
- [x] Prototype chain lookup

#### Arrays
- [x] Array literals
- [x] Array element access
- [x] Spread in arrays (`[...arr]`)
- [ ] Array holes (`[1, , 3]`)
- [x] `length` property

#### Destructuring
- [x] Object destructuring in declarations
- [x] Array destructuring in declarations
- [x] Nested destructuring
- [x] Default values in destructuring
- [x] Rest in destructuring (`{ a, ...rest }`)
- [ ] Destructuring in function parameters
- [ ] Destructuring in assignment expressions

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
- [ ] Private fields (`#field`)
- [ ] Private methods (`#method()`)
- [ ] Static initialization blocks

#### Error Handling
- [x] `try` / `catch` / `finally`
- [x] `throw` statement
- [ ] Error stack traces
- [ ] Custom error types

#### Modules (ES Modules)
- [x] `import` declarations (parsing)
- [x] `export` declarations (parsing)
- [x] Named imports/exports
- [x] Default imports/exports
- [x] Namespace imports (`import * as`)
- [x] Re-exports (`export { x } from`)
- [ ] Module resolution (relative paths)
- [ ] Module resolution (node_modules)
- [ ] Module caching
- [ ] Circular dependency handling
- [ ] Dynamic `import()` (returns value synchronously)

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
- [ ] `namespace` / `module` declarations
- [ ] Declaration merging
- [x] Accessibility modifiers (`public`, `private`, `protected`) - parsed, ignored
- [x] `abstract` classes - parsed, ignored
- [x] `implements` clause - parsed, ignored

### Built-in Objects & Methods

#### Global Functions
- [x] `parseInt(string, radix?)`
- [x] `parseFloat(string)`
- [x] `isNaN(value)`
- [x] `isFinite(value)`
- [ ] `encodeURI(uri)`
- [ ] `decodeURI(uri)`
- [ ] `encodeURIComponent(str)`
- [ ] `decodeURIComponent(str)`

#### Object
- [x] `Object.keys(obj)`
- [x] `Object.values(obj)`
- [x] `Object.entries(obj)`
- [x] `Object.assign(target, ...sources)`
- [ ] `Object.freeze(obj)`
- [ ] `Object.seal(obj)`
- [ ] `Object.isFrozen(obj)`
- [ ] `Object.isSealed(obj)`
- [ ] `Object.getOwnPropertyNames(obj)`
- [ ] `Object.getOwnPropertyDescriptor(obj, prop)`
- [ ] `Object.defineProperty(obj, prop, descriptor)`
- [ ] `Object.defineProperties(obj, props)`
- [ ] `Object.getPrototypeOf(obj)`
- [ ] `Object.setPrototypeOf(obj, proto)`
- [ ] `Object.create(proto, props?)`
- [ ] `Object.fromEntries(iterable)`
- [ ] `Object.hasOwn(obj, prop)`
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
- [ ] `Array.prototype.findLast(predicate)`
- [ ] `Array.prototype.findLastIndex(predicate)`
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
- [ ] `Array.prototype.entries()`
- [ ] `Array.prototype.keys()`
- [ ] `Array.prototype.values()`
- [x] `Array.prototype.at(index)`
- [ ] `Array.prototype.toReversed()`
- [ ] `Array.prototype.toSorted(compareFn?)`
- [ ] `Array.prototype.toSpliced(start, deleteCount?, ...items)`
- [ ] `Array.prototype.with(index, value)`

#### String
- [x] `String.fromCharCode(...codes)`
- [ ] `String.fromCodePoint(...codePoints)`
- [x] `String.prototype.charAt(index)`
- [x] `String.prototype.charCodeAt(index)`
- [ ] `String.prototype.codePointAt(index)`
- [x] `String.prototype.concat(...strings)`
- [x] `String.prototype.includes(search, position?)`
- [x] `String.prototype.startsWith(search, position?)`
- [x] `String.prototype.endsWith(search, length?)`
- [x] `String.prototype.indexOf(search, position?)`
- [x] `String.prototype.lastIndexOf(search, position?)`
- [x] `String.prototype.slice(start?, end?)`
- [x] `String.prototype.substring(start, end?)`
- [ ] `String.prototype.substr(start, length?)` (deprecated)
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
- [ ] `String.prototype.match(regexp)`
- [ ] `String.prototype.matchAll(regexp)`
- [ ] `String.prototype.search(regexp)`
- [x] `String.prototype.at(index)`
- [ ] `String.prototype.normalize(form?)`
- [ ] `String.prototype.localeCompare(other)`

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

#### Map
- [ ] `new Map(iterable?)`
- [ ] `Map.prototype.get(key)`
- [ ] `Map.prototype.set(key, value)`
- [ ] `Map.prototype.has(key)`
- [ ] `Map.prototype.delete(key)`
- [ ] `Map.prototype.clear()`
- [ ] `Map.prototype.size`
- [ ] `Map.prototype.keys()`
- [ ] `Map.prototype.values()`
- [ ] `Map.prototype.entries()`
- [ ] `Map.prototype.forEach(callback)`

#### Set
- [ ] `new Set(iterable?)`
- [ ] `Set.prototype.add(value)`
- [ ] `Set.prototype.has(value)`
- [ ] `Set.prototype.delete(value)`
- [ ] `Set.prototype.clear()`
- [ ] `Set.prototype.size`
- [ ] `Set.prototype.keys()`
- [ ] `Set.prototype.values()`
- [ ] `Set.prototype.entries()`
- [ ] `Set.prototype.forEach(callback)`

#### Date
- [ ] `new Date()`
- [ ] `new Date(timestamp)`
- [ ] `new Date(dateString)`
- [ ] `new Date(year, month, day?, ...)`
- [ ] `Date.now()`
- [ ] `Date.parse(dateString)`
- [ ] `Date.UTC(year, month, day?, ...)`
- [ ] `Date.prototype.getTime()`
- [ ] `Date.prototype.getFullYear()`, `getMonth()`, `getDate()`, etc.
- [ ] `Date.prototype.setFullYear()`, `setMonth()`, `setDate()`, etc.
- [ ] `Date.prototype.toISOString()`
- [ ] `Date.prototype.toJSON()`
- [ ] `Date.prototype.toString()`
- [ ] `Date.prototype.toDateString()`
- [ ] `Date.prototype.toTimeString()`

#### RegExp
- [ ] RegExp literals (`/pattern/flags`)
- [ ] `new RegExp(pattern, flags?)`
- [ ] `RegExp.prototype.test(string)`
- [ ] `RegExp.prototype.exec(string)`
- [ ] `RegExp.prototype.source`
- [ ] `RegExp.prototype.flags`
- [ ] `RegExp.prototype.global`
- [ ] `RegExp.prototype.ignoreCase`
- [ ] `RegExp.prototype.multiline`
- [ ] `RegExp.prototype.dotAll`
- [ ] `RegExp.prototype.unicode`
- [ ] `RegExp.prototype.sticky`

#### Error Types
- [ ] `Error`
- [ ] `TypeError`
- [ ] `ReferenceError`
- [ ] `SyntaxError`
- [ ] `RangeError`
- [ ] `URIError`
- [ ] `EvalError`
- [ ] `Error.prototype.name`
- [ ] `Error.prototype.message`
- [ ] `Error.prototype.stack`

#### Console
- [x] `console.log(...args)`
- [x] `console.error(...args)`
- [x] `console.warn(...args)`
- [x] `console.info(...args)`
- [x] `console.debug(...args)`
- [ ] `console.table(data)`
- [ ] `console.dir(obj)`
- [ ] `console.time(label)`
- [ ] `console.timeEnd(label)`

### Rust Integration

#### Serde Bridge
- [ ] `JsValue` → `serde_json::Value`
- [ ] `serde_json::Value` → `JsValue`
- [ ] `JsValue` → Rust struct (via Deserialize)
- [ ] Rust struct → `JsValue` (via Serialize)
- [ ] Handle `undefined` vs `null` in serialization
- [ ] Preserve object key order

#### Public API
- [ ] `Runtime::new()` - Create runtime instance
- [ ] `Runtime::eval(source)` - Evaluate source string
- [ ] `Runtime::load_module(path)` - Load and cache module
- [ ] `Runtime::call_function<T, R>(name, args)` - Call exported function
- [ ] `Runtime::get_export<T>(name)` - Get exported value
- [ ] Error type conversion to Rust errors

#### Configuration
- [ ] Custom module resolver
- [ ] Global value injection
- [ ] Execution timeout/limits
- [ ] Memory limits

### Target Use Case

```rust
let runtime = Runtime::new();
runtime.load_module("config.ts")?;

let manifest: K8sDeployment = runtime.call_function("generateDeployment", &DeploymentInput {
    name: "my-app",
    replicas: 3,
    image: "nginx:latest",
})?;
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           PUBLIC API (lib.rs)                           │
│  Runtime::new() → runtime.load_module(path) → module.call(name, args)   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
            │    Serde     │ │    Module    │ │   Error      │
            │    Bridge    │ │    System    │ │   Handling   │
            └──────────────┘ └──────────────┘ └──────────────┘
                    │               │               │
                    └───────────────┼───────────────┘
                                    ▼
            ┌─────────────────────────────────────────────────┐
            │                 INTERPRETER                      │
            │  ┌─────────────┐ ┌─────────────┐ ┌────────────┐ │
            │  │ Evaluator   │ │   Scope     │ │  Control   │ │
            │  │ (expr/stmt) │ │   Chain     │ │   Flow     │ │
            │  └─────────────┘ └─────────────┘ └────────────┘ │
            └─────────────────────────────────────────────────┘
                                    │
            ┌─────────────────────────────────────────────────┐
            │                   RUNTIME                        │
            │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐ │
            │  │ JsValue │ │ Object  │ │Prototype│ │Built-  │ │
            │  │  enum   │ │ Model   │ │ Chain   │ │  ins   │ │
            │  └─────────┘ └─────────┘ └─────────┘ └────────┘ │
            └─────────────────────────────────────────────────┘
                                    │
            ┌─────────────────────────────────────────────────┐
            │                    PARSER                        │
            │  ┌─────────────┐         ┌────────────────────┐ │
            │  │ Recursive   │────────▶│       AST          │ │
            │  │  Descent    │         │  (Typed Nodes)     │ │
            │  └─────────────┘         └────────────────────┘ │
            └─────────────────────────────────────────────────┘
                                    │
            ┌─────────────────────────────────────────────────┐
            │                    LEXER                         │
            │  Source Code ──▶ Token Stream ──▶ Span Info     │
            └─────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/
├── lib.rs          # Public API: Runtime, eval()
├── error.rs        # Error types: JsError, SourceLocation, StackFrame
├── lexer.rs        # Tokenizer: Lexer, Token, TokenKind, Span
├── ast.rs          # AST nodes: Statement, Expression, Pattern, TypeAnnotation
├── parser.rs       # Parser: recursive descent + Pratt parsing
├── value.rs        # Runtime values: JsValue, JsObject, Environment
└── interpreter.rs  # Evaluator: statement/expression execution
```

---

## Component Details

### 1. Lexer (`lexer.rs`)

The lexer converts source text into a stream of tokens.

#### Token Types

```rust
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),
    TemplateHead(String),
    TemplateMiddle(String),
    TemplateTail(String),
    True, False, Null,

    // Identifiers & Keywords
    Identifier(String),

    // JS Keywords
    Let, Const, Var, Function, Return, If, Else,
    For, While, Do, Break, Continue, Switch, Case, Default,
    Try, Catch, Finally, Throw, New, This, Super,
    Class, Extends, Static, Import, Export, From, As,
    Typeof, Instanceof, In, Of, Void, Delete, Debugger,

    // TS Keywords (parsed, ignored at runtime)
    Type, Interface, Enum, Declare, Abstract, Readonly,
    Public, Private, Protected, Implements,
    Any, Unknown, Never, Keyof, Infer,

    // Operators
    Plus, Minus, Star, Slash, Percent, StarStar,
    Eq, EqEq, EqEqEq, BangEq, BangEqEq,
    Lt, LtEq, Gt, GtEq, LtLt, GtGt, GtGtGt,
    Amp, AmpAmp, Pipe, PipePipe, Caret, Tilde, Bang,
    Question, QuestionQuestion, QuestionDot,
    // ... assignment operators, punctuation

    Eof,
}
```

#### Key Features

- Handles all number formats (decimal, hex, octal, binary, floats, exponents)
- String literals with escape sequences
- Template literal support with interpolation
- Comment handling (single-line `//`, multi-line `/* */`)
- Tracks line/column for error reporting
- Newline tracking for automatic semicolon insertion (ASI)

### 2. AST (`ast.rs`)

The Abstract Syntax Tree represents parsed TypeScript structure.

#### Statement Types

```rust
pub enum Statement {
    // Declarations
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    ClassDeclaration(ClassDeclaration),

    // TypeScript (no-op at runtime)
    TypeAlias(TypeAliasDeclaration),
    InterfaceDeclaration(InterfaceDeclaration),
    EnumDeclaration(EnumDeclaration),

    // Control Flow
    Block(BlockStatement),
    If(IfStatement),
    Switch(SwitchStatement),
    For(ForStatement),
    ForIn(ForInStatement),
    ForOf(ForOfStatement),
    While(WhileStatement),
    DoWhile(DoWhileStatement),
    Try(TryStatement),

    // Jump
    Return(ReturnStatement),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Throw(ThrowStatement),

    // Module
    Import(ImportDeclaration),
    Export(ExportDeclaration),

    // Other
    Expression(ExpressionStatement),
    Empty,
    Debugger,
}
```

#### Expression Types

```rust
pub enum Expression {
    // Literals
    Literal(Literal),
    Array(ArrayExpression),
    Object(ObjectExpression),
    Function(FunctionExpression),
    ArrowFunction(ArrowFunctionExpression),
    Class(ClassExpression),
    Template(TemplateLiteral),

    // Identifiers
    Identifier(Identifier),
    This,
    Super,

    // Operations
    Unary(UnaryExpression),
    Binary(BinaryExpression),
    Logical(LogicalExpression),
    Conditional(ConditionalExpression),
    Assignment(AssignmentExpression),
    Update(UpdateExpression),
    Sequence(SequenceExpression),

    // Access
    Member(MemberExpression),
    Call(CallExpression),
    New(NewExpression),

    // TypeScript (runtime behavior)
    TypeAssertion(TypeAssertionExpression),  // x as T → evaluates to x
    NonNull(NonNullExpression),              // x! → evaluates to x
}
```

#### Pattern Types (Destructuring)

```rust
pub enum Pattern {
    Identifier(Identifier),
    Object(ObjectPattern),
    Array(ArrayPattern),
    Rest(RestElement),
    Assignment(AssignmentPattern),  // { x = default }
}
```

### 3. Parser (`parser.rs`)

Recursive descent parser with Pratt parsing for expressions.

#### Operator Precedence

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 4 | `??` `\|\|` | Left |
| 5 | `&&` | Left |
| 6-8 | `\|` `^` `&` (bitwise) | Left |
| 9 | `==` `!=` `===` `!==` | Left |
| 10 | `<` `<=` `>` `>=` `in` `instanceof` | Left |
| 11 | `<<` `>>` `>>>` | Left |
| 12 | `+` `-` | Left |
| 13 | `*` `/` `%` | Left |
| 14 | `**` | Right |

#### Key Implementation Details

```rust
/// Pratt parser for binary expressions
fn parse_binary_expression(&mut self, min_prec: u8) -> Result<Expression, JsError> {
    let mut left = self.parse_unary_expression()?;

    loop {
        let (op, prec, is_logical) = match self.current_binary_op() {
            Some(info) => info,
            None => break,
        };

        if prec < min_prec {
            break;
        }

        self.advance();
        let next_prec = if op == BinaryOp::Exp { prec } else { prec + 1 };
        let right = self.parse_binary_expression(next_prec)?;

        left = Expression::Binary(BinaryExpression {
            operator: op,
            left: Box::new(left),
            right: Box::new(right),
            span,
        });
    }

    Ok(left)
}
```

#### TypeScript Handling

- Type annotations are parsed but stored separately
- `type`, `interface` declarations become no-ops
- `enum` declarations compile to object literals
- Type assertions (`x as T`, `<T>x`) evaluate to just `x`
- Non-null assertions (`x!`) evaluate to just `x`

### 4. Runtime Values (`value.rs`)

#### JsValue Enum

```rust
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(JsString),
    Object(JsObjectRef),
}

pub type JsObjectRef = Rc<RefCell<JsObject>>;
```

#### Object Model

```rust
pub struct JsObject {
    pub prototype: Option<JsObjectRef>,
    pub properties: IndexMap<PropertyKey, Property>,
    pub exotic: ExoticObject,
}

pub enum ExoticObject {
    Ordinary,
    Array { length: u32 },
    Function(JsFunction),
}

pub enum JsFunction {
    Interpreted(InterpretedFunction),
    Native(NativeFunction),
}
```

#### Property Keys

```rust
pub enum PropertyKey {
    String(JsString),
    Index(u32),  // Array index optimization
}
```

#### Environment (Scope Chain)

```rust
pub struct Environment {
    bindings: HashMap<String, Binding>,
    outer: Option<Box<Environment>>,
}

pub struct Binding {
    value: JsValue,
    mutable: bool,      // false for const
    initialized: bool,  // for TDZ (temporal dead zone)
}
```

### 5. Interpreter (`interpreter.rs`)

#### Completion Records

```rust
pub enum Completion {
    Normal(JsValue),
    Return(JsValue),
    Break(Option<String>),
    Continue(Option<String>),
}
```

#### Statement Execution

```rust
impl Interpreter {
    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<Completion, JsError> {
        match stmt {
            Statement::VariableDeclaration(decl) => { /* ... */ }
            Statement::FunctionDeclaration(decl) => { /* ... */ }
            Statement::If(if_stmt) => { /* ... */ }
            Statement::For(for_stmt) => { /* ... */ }
            Statement::Return(ret) => { /* ... */ }
            // TypeScript no-ops
            Statement::TypeAlias(_) | Statement::InterfaceDeclaration(_) => {
                Ok(Completion::Normal(JsValue::Undefined))
            }
            // ...
        }
    }
}
```

#### Expression Evaluation

```rust
impl Interpreter {
    pub fn evaluate(&mut self, expr: &Expression) -> Result<JsValue, JsError> {
        match expr {
            Expression::Literal(lit) => self.evaluate_literal(lit),
            Expression::Identifier(id) => self.env.get(&id.name),
            Expression::Binary(bin) => self.evaluate_binary(bin),
            Expression::Call(call) => self.evaluate_call(call),
            // TypeScript runtime behavior
            Expression::TypeAssertion(ta) => self.evaluate(&ta.expression),
            Expression::NonNull(nn) => self.evaluate(&nn.expression),
            // ...
        }
    }
}
```

#### Built-in Globals

Currently implemented:
- `console.log` - Output to stdout
- `JSON.parse` - Parse JSON string to value
- `JSON.stringify` - Convert value to JSON string
- `Object.keys` - Get object's own enumerable property names
- `Object.values` - Get object's own enumerable property values
- `Object.entries` - Get object's own enumerable [key, value] pairs
- `Object.assign` - Copy properties from sources to target
- `Array.isArray` - Check if value is an array

---

## Error Handling

```rust
pub enum JsError {
    SyntaxError { message: String, location: SourceLocation },
    TypeError { message: String },
    ReferenceError { name: String },
    RangeError { message: String },
    RuntimeError { kind: String, message: String, stack: Vec<StackFrame> },
}

pub struct SourceLocation {
    pub file: Option<PathBuf>,
    pub line: u32,
    pub column: u32,
    pub length: u32,
}

pub struct StackFrame {
    pub function_name: Option<String>,
    pub file: Option<PathBuf>,
    pub line: u32,
    pub column: u32,
}
```

---

## Implementation Milestones

### Milestone 1: Basic Expressions ✅ Complete

- [x] Project setup, Cargo.toml
- [x] Token definitions
- [x] Lexer for literals, operators, punctuation
- [x] AST for expressions
- [x] Parser for expressions (Pratt parsing)
- [x] JsValue enum (primitives + objects)
- [x] Expression evaluator (arithmetic, comparison, logical)
- [x] Basic tests (25 passing)

### Milestone 2: Variables & Functions (Planned)

- [ ] Variable declarations (let, const, var)
- [ ] Scope/environment chain
- [ ] Function declarations & expressions
- [ ] Arrow functions with closure capture
- [ ] Function calls with `this` binding
- [ ] Default parameters
- [ ] Rest parameters

### Milestone 3: Objects & Arrays (Planned)

- [ ] Object literals with methods
- [ ] Array literals
- [ ] Property access (dot and bracket)
- [ ] Destructuring assignment
- [ ] Spread operator
- [ ] Object/Array prototype methods

### Milestone 4: Control Flow & Classes (Planned)

- [ ] if/else, switch/case
- [ ] for, while, do-while
- [ ] for-in, for-of
- [ ] try/catch/finally
- [ ] Class declarations
- [ ] Inheritance (extends)
- [ ] Static methods and properties

### Milestone 5: Built-ins (Planned)

- [ ] Complete Object methods
- [ ] Complete Array methods (map, filter, reduce, etc.)
- [ ] String methods
- [ ] Number, Math
- [ ] Map, Set
- [ ] Date (basic)
- [ ] RegExp (basic)

### Milestone 6: Modules (Planned)

- [ ] ES module parsing (import/export)
- [ ] Module resolution
- [ ] Module caching
- [ ] Circular dependency handling
- [ ] TypeScript enum compilation

### Milestone 7: Serde Integration (Planned)

- [ ] `Serialize` trait for JsValue → Rust
- [ ] `Deserialize` trait for Rust → JsValue
- [ ] Public Runtime API
- [ ] Integration tests

---

## Testing Strategy

### Unit Tests

Each module has inline tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic() {
        assert_eq!(eval("1 + 2"), JsValue::Number(3.0));
        assert_eq!(eval("2 ** 3"), JsValue::Number(8.0));
    }
}
```

### Test Categories

- **Lexer tests**: Token stream verification
- **Parser tests**: AST structure verification
- **Value tests**: Type coercion, equality
- **Interpreter tests**: Expression evaluation, control flow

### Running Tests

```bash
cargo test              # Run all tests
cargo test lexer        # Run lexer tests only
cargo test -- --nocapture  # Show test output
```

---

## Dependencies

```toml
[dependencies]
thiserror = "1.0"       # Error derive macros
serde = "1.0"           # Serialization framework
serde_json = "1.0"      # JSON support
indexmap = "2.0"        # Ordered map for properties
unicode-xid = "0.2"     # Unicode identifier validation
regex = "1.10"          # RegExp built-in
chrono = "0.4"          # Date built-in

[dev-dependencies]
pretty_assertions = "1.4"  # Better test diffs
```

---

## Future Enhancements

### Performance Optimizations

- Bytecode compilation for hot paths
- Object shape optimization (hidden classes)
- String interning
- Property access caching

### Additional Features

- Source maps for debugging
- REPL mode
- Optional type checking mode
- Decorator support

### Compatibility

- Test262 conformance testing
- Node.js built-in stubs (for config files that use them)
