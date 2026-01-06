/**
 * End-to-end tests for tsrun WASM module
 *
 * Run with: node test.js
 */

import { TsRunner } from './pkg/tsrun.js';

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

function assertEqual(actual, expected, message = '') {
    if (actual !== expected) {
        throw new Error(`${message}\n  Expected: ${JSON.stringify(expected)}\n  Actual: ${JSON.stringify(actual)}`);
    }
}

function assertTrue(value, message = '') {
    if (!value) {
        throw new Error(message || 'Expected true');
    }
}

function assertFalse(value, message = '') {
    if (value) {
        throw new Error(message || 'Expected false');
    }
}

function assertContains(str, substring, message = '') {
    if (!str.includes(substring)) {
        throw new Error(`${message}\n  Expected to contain: ${JSON.stringify(substring)}\n  Actual: ${JSON.stringify(str)}`);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test Suite
// ═══════════════════════════════════════════════════════════════════════════════

console.log('\n═══════════════════════════════════════════════════════════════');
console.log('tsrun WASM End-to-End Tests');
console.log('═══════════════════════════════════════════════════════════════\n');

// ─────────────────────────────────────────────────────────────────────────────
// Basic Execution Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('── Basic Execution ──\n');

test('TsRunner can be instantiated', () => {
    const runner = new TsRunner();
    assertTrue(runner !== null);
});

test('Simple number expression', () => {
    const runner = new TsRunner();
    const result = runner.run('1 + 2 * 3');
    assertTrue(result.success, 'Expected success');
    assertEqual(result.value, '7');
});

test('String concatenation', () => {
    const runner = new TsRunner();
    const result = runner.run('"Hello" + " " + "World"');
    assertTrue(result.success);
    assertEqual(result.value, '"Hello World"');
});

test('Boolean expression', () => {
    const runner = new TsRunner();
    const result = runner.run('true && false');
    assertTrue(result.success);
    assertEqual(result.value, 'false');
});

test('Null and undefined', () => {
    const runner = new TsRunner();

    let result = runner.run('null');
    assertTrue(result.success);
    assertEqual(result.value, 'null');

    result = runner.run('undefined');
    assertTrue(result.success);
    assertEqual(result.value, 'undefined');
});

// ─────────────────────────────────────────────────────────────────────────────
// Variable and Function Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Variables and Functions ──\n');

test('Variable declaration and usage', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const x = 10;
        const y = 20;
        x + y
    `);
    assertTrue(result.success);
    assertEqual(result.value, '30');
});

test('Let variable reassignment', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        let x = 5;
        x = x * 2;
        x
    `);
    assertTrue(result.success);
    assertEqual(result.value, '10');
});

test('Function declaration and call', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        function add(a, b) {
            return a + b;
        }
        add(3, 4)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '7');
});

test('Arrow function', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const multiply = (a, b) => a * b;
        multiply(6, 7)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '42');
});

test('Closure', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        function makeCounter() {
            let count = 0;
            return () => ++count;
        }
        const counter = makeCounter();
        counter();
        counter();
        counter()
    `);
    assertTrue(result.success);
    assertEqual(result.value, '3');
});

test('Recursion (fibonacci)', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        function fib(n) {
            if (n <= 1) return n;
            return fib(n - 1) + fib(n - 2);
        }
        fib(10)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '55');
});

// ─────────────────────────────────────────────────────────────────────────────
// TypeScript Features
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── TypeScript Features ──\n');

test('Type annotations are parsed and ignored', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        function greet(name: string): string {
            return "Hello, " + name;
        }
        greet("TypeScript")
    `);
    assertTrue(result.success);
    assertEqual(result.value, '"Hello, TypeScript"');
});

test('Interface declarations', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        interface Point {
            x: number;
            y: number;
        }
        const p: Point = { x: 3, y: 4 };
        Math.sqrt(p.x * p.x + p.y * p.y)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '5');
});

test('Type alias', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        type StringOrNumber = string | number;
        const value: StringOrNumber = 42;
        value
    `);
    assertTrue(result.success);
    assertEqual(result.value, '42');
});

test('Generic function syntax', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        function identity<T>(x: T): T {
            return x;
        }
        identity(42)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '42');
});

test('Enum declaration', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        enum Color { Red, Green, Blue }
        Color.Green
    `);
    assertTrue(result.success);
    assertEqual(result.value, '1');
});

// ─────────────────────────────────────────────────────────────────────────────
// Array Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Arrays ──\n');

test('Array literal', () => {
    const runner = new TsRunner();
    const result = runner.run('[1, 2, 3].length');
    assertTrue(result.success);
    assertEqual(result.value, '3');
});

test('Array.map', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const arr = [1, 2, 3];
        const doubled = arr.map(x => x * 2);
        doubled[1]
    `);
    assertTrue(result.success);
    assertEqual(result.value, '4');
});

test('Array.filter', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const arr = [1, 2, 3, 4, 5];
        const evens = arr.filter(x => x % 2 === 0);
        evens.length
    `);
    assertTrue(result.success);
    assertEqual(result.value, '2');
});

test('Array.reduce', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const arr = [1, 2, 3, 4, 5];
        arr.reduce((sum, x) => sum + x, 0)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '15');
});

test('Spread operator', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const a = [1, 2];
        const b = [3, 4];
        const c = [...a, ...b];
        c.length
    `);
    assertTrue(result.success);
    assertEqual(result.value, '4');
});

test('Destructuring', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const [first, second, ...rest] = [1, 2, 3, 4, 5];
        first + second + rest.length
    `);
    assertTrue(result.success);
    assertEqual(result.value, '6');
});

// ─────────────────────────────────────────────────────────────────────────────
// Object Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Objects ──\n');

test('Object literal', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const obj = { a: 1, b: 2 };
        obj.a + obj.b
    `);
    assertTrue(result.success);
    assertEqual(result.value, '3');
});

test('Object destructuring', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const { x, y } = { x: 10, y: 20 };
        x * y
    `);
    assertTrue(result.success);
    assertEqual(result.value, '200');
});

test('Object.keys', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const obj = { a: 1, b: 2, c: 3 };
        Object.keys(obj).length
    `);
    assertTrue(result.success);
    assertEqual(result.value, '3');
});

test('Object.values', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const obj = { a: 1, b: 2, c: 3 };
        Object.values(obj).reduce((sum, x) => sum + x, 0)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '6');
});

// ─────────────────────────────────────────────────────────────────────────────
// Class Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Classes ──\n');

test('Class declaration', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        class Point {
            constructor(x, y) {
                this.x = x;
                this.y = y;
            }
            distance() {
                return Math.sqrt(this.x * this.x + this.y * this.y);
            }
        }
        const p = new Point(3, 4);
        p.distance()
    `);
    assertTrue(result.success);
    assertEqual(result.value, '5');
});

test('Class inheritance', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        class Animal {
            constructor(name) {
                this.name = name;
            }
            speak() {
                return this.name + " makes a sound";
            }
        }
        class Dog extends Animal {
            speak() {
                return this.name + " barks";
            }
        }
        const dog = new Dog("Rex");
        dog.speak()
    `);
    assertTrue(result.success);
    assertEqual(result.value, '"Rex barks"');
});

test('Static methods', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        class Calculator {
            static add(a, b) {
                return a + b;
            }
        }
        Calculator.add(5, 3)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '8');
});

// ─────────────────────────────────────────────────────────────────────────────
// Control Flow Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Control Flow ──\n');

test('If-else statement', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        function classify(n) {
            if (n < 0) return "negative";
            else if (n === 0) return "zero";
            else return "positive";
        }
        classify(-5)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '"negative"');
});

test('For loop', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        let sum = 0;
        for (let i = 1; i <= 10; i++) {
            sum += i;
        }
        sum
    `);
    assertTrue(result.success);
    assertEqual(result.value, '55');
});

test('While loop', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        let n = 10;
        let factorial = 1;
        while (n > 1) {
            factorial *= n;
            n--;
        }
        factorial
    `);
    assertTrue(result.success);
    assertEqual(result.value, '3628800');
});

test('For-of loop', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const arr = [1, 2, 3, 4, 5];
        let sum = 0;
        for (const x of arr) {
            sum += x;
        }
        sum
    `);
    assertTrue(result.success);
    assertEqual(result.value, '15');
});

test('Switch statement', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        function dayName(n) {
            switch (n) {
                case 0: return "Sunday";
                case 1: return "Monday";
                case 2: return "Tuesday";
                default: return "Unknown";
            }
        }
        dayName(1)
    `);
    assertTrue(result.success);
    assertEqual(result.value, '"Monday"');
});

test('Ternary operator', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const x = 10;
        x > 5 ? "big" : "small"
    `);
    assertTrue(result.success);
    assertEqual(result.value, '"big"');
});

// ─────────────────────────────────────────────────────────────────────────────
// Error Handling Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Error Handling ──\n');

test('Syntax error reports correctly', () => {
    const runner = new TsRunner();
    const result = runner.run('const x = ;'); // Invalid syntax
    assertFalse(result.success);
    assertTrue(result.error !== null);
    assertContains(result.error, 'Parse error');
});

test('Reference error for undefined variable', () => {
    const runner = new TsRunner();
    const result = runner.run('undefinedVariable');
    assertFalse(result.success);
    assertTrue(result.error !== null);
});

test('Type error: calling non-function', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const x = 42;
        x()
    `);
    assertFalse(result.success);
    assertTrue(result.error !== null);
});

test('Try-catch catches errors', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        let caught = false;
        try {
            throw new Error("test error");
        } catch (e) {
            caught = true;
        }
        caught
    `);
    assertTrue(result.success);
    assertEqual(result.value, 'true');
});

// ─────────────────────────────────────────────────────────────────────────────
// Built-in Objects Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Built-in Objects ──\n');

test('Math.abs', () => {
    const runner = new TsRunner();
    const result = runner.run('Math.abs(-42)');
    assertTrue(result.success);
    assertEqual(result.value, '42');
});

test('Math.max/min', () => {
    const runner = new TsRunner();
    let result = runner.run('Math.max(1, 5, 3)');
    assertTrue(result.success);
    assertEqual(result.value, '5');

    result = runner.run('Math.min(1, 5, 3)');
    assertTrue(result.success);
    assertEqual(result.value, '1');
});

test('Math.floor/ceil/round', () => {
    const runner = new TsRunner();
    let result = runner.run('Math.floor(3.7)');
    assertEqual(result.value, '3');

    result = runner.run('Math.ceil(3.2)');
    assertEqual(result.value, '4');

    result = runner.run('Math.round(3.5)');
    assertEqual(result.value, '4');
});

test('String methods', () => {
    const runner = new TsRunner();
    let result = runner.run('"hello".toUpperCase()');
    assertTrue(result.success);
    assertEqual(result.value, '"HELLO"');

    result = runner.run('"  trim  ".trim()');
    assertTrue(result.success);
    assertEqual(result.value, '"trim"');
});

test('JSON.stringify', () => {
    const runner = new TsRunner();
    const result = runner.run('JSON.stringify({ a: 1, b: [2, 3] })');
    assertTrue(result.success);
    // The result is a JSON string wrapped in quotes
    assertContains(result.value, '"a":1');
    assertContains(result.value, '"b":[2,3]');
});

test('JSON.parse', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const obj = JSON.parse('{"x": 42}');
        obj.x
    `);
    assertTrue(result.success);
    assertEqual(result.value, '42');
});

// ─────────────────────────────────────────────────────────────────────────────
// Runner State Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Runner State ──\n');

test('Runner reset clears state', () => {
    const runner = new TsRunner();

    // Define a variable
    runner.run('const testVar = 123');

    // Reset the runner
    runner.reset();

    // Variable should no longer exist
    const result = runner.run('testVar');
    assertFalse(result.success);
});

test('Each run is independent (playground mode)', () => {
    const runner = new TsRunner();

    // Each run is independent - this is the expected behavior for a playground
    // where users enter complete programs
    const result1 = runner.run('const x = 42; x');
    assertTrue(result1.success);
    assertEqual(result1.value, '42');

    // A new run starts fresh
    const result2 = runner.run('const y = 100; y');
    assertTrue(result2.success);
    assertEqual(result2.value, '100');
});

// ─────────────────────────────────────────────────────────────────────────────
// Console Output Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Console Output ──\n');

test('Console output is captured', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log("Hello, World!")');
    assertTrue(result.success);

    const output = result.console_output;
    assertTrue(output.length === 1, 'Expected 1 console output entry');
    assertEqual(output[0].level, 'log');
    assertEqual(output[0].message, 'Hello, World!');
});

test('Multiple console outputs are captured in order', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        console.log("first");
        console.log("second");
        console.log("third");
    `);
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 3);
    assertEqual(output[0].message, 'first');
    assertEqual(output[1].message, 'second');
    assertEqual(output[2].message, 'third');
});

test('Console levels are captured correctly', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        console.log("log");
        console.info("info");
        console.debug("debug");
        console.warn("warn");
        console.error("error");
    `);
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 5);
    assertEqual(output[0].level, 'log');
    assertEqual(output[1].level, 'info');
    assertEqual(output[2].level, 'debug');
    assertEqual(output[3].level, 'warn');
    assertEqual(output[4].level, 'error');
});

test('Console output with multiple arguments', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log("x =", 42)');
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 1);
    assertEqual(output[0].message, 'x = 42');
});

// ─────────────────────────────────────────────────────────────────────────────
// Results Summary
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n═══════════════════════════════════════════════════════════════');
console.log(`Results: ${passed} passed, ${failed} failed`);
console.log('═══════════════════════════════════════════════════════════════\n');

if (failed > 0) {
    process.exit(1);
}
