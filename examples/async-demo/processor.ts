// Async data processing utilities
// Demonstrates: async transformation, error handling, Promise chaining

// ============================================================================
// Types
// ============================================================================

interface ProcessedData<T> {
  success: boolean;
  data?: T;
  error?: string;
}

interface Statistics {
  count: number;
  items: string[];
}

// ============================================================================
// Async Processing Functions
// ============================================================================

/**
 * Transform data with error handling
 */
export async function safeProcess<T, R>(
  input: T,
  processor: (data: T) => R
): Promise<ProcessedData<R>> {
  try {
    const result = processor(input);
    return { success: true, data: result };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

/**
 * Map over array asynchronously
 */
export async function asyncMap<T, R>(
  items: T[],
  mapper: (item: T) => Promise<R>
): Promise<R[]> {
  const promises = items.map(mapper);
  return Promise.all(promises);
}

/**
 * Filter array asynchronously
 */
export async function asyncFilter<T>(
  items: T[],
  predicate: (item: T) => Promise<boolean>
): Promise<T[]> {
  const results: T[] = [];
  for (const item of items) {
    if (await predicate(item)) {
      results.push(item);
    }
  }
  return results;
}

/**
 * Aggregate results from multiple async sources
 */
export async function aggregateResults<T>(
  sources: Promise<T[]>[]
): Promise<T[]> {
  const allArrays = await Promise.all(sources);
  return allArrays.flat();
}

/**
 * Calculate statistics from async data
 */
export async function calculateStats<T>(
  fetchData: () => Promise<T[]>,
  getName: (item: T) => string
): Promise<Statistics> {
  const data = await fetchData();
  return {
    count: data.length,
    items: data.map(getName),
  };
}

/**
 * Chain multiple async operations
 */
export async function pipeline<A, B, C>(
  initial: Promise<A>,
  step1: (a: A) => Promise<B>,
  step2: (b: B) => Promise<C>
): Promise<C> {
  const a = await initial;
  const b = await step1(a);
  return step2(b);
}

/**
 * Retry an async operation with attempts
 */
export async function retry<T>(
  operation: () => Promise<T>,
  maxAttempts: number
): Promise<T | null> {
  let attempts = 0;
  while (attempts < maxAttempts) {
    try {
      return await operation();
    } catch (e) {
      attempts++;
      if (attempts >= maxAttempts) {
        return null;
      }
    }
  }
  return null;
}

/**
 * Process all items and collect results with error handling
 */
export async function processAllSettled<T, R>(
  items: T[],
  processor: (item: T) => Promise<R>
): Promise<{ fulfilled: R[]; rejected: string[] }> {
  const results = await Promise.allSettled(items.map(processor));

  const fulfilled: R[] = [];
  const rejected: string[] = [];

  for (const result of results) {
    if (result.status === "fulfilled") {
      fulfilled.push(result.value);
    } else {
      rejected.push(String(result.reason));
    }
  }

  return { fulfilled, rejected };
}
