// tsrun Playground - Main JavaScript
// Loads the WASM module and handles UI interaction

let runner = null;
let wasmLoaded = false;

// DOM elements
const codeEl = document.getElementById('code');
const outputEl = document.getElementById('output');
const statusEl = document.getElementById('status');
const runBtn = document.getElementById('run-btn');
const clearBtn = document.getElementById('clear-btn');
const examplesSelect = document.getElementById('examples');

// Initialize WASM module
async function initWasm() {
    setStatus('loading', 'Loading WASM...');

    try {
        const wasm = await import('./pkg/tsrun.js');
        await wasm.default();

        runner = new wasm.TsRunner();
        wasmLoaded = true;

        setStatus('success', 'Ready');
        runBtn.disabled = false;

        console.log('tsrun WASM module loaded successfully');
    } catch (err) {
        setStatus('error', 'Failed to load WASM');
        console.error('Failed to load WASM module:', err);
        appendOutput('error', `Failed to load WASM module: ${err.message}\n\nMake sure you have built the WASM module:\n  ./build.sh`);
    }
}

// Set status indicator
function setStatus(type, message) {
    statusEl.textContent = message;
    statusEl.className = `status ${type}`;
}

// Clear output
function clearOutput() {
    outputEl.innerHTML = '';
}

// Append output line
function appendOutput(level, message) {
    const line = document.createElement('div');
    line.className = `output-line output-${level}`;
    line.textContent = message;
    outputEl.appendChild(line);
    outputEl.scrollTop = outputEl.scrollHeight;
}

// Display console output from result
function displayOutput(result) {
    clearOutput();

    // Show console output from the WASM module
    const consoleOutput = result.console_output;
    if (consoleOutput && consoleOutput.length > 0) {
        for (const entry of consoleOutput) {
            appendOutput(entry.level, entry.message);
        }
    }

    // Show result or error
    if (result.error) {
        appendOutput('error', `\nError: ${result.error}`);
    } else if (result.value && result.value !== 'undefined') {
        const resultLine = document.createElement('div');
        resultLine.className = 'output-line output-result';
        resultLine.textContent = `=> ${result.value}`;
        outputEl.appendChild(resultLine);
    }
}

// Run the code
async function runCode() {
    if (!wasmLoaded || !runner) {
        setStatus('error', 'WASM not loaded');
        return;
    }

    const code = codeEl.value;
    if (!code.trim()) {
        clearOutput();
        appendOutput('info', 'No code to run');
        return;
    }

    clearOutput();
    setStatus('loading', 'Running...');
    runBtn.disabled = true;

    try {
        const result = runner.run(code);
        displayOutput(result);

        if (result.error) {
            setStatus('error', 'Error');
        } else {
            setStatus('success', 'Done');
        }
    } catch (err) {
        setStatus('error', 'Error');
        appendOutput('error', `Unexpected error: ${err.message}`);
        console.error('Execution error:', err);
    } finally {
        runBtn.disabled = false;
    }
}

// Load example code
function loadExample(name) {
    if (name && EXAMPLES[name]) {
        codeEl.value = EXAMPLES[name].code;
        clearOutput();
        setStatus('success', 'Ready');
    }
}

// Event listeners
runBtn.addEventListener('click', runCode);

clearBtn.addEventListener('click', () => {
    codeEl.value = '';
    clearOutput();
    setStatus('success', 'Ready');
    examplesSelect.value = '';
});

examplesSelect.addEventListener('change', (e) => {
    loadExample(e.target.value);
});

// Keyboard shortcuts
codeEl.addEventListener('keydown', (e) => {
    // Ctrl/Cmd + Enter to run
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
        e.preventDefault();
        runCode();
    }

    // Tab key for indentation
    if (e.key === 'Tab') {
        e.preventDefault();
        const start = codeEl.selectionStart;
        const end = codeEl.selectionEnd;
        codeEl.value = codeEl.value.substring(0, start) + '    ' + codeEl.value.substring(end);
        codeEl.selectionStart = codeEl.selectionEnd = start + 4;
    }
});

// ═══════════════════════════════════════════════════════════════════════════════
// Examples
// ═══════════════════════════════════════════════════════════════════════════════

const EXAMPLES = {
    'hello-world': {
        name: 'Hello World',
        code: `// Welcome to the tsrun Playground!
// This is a TypeScript interpreter running in your browser via WebAssembly.

console.log("Hello, World!");
console.log("Welcome to tsrun!");

// Try modifying this code and click "Run" (or press Ctrl+Enter)`
    },

    'variables': {
        name: 'Variables & Types',
        code: `// Variables with let and const
let counter = 0;
const PI = 3.14159;

// TypeScript type annotations (parsed but not enforced at runtime)
let message: string = "Hello";
let count: number = 42;
let active: boolean = true;

console.log("message:", message);
console.log("count:", count);
console.log("active:", active);
console.log("PI:", PI);

// Variable reassignment
counter = counter + 1;
console.log("counter:", counter);`
    },

    'functions': {
        name: 'Functions',
        code: `// Regular function declaration
function greet(name: string): string {
    return "Hello, " + name + "!";
}

// Arrow function
const add = (a: number, b: number): number => a + b;

// Function with default parameters
function power(base: number, exponent: number = 2): number {
    let result = 1;
    for (let i = 0; i < exponent; i++) {
        result *= base;
    }
    return result;
}

// Rest parameters
function sum(...numbers: number[]): number {
    return numbers.reduce((acc, n) => acc + n, 0);
}

console.log(greet("TypeScript"));
console.log("2 + 3 =", add(2, 3));
console.log("5^3 =", power(5, 3));
console.log("5^2 =", power(5)); // Uses default exponent
console.log("sum(1,2,3,4,5) =", sum(1, 2, 3, 4, 5));`
    },

    'closures': {
        name: 'Closures',
        code: `// Closures capture variables from their enclosing scope

function makeCounter() {
    let count = 0;
    return function() {
        count++;
        return count;
    };
}

const counter1 = makeCounter();
const counter2 = makeCounter();

console.log("counter1:", counter1()); // 1
console.log("counter1:", counter1()); // 2
console.log("counter1:", counter1()); // 3
console.log("counter2:", counter2()); // 1 (separate instance)
console.log("counter1:", counter1()); // 4

// Closure with parameters
function multiplier(factor: number) {
    return (x: number) => x * factor;
}

const double = multiplier(2);
const triple = multiplier(3);

console.log("double(5):", double(5));
console.log("triple(5):", triple(5));`
    },

    'arrays': {
        name: 'Arrays',
        code: `// Array creation and access
const numbers = [1, 2, 3, 4, 5];
const mixed = [1, "two", true, null];

console.log("numbers:", numbers);
console.log("first:", numbers[0]);
console.log("length:", numbers.length);

// Array methods
console.log("\\n--- Array Methods ---");

// map - transform each element
const doubled = numbers.map(x => x * 2);
console.log("doubled:", doubled);

// filter - keep elements matching condition
const evens = numbers.filter(x => x % 2 === 0);
console.log("evens:", evens);

// reduce - accumulate to single value
const sum = numbers.reduce((acc, x) => acc + x, 0);
console.log("sum:", sum);

// find - first matching element
const firstBig = numbers.find(x => x > 3);
console.log("first > 3:", firstBig);

// some/every - test conditions
console.log("some > 3:", numbers.some(x => x > 3));
console.log("every > 0:", numbers.every(x => x > 0));

// forEach
console.log("\\nforEach:");
numbers.forEach((x, i) => console.log("  [" + i + "] =", x));

// Spread operator
const more = [...numbers, 6, 7, 8];
console.log("\\nspread:", more);

// Array destructuring
const [first, second, ...rest] = numbers;
console.log("first:", first, "second:", second, "rest:", rest);`
    },

    'objects': {
        name: 'Objects',
        code: `// Object literals
const person = {
    name: "Alice",
    age: 30,
    city: "Wonderland"
};

console.log("person:", person);
console.log("name:", person.name);
console.log("age:", person.age);

// Computed property names
const key = "dynamicKey";
const obj = {
    [key]: "dynamic value",
    ["computed" + "Name"]: 42
};
console.log("\\ncomputed props:", obj);

// Object methods
console.log("\\n--- Object Methods ---");
console.log("keys:", Object.keys(person));
console.log("values:", Object.values(person));
console.log("entries:", Object.entries(person));

// Object destructuring
const { name, age } = person;
console.log("\\ndestructured - name:", name, "age:", age);

// Nested destructuring
const data = {
    user: { id: 1, email: "alice@example.com" },
    settings: { theme: "dark" }
};
const { user: { email }, settings: { theme } } = data;
console.log("email:", email, "theme:", theme);

// Spread with objects
const updated = { ...person, age: 31, job: "Developer" };
console.log("\\nupdated:", updated);`
    },

    'classes': {
        name: 'Classes',
        code: `// Class declaration
class Animal {
    name: string;

    constructor(name: string) {
        this.name = name;
    }

    speak(): string {
        return this.name + " makes a sound";
    }
}

// Inheritance
class Dog extends Animal {
    breed: string;

    constructor(name: string, breed: string) {
        super(name);
        this.breed = breed;
    }

    speak(): string {
        return this.name + " barks!";
    }

    fetch(): string {
        return this.name + " fetches the ball";
    }
}

// Static members
class MathUtils {
    static PI = 3.14159;

    static circleArea(radius: number): number {
        return MathUtils.PI * radius * radius;
    }
}

// Usage
const animal = new Animal("Generic Animal");
console.log(animal.speak());

const dog = new Dog("Rex", "German Shepherd");
console.log(dog.speak());
console.log(dog.fetch());
console.log("breed:", dog.breed);

console.log("\\n--- Static Members ---");
console.log("PI:", MathUtils.PI);
console.log("Circle area (r=5):", MathUtils.circleArea(5));`
    },

    'typescript-types': {
        name: 'TypeScript Types',
        code: `// Type annotations (parsed but not enforced)
let num: number = 42;
let str: string = "hello";
let bool: boolean = true;
let arr: number[] = [1, 2, 3];

// Interface declaration
interface User {
    id: number;
    name: string;
    email?: string; // Optional property
}

const user: User = {
    id: 1,
    name: "Alice",
    email: "alice@example.com"
};

console.log("User:", user);

// Type alias
type StringOrNumber = string | number;
type Point = { x: number; y: number };

const value: StringOrNumber = 42;
const point: Point = { x: 10, y: 20 };

console.log("value:", value);
console.log("point:", point);

// Generic function
function identity<T>(x: T): T {
    return x;
}

console.log("identity(42):", identity(42));
console.log("identity('hello'):", identity("hello"));

// Enum
enum Color {
    Red,
    Green,
    Blue
}

enum Status {
    Pending = "pending",
    Active = "active",
    Completed = "completed"
}

console.log("\\n--- Enums ---");
console.log("Color.Red:", Color.Red);
console.log("Color.Green:", Color.Green);
console.log("Status.Active:", Status.Active);`
    },

    'control-flow': {
        name: 'Control Flow',
        code: `// If-else statements
function classify(n: number): string {
    if (n < 0) {
        return "negative";
    } else if (n === 0) {
        return "zero";
    } else {
        return "positive";
    }
}

console.log("classify(-5):", classify(-5));
console.log("classify(0):", classify(0));
console.log("classify(10):", classify(10));

// Switch statement
function dayName(day: number): string {
    switch (day) {
        case 0: return "Sunday";
        case 1: return "Monday";
        case 2: return "Tuesday";
        case 3: return "Wednesday";
        case 4: return "Thursday";
        case 5: return "Friday";
        case 6: return "Saturday";
        default: return "Invalid day";
    }
}

console.log("\\nDay 3:", dayName(3));

// Loops
console.log("\\n--- For Loop ---");
for (let i = 1; i <= 5; i++) {
    console.log("i =", i);
}

console.log("\\n--- While Loop ---");
let count = 3;
while (count > 0) {
    console.log("countdown:", count);
    count--;
}
console.log("Liftoff!");

console.log("\\n--- For-of Loop ---");
const fruits = ["apple", "banana", "cherry"];
for (const fruit of fruits) {
    console.log("fruit:", fruit);
}

// Ternary operator
const age = 20;
const status = age >= 18 ? "adult" : "minor";
console.log("\\nAge", age, "is", status);`
    },

    'error-handling': {
        name: 'Error Handling',
        code: `// Try-catch-finally
function divide(a: number, b: number): number {
    if (b === 0) {
        throw new Error("Division by zero!");
    }
    return a / b;
}

try {
    console.log("10 / 2 =", divide(10, 2));
    console.log("10 / 0 =", divide(10, 0));
} catch (e) {
    console.error("Caught error:", e.message);
} finally {
    console.log("Division complete");
}

// Custom error types
class ValidationError extends Error {
    field: string;

    constructor(message: string, field: string) {
        super(message);
        this.field = field;
    }
}

function validateAge(age: number): void {
    if (age < 0) {
        throw new ValidationError("Age cannot be negative", "age");
    }
    if (age > 150) {
        throw new ValidationError("Age seems unrealistic", "age");
    }
    console.log("Age", age, "is valid");
}

console.log("\\n--- Custom Errors ---");
try {
    validateAge(25);
    validateAge(-5);
} catch (e) {
    if (e instanceof ValidationError) {
        console.error("Validation failed for field:", e.field);
        console.error("Message:", e.message);
    }
}

// Re-throwing errors
console.log("\\n--- Re-throwing ---");
try {
    try {
        throw new Error("Inner error");
    } catch (e) {
        console.log("Caught and re-throwing...");
        throw e;
    }
} catch (e) {
    console.log("Outer catch:", e.message);
}`
    },

    'async-patterns': {
        name: 'Promises & Generators',
        code: `// Promise creation and chaining
const promise = new Promise((resolve, reject) => {
    resolve(42);
});

promise
    .then(value => {
        console.log("Promise resolved:", value);
        return value * 2;
    })
    .then(doubled => {
        console.log("Doubled:", doubled);
    });

// Promise.resolve/reject
Promise.resolve("immediate value")
    .then(v => console.log("Immediate:", v));

// Generator functions
function* countdown(start: number) {
    while (start > 0) {
        yield start;
        start--;
    }
    return "Done!";
}

console.log("\\n--- Generator ---");
const gen = countdown(3);
console.log(gen.next()); // { value: 3, done: false }
console.log(gen.next()); // { value: 2, done: false }
console.log(gen.next()); // { value: 1, done: false }
console.log(gen.next()); // { value: "Done!", done: true }

// Iterating over generator
console.log("\\n--- Generator iteration ---");
for (const n of countdown(5)) {
    console.log("count:", n);
}

// Generator with yield*
function* concat<T>(...iters: Iterable<T>[]) {
    for (const iter of iters) {
        yield* iter;
    }
}

const combined = [...concat([1, 2], [3, 4], [5])];
console.log("\\ncombined:", combined);`
    },

    'map-set': {
        name: 'Map & Set',
        code: `// Map - key-value pairs with any key type
const map = new Map();
map.set("name", "Alice");
map.set(42, "the answer");
map.set(true, "yes");

console.log("--- Map ---");
console.log("size:", map.size);
console.log("get('name'):", map.get("name"));
console.log("get(42):", map.get(42));
console.log("has(true):", map.has(true));
console.log("has('missing'):", map.has("missing"));

console.log("\\nIterating Map:");
for (const [key, value] of map) {
    console.log(" ", key, "=>", value);
}

// Set - unique values
const set = new Set([1, 2, 3, 2, 1, 4]);
console.log("\\n--- Set ---");
console.log("values:", [...set]); // Duplicates removed
console.log("size:", set.size);
console.log("has(2):", set.has(2));
console.log("has(10):", set.has(10));

set.add(5);
set.delete(1);
console.log("after add(5), delete(1):", [...set]);

// Set operations
const a = new Set([1, 2, 3, 4]);
const b = new Set([3, 4, 5, 6]);

// Union
const union = new Set([...a, ...b]);
console.log("\\nunion:", [...union]);

// Intersection
const intersection = new Set([...a].filter(x => b.has(x)));
console.log("intersection:", [...intersection]);

// Difference
const difference = new Set([...a].filter(x => !b.has(x)));
console.log("difference (a-b):", [...difference]);`
    },

    'regex': {
        name: 'Regular Expressions',
        code: `// Regular expression basics
const text = "The quick brown fox jumps over the lazy dog";

// test - check if pattern matches
console.log("/fox/.test():", /fox/.test(text));
console.log("/cat/.test():", /cat/.test(text));

// match - find matches
console.log("\\nmatch(/[a-z]+o[a-z]+/):", text.match(/[a-z]+o[a-z]+/));

// Replace
const replaced = text.replace(/fox/, "cat");
console.log("\\nreplace fox->cat:", replaced);

// Global flag
const vowels = text.match(/[aeiou]/g);
console.log("\\nall vowels:", vowels);

// Case insensitive
console.log("/THE/i.test():", /THE/i.test(text));

// Split with regex
const words = "one,two;three four".split(/[,;\\s]+/);
console.log("\\nsplit by delimiters:", words);

// Email validation example
function isValidEmail(email: string): boolean {
    const pattern = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$/;
    return pattern.test(email);
}

console.log("\\n--- Email Validation ---");
console.log("test@example.com:", isValidEmail("test@example.com"));
console.log("invalid-email:", isValidEmail("invalid-email"));
console.log("user@domain.org:", isValidEmail("user@domain.org"));`
    },

    'math': {
        name: 'Math Operations',
        code: `// Basic math
console.log("--- Basic Operations ---");
console.log("2 + 3 =", 2 + 3);
console.log("10 - 4 =", 10 - 4);
console.log("6 * 7 =", 6 * 7);
console.log("15 / 4 =", 15 / 4);
console.log("15 % 4 =", 15 % 4);  // Modulo
console.log("2 ** 10 =", 2 ** 10); // Exponentiation

// Math object
console.log("\\n--- Math Object ---");
console.log("Math.PI:", Math.PI);
console.log("Math.E:", Math.E);
console.log("Math.abs(-42):", Math.abs(-42));
console.log("Math.floor(3.7):", Math.floor(3.7));
console.log("Math.ceil(3.2):", Math.ceil(3.2));
console.log("Math.round(3.5):", Math.round(3.5));
console.log("Math.max(1, 5, 3):", Math.max(1, 5, 3));
console.log("Math.min(1, 5, 3):", Math.min(1, 5, 3));
console.log("Math.sqrt(16):", Math.sqrt(16));
console.log("Math.pow(2, 8):", Math.pow(2, 8));

// Trigonometry
console.log("\\n--- Trigonometry ---");
console.log("Math.sin(0):", Math.sin(0));
console.log("Math.cos(0):", Math.cos(0));
console.log("Math.sin(Math.PI/2):", Math.sin(Math.PI / 2));

// Random numbers
console.log("\\n--- Random ---");
console.log("Math.random():", Math.random());
console.log("Math.random():", Math.random());

// Random integer between min and max
function randomInt(min: number, max: number): number {
    return Math.floor(Math.random() * (max - min + 1)) + min;
}
console.log("randomInt(1, 10):", randomInt(1, 10));
console.log("randomInt(1, 10):", randomInt(1, 10));`
    },

    'strings': {
        name: 'String Operations',
        code: `// String basics
const str = "Hello, World!";

console.log("--- String Properties ---");
console.log("length:", str.length);
console.log("charAt(0):", str.charAt(0));
console.log("[0]:", str[0]);

console.log("\\n--- Case Conversion ---");
console.log("toUpperCase():", str.toUpperCase());
console.log("toLowerCase():", str.toLowerCase());

console.log("\\n--- Searching ---");
console.log("indexOf('o'):", str.indexOf('o'));
console.log("lastIndexOf('o'):", str.lastIndexOf('o'));
console.log("includes('World'):", str.includes('World'));
console.log("startsWith('Hello'):", str.startsWith('Hello'));
console.log("endsWith('!'):", str.endsWith('!'));

console.log("\\n--- Extraction ---");
console.log("slice(0, 5):", str.slice(0, 5));
console.log("slice(-6):", str.slice(-6));
console.log("substring(7, 12):", str.substring(7, 12));

console.log("\\n--- Modification ---");
console.log("replace('World', 'TypeScript'):", str.replace('World', 'TypeScript'));
console.log("trim():", "  padded  ".trim());
console.log("padStart(20, '-'):", str.padStart(20, '-'));
console.log("padEnd(20, '-'):", str.padEnd(20, '-'));

console.log("\\n--- Split & Join ---");
const words = str.split(', ');
console.log("split(', '):", words);
console.log("join(' | '):", words.join(' | '));

// Template literals
console.log("\\n--- Template Literals ---");
const name = "Alice";
const age = 30;
console.log(\`Hello, \${name}! You are \${age} years old.\`);
console.log(\`2 + 2 = \${2 + 2}\`);

// Multi-line strings
const multiline = \`Line 1
Line 2
Line 3\`;
console.log("\\nMulti-line:");
console.log(multiline);`
    },

    'json': {
        name: 'JSON Operations',
        code: `// JSON.stringify - convert to JSON string
const data = {
    name: "Alice",
    age: 30,
    hobbies: ["reading", "coding", "gaming"],
    address: {
        city: "Wonderland",
        zip: "12345"
    }
};

console.log("--- JSON.stringify ---");
const jsonStr = JSON.stringify(data);
console.log("Compact:", jsonStr);

const prettyJson = JSON.stringify(data, null, 2);
console.log("\\nPretty printed:");
console.log(prettyJson);

// Selective serialization
const partial = JSON.stringify(data, ["name", "age"]);
console.log("\\nPartial (name, age only):", partial);

// JSON.parse - convert from JSON string
console.log("\\n--- JSON.parse ---");
const parsed = JSON.parse('{"x": 1, "y": 2, "name": "point"}');
console.log("Parsed:", parsed);
console.log("x:", parsed.x);
console.log("name:", parsed.name);

// Round-trip
const original = { a: 1, b: [2, 3], c: { d: 4 } };
const roundTrip = JSON.parse(JSON.stringify(original));
console.log("\\nRound-trip:", roundTrip);

// Handling special values
console.log("\\n--- Special Values ---");
console.log("undefined:", JSON.stringify({ x: undefined }));
console.log("null:", JSON.stringify({ x: null }));
console.log("NaN:", JSON.stringify({ x: NaN }));
console.log("Infinity:", JSON.stringify({ x: Infinity }));`
    },

    'date': {
        name: 'Date & Time',
        code: `// Current date/time
const now = new Date();
console.log("Current date:", now.toString());
console.log("ISO format:", now.toISOString());

// Date components
console.log("\\n--- Date Components ---");
console.log("Year:", now.getFullYear());
console.log("Month (0-11):", now.getMonth());
console.log("Day of month:", now.getDate());
console.log("Day of week (0-6):", now.getDay());
console.log("Hours:", now.getHours());
console.log("Minutes:", now.getMinutes());
console.log("Seconds:", now.getSeconds());
console.log("Milliseconds:", now.getMilliseconds());

// Timestamps
console.log("\\n--- Timestamps ---");
console.log("getTime():", now.getTime());
console.log("Date.now():", Date.now());

// Creating specific dates
console.log("\\n--- Creating Dates ---");
const christmas = new Date(2024, 11, 25); // Month is 0-indexed
console.log("Christmas 2024:", christmas.toDateString());

const fromTimestamp = new Date(0);
console.log("Unix epoch:", fromTimestamp.toISOString());

// Date arithmetic
console.log("\\n--- Date Arithmetic ---");
const tomorrow = new Date(now);
tomorrow.setDate(tomorrow.getDate() + 1);
console.log("Tomorrow:", tomorrow.toDateString());

const nextWeek = new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000);
console.log("Next week:", nextWeek.toDateString());

// Comparing dates
const date1 = new Date(2024, 0, 1);
const date2 = new Date(2024, 6, 1);
console.log("\\nJan 1 < Jul 1:", date1 < date2);
console.log("Difference (days):", Math.floor((date2 - date1) / (24 * 60 * 60 * 1000)));`
    },

    'destructuring': {
        name: 'Destructuring',
        code: `// Array destructuring
console.log("--- Array Destructuring ---");
const colors = ["red", "green", "blue", "yellow"];

const [first, second] = colors;
console.log("first:", first, "second:", second);

// Skip elements
const [, , third] = colors;
console.log("third:", third);

// Rest pattern
const [head, ...tail] = colors;
console.log("head:", head);
console.log("tail:", tail);

// Default values
const [a, b, c, d, e = "purple"] = colors;
console.log("e (default):", e);

// Swapping variables
let x = 1, y = 2;
[x, y] = [y, x];
console.log("swapped: x =", x, "y =", y);

// Object destructuring
console.log("\\n--- Object Destructuring ---");
const person = { name: "Alice", age: 30, city: "Wonderland" };

const { name, age } = person;
console.log("name:", name, "age:", age);

// Rename variables
const { name: personName, city: location } = person;
console.log("personName:", personName, "location:", location);

// Default values
const { country = "Unknown" } = person;
console.log("country (default):", country);

// Nested destructuring
console.log("\\n--- Nested Destructuring ---");
const user = {
    id: 1,
    profile: {
        firstName: "Bob",
        lastName: "Smith"
    },
    roles: ["admin", "user"]
};

const { profile: { firstName, lastName }, roles: [primaryRole] } = user;
console.log("firstName:", firstName);
console.log("lastName:", lastName);
console.log("primaryRole:", primaryRole);

// Function parameter destructuring
console.log("\\n--- Parameter Destructuring ---");
function greet({ name, greeting = "Hello" }: { name: string, greeting?: string }) {
    return greeting + ", " + name + "!";
}

console.log(greet({ name: "World" }));
console.log(greet({ name: "TypeScript", greeting: "Welcome" }));`
    },

    'spread-rest': {
        name: 'Spread & Rest',
        code: `// Spread with arrays
console.log("--- Array Spread ---");
const arr1 = [1, 2, 3];
const arr2 = [4, 5, 6];
const combined = [...arr1, ...arr2];
console.log("combined:", combined);

// Insert elements
const withMiddle = [0, ...arr1, 3.5, ...arr2, 7];
console.log("with insertions:", withMiddle);

// Clone array
const clone = [...arr1];
clone.push(99);
console.log("original:", arr1);
console.log("clone:", clone);

// Spread with objects
console.log("\\n--- Object Spread ---");
const defaults = { theme: "dark", language: "en", notifications: true };
const userPrefs = { language: "fr", fontSize: 14 };
const merged = { ...defaults, ...userPrefs };
console.log("merged:", merged);

// Clone and modify
const original = { x: 1, y: 2 };
const modified = { ...original, z: 3, x: 10 };
console.log("original:", original);
console.log("modified:", modified);

// Rest parameters
console.log("\\n--- Rest Parameters ---");
function sum(...numbers: number[]): number {
    return numbers.reduce((acc, n) => acc + n, 0);
}
console.log("sum(1,2,3):", sum(1, 2, 3));
console.log("sum(1,2,3,4,5):", sum(1, 2, 3, 4, 5));

function logAll(first: string, ...rest: string[]): void {
    console.log("first:", first);
    console.log("rest:", rest);
}
logAll("a", "b", "c", "d");

// Spread in function calls
console.log("\\n--- Spread in Function Calls ---");
const numbers = [5, 2, 8, 1, 9];
console.log("max:", Math.max(...numbers));
console.log("min:", Math.min(...numbers));`
    },

    'recursion': {
        name: 'Recursion',
        code: `// Factorial
function factorial(n: number): number {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

console.log("--- Factorial ---");
for (let i = 0; i <= 10; i++) {
    console.log(i + "! =", factorial(i));
}

// Fibonacci
function fibonacci(n: number): number {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log("\\n--- Fibonacci ---");
const fibs = [];
for (let i = 0; i <= 15; i++) {
    fibs.push(fibonacci(i));
}
console.log(fibs.join(", "));

// Sum of nested arrays
function deepSum(arr: any[]): number {
    let sum = 0;
    for (const item of arr) {
        if (Array.isArray(item)) {
            sum += deepSum(item);
        } else if (typeof item === 'number') {
            sum += item;
        }
    }
    return sum;
}

console.log("\\n--- Deep Sum ---");
const nested = [1, [2, 3], [[4, 5], 6], [[[7]]]];
console.log("nested:", JSON.stringify(nested));
console.log("deepSum:", deepSum(nested));

// Binary search (recursive)
function binarySearch(arr: number[], target: number, low = 0, high = arr.length - 1): number {
    if (low > high) return -1;

    const mid = Math.floor((low + high) / 2);
    if (arr[mid] === target) return mid;
    if (arr[mid] > target) return binarySearch(arr, target, low, mid - 1);
    return binarySearch(arr, target, mid + 1, high);
}

console.log("\\n--- Binary Search ---");
const sorted = [1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
console.log("array:", sorted);
console.log("find 7:", binarySearch(sorted, 7));
console.log("find 15:", binarySearch(sorted, 15));
console.log("find 6:", binarySearch(sorted, 6));`
    },

    'higher-order': {
        name: 'Higher-Order Functions',
        code: `// Functions that take functions as arguments
console.log("--- Higher-Order Functions ---");

// Custom map implementation
function myMap<T, U>(arr: T[], fn: (item: T) => U): U[] {
    const result: U[] = [];
    for (const item of arr) {
        result.push(fn(item));
    }
    return result;
}

const numbers = [1, 2, 3, 4, 5];
console.log("myMap (square):", myMap(numbers, x => x * x));

// Custom filter
function myFilter<T>(arr: T[], predicate: (item: T) => boolean): T[] {
    const result: T[] = [];
    for (const item of arr) {
        if (predicate(item)) {
            result.push(item);
        }
    }
    return result;
}

console.log("myFilter (even):", myFilter(numbers, x => x % 2 === 0));

// Function composition
function compose<T>(...fns: ((x: T) => T)[]): (x: T) => T {
    return (x: T) => fns.reduceRight((acc, fn) => fn(acc), x);
}

const addOne = (x: number) => x + 1;
const double = (x: number) => x * 2;
const square = (x: number) => x * x;

const composed = compose(addOne, double, square);
console.log("\\ncompose(addOne, double, square)(3):", composed(3));
// (3^2) * 2 + 1 = 9 * 2 + 1 = 19

// Pipe (left-to-right composition)
function pipe<T>(...fns: ((x: T) => T)[]): (x: T) => T {
    return (x: T) => fns.reduce((acc, fn) => fn(acc), x);
}

const piped = pipe(square, double, addOne);
console.log("pipe(square, double, addOne)(3):", piped(3));
// ((3^2) * 2) + 1 = 19

// Partial application
function partial<T, U, V>(fn: (a: T, b: U) => V, a: T): (b: U) => V {
    return (b: U) => fn(a, b);
}

const multiply = (a: number, b: number) => a * b;
const multiplyBy5 = partial(multiply, 5);

console.log("\\nmultiplyBy5(3):", multiplyBy5(3));
console.log("multiplyBy5(7):", multiplyBy5(7));

// Memoization
function memoize<T extends (...args: any[]) => any>(fn: T): T {
    const cache = new Map();
    return ((...args: any[]) => {
        const key = JSON.stringify(args);
        if (cache.has(key)) {
            return cache.get(key);
        }
        const result = fn(...args);
        cache.set(key, result);
        return result;
    }) as T;
}

const expensiveFib = memoize((n: number): number => {
    if (n <= 1) return n;
    return expensiveFib(n - 1) + expensiveFib(n - 2);
});

console.log("\\nmemoized fib(35):", expensiveFib(35));`
    },

    'symbol': {
        name: 'Symbols',
        code: `// Symbols are unique identifiers
const sym1 = Symbol("description");
const sym2 = Symbol("description");

console.log("--- Symbol Basics ---");
console.log("sym1:", sym1.toString());
console.log("sym2:", sym2.toString());
console.log("sym1 === sym2:", sym1 === sym2); // Always false

// Symbols as object keys
const obj = {
    [sym1]: "value1",
    regularKey: "value2"
};

console.log("\\n--- Symbols as Keys ---");
console.log("obj[sym1]:", obj[sym1]);
console.log("Object.keys(obj):", Object.keys(obj)); // Doesn't include symbols

// Well-known symbols
console.log("\\n--- Well-Known Symbols ---");

// Symbol.iterator - makes objects iterable
const range = {
    start: 1,
    end: 5,
    [Symbol.iterator]() {
        let current = this.start;
        const end = this.end;
        return {
            next() {
                if (current <= end) {
                    return { value: current++, done: false };
                }
                return { value: undefined, done: true };
            }
        };
    }
};

console.log("Custom iterable range(1,5):", [...range]);

// Symbol.toStringTag
const custom = {
    [Symbol.toStringTag]: "MyCustomObject"
};
console.log("toString:", Object.prototype.toString.call(custom));

// Symbol.for - global symbol registry
console.log("\\n--- Global Symbols ---");
const globalSym1 = Symbol.for("app.id");
const globalSym2 = Symbol.for("app.id");
console.log("Symbol.for creates same symbol:", globalSym1 === globalSym2);
console.log("Symbol.keyFor:", Symbol.keyFor(globalSym1));`
    },

    'proxy': {
        name: 'Proxy & Reflect',
        code: `// Proxy allows intercepting operations on objects
console.log("--- Basic Proxy ---");

const target = { name: "Alice", age: 30 };
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

// Validation proxy
console.log("\\n--- Validation Proxy ---");

function createValidated<T extends object>(obj: T): T {
    return new Proxy(obj, {
        set(target: any, prop: string, value: any) {
            if (prop === "age" && (typeof value !== "number" || value < 0)) {
                throw new Error("Age must be a positive number");
            }
            target[prop] = value;
            return true;
        }
    });
}

const person = createValidated({ name: "Bob", age: 25 });
person.name = "Charlie"; // OK
person.age = 30; // OK
console.log("Valid updates:", person);

try {
    person.age = -5; // Will throw
} catch (e) {
    console.log("Caught:", e.message);
}

// Observable proxy
console.log("\\n--- Observable Proxy ---");

function observable<T extends object>(obj: T, onChange: (prop: string, value: any) => void): T {
    return new Proxy(obj, {
        set(target: any, prop: string, value: any) {
            target[prop] = value;
            onChange(prop, value);
            return true;
        }
    });
}

const state = observable({ count: 0 }, (prop, value) => {
    console.log("  State changed:", prop, "->", value);
});

state.count = 1;
state.count = 2;
state.count = 3;

// Reflect API
console.log("\\n--- Reflect API ---");
const obj2 = { x: 1, y: 2 };

console.log("Reflect.get:", Reflect.get(obj2, "x"));
Reflect.set(obj2, "z", 3);
console.log("After Reflect.set:", obj2);
console.log("Reflect.has:", Reflect.has(obj2, "y"));
console.log("Reflect.ownKeys:", Reflect.ownKeys(obj2));`
    }
};

// Populate examples dropdown
function populateExamples() {
    for (const [key, example] of Object.entries(EXAMPLES)) {
        const option = document.createElement('option');
        option.value = key;
        option.textContent = example.name;
        examplesSelect.appendChild(option);
    }
}

// Initialize
runBtn.disabled = true;
populateExamples();
loadExample('hello-world');
initWasm();
