// Memory Comparison Test
// Demonstrates the difference between:
// 1. Objects that go out of scope (memory freed)
// 2. Objects kept in an array (memory retained)
//
// Run this and compare the memory behavior:
//   /usr/bin/time -v ./target/debug/tsrun examples/memory-management/comparison.ts

const ITERATIONS: number = 1000;

console.log("=== Memory Comparison Test ===");
console.log("Iterations:", ITERATIONS);
console.log("");

// ============================================================================
// TEST A: Objects go out of scope - memory should stay stable
// ============================================================================

console.log("--- Test A: Objects Released ---");
console.log("Creating and discarding objects...");

let sumA: number = 0;

for (let i = 0; i < ITERATIONS; i++) {
    // Create an object with some data
    const obj = {
        id: i,
        data: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        name: "temp_" + i
    };

    // Use it
    sumA = sumA + obj.id + obj.data.length;

    // obj goes out of scope - should be freed immediately
    // Memory should NOT grow as we iterate
}

console.log("Test A complete. Sum:", sumA);
console.log("Memory should be stable during this test.");
console.log("");

// ============================================================================
// TEST B: Objects retained in array - memory SHOULD grow
// ============================================================================

console.log("--- Test B: Objects Retained ---");
console.log("Creating and KEEPING objects in an array...");

const retained: Array<{id: number; data: number[]; name: string}> = [];
let sumB: number = 0;

for (let i = 0; i < ITERATIONS; i++) {
    // Create an object with some data
    const obj = {
        id: i,
        data: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        name: "kept_" + i
    };

    // Keep it in the array
    retained.push(obj);

    // Use it
    sumB = sumB + obj.id + obj.data.length;

    // obj is still referenced by 'retained' - will NOT be freed
    // Memory WILL grow as we iterate
}

console.log("Test B complete. Sum:", sumB);
console.log("Retained array size:", retained.length);
console.log("Memory grew during this test (as expected).");
console.log("");

// ============================================================================
// TEST C: Closures that escape vs closures that don't
// ============================================================================

console.log("--- Test C: Closure Comparison ---");

// C1: Closures that are used and discarded
let closureSum1: number = 0;
for (let i = 0; i < ITERATIONS; i++) {
    const captured = i * 2;
    const fn = (): number => captured + 1;
    closureSum1 = closureSum1 + fn();
    // fn goes out of scope - captured environment freed
}
console.log("C1 (closures discarded) sum:", closureSum1);

// C2: Closures kept in array
const keptClosures: Array<() => number> = [];
for (let i = 0; i < 100; i++) {
    const captured = i * 2;
    keptClosures.push(() => captured + 1);
    // Closure is kept - environment stays alive
}
let closureSum2: number = 0;
for (const fn of keptClosures) {
    closureSum2 = closureSum2 + fn();
}
console.log("C2 (closures retained) sum:", closureSum2);
console.log("Retained closures:", keptClosures.length);
console.log("");

// ============================================================================
// Summary
// ============================================================================

console.log("=== Summary ===");
console.log("Test A: Objects created and freed each iteration");
console.log("  -> Memory stable, only current iteration's object in memory");
console.log("");
console.log("Test B: Objects kept in array");
console.log("  -> Memory grows, all " + retained.length + " objects in memory");
console.log("");
console.log("Test C: Closures discarded vs retained");
console.log("  -> Discarded closures free their environments");
console.log("  -> Retained closures keep environments alive");
console.log("");
console.log("This demonstrates proper garbage collection behavior:");
console.log("- Unreferenced objects are freed");
console.log("- Referenced objects stay alive");
