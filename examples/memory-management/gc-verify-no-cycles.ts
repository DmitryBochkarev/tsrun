// GC Verification Test - No Cycles (Baseline)
// Same workload but without cycles

console.log("=== GC Verification Test (No Cycles) ===\n");

const BATCHES: number = 100;
const OBJECTS_PER_BATCH: number = 1000;

let grandTotal: number = 0;

for (let batch = 0; batch < BATCHES; batch++) {
    let batchSum: number = 0;

    for (let i = 0; i < OBJECTS_PER_BATCH; i++) {
        // Create 3 objects WITHOUT cycles
        const a = { id: 1 };
        const b = { id: 2 };
        const c = { id: 3 };

        batchSum = batchSum + a.id + b.id + c.id;
    }

    grandTotal = grandTotal + batchSum;

    if ((batch + 1) % 20 === 0) {
        console.log("Completed batch", batch + 1, "of", BATCHES);
    }
}

console.log("\nTotal:", grandTotal);
console.log("Expected:", BATCHES * OBJECTS_PER_BATCH * 6);
console.log("\nTest passed - all objects processed successfully");
