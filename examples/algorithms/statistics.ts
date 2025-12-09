// Statistical functions
// Demonstrates: Math functions, array methods, reduce

// Calculate mean (average)
export function mean(arr: number[]): number {
  if (arr.length === 0) return 0;
  return arr.reduce((sum, val) => sum + val, 0) / arr.length;
}

// Calculate median
export function median(arr: number[]): number {
  if (arr.length === 0) return 0;

  const sorted = [...arr].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);

  if (sorted.length % 2 === 0) {
    return (sorted[mid - 1] + sorted[mid]) / 2;
  }

  return sorted[mid];
}

// Calculate mode (most frequent value)
export function mode(arr: number[]): number[] {
  if (arr.length === 0) return [];

  const frequency: { [key: number]: number } = {};
  let maxFreq = 0;

  for (const val of arr) {
    frequency[val] = (frequency[val] || 0) + 1;
    if (frequency[val] > maxFreq) {
      maxFreq = frequency[val];
    }
  }

  const modes: number[] = [];
  for (const key in frequency) {
    if (frequency[key] === maxFreq) {
      modes.push(Number(key));
    }
  }

  return modes;
}

// Calculate variance
export function variance(arr: number[]): number {
  if (arr.length === 0) return 0;

  const avg = mean(arr);
  const squaredDiffs = arr.map((val) => Math.pow(val - avg, 2));
  return mean(squaredDiffs);
}

// Calculate standard deviation
export function standardDeviation(arr: number[]): number {
  return Math.sqrt(variance(arr));
}

// Calculate range
export function range(arr: number[]): number {
  if (arr.length === 0) return 0;
  return Math.max(...arr) - Math.min(...arr);
}

// Calculate sum
export function sum(arr: number[]): number {
  return arr.reduce((acc, val) => acc + val, 0);
}

// Calculate product
export function product(arr: number[]): number {
  return arr.reduce((acc, val) => acc * val, 1);
}

// Percentile calculation
export function percentile(arr: number[], p: number): number {
  if (arr.length === 0) return 0;
  if (p < 0 || p > 100) return 0;

  const sorted = [...arr].sort((a, b) => a - b);
  const index = (p / 100) * (sorted.length - 1);
  const lower = Math.floor(index);
  const upper = Math.ceil(index);

  if (lower === upper) {
    return sorted[lower];
  }

  const fraction = index - lower;
  return sorted[lower] * (1 - fraction) + sorted[upper] * fraction;
}
