// GC Debug Test
// Creates objects in batches to understand GC behavior
//
// This test helps verify that:
// 1. Objects are actually being allocated
// 2. Memory stays bounded between batches
// 3. Cycles are properly collected

console.log("=== GC Debug Test ===\n");

// Test in batches to observe memory behavior
const BATCHES: number = 10;
const OBJECTS_PER_BATCH: number = 1000;

console.log("Batches:", BATCHES);
console.log("Objects per batch:", OBJECTS_PER_BATCH);
console.log("Total objects:", BATCHES * OBJECTS_PER_BATCH);
console.log("");

for (let batch = 0; batch < BATCHES; batch++) {
    console.log("Starting batch", batch + 1);

    // Create many cyclic objects in this batch
    for (let i = 0; i < OBJECTS_PER_BATCH; i++) {
        // Create a small cycle
        const a: { id: number; ref: any } = { id: i, ref: null };
        const b: { id: number; ref: any } = { id: i + 1, ref: null };
        a.ref = b;
        b.ref = a;

        // Use them briefly
        const _ = a.id + b.id;

        // a, b go out of scope here - should be collectible
    }

    console.log("Batch", batch + 1, "complete");
}

console.log("\n=== Test Complete ===");
console.log("If GC is working, memory should be bounded.");
console.log("Run with: /usr/bin/time -v to check 'Maximum resident set size'");
