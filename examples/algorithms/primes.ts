// Prime number algorithms
// Demonstrates: loops, conditionals, Math functions, bitwise operations

// Check if a number is prime
export function isPrime(n: number): boolean {
  if (n < 2) return false;
  if (n === 2) return true;
  if (n % 2 === 0) return false;

  const sqrt = Math.floor(Math.sqrt(n));
  for (let i = 3; i <= sqrt; i += 2) {
    if (n % i === 0) return false;
  }

  return true;
}

// Sieve of Eratosthenes - find all primes up to n
export function sieveOfEratosthenes(n: number): number[] {
  if (n < 2) return [];

  // Use array of booleans
  const sieve: boolean[] = [];
  for (let i = 0; i <= n; i++) {
    sieve.push(true);
  }
  sieve[0] = false;
  sieve[1] = false;

  for (let i = 2; i * i <= n; i++) {
    if (sieve[i]) {
      for (let j = i * i; j <= n; j += i) {
        sieve[j] = false;
      }
    }
  }

  const primes: number[] = [];
  for (let i = 2; i <= n; i++) {
    if (sieve[i]) {
      primes.push(i);
    }
  }

  return primes;
}

// Get prime factors of a number
export function primeFactors(n: number): number[] {
  const factors: number[] = [];
  let num = n;

  // Handle 2 separately
  while (num % 2 === 0) {
    factors.push(2);
    num = num / 2;
  }

  // Check odd numbers
  for (let i = 3; i <= Math.sqrt(num); i += 2) {
    while (num % i === 0) {
      factors.push(i);
      num = num / i;
    }
  }

  // If num is a prime greater than 2
  if (num > 2) {
    factors.push(num);
  }

  return factors;
}

// Greatest Common Divisor using Euclidean algorithm
export function gcd(a: number, b: number): number {
  a = Math.abs(a);
  b = Math.abs(b);
  while (b !== 0) {
    const temp = b;
    b = a % b;
    a = temp;
  }
  return a;
}

// Least Common Multiple
export function lcm(a: number, b: number): number {
  return Math.abs(a * b) / gcd(a, b);
}
