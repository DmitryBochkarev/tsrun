/**
 * End-to-end tests for tsrun WASM module
 *
 * Tests the WASM module with playground examples.
 * Detailed language feature tests are in the Rust test suite.
 *
 * Run with: node test.js
 */

import {
    TsRunner,
    STEP_CONTINUE,
    STEP_COMPLETE,
    STEP_NEED_IMPORTS,
    STEP_SUSPENDED,
    STEP_DONE,
    STEP_ERROR
} from './pkg/tsrun.js';

// Step status constants for readability (these are functions that return values)
const StepStatus = {
    CONTINUE: STEP_CONTINUE(),
    COMPLETE: STEP_COMPLETE(),
    NEED_IMPORTS: STEP_NEED_IMPORTS(),
    SUSPENDED: STEP_SUSPENDED(),
    DONE: STEP_DONE(),
    ERROR: STEP_ERROR()
};

// Helper function to run code using the step-based API
// Returns { success, error, value_handle, console_output }
function runCode(runner, code) {
    const prepResult = runner.prepare(code, 'test.ts');
    if (prepResult.status === StepStatus.ERROR) {
        return {
            success: false,
            error: prepResult.error,
            value_handle: 0,
            console_output: prepResult.console_output
        };
    }

    let allConsole = [...(prepResult.console_output || [])];

    // Run until completion
    while (true) {
        const result = runner.step();
        allConsole = allConsole.concat(result.console_output || []);

        switch (result.status) {
            case StepStatus.CONTINUE:
                continue;
            case StepStatus.COMPLETE:
                return {
                    success: true,
                    error: null,
                    value_handle: result.value_handle,
                    console_output: allConsole
                };
            case StepStatus.DONE:
                return {
                    success: true,
                    error: null,
                    value_handle: 0,
                    console_output: allConsole
                };
            case StepStatus.ERROR:
                return {
                    success: false,
                    error: result.error,
                    value_handle: 0,
                    console_output: allConsole
                };
            case StepStatus.NEED_IMPORTS:
                return {
                    success: false,
                    error: 'Module imports not supported: ' + runner.get_import_requests().join(', '),
                    value_handle: 0,
                    console_output: allConsole
                };
            case StepStatus.SUSPENDED:
                // For tests, just return error - we don't handle async in tests
                return {
                    success: false,
                    error: 'Async operations not supported in tests',
                    value_handle: 0,
                    console_output: allConsole
                };
        }
    }
}

// Test utilities
let passed = 0;
let failed = 0;

function test(name, fn) {
    try {
        fn();
        console.log(`✓ ${name}`);
        passed++;
    } catch (e) {
        console.log(`✗ ${name}`);
        console.log(`  Error: ${e.message}`);
        failed++;
    }
}

function assertTrue(value, message = '') {
    if (!value) {
        throw new Error(message || 'Expected true');
    }
}

function assertEqual(actual, expected, message = '') {
    if (actual !== expected) {
        throw new Error(`${message}\n  Expected: ${JSON.stringify(expected)}\n  Actual: ${JSON.stringify(actual)}`);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test Suite
// ═══════════════════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════════');
console.log('tsrun WASM End-to-End Tests');
console.log('═══════════════════════════════════════════════════════════════\n');

// ─────────────────────────────────────────────────────────────────────────────
// WASM Module Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('── WASM Module ──\n');

test('TsRunner can be instantiated', () => {
    const runner = new TsRunner();
    assertTrue(runner !== null);
});

test('Basic execution works', () => {
    const runner = new TsRunner();
    const result = runCode(runner, '1 + 2 * 3');
    assertTrue(result.success, 'Expected success');
    assertTrue(result.value_handle !== 0, 'Should have value handle');
    assertEqual(runner.value_as_number(result.value_handle), 7);
});

test('Console output is captured', () => {
    const runner = new TsRunner();
    const result = runCode(runner, 'console.log("Hello")');
    assertTrue(result.success);
    assertTrue(result.console_output.length === 1);
    assertEqual(result.console_output[0].message, 'Hello');
});

test('Errors are reported', () => {
    const runner = new TsRunner();
    const result = runCode(runner, 'undefinedVariable');
    assertTrue(!result.success);
    assertTrue(result.error !== null);
});

test('Fresh runner for each execution', () => {
    // Each TsRunner instance starts fresh - no shared state
    const runner1 = new TsRunner();
    runCode(runner1, 'const x = 123');

    const runner2 = new TsRunner();
    const result = runCode(runner2, 'x');
    assertTrue(!result.success, 'New runner should not have previous state');
});

// ─────────────────────────────────────────────────────────────────────────────
// Value Handle System Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Value Handle System ──\n');

test('Value creation: primitives', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter

    const numHandle = runner.create_number(42);
    assertTrue(numHandle !== 0, 'Should return non-zero handle');
    assertEqual(runner.get_value_type(numHandle), 'number');
    assertEqual(runner.value_as_number(numHandle), 42);

    const strHandle = runner.create_string('hello');
    assertEqual(runner.get_value_type(strHandle), 'string');
    assertEqual(runner.value_as_string(strHandle), 'hello');

    const boolHandle = runner.create_bool(true);
    assertEqual(runner.get_value_type(boolHandle), 'boolean');
    assertEqual(runner.value_as_bool(boolHandle), true);

    const nullHandle = runner.create_null();
    assertEqual(runner.get_value_type(nullHandle), 'null');
    assertTrue(runner.value_is_null(nullHandle));

    const undefHandle = runner.create_undefined();
    assertEqual(runner.get_value_type(undefHandle), 'undefined');
    assertTrue(runner.value_is_undefined(undefHandle));
});

test('Value creation: object and array', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter (stays alive after completion)

    const objHandle = runner.create_object();
    assertTrue(objHandle !== 0, 'Should return non-zero handle');
    assertEqual(runner.get_value_type(objHandle), 'object');
    assertTrue(!runner.value_is_array(objHandle), 'Plain object is not an array');

    const arrHandle = runner.create_array();
    assertTrue(arrHandle !== 0, 'Should return non-zero handle');
    assertEqual(runner.get_value_type(arrHandle), 'object');
    assertTrue(runner.value_is_array(arrHandle), 'Array should be detected');
    assertEqual(runner.array_length(arrHandle), 0);
});

test('Object operations: get/set/delete property', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter (stays alive after completion)

    const objHandle = runner.create_object();
    const valHandle = runner.create_number(123);

    // Set property
    assertTrue(runner.set_property(objHandle, 'x', valHandle));
    assertTrue(runner.has_property(objHandle, 'x'));

    // Get property
    const gotHandle = runner.get_property(objHandle, 'x');
    assertEqual(runner.value_as_number(gotHandle), 123);

    // Get keys
    const keys = runner.get_keys(objHandle);
    assertTrue(keys.includes('x'), 'Keys should include "x"');

    // Delete property
    assertTrue(runner.delete_property(objHandle, 'x'));
    assertTrue(!runner.has_property(objHandle, 'x'));
});

test('Array operations: get/set/push', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter (stays alive after completion)

    const arrHandle = runner.create_array();
    assertEqual(runner.array_length(arrHandle), 0);

    // Push values
    const val1 = runner.create_number(10);
    const val2 = runner.create_number(20);
    assertTrue(runner.push(arrHandle, val1));
    assertTrue(runner.push(arrHandle, val2));
    assertEqual(runner.array_length(arrHandle), 2);

    // Get by index
    const elem0 = runner.get_index(arrHandle, 0);
    assertEqual(runner.value_as_number(elem0), 10);

    const elem1 = runner.get_index(arrHandle, 1);
    assertEqual(runner.value_as_number(elem1), 20);

    // Set by index
    const val3 = runner.create_number(30);
    assertTrue(runner.set_index(arrHandle, 0, val3));
    const updated = runner.get_index(arrHandle, 0);
    assertEqual(runner.value_as_number(updated), 30);
});

test('Handle lifecycle: release and duplicate', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter

    const handle1 = runner.create_number(42);
    assertEqual(runner.value_as_number(handle1), 42);

    // Duplicate creates new handle
    const handle2 = runner.duplicate_handle(handle1);
    assertTrue(handle2 !== handle1, 'Duplicate should have different ID');
    assertEqual(runner.value_as_number(handle2), 42);

    // Release original
    runner.release_handle(handle1);
    // Duplicate should still work
    assertEqual(runner.value_as_number(handle2), 42);

    // Invalid handle returns default values
    assertTrue(Number.isNaN(runner.value_as_number(0)), 'Handle 0 should return NaN');
    assertEqual(runner.get_value_type(0), 'undefined');
});

test('Export access: get_export and get_export_names', () => {
    const runner = new TsRunner();
    const result = runCode(runner, `
        export const VERSION = "1.0.0";
        export const count = 42;
        export function greet() { return "hello"; }
    `);
    assertTrue(result.success, `Export test failed: ${result.error}`);

    // Get export names
    const names = runner.get_export_names();
    assertTrue(names.includes('VERSION'), 'Should have VERSION export');
    assertTrue(names.includes('count'), 'Should have count export');
    assertTrue(names.includes('greet'), 'Should have greet export');

    // Get specific exports
    const versionHandle = runner.get_export('VERSION');
    assertTrue(versionHandle !== 0, 'VERSION export should exist');
    assertEqual(runner.get_value_type(versionHandle), 'string');
    assertEqual(runner.value_as_string(versionHandle), '1.0.0');

    const countHandle = runner.get_export('count');
    assertEqual(runner.value_as_number(countHandle), 42);

    const greetHandle = runner.get_export('greet');
    assertTrue(runner.value_is_function(greetHandle), 'greet should be a function');

    // Non-existent export returns 0
    const noExport = runner.get_export('nonexistent');
    assertEqual(noExport, 0, 'Non-existent export should return 0');
});

test('Handles are cleared on prepare()', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter

    const handle1 = runner.create_number(42);
    assertEqual(runner.value_as_number(handle1), 42);

    // Prepare new code - should clear handles
    runner.prepare('2', 'test.ts');

    // Old handle should now be invalid (return default values)
    assertTrue(Number.isNaN(runner.value_as_number(handle1)), 'Handle should be invalid after prepare()');
});

test('Value inspection edge cases', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter

    // NaN for non-number
    const strHandle = runner.create_string('not a number');
    assertTrue(Number.isNaN(runner.value_as_number(strHandle)));

    // undefined for non-string
    const numHandle = runner.create_number(42);
    assertEqual(runner.value_as_string(numHandle), undefined);

    // undefined for non-bool
    assertEqual(runner.value_as_bool(numHandle), undefined);

    // is_array returns false for non-arrays
    assertTrue(!runner.value_is_array(numHandle));
    assertTrue(!runner.value_is_function(numHandle));
});

test('Step result: value_handle for objects', () => {
    const runner = new TsRunner();
    const result = runCode(runner, '({ x: 10, y: 20 })');
    assertTrue(result.success, 'Expected success');
    assertTrue(result.value_handle !== 0, 'Should have value handle');
    assertEqual(runner.get_value_type(result.value_handle), 'object');

    const xHandle = runner.get_property(result.value_handle, 'x');
    assertEqual(runner.value_as_number(xHandle), 10);

    const yHandle = runner.get_property(result.value_handle, 'y');
    assertEqual(runner.value_as_number(yHandle), 20);
});

test('Error creation: create_error', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter

    const errHandle = runner.create_error('Something went wrong');
    assertTrue(errHandle !== 0, 'Should return non-zero handle');
    assertEqual(runner.get_value_type(errHandle), 'object');

    // Check error properties
    const nameHandle = runner.get_property(errHandle, 'name');
    assertEqual(runner.value_as_string(nameHandle), 'Error');

    const msgHandle = runner.get_property(errHandle, 'message');
    assertEqual(runner.value_as_string(msgHandle), 'Something went wrong');

    const stackHandle = runner.get_property(errHandle, 'stack');
    assertEqual(runner.value_as_string(stackHandle), 'Error: Something went wrong');
});

test('Promise creation returns handle', () => {
    const runner = new TsRunner();
    runCode(runner, '1'); // Initialize interpreter

    const promiseHandle = runner.create_promise();
    assertTrue(promiseHandle !== 0, 'Should return non-zero handle');
    assertEqual(runner.get_value_type(promiseHandle), 'object');
});

// ─────────────────────────────────────────────────────────────────────────────
// Playground Examples
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Playground Examples ──\n');

// All playground examples - must run without errors
const PLAYGROUND_EXAMPLES = {
    'hello-world': `// Welcome to the tsrun Playground!
console.log("Hello, World!");
console.log("Welcome to tsrun!");`,

    'variables': `let counter = 0;
const PI = 3.14159;
let message: string = "Hello";
let count: number = 42;
let active: boolean = true;
console.log("message:", message);
console.log("count:", count);
counter = counter + 1;
console.log("counter:", counter);`,

    'functions': `function greet(name: string): string {
    return "Hello, " + name + "!";
}
const add = (a: number, b: number): number => a + b;
function power(base: number, exponent: number = 2): number {
    let result = 1;
    for (let i = 0; i < exponent; i++) {
        result *= base;
    }
    return result;
}
function sum(...numbers: number[]): number {
    return numbers.reduce((acc, n) => acc + n, 0);
}
console.log(greet("TypeScript"));
console.log("2 + 3 =", add(2, 3));
console.log("5^3 =", power(5, 3));
console.log("sum(1,2,3,4,5) =", sum(1, 2, 3, 4, 5));`,

    'closures': `function makeCounter() {
    let count = 0;
    return function() {
        count++;
        return count;
    };
}
const counter1 = makeCounter();
const counter2 = makeCounter();
console.log("counter1:", counter1());
console.log("counter1:", counter1());
console.log("counter2:", counter2());
function multiplier(factor: number) {
    return (x: number) => x * factor;
}
const double = multiplier(2);
const triple = multiplier(3);
console.log("double(5):", double(5));
console.log("triple(5):", triple(5));`,

    'arrays': `const numbers = [1, 2, 3, 4, 5];
console.log("numbers:", numbers);
console.log("first:", numbers[0]);
console.log("length:", numbers.length);
const doubled = numbers.map(x => x * 2);
console.log("doubled:", doubled);
const evens = numbers.filter(x => x % 2 === 0);
console.log("evens:", evens);
const sum = numbers.reduce((acc, x) => acc + x, 0);
console.log("sum:", sum);
const [first, second, ...rest] = numbers;
console.log("first:", first, "second:", second, "rest:", rest);`,

    'objects': `const person = {
    name: "Alice",
    age: 30,
    city: "Wonderland"
};
console.log("person:", person);
console.log("name:", person.name);
console.log("keys:", Object.keys(person));
console.log("values:", Object.values(person));
const { name, age } = person;
console.log("destructured - name:", name, "age:", age);
const updated = { ...person, age: 31, job: "Developer" };
console.log("updated:", updated);`,

    'classes': `class Animal {
    name: string;
    constructor(name: string) {
        this.name = name;
    }
    speak(): string {
        return this.name + " makes a sound";
    }
}
class Dog extends Animal {
    breed: string;
    constructor(name: string, breed: string) {
        super(name);
        this.breed = breed;
    }
    speak(): string {
        return this.name + " barks!";
    }
}
class MathUtils {
    static PI = 3.14159;
    static circleArea(radius: number): number {
        return MathUtils.PI * radius * radius;
    }
}
const animal = new Animal("Generic Animal");
console.log(animal.speak());
const dog = new Dog("Rex", "German Shepherd");
console.log(dog.speak());
console.log("PI:", MathUtils.PI);
console.log("Circle area (r=5):", MathUtils.circleArea(5));`,

    'typescript-types': `let num: number = 42;
let str: string = "hello";
interface User {
    id: number;
    name: string;
    email?: string;
}
const user: User = {
    id: 1,
    name: "Alice",
    email: "alice@example.com"
};
console.log("User:", user);
type StringOrNumber = string | number;
const value: StringOrNumber = 42;
console.log("value:", value);
function identity<T>(x: T): T {
    return x;
}
console.log("identity(42):", identity(42));
enum Color { Red, Green, Blue }
console.log("Color.Green:", Color.Green);`,

    'control-flow': `function classify(n: number): string {
    if (n < 0) return "negative";
    else if (n === 0) return "zero";
    else return "positive";
}
console.log("classify(-5):", classify(-5));
console.log("classify(0):", classify(0));
console.log("classify(10):", classify(10));
let sum = 0;
for (let i = 1; i <= 5; i++) {
    sum += i;
}
console.log("sum 1-5:", sum);
const fruits = ["apple", "banana"];
for (const fruit of fruits) {
    console.log("fruit:", fruit);
}
const x = 10;
const status = x > 5 ? "big" : "small";
console.log("status:", status);`,

    'error-handling': `function divide(a: number, b: number): number {
    if (b === 0) throw new Error("Division by zero!");
    return a / b;
}
try {
    console.log("10 / 2 =", divide(10, 2));
    console.log("10 / 0 =", divide(10, 0));
} catch (e) {
    console.log("Caught error:", e.message);
} finally {
    console.log("Division complete");
}`,

    'async-patterns': `const promise = new Promise((resolve, reject) => {
    resolve(42);
});
promise.then(value => {
    console.log("Promise resolved:", value);
    return value * 2;
}).then(doubled => {
    console.log("Doubled:", doubled);
});
function* countdown(start: number) {
    while (start > 0) {
        yield start;
        start--;
    }
    return "Done!";
}
const gen = countdown(3);
console.log(gen.next());
console.log(gen.next());
console.log(gen.next());
console.log(gen.next());`,

    'map-set': `const map = new Map();
map.set("name", "Alice");
map.set(42, "the answer");
console.log("size:", map.size);
console.log("get('name'):", map.get("name"));
console.log("get(42):", map.get(42));
for (const [key, value] of map) {
    console.log(" ", key, "=>", value);
}
const set = new Set([1, 2, 3, 2, 1, 4]);
console.log("set values:", [...set]);
console.log("size:", set.size);
console.log("has(2):", set.has(2));`,

    'regex': `const text = "The quick brown fox jumps over the lazy dog";
console.log("/fox/.test():", /fox/.test(text));
console.log("/cat/.test():", /cat/.test(text));
const replaced = text.replace(/fox/, "cat");
console.log("replace fox->cat:", replaced);
const vowels = text.match(/[aeiou]/g);
console.log("all vowels:", vowels);
function isValidEmail(email: string): boolean {
    const pattern = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$/;
    return pattern.test(email);
}
console.log("test@example.com:", isValidEmail("test@example.com"));
console.log("invalid-email:", isValidEmail("invalid-email"));`,

    'math': `console.log("2 + 3 =", 2 + 3);
console.log("2 ** 10 =", 2 ** 10);
console.log("Math.PI:", Math.PI);
console.log("Math.abs(-42):", Math.abs(-42));
console.log("Math.floor(3.7):", Math.floor(3.7));
console.log("Math.ceil(3.2):", Math.ceil(3.2));
console.log("Math.max(1, 5, 3):", Math.max(1, 5, 3));
console.log("Math.sqrt(16):", Math.sqrt(16));`,

    'strings': `const str = "Hello, World!";
console.log("length:", str.length);
console.log("toUpperCase():", str.toUpperCase());
console.log("toLowerCase():", str.toLowerCase());
console.log("indexOf('o'):", str.indexOf('o'));
console.log("includes('World'):", str.includes('World'));
console.log("slice(0, 5):", str.slice(0, 5));
console.log("replace('World', 'TypeScript'):", str.replace('World', 'TypeScript'));
const words = str.split(', ');
console.log("split:', '):", words);
const name = "Alice";
const age = 30;
console.log(\`Hello, \${name}! You are \${age} years old.\`);`,

    'json': `const data = {
    name: "Alice",
    age: 30,
    hobbies: ["reading", "coding"]
};
console.log("JSON.stringify:", JSON.stringify(data));
const parsed = JSON.parse('{"x": 1, "y": 2}');
console.log("Parsed:", parsed);
console.log("x:", parsed.x);`,

    'date': `const now = new Date();
console.log("Current date:", now.toString());
console.log("Year:", now.getFullYear());
console.log("Month (0-11):", now.getMonth());
console.log("getTime():", now.getTime());
const christmas = new Date(2024, 11, 25);
console.log("Christmas 2024:", christmas.toDateString());
const fromTimestamp = new Date(0);
console.log("Unix epoch:", fromTimestamp.toISOString());`,

    'destructuring': `const colors = ["red", "green", "blue", "yellow"];
const [first, second] = colors;
console.log("first:", first, "second:", second);
const [head, ...tail] = colors;
console.log("head:", head);
console.log("tail:", tail);
let x = 1, y = 2;
[x, y] = [y, x];
console.log("swapped: x =", x, "y =", y);
const person = { name: "Alice", age: 30, city: "Wonderland" };
const { name, age } = person;
console.log("name:", name, "age:", age);`,

    'spread-rest': `const arr1 = [1, 2, 3];
const arr2 = [4, 5, 6];
const combined = [...arr1, ...arr2];
console.log("combined:", combined);
const defaults = { theme: "dark", language: "en" };
const userPrefs = { language: "fr", fontSize: 14 };
const merged = { ...defaults, ...userPrefs };
console.log("merged:", merged);
function sum(...numbers: number[]): number {
    return numbers.reduce((acc, n) => acc + n, 0);
}
console.log("sum(1,2,3):", sum(1, 2, 3));
const numbers = [5, 2, 8, 1, 9];
console.log("max:", Math.max(...numbers));`,

    'recursion': `function factorial(n: number): number {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
console.log("5! =", factorial(5));
function fibonacci(n: number): number {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}
const fibs = [];
for (let i = 0; i <= 10; i++) {
    fibs.push(fibonacci(i));
}
console.log("fib:", fibs.join(", "));`,

    'higher-order': `function myMap<T, U>(arr: T[], fn: (item: T) => U): U[] {
    const result: U[] = [];
    for (const item of arr) {
        result.push(fn(item));
    }
    return result;
}
const numbers = [1, 2, 3, 4, 5];
console.log("myMap (square):", myMap(numbers, x => x * x));
const addOne = (x: number) => x + 1;
const double = (x: number) => x * 2;
const composed = (x: number) => addOne(double(x));
console.log("composed(3):", composed(3));`,

    'symbol': `const sym1 = Symbol("description");
const sym2 = Symbol("description");
console.log("sym1:", sym1.toString());
console.log("sym1 === sym2:", sym1 === sym2);
const obj = {
    [sym1]: "value1",
    regularKey: "value2"
};
console.log("obj[sym1]:", obj[sym1]);
console.log("Object.keys(obj):", Object.keys(obj));
const globalSym1 = Symbol.for("app.id");
const globalSym2 = Symbol.for("app.id");
console.log("Symbol.for creates same symbol:", globalSym1 === globalSym2);`,

    'proxy': `const target = { name: "Alice", age: 30 };
const handler = {
    get(target: any, prop: string) {
        console.log("  [GET]", prop);
        return target[prop];
    },
    set(target: any, prop: string, value: any) {
        console.log("  [SET]", prop, "=", value);
        target[prop] = value;
        return true;
    }
};
const proxy = new Proxy(target, handler);
console.log("Getting name:", proxy.name);
proxy.age = 31;
console.log("New age:", proxy.age);
const obj = { x: 1, y: 2 };
console.log("Reflect.get:", Reflect.get(obj, "x"));
Reflect.set(obj, "z", 3);
console.log("After Reflect.set:", obj);`,

    'config-generator': `interface Location {
    path: string;
    proxy_pass?: string;
    root?: string;
}
interface ServerConfig {
    listen: number;
    server_name: string;
    locations: Location[];
    ssl?: boolean;
}
function generateNginxConfig(config: ServerConfig): string {
    const lines: string[] = [];
    lines.push("server {");
    lines.push(\`    listen \${config.listen}\${config.ssl ? " ssl" : ""};\`);
    lines.push(\`    server_name \${config.server_name};\`);
    for (const loc of config.locations) {
        lines.push(\`    location \${loc.path} {\`);
        if (loc.proxy_pass) {
            lines.push(\`        proxy_pass \${loc.proxy_pass};\`);
        }
        if (loc.root) {
            lines.push(\`        root \${loc.root};\`);
        }
        lines.push("    }");
    }
    lines.push("}");
    return lines.join("\\n");
}
const myServer: ServerConfig = {
    listen: 443,
    server_name: "example.com",
    ssl: true,
    locations: [
        { path: "/", root: "/var/www/html" },
        { path: "/api", proxy_pass: "http://localhost:3000" }
    ]
};
console.log(generateNginxConfig(myServer));`,

    'template-engine': `interface TemplateContext {
    [key: string]: string | number | boolean | TemplateContext | TemplateContext[];
}
function render(template: string, context: TemplateContext): string {
    let result = template;
    const eachRegex = /\\{\\{#each (\\w+)\\}\\}([\\s\\S]*?)\\{\\{\\/each\\}\\}/g;
    result = result.replace(eachRegex, (match: string, key: string, inner: string) => {
        const items = context[key];
        if (!Array.isArray(items)) return "";
        return items.map((item: TemplateContext) => render(inner, item)).join("");
    });
    const ifRegex = /\\{\\{#if (\\w+)\\}\\}([\\s\\S]*?)\\{\\{\\/if\\}\\}/g;
    result = result.replace(ifRegex, (match: string, key: string, inner: string) => {
        return context[key] ? render(inner, context) : "";
    });
    const varRegex = /\\{\\{(\\w+)\\}\\}/g;
    result = result.replace(varRegex, (match: string, key: string) => {
        const val = context[key];
        return val !== undefined ? String(val) : "";
    });
    return result;
}
const emailTemplate = \`Hello {{name}}!
{{#if premium}}Premium member!{{/if}}
Orders:
{{#each orders}}
- {{product}}: \\\${{price}}
{{/each}}\`;
const data: TemplateContext = {
    name: "Alice",
    premium: true,
    orders: [
        { product: "Laptop", price: 999 },
        { product: "Mouse", price: 29 }
    ]
};
console.log(render(emailTemplate, data));`,

    'data-pipeline': `interface SalesRecord {
    id: number;
    product: string;
    quantity: number;
    price: number;
    status: string;
}
const salesData: SalesRecord[] = [
    { id: 1, product: "Widget A", quantity: 10, price: 25, status: "completed" },
    { id: 2, product: "Widget B", quantity: 5, price: 50, status: "completed" },
    { id: 3, product: "Widget A", quantity: 8, price: 25, status: "pending" },
    { id: 4, product: "Widget C", quantity: 20, price: 15, status: "completed" },
];
console.log("Raw data:", salesData.length, "records");
const completed = salesData.filter(r => r.status === "completed");
console.log("Completed:", completed.length, "records");
const withTotals = completed.map(r => ({
    ...r,
    total: r.quantity * r.price
}));
const grandTotal = withTotals.reduce((sum, r) => sum + r.total, 0);
console.log("Grand Total: $" + grandTotal);
const byProduct: Record<string, number> = {};
for (const r of withTotals) {
    byProduct[r.product] = (byProduct[r.product] || 0) + r.total;
}
for (const [product, total] of Object.entries(byProduct)) {
    console.log(\`  \${product}: $\${total}\`);
}`,

    'state-machine': `// TypeScript enums as state machine states and events
enum OrderState {
    Pending = "pending",
    Confirmed = "confirmed",
    Shipped = "shipped",
    Delivered = "delivered",
    Cancelled = "cancelled"
}

enum OrderEvent {
    Confirm = "confirm",
    Ship = "ship",
    Deliver = "deliver",
    Cancel = "cancel"
}

// Type-safe transition table using computed property keys
const transitions: Record<OrderState, Partial<Record<OrderEvent, OrderState>>> = {
    [OrderState.Pending]: {
        [OrderEvent.Confirm]: OrderState.Confirmed,
        [OrderEvent.Cancel]: OrderState.Cancelled
    },
    [OrderState.Confirmed]: {
        [OrderEvent.Ship]: OrderState.Shipped,
        [OrderEvent.Cancel]: OrderState.Cancelled
    },
    [OrderState.Shipped]: {
        [OrderEvent.Deliver]: OrderState.Delivered
    },
    [OrderState.Delivered]: {},
    [OrderState.Cancelled]: {}
};

class Order {
    constructor(
        public id: string,
        public state: OrderState = OrderState.Pending
    ) {}

    transition(event: OrderEvent): boolean {
        const nextState = transitions[this.state][event];
        if (nextState) {
            console.log(\`Order \${this.id}: \${this.state} -> \${nextState}\`);
            this.state = nextState;
            return true;
        }
        console.log(\`Order \${this.id}: Invalid transition \${event} from \${this.state}\`);
        return false;
    }
}

const order = new Order("ORD-001");
console.log("Initial state:", order.state);
order.transition(OrderEvent.Confirm);
order.transition(OrderEvent.Ship);
order.transition(OrderEvent.Deliver);
// This should fail - can't cancel a delivered order
order.transition(OrderEvent.Cancel);
console.log("Final state:", order.state);`
};

// Run all playground examples
for (const [name, code] of Object.entries(PLAYGROUND_EXAMPLES)) {
    test(`Playground: ${name}`, () => {
        const runner = new TsRunner();
        const result = runCode(runner, code);
        assertTrue(result.success, `Example '${name}' failed: ${result.error}`);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Results Summary
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n═══════════════════════════════════════════════════════════════');
console.log(`Results: ${passed} passed, ${failed} failed`);
console.log('═══════════════════════════════════════════════════════════════\n');

if (failed > 0) {
    process.exit(1);
}
