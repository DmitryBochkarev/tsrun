// Memory Stress Test
// This test creates and discards many objects to verify memory doesn't grow unbounded
//
// Run multiple times and compare memory usage:
//   /usr/bin/time -v ./target/debug/tsrun examples/memory-management/stress-test.ts
//
// If objects are being collected properly:
// - Memory usage should remain stable regardless of iteration count
// - Increasing ROUNDS should NOT proportionally increase peak memory

const ROUNDS: number = 10;
const OBJECTS_PER_ROUND: number = 200;
const ARRAY_SIZE: number = 50;

console.log("=== Memory Stress Test ===");
console.log("Rounds:", ROUNDS);
console.log("Objects per round:", OBJECTS_PER_ROUND);
console.log("Array size:", ARRAY_SIZE);
console.log("");

let grandTotal: number = 0;

for (let round = 0; round < ROUNDS; round++) {
    let roundSum: number = 0;

    // Create many objects this round
    for (let i = 0; i < OBJECTS_PER_ROUND; i++) {
        // Object with nested structure
        const obj = {
            id: round * OBJECTS_PER_ROUND + i,
            name: "object_" + i,
            data: {
                values: [] as number[],
                metadata: {
                    created: round,
                    index: i
                }
            }
        };

        // Fill the array
        for (let j = 0; j < ARRAY_SIZE; j++) {
            obj.data.values.push(j * i);
        }

        // Create a closure that captures local state
        const compute = (): number => {
            return obj.id + obj.data.values.length;
        };

        roundSum = roundSum + compute();

        // obj and compute go out of scope here
        // All memory should be freed
    }

    grandTotal = grandTotal + roundSum;
    console.log("Round " + (round + 1) + " complete, sum: " + roundSum);
}

console.log("");
console.log("Grand total:", grandTotal);
console.log("Test complete - check memory usage with /usr/bin/time -v");
