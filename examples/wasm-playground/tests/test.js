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
// Map Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Map ──\n');

test('Map constructor empty', () => {
    const runner = new TsRunner();
    const result = runner.run('new Map().size');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '0');
});

test('Map constructor with entries', () => {
    const runner = new TsRunner();
    const result = runner.run('new Map([["a", 1], ["b", 2]]).size');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '2');
});

test('Map.set and Map.get', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map();
        m.set("key", "value");
        m.get("key")
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '"value"');
});

test('Map.get returns undefined for missing key', () => {
    const runner = new TsRunner();
    const result = runner.run('new Map().get("missing")');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'undefined');
});

test('Map.has', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map([["a", 1]]);
        [m.has("a"), m.has("b")]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '[true, false]');
});

test('Map.delete', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map([["a", 1], ["b", 2]]);
        const deleted = m.delete("a");
        [deleted, m.has("a"), m.size]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '[true, false, 1]');
});

test('Map.clear', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map([["a", 1], ["b", 2]]);
        m.clear();
        m.size
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '0');
});

test('Map.set returns the map (chaining)', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map();
        m.set("a", 1).set("b", 2).set("c", 3);
        m.size
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

test('Map.forEach', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map([["a", 1], ["b", 2]]);
        let sum = 0;
        m.forEach((value, key) => { sum += value; });
        sum
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

test('Map.keys', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map([["a", 1], ["b", 2]]);
        const keys = [...m.keys()];
        keys.length
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '2');
});

test('Map.values', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map([["a", 1], ["b", 2]]);
        const values = [...m.values()];
        values.reduce((a, b) => a + b, 0)
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

test('Map.entries', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map([["a", 1], ["b", 2]]);
        const entries = [...m.entries()];
        entries.length
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '2');
});

test('Map iteration with for-of', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map([["a", 1], ["b", 2], ["c", 3]]);
        let sum = 0;
        for (const [key, value] of m) {
            sum += value;
        }
        sum
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '6');
});

test('Map with object keys', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const key1 = { id: 1 };
        const key2 = { id: 2 };
        const m = new Map();
        m.set(key1, "first");
        m.set(key2, "second");
        [m.get(key1), m.get(key2), m.size]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '["first", "second", 2]');
});

test('Map overwrites existing key', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const m = new Map();
        m.set("key", "first");
        m.set("key", "second");
        [m.get("key"), m.size]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '["second", 1]');
});

// ─────────────────────────────────────────────────────────────────────────────
// Set Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Set ──\n');

test('Set constructor empty', () => {
    const runner = new TsRunner();
    const result = runner.run('new Set().size');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '0');
});

test('Set constructor with values', () => {
    const runner = new TsRunner();
    const result = runner.run('new Set([1, 2, 3]).size');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

test('Set.add', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set();
        s.add(1);
        s.add(2);
        s.size
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '2');
});

test('Set.add returns the set (chaining)', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set();
        s.add(1).add(2).add(3);
        s.size
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

test('Set.has', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set([1, 2, 3]);
        [s.has(2), s.has(4)]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '[true, false]');
});

test('Set.delete', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set([1, 2, 3]);
        const deleted = s.delete(2);
        [deleted, s.has(2), s.size]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '[true, false, 2]');
});

test('Set.clear', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set([1, 2, 3]);
        s.clear();
        s.size
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '0');
});

test('Set deduplicates values', () => {
    const runner = new TsRunner();
    const result = runner.run('new Set([1, 2, 2, 3, 3, 3]).size');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

test('Set.forEach', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set([1, 2, 3]);
        let sum = 0;
        s.forEach(value => { sum += value; });
        sum
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '6');
});

test('Set.keys (same as values)', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set([1, 2, 3]);
        const keys = [...s.keys()];
        keys.reduce((a, b) => a + b, 0)
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '6');
});

test('Set.values', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set([1, 2, 3]);
        const values = [...s.values()];
        values.length
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

test('Set.entries', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set(["a", "b"]);
        const entries = [...s.entries()];
        entries[0][0] === entries[0][1]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'true');
});

test('Set iteration with for-of', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set([1, 2, 3, 4]);
        let sum = 0;
        for (const value of s) {
            sum += value;
        }
        sum
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '10');
});

test('Set with string values', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set(["apple", "banana", "apple"]);
        [s.size, s.has("apple"), s.has("cherry")]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '[2, true, false]');
});

test('Set spread into array', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const s = new Set([3, 1, 2]);
        const arr = [...s];
        arr.length
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

// ─────────────────────────────────────────────────────────────────────────────
// Value Formatting Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── Value Formatting ──\n');

test('Array formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('[1, 2, 3]');
    assertTrue(result.success);
    assertEqual(result.value, '[1, 2, 3]');
});

test('Nested array formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('[[1, 2], [3, 4]]');
    assertTrue(result.success);
    assertEqual(result.value, '[[1, 2], [3, 4]]');
});

test('Object formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('({ a: 1, b: 2 })');
    assertTrue(result.success);
    assertContains(result.value, 'a: 1');
    assertContains(result.value, 'b: 2');
});

test('Nested object formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('({ x: { y: 1 } })');
    assertTrue(result.success);
    assertContains(result.value, 'x: { y: 1 }');
});

test('Mixed array and object formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('({ arr: [1, 2], obj: { x: 3 } })');
    assertTrue(result.success);
    assertContains(result.value, 'arr: [1, 2]');
    assertContains(result.value, 'obj: { x: 3 }');
});

test('Function formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('function foo() {}; foo');
    assertTrue(result.success);
    assertContains(result.value, '[Function: foo]');
});

test('Arrow function formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('const f = () => {}; f');
    assertTrue(result.success);
    assertContains(result.value, '[Function');
});

test('Map formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('new Map([["a", 1], ["b", 2]])');
    assertTrue(result.success);
    assertContains(result.value, 'Map(2)');
    assertContains(result.value, '"a" => 1');
});

test('Set formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('new Set([1, 2, 3])');
    assertTrue(result.success);
    assertContains(result.value, 'Set(3)');
});

test('Date formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('new Date(0)');
    assertTrue(result.success);
    assertContains(result.value, 'Date(');
});

test('Empty object formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('({})');
    assertTrue(result.success);
    assertEqual(result.value, '{}');
});

test('Empty array formatting', () => {
    const runner = new TsRunner();
    const result = runner.run('[]');
    assertTrue(result.success);
    assertEqual(result.value, '[]');
});

// ─────────────────────────────────────────────────────────────────────────────
// RegExp Tests
// ─────────────────────────────────────────────────────────────────────────────

console.log('\n── RegExp ──\n');

test('RegExp literal basic', () => {
    const runner = new TsRunner();
    const result = runner.run('/hello/.test("hello world")');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'true');
});

test('RegExp literal with flags', () => {
    const runner = new TsRunner();
    const result = runner.run('/HELLO/i.test("hello world")');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'true');
});

test('RegExp.test() method', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const re = /\\d+/;
        re.test("abc123def")
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'true');
});

test('RegExp.test() no match', () => {
    const runner = new TsRunner();
    const result = runner.run('/\\d+/.test("no digits here")');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'false');
});

test('RegExp.exec() with match', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const re = /\\d+/;
        const match = re.exec("abc123def");
        match[0]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '"123"');
});

test('RegExp.exec() no match returns null', () => {
    const runner = new TsRunner();
    const result = runner.run('/\\d+/.exec("no digits")');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'null');
});

test('RegExp.exec() with groups', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const re = /(\\w+)@(\\w+)/;
        const match = re.exec("user@domain");
        match[1] + " at " + match[2]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '"user at domain"');
});

test('RegExp constructor', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const re = new RegExp("hello", "i");
        re.test("HELLO")
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'true');
});

test('RegExp source property', () => {
    const runner = new TsRunner();
    const result = runner.run('/hello\\d+/.source');
    assertTrue(result.success, `Expected success: ${result.error}`);
    // The source is the literal pattern, escaped once for the JS string output
    assertEqual(result.value, '"hello\\d+"');
});

test('RegExp flags property', () => {
    const runner = new TsRunner();
    const result = runner.run('/test/gi.flags');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '"gi"');
});

test('String.match() with RegExp', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const match = "hello123world".match(/\\d+/);
        match[0]
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '"123"');
});

test('String.match() global flag', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const matches = "a1b2c3".match(/\\d/g);
        matches.length
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '3');
});

test('String.replace() with RegExp', () => {
    const runner = new TsRunner();
    const result = runner.run('"hello world".replace(/world/, "there")');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '"hello there"');
});

test('String.replace() global flag', () => {
    const runner = new TsRunner();
    const result = runner.run('"a1b2c3".replace(/\\d/g, "X")');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '"aXbXcX"');
});

test('String.split() with RegExp', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const parts = "a1b2c3".split(/\\d/);
        parts.join("-")
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '"a-b-c-"');
});

test('String.search() with RegExp', () => {
    const runner = new TsRunner();
    const result = runner.run('"hello123world".search(/\\d+/)');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '5');
});

test('String.search() no match', () => {
    const runner = new TsRunner();
    const result = runner.run('"hello".search(/\\d+/)');
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, '-1');
});

test('RegExp case insensitive flag', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const re = /hello/i;
        [re.test("HELLO"), re.test("HeLLo"), re.test("hello")].every(x => x)
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'true');
});

test('RegExp multiline flag', () => {
    const runner = new TsRunner();
    const result = runner.run(`
        const text = "line1\\nline2";
        const re = /^line2/m;
        re.test(text)
    `);
    assertTrue(result.success, `Expected success: ${result.error}`);
    assertEqual(result.value, 'true');
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

test('Console.log formats objects with contents', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log({a: 1, b: 2})');
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 1);
    assertContains(output[0].message, 'a: 1');
    assertContains(output[0].message, 'b: 2');
});

test('Console.log formats arrays with contents', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log([1, 2, 3])');
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 1);
    assertEqual(output[0].message, '[1, 2, 3]');
});

test('Console.log formats nested objects', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log({a: {b: [1, 2, 3]}, c: 4})');
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 1);
    assertContains(output[0].message, 'a: { b: [1, 2, 3] }');
    assertContains(output[0].message, 'c: 4');
});

test('Console.log formats Map with contents', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log(new Map([["x", 1], ["y", 2]]))');
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 1);
    assertContains(output[0].message, 'Map(2)');
    assertContains(output[0].message, 'x => 1');
});

test('Console.log formats Set with contents', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log(new Set([1, 2, 3]))');
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 1);
    assertContains(output[0].message, 'Set(3)');
    assertContains(output[0].message, '1');
    assertContains(output[0].message, '2');
    assertContains(output[0].message, '3');
});

test('Console.log formats mixed arguments correctly', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log("obj:", {x: 1}, "arr:", [1, 2])');
    assertTrue(result.success);

    const output = result.console_output;
    assertEqual(output.length, 1);
    assertContains(output[0].message, 'obj: { x: 1 }');
    assertContains(output[0].message, 'arr: [1, 2]');
});

// ─────────────────────────────────────────────────────────────────────────────
// Playground Examples Tests
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

    'state-machine': `type VendingState = "idle" | "selecting" | "dispensing";
type VendingEvent = "insert_coin" | "select_item" | "dispense_complete";
class VendingMachine {
    state: VendingState = "idle";
    balance: number = 0;
    items: Map<string, { name: string; price: number }> = new Map([
        ["A1", { name: "Cola", price: 150 }],
        ["A2", { name: "Chips", price: 100 }],
    ]);
    transition(event: VendingEvent, data?: any): string {
        const prevState = this.state;
        let message = "";
        switch (this.state) {
            case "idle":
                if (event === "insert_coin") {
                    this.balance = data || 0;
                    this.state = "selecting";
                    message = \`Inserted $\${(this.balance / 100).toFixed(2)}\`;
                }
                break;
            case "selecting":
                if (event === "select_item") {
                    const item = this.items.get(data);
                    if (item && this.balance >= item.price) {
                        this.state = "dispensing";
                        message = \`Dispensing \${item.name}...\`;
                    }
                }
                break;
            case "dispensing":
                if (event === "dispense_complete") {
                    this.balance = 0;
                    this.state = "idle";
                    message = "Complete!";
                }
                break;
        }
        console.log(\`[\${prevState} -> \${this.state}] \${message}\`);
        return message;
    }
}
const vm = new VendingMachine();
vm.transition("insert_coin", 150);
vm.transition("select_item", "A1");
vm.transition("dispense_complete");`
};

// Run all playground examples
for (const [name, code] of Object.entries(PLAYGROUND_EXAMPLES)) {
    test(`Playground: ${name}`, () => {
        const runner = new TsRunner();
        const result = runner.run(code);
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
