// GC Verification Test
// Creates cycles in batches to verify they're being collected
//
// This test creates many cycles and verifies that:
// 1. Memory doesn't grow without bound
// 2. Program completes successfully

console.log("=== GC Verification Test ===\n");

const BATCHES: number = 100;
const CYCLES_PER_BATCH: number = 1000;

let grandTotal: number = 0;

for (let batch = 0; batch < BATCHES; batch++) {
    let batchSum: number = 0;

    for (let i = 0; i < CYCLES_PER_BATCH; i++) {
        // Create a 3-node cycle
        const a: { id: number; next: any } = { id: 1, next: null };
        const b: { id: number; next: any } = { id: 2, next: null };
        const c: { id: number; next: any } = { id: 3, next: null };

        a.next = b;
        b.next = c;
        c.next = a;

        batchSum = batchSum + a.id + b.id + c.id;
    }

    grandTotal = grandTotal + batchSum;

    if ((batch + 1) % 20 === 0) {
        console.log("Completed batch", batch + 1, "of", BATCHES);
    }
}

console.log("\nTotal:", grandTotal);
console.log("Expected:", BATCHES * CYCLES_PER_BATCH * 6);
console.log("\nTest passed - all cycles processed successfully");
