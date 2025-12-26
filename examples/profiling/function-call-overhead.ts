// Benchmark for function call overhead - measuring vector allocation cost
// Tests different argument counts to see impact of Vec allocation

// Baseline: no-op functions with varying argument counts
function call0(): number { return 0; }
function call1(a: number): number { return a; }
function call2(a: number, b: number): number { return a + b; }
function call4(a: number, b: number, c: number, d: number): number { return a + b + c + d; }
function call8(a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number): number {
    return a + b + c + d + e + f + g + h;
}

// Method calls (uses Op::CallMethod)
const obj = {
    method0(): number { return 0; },
    method2(a: number, b: number): number { return a + b; },
    method4(a: number, b: number, c: number, d: number): number { return a + b + c + d; },
};

const ITERATIONS = 100000;

// Warm up JIT (for comparison with other runtimes)
for (let i = 0; i < 1000; i++) {
    call0();
    call2(1, 2);
}

console.log("=== Function Call Overhead Benchmark ===");
console.log("Iterations:", ITERATIONS);
console.log("");

// Test 0-arg calls
let start = Date.now();
let sum = 0;
for (let i = 0; i < ITERATIONS; i++) {
    sum += call0();
}
console.log("call0() x " + ITERATIONS + ": " + (Date.now() - start) + "ms (sum=" + sum + ")");

// Test 1-arg calls
start = Date.now();
sum = 0;
for (let i = 0; i < ITERATIONS; i++) {
    sum += call1(i);
}
console.log("call1(n) x " + ITERATIONS + ": " + (Date.now() - start) + "ms (sum=" + sum + ")");

// Test 2-arg calls
start = Date.now();
sum = 0;
for (let i = 0; i < ITERATIONS; i++) {
    sum += call2(i, i + 1);
}
console.log("call2(n,n) x " + ITERATIONS + ": " + (Date.now() - start) + "ms (sum=" + sum + ")");

// Test 4-arg calls
start = Date.now();
sum = 0;
for (let i = 0; i < ITERATIONS; i++) {
    sum += call4(i, i + 1, i + 2, i + 3);
}
console.log("call4(n,n,n,n) x " + ITERATIONS + ": " + (Date.now() - start) + "ms (sum=" + sum + ")");

// Test 8-arg calls
start = Date.now();
sum = 0;
for (let i = 0; i < ITERATIONS; i++) {
    sum += call8(i, i + 1, i + 2, i + 3, i + 4, i + 5, i + 6, i + 7);
}
console.log("call8(n,n,n,n,n,n,n,n) x " + ITERATIONS + ": " + (Date.now() - start) + "ms (sum=" + sum + ")");

console.log("");
console.log("--- Method Calls (Op::CallMethod) ---");

// Test method 0-arg
start = Date.now();
sum = 0;
for (let i = 0; i < ITERATIONS; i++) {
    sum += obj.method0();
}
console.log("obj.method0() x " + ITERATIONS + ": " + (Date.now() - start) + "ms (sum=" + sum + ")");

// Test method 2-arg
start = Date.now();
sum = 0;
for (let i = 0; i < ITERATIONS; i++) {
    sum += obj.method2(i, i + 1);
}
console.log("obj.method2(n,n) x " + ITERATIONS + ": " + (Date.now() - start) + "ms (sum=" + sum + ")");

// Test method 4-arg
start = Date.now();
sum = 0;
for (let i = 0; i < ITERATIONS; i++) {
    sum += obj.method4(i, i + 1, i + 2, i + 3);
}
console.log("obj.method4(n,n,n,n) x " + ITERATIONS + ": " + (Date.now() - start) + "ms (sum=" + sum + ")");

console.log("");
console.log("--- Recursive Calls (stress test) ---");

// Fibonacci - heavy recursive function calls
function fib(n: number): number {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

start = Date.now();
const fibResult = fib(25);
console.log("fib(25) = " + fibResult + ": " + (Date.now() - start) + "ms");

// Ackermann - extremely call-heavy
function ack(m: number, n: number): number {
    if (m === 0) return n + 1;
    if (n === 0) return ack(m - 1, 1);
    return ack(m - 1, ack(m, n - 1));
}

start = Date.now();
const ackResult = ack(3, 4);  // 3,6 exceeds stack limit
console.log("ack(3,4) = " + ackResult + ": " + (Date.now() - start) + "ms");

console.log("");
console.log("=== Benchmark Complete ===");
