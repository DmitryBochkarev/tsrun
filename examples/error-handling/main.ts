// Error Handling Showcase
// Demonstrates try/catch/finally and custom errors

import {
    ValidationError,
    validateRequired,
    validateString,
    validateNumber,
    validateMinLength,
    validateRange,
    validateEmail
} from "./validators";

import {
    Result,
    ok,
    err,
    tryFn,
    safeDivide,
    safeGet,
    safeProperty,
    safeJsonParse,
    safeParseInt,
    retryOperation,
    unwrap,
    unwrapOr,
    mapResult,
    chainResult
} from "./safe-ops";

console.log("=== Error Handling Showcase ===\n");

// --- Basic try/catch ---
console.log("--- Basic try/catch ---");

function riskyOperation(shouldFail: boolean): number {
    if (shouldFail) {
        throw new Error("Operation failed!");
    }
    return 42;
}

try {
    const result: number = riskyOperation(false);
    console.log("Success:", result);
} catch (e) {
    console.log("Caught error:", e);
}

try {
    const result: number = riskyOperation(true);
    console.log("Success:", result);
} catch (e) {
    if (e instanceof Error) {
        console.log("Caught Error:", e.message);
    }
}

// --- try/catch/finally ---
console.log("\n--- try/catch/finally ---");

function processWithCleanup(value: number): string {
    console.log("  Starting process...");
    try {
        console.log("  Processing value:", value);
        if (value < 0) {
            throw new RangeError("Value cannot be negative");
        }
        return "Processed: " + value;
    } catch (e) {
        if (e instanceof Error) {
            console.log("  Error caught:", e.message);
        }
        return "Error occurred";
    } finally {
        console.log("  Cleanup complete");
    }
}

console.log("Result 1:", processWithCleanup(10));
console.log("Result 2:", processWithCleanup(-5));

// --- Different Error Types ---
console.log("\n--- Different Error Types ---");

function throwErrorType(type: string): void {
    switch (type) {
        case "Error":
            throw new Error("Generic error");
        case "TypeError":
            throw new TypeError("Type mismatch");
        case "RangeError":
            throw new RangeError("Value out of range");
        case "ReferenceError":
            throw new ReferenceError("Variable not found");
        case "SyntaxError":
            throw new SyntaxError("Invalid syntax");
        default:
            throw new Error("Unknown error type");
    }
}

const errorTypes: string[] = ["Error", "TypeError", "RangeError", "ReferenceError", "SyntaxError"];

for (const type of errorTypes) {
    try {
        throwErrorType(type);
    } catch (e) {
        if (e instanceof Error) {
            console.log("Caught " + e.name + ": " + e.message);
        }
    }
}

// --- Custom ValidationError ---
console.log("\n--- Custom ValidationError ---");

interface User {
    name: string;
    age: number;
    email: string;
}

function validateUser(user: User): void {
    validateRequired(user.name, "name");
    validateString(user.name, "name");
    validateMinLength(user.name, 2, "name");

    validateRequired(user.age, "age");
    validateNumber(user.age, "age");
    validateRange(user.age, 0, 150, "age");

    validateRequired(user.email, "email");
    validateEmail(user.email);
}

const validUser: User = { name: "Alice", age: 30, email: "alice@example.com" };
const invalidUsers: User[] = [
    { name: "", age: 30, email: "alice@example.com" },
    { name: "Bob", age: -5, email: "bob@example.com" },
    { name: "Charlie", age: 25, email: "invalid-email" }
];

console.log("\nValidating valid user:");
try {
    validateUser(validUser);
    console.log("  User is valid!");
} catch (e) {
    if (e instanceof Error) {
        console.log("  Validation failed:", e.message);
    }
}

console.log("\nValidating invalid users:");
for (const user of invalidUsers) {
    try {
        validateUser(user);
        console.log("  " + user.name + " is valid");
    } catch (e) {
        if (e instanceof ValidationError) {
            console.log("  ValidationError [" + e.field + "]: " + e.message);
        } else if (e instanceof Error) {
            console.log("  " + e.name + ": " + e.message);
        }
    }
}

// --- Manual Batch Validation ---
console.log("\n--- Batch Validation ---");

const testUser = {
    name: "A",
    age: 200,
    email: "bad"
};

// Perform validation manually - demonstrating error collection
function collectErrors(): string[] {
    const result: string[] = [];

    // Name validation
    if (testUser.name.length < 2) {
        result.push("name must be at least 2 characters");
    }

    // Age validation
    if (testUser.age < 0 || testUser.age > 150) {
        result.push("age must be between 0 and 150");
    }

    // Email validation
    if (!testUser.email.includes("@") || !testUser.email.includes(".")) {
        result.push("Invalid email format");
    }

    return result;
}

const batchErrors: string[] = collectErrors();
console.log("Valid:", batchErrors.length === 0);
console.log("Errors:");
for (const error of batchErrors) {
    console.log("  - " + error);
}

// --- Result Type Pattern ---
console.log("\n--- Result Type Pattern ---");

console.log("\nSafe division:");
console.log("  10 / 2 =", safeDivide(10, 2));
console.log("  10 / 0 =", safeDivide(10, 0));

console.log("\nSafe array access:");
const arr: number[] = [1, 2, 3];
console.log("  arr[1] =", safeGet(arr, 1));
console.log("  arr[10] =", safeGet(arr, 10));

console.log("\nSafe property access:");
const obj = { foo: "bar" };
console.log("  obj.foo =", safeProperty(obj, "foo"));
console.log("  obj.baz =", safeProperty(obj, "baz"));

console.log("\nSafe JSON parse:");
console.log("  Valid JSON:", safeJsonParse('{"a": 1}'));
console.log("  Invalid JSON:", safeJsonParse('not json'));

console.log("\nSafe parseInt:");
console.log("  '42' =", safeParseInt("42"));
console.log("  'abc' =", safeParseInt("abc"));

// --- Result Combinators ---
console.log("\n--- Result Combinators ---");

const numResult = safeParseInt("42");
console.log("Original:", numResult);

const doubled = mapResult(numResult, (n) => n * 2);
console.log("Doubled:", doubled);

const chained = chainResult(numResult, (n) => safeDivide(n, 2));
console.log("Chained (divided by 2):", chained);

console.log("\nUnwrap with default:");
console.log("  unwrapOr(ok(10), 0) =", unwrapOr(ok(10), 0));
console.log("  unwrapOr(err(new Error()), 0) =", unwrapOr(err(new Error("oops")), 0));

// --- Retry Pattern ---
console.log("\n--- Retry Pattern ---");

let attemptCount: number = 0;

function flakyOperation(): string {
    attemptCount++;
    if (attemptCount < 3) {
        throw new Error("Temporary failure (attempt " + attemptCount + ")");
    }
    return "Success on attempt " + attemptCount;
}

attemptCount = 0;
const retryResult = retryOperation(flakyOperation, 5);
console.log("Retry result:", retryResult);

attemptCount = 0;
const failedRetry = retryOperation(() => {
    throw new Error("Always fails");
}, 3);
console.log("Failed retry:", failedRetry);

// --- Error Propagation ---
console.log("\n--- Error Propagation ---");

function level1(): number {
    return level2();
}

function level2(): number {
    return level3();
}

function level3(): number {
    throw new Error("Error from level 3");
}

try {
    level1();
} catch (e) {
    if (e instanceof Error) {
        console.log("Caught at top level:", e.message);
    }
}

// --- Nested try/catch ---
console.log("\n--- Nested try/catch ---");

function outerFunction(): string {
    try {
        try {
            throw new Error("Inner error");
        } catch (e) {
            if (e instanceof Error) {
                console.log("  Inner catch:", e.message);
            }
            throw new Error("Re-thrown as new error");
        }
    } catch (e) {
        if (e instanceof Error) {
            console.log("  Outer catch:", e.message);
        }
        return "Handled in outer";
    }
}

console.log("Result:", outerFunction());

// --- Error in finally ---
console.log("\n--- Return in finally ---");

function finallyReturn(): string {
    try {
        return "from try";
    } finally {
        // Note: In real JS, return in finally would override try's return
        // Our interpreter may handle this differently
        console.log("  finally block executed");
    }
}

console.log("Result:", finallyReturn());

// --- Conditional Error Handling ---
console.log("\n--- Conditional Error Handling ---");

function handleSpecificError(errorType: string): string {
    try {
        throwErrorType(errorType);
        return "No error";
    } catch (e) {
        if (e instanceof TypeError) {
            return "Handled TypeError specially";
        } else if (e instanceof RangeError) {
            return "Handled RangeError specially";
        } else if (e instanceof Error) {
            return "Handled generic Error: " + e.message;
        }
        return "Unknown error";
    }
}

console.log("TypeError:", handleSpecificError("TypeError"));
console.log("RangeError:", handleSpecificError("RangeError"));
console.log("Error:", handleSpecificError("Error"));

console.log("\n=== Demo Complete ===");
