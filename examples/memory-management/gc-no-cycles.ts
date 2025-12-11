// GC Test - No Cycles (Baseline)
// Tests memory with same object count but NO cycles
// Compare with gc-scale-test.ts to see cycle overhead

const TOTAL_OBJECTS: number = 50000;

console.log("=== GC No-Cycles Baseline ===");
console.log("Total objects to create:", TOTAL_OBJECTS);
console.log("");

let totalSum: number = 0;

for (let i = 0; i < TOTAL_OBJECTS; i++) {
    // Create 3 objects WITHOUT cycles
    const a = { id: i * 3 };
    const b = { id: i * 3 + 1 };
    const c = { id: i * 3 + 2 };

    totalSum = totalSum + a.id + b.id + c.id;
}

console.log("Total sum:", totalSum);
console.log("Test complete.");
