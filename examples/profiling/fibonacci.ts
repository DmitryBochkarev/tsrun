// Fibonacci benchmark for function call overhead testing
function fib(n: number): number {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

const start = Date.now();
const result = fib(30);
const elapsed = Date.now() - start;

console.log("Fibonacci(30):", result);
console.log("Time:", elapsed, "ms");
