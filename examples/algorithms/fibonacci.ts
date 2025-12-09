// Fibonacci implementations
// Demonstrates: recursion, memoization, iterative algorithms

// Recursive Fibonacci (simple but slow for large n)
export function fibRecursive(n: number): number {
  if (n <= 1) return n;
  return fibRecursive(n - 1) + fibRecursive(n - 2);
}

// Iterative Fibonacci (efficient)
export function fibIterative(n: number): number {
  if (n <= 1) return n;

  let prev = 0;
  let curr = 1;

  for (let i = 2; i <= n; i++) {
    const next = prev + curr;
    prev = curr;
    curr = next;
  }

  return curr;
}

// Memoized Fibonacci using closure
export function createMemoizedFib(): (n: number) => number {
  const cache: { [key: number]: number } = {};

  return function fib(n: number): number {
    if (n in cache) return cache[n];
    if (n <= 1) return n;

    const result = fib(n - 1) + fib(n - 2);
    cache[n] = result;
    return result;
  };
}

// Generate first n Fibonacci numbers
export function fibSequence(count: number): number[] {
  const result: number[] = [];
  let prev = 0;
  let curr = 1;

  for (let i = 0; i < count; i++) {
    result.push(prev);
    const next = prev + curr;
    prev = curr;
    curr = next;
  }

  return result;
}
