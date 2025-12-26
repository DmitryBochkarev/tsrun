// Benchmark: recursive fib(25) - 242,785 function calls
function fib(n: number): number {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}
const r = fib(25);
