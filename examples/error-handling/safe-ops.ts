// Safe operation wrappers that handle errors gracefully

export interface Result<T, E = Error> {
    success: boolean;
    value?: T;
    error?: E;
}

export function ok<T>(value: T): Result<T> {
    return { success: true, value: value };
}

export function err<E = Error>(error: E): Result<never, E> {
    return { success: false, error: error };
}

// Try wrapper - converts exceptions to Result
export function tryFn<T>(fn: () => T): Result<T> {
    try {
        return ok(fn());
    } catch (e) {
        if (e instanceof Error) {
            return err(e);
        }
        return err(new Error(String(e)));
    }
}

// Safe division
export function safeDivide(a: number, b: number): Result<number> {
    if (b === 0) {
        return err(new Error("Division by zero"));
    }
    return ok(a / b);
}

// Safe array access
export function safeGet<T>(arr: T[], index: number): Result<T> {
    if (index < 0 || index >= arr.length) {
        return err(new RangeError("Index " + index + " out of bounds (0-" + (arr.length - 1) + ")"));
    }
    return ok(arr[index]);
}

// Safe property access
export function safeProperty<T>(obj: any, key: string): Result<T> {
    if (obj === null || obj === undefined) {
        return err(new TypeError("Cannot read property '" + key + "' of " + obj));
    }
    if (!(key in obj)) {
        return err(new ReferenceError("Property '" + key + "' does not exist"));
    }
    return ok(obj[key]);
}

// Safe JSON parse
export function safeJsonParse(json: string): Result<any> {
    try {
        return ok(JSON.parse(json));
    } catch (e) {
        if (e instanceof Error) {
            return err(new SyntaxError("Invalid JSON: " + e.message));
        }
        return err(new SyntaxError("Invalid JSON"));
    }
}

// Safe parseInt
export function safeParseInt(str: string, radix: number = 10): Result<number> {
    const result = parseInt(str, radix);
    if (Number.isNaN(result)) {
        return err(new Error("Cannot parse '" + str + "' as integer"));
    }
    return ok(result);
}

// Safe parseFloat
export function safeParseFloat(str: string): Result<number> {
    const result = parseFloat(str);
    if (Number.isNaN(result)) {
        return err(new Error("Cannot parse '" + str + "' as float"));
    }
    return ok(result);
}

// Retry with exponential backoff simulation
// Note: Simplified to avoid cross-module closure issues
export function retryOperation<T>(
    operation: () => T,
    maxAttempts: number
): Result<T> {
    for (let attempt: number = 1; attempt <= maxAttempts; attempt++) {
        const result = tryFn(operation);
        if (result.success) {
            return result;
        }
        // Continue to next attempt on failure
    }

    return err(new Error("Operation failed after " + maxAttempts + " attempts"));
}

// Unwrap Result or throw
export function unwrap<T>(result: Result<T>): T {
    if (result.success && result.value !== undefined) {
        return result.value;
    }
    throw result.error || new Error("Result has no value");
}

// Unwrap with default
export function unwrapOr<T>(result: Result<T>, defaultValue: T): T {
    if (result.success && result.value !== undefined) {
        return result.value;
    }
    return defaultValue;
}

// Map over Result
export function mapResult<T, U>(result: Result<T>, fn: (value: T) => U): Result<U> {
    if (result.success && result.value !== undefined) {
        return tryFn(() => fn(result.value as T));
    }
    return err(result.error || new Error("Result has no value"));
}

// Chain Results (flatMap)
export function chainResult<T, U>(result: Result<T>, fn: (value: T) => Result<U>): Result<U> {
    if (result.success && result.value !== undefined) {
        return fn(result.value);
    }
    return err(result.error || new Error("Result has no value"));
}
