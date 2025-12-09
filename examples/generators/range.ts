// Range generator - creates sequences of numbers
// Demonstrates: function*, yield, parameters

/**
 * Generate numbers from start to end (exclusive)
 */
export function* range(start: number, end: number, step: number = 1): Generator<number> {
  for (let i = start; i < end; i += step) {
    yield i;
  }
}

/**
 * Generate numbers from start to end (inclusive)
 */
export function* rangeInclusive(start: number, end: number): Generator<number> {
  for (let i = start; i <= end; i++) {
    yield i;
  }
}

/**
 * Generate a countdown from n to 1
 */
export function* countdown(n: number): Generator<number> {
  while (n > 0) {
    yield n;
    n--;
  }
}

/**
 * Take first n items from a generator
 */
export function* take<T>(gen: Generator<T>, n: number): Generator<T> {
  let count = 0;
  for (const value of gen) {
    if (count >= n) break;
    yield value;
    count++;
  }
}

/**
 * Skip first n items from a generator
 */
export function* skip<T>(gen: Generator<T>, n: number): Generator<T> {
  let count = 0;
  for (const value of gen) {
    if (count >= n) {
      yield value;
    }
    count++;
  }
}
