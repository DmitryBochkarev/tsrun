// Fibonacci sequence generators
// Demonstrates: infinite generators, lazy evaluation

/**
 * Infinite Fibonacci sequence generator
 */
export function* fibonacci(): Generator<number> {
  let a = 0;
  let b = 1;
  while (true) {
    yield a;
    const temp = a;
    a = b;
    b = temp + b;
  }
}

/**
 * Fibonacci sequence up to a maximum value
 */
export function* fibonacciUpto(max: number): Generator<number> {
  let a = 0;
  let b = 1;
  while (a <= max) {
    yield a;
    const temp = a;
    a = b;
    b = temp + b;
  }
}

/**
 * First n Fibonacci numbers
 */
export function* fibonacciN(n: number): Generator<number> {
  let a = 0;
  let b = 1;
  for (let i = 0; i < n; i++) {
    yield a;
    const temp = a;
    a = b;
    b = temp + b;
  }
}

/**
 * Lucas numbers (similar to Fibonacci, starts with 2, 1)
 */
export function* lucas(): Generator<number> {
  let a = 2;
  let b = 1;
  while (true) {
    yield a;
    const temp = a;
    a = b;
    b = temp + b;
  }
}

/**
 * Tribonacci sequence (sum of last 3 numbers)
 */
export function* tribonacci(): Generator<number> {
  let a = 0;
  let b = 0;
  let c = 1;
  while (true) {
    yield a;
    const next = a + b + c;
    a = b;
    b = c;
    c = next;
  }
}
