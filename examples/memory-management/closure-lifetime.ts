// Closure Lifetime Test
// Closures keep their captured environment alive until they are no longer referenced

export interface ClosureResult {
    iterations: number;
    value: number;
}

export function testClosureLifetime(): ClosureResult {
    let value: number = 0;
    const iterations: number = 5000;

    for (let i = 0; i < iterations; i++) {
        // Create a closure that captures 'i'
        const capturedValue = i * 2;
        const fn = (): number => capturedValue + 1;

        // Use the closure
        value = value + fn();

        // fn goes out of scope here
        // The captured environment should be freed since no references remain
    }

    return { iterations, value };
}

// Test that closures properly extend lifetime when needed
export function testClosureExtension(): number {
    const closures: Array<() => number> = [];

    // Create closures in a loop
    for (let i = 0; i < 100; i++) {
        const captured = i;
        closures.push(() => captured * 2);
    }

    // All closures still exist, their captured environments should be alive
    let sum: number = 0;
    for (const fn of closures) {
        sum = sum + fn();
    }

    // closures array goes out of scope after return
    // All captured environments should then be freed
    return sum;
}

// Test nested closures
export function testNestedClosures(): number {
    let result: number = 0;

    for (let i = 0; i < 1000; i++) {
        const outer = i;

        const outerFn = (): (() => number) => {
            const inner = outer + 1;
            return (): number => inner * 2;
        };

        const innerFn = outerFn();
        result = result + innerFn();

        // innerFn, outerFn go out of scope
        // Captured environments for both should be freed
    }

    return result;
}

// Test closure over loop variable with let (each iteration gets its own binding)
export function testLoopClosures(): number {
    const fns: Array<() => number> = [];

    for (let i = 0; i < 10; i++) {
        // With 'let', each iteration has its own 'i'
        fns.push(() => i);
    }

    let sum: number = 0;
    for (const fn of fns) {
        sum = sum + fn();
    }

    // Should be 0+1+2+...+9 = 45
    return sum;
}

// Test that reassigning a closure variable frees the old closure
export function testClosureReassignment(): number {
    let fn: () => number = () => 0;
    let sum: number = 0;

    for (let i = 0; i < 1000; i++) {
        const captured = i;
        // Old closure should be freed when we reassign
        fn = () => captured;
        sum = sum + fn();
    }

    return sum;
}
