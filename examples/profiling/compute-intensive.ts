// CPU-intensive test for profiling
// Tests: recursive calls, property access, array operations

function fib(n: number): number {
    if (n <= 1) return n;
    return fib(n - 1) + fib(n - 2);
}

// Object property access stress test
function propertyAccess(): number {
    let sum = 0;
    for (let i = 0; i < 10000; i++) {
        const obj = { x: i, y: i * 2, z: i * 3 };
        sum += obj.x + obj.y + obj.z;
    }
    return sum;
}

// Array operations
function arrayOps(): number {
    const arr: number[] = [];
    for (let i = 0; i < 1000; i++) {
        arr.push(i);
    }
    return arr.reduce((a: number, b: number) => a + b, 0);
}

// Closure stress test
function closureTest(): number {
    const fns: (() => number)[] = [];
    for (let i = 0; i < 1000; i++) {
        fns.push(() => i * 2);
    }
    let sum = 0;
    for (const fn of fns) {
        sum += fn();
    }
    return sum;
}

// String operations
function stringOps(): number {
    let s = "";
    for (let i = 0; i < 500; i++) {
        s += String(i);
    }
    return s.length;
}

// Run tests
console.log("=== Compute Intensive Profile Test ===");
console.log("Fibonacci(25):", fib(25));
console.log("Property access (10k objects):", propertyAccess());
console.log("Array ops (1k elements):", arrayOps());
console.log("Closure test (1k closures):", closureTest());
console.log("String ops (500 concats):", stringOps());
console.log("=== Complete ===");
