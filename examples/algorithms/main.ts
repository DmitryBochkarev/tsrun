// Math/Algorithm Showcase - Simplified for debugging
// Demonstrates: Math object, Number methods, recursion, control flow

import { fibRecursive, fibIterative } from "./fibonacci";
import { isPrime, gcd, lcm } from "./primes";
import { mean, median, sum, variance, standardDeviation } from "./statistics";

// ═══════════════════════════════════════════════════════════════════════════
// Fibonacci Demonstrations
// ═══════════════════════════════════════════════════════════════════════════

const fibResults = {
  recursive: {
    fib10: fibRecursive(10),
    fib15: fibRecursive(15),
  },
  iterative: {
    fib10: fibIterative(10),
    fib50: fibIterative(50),
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// Prime Number Demonstrations
// ═══════════════════════════════════════════════════════════════════════════

const primeResults = {
  primeChecks: {
    "2": isPrime(2),
    "17": isPrime(17),
    "100": isPrime(100),
    "97": isPrime(97),
  },
  gcdExamples: {
    "gcd(48,18)": gcd(48, 18),
    "gcd(100,25)": gcd(100, 25),
  },
  lcmExamples: {
    "lcm(4,6)": lcm(4, 6),
    "lcm(21,6)": lcm(21, 6),
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// Statistics Demonstrations
// ═══════════════════════════════════════════════════════════════════════════

const sampleData = [12, 15, 18, 22, 22, 25, 28, 30, 35, 40];

const statsResults = {
  data: sampleData,
  sum: sum(sampleData),
  mean: mean(sampleData),
  median: median(sampleData),
  variance: Math.round(variance(sampleData) * 100) / 100,
  standardDeviation: Math.round(standardDeviation(sampleData) * 100) / 100,
};

// ═══════════════════════════════════════════════════════════════════════════
// Math Object Demonstrations
// ═══════════════════════════════════════════════════════════════════════════

const mathResults = {
  constants: {
    PI: Math.PI,
    E: Math.E,
    SQRT2: Math.SQRT2,
  },
  exponential: {
    "pow(2,10)": Math.pow(2, 10),
    "sqrt(144)": Math.sqrt(144),
    "cbrt(27)": Math.cbrt(27),
  },
  rounding: {
    "floor(3.7)": Math.floor(3.7),
    "ceil(3.2)": Math.ceil(3.2),
    "round(3.5)": Math.round(3.5),
  },
  comparison: {
    "min(5,3,9,1)": Math.min(5, 3, 9, 1),
    "max(5,3,9,1)": Math.max(5, 3, 9, 1),
    "abs(-42)": Math.abs(-42),
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// Bitwise Operations
// ═══════════════════════════════════════════════════════════════════════════

const bitwiseResults = {
  and: 12 & 10,
  or: 12 | 10,
  xor: 12 ^ 10,
  leftShift: 5 << 2,
  rightShift: 20 >> 2,
};

// ═══════════════════════════════════════════════════════════════════════════
// Output Results
// ═══════════════════════════════════════════════════════════════════════════

const results = {
  fibonacci: fibResults,
  primes: primeResults,
  statistics: statsResults,
  math: mathResults,
  bitwise: bitwiseResults,
};

JSON.stringify(results, null, 2);
