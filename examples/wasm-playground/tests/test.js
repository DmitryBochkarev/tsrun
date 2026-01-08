/**
 * End-to-end tests for tsrun WASM module
 *
 * Tests the WASM module with playground examples.
 * Detailed language feature tests are in the Rust test suite.
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
    const result = runner.run('1 + 2 * 3');
    assertTrue(result.success, 'Expected success');
    assertEqual(result.value, '7');
});

test('Console output is captured', () => {
    const runner = new TsRunner();
    const result = runner.run('console.log("Hello")');
    assertTrue(result.success);
    assertTrue(result.console_output.length === 1);
    assertEqual(result.console_output[0].message, 'Hello');
});

test('Errors are reported', () => {
    const runner = new TsRunner();
    const result = runner.run('undefinedVariable');
    assertTrue(!result.success);
    assertTrue(result.error !== null);
});

test('Runner reset clears state', () => {
    const runner = new TsRunner();
    runner.run('const x = 123');
    runner.reset();
    const result = runner.run('x');
    assertTrue(!result.success);
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
