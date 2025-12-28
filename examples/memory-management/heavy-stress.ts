// Heavy Stress Test for Profiling
// This creates a much larger workload suitable for perf profiling
//
// Run with:
//   perf record -g ./target/profiling/tsrun examples/memory-management/heavy-stress.ts
//   perf report --stdio --sort=symbol --no-children | head -50

const ROUNDS: number = 100;
const OBJECTS_PER_ROUND: number = 500;
const ARRAY_SIZE: number = 100;

let grandTotal: number = 0;

for (let round = 0; round < ROUNDS; round++) {
    let roundSum: number = 0;

    for (let i = 0; i < OBJECTS_PER_ROUND; i++) {
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

        for (let j = 0; j < ARRAY_SIZE; j++) {
            obj.data.values.push(j * i);
        }

        const compute = (): number => {
            return obj.id + obj.data.values.length;
        };

        roundSum = roundSum + compute();
    }

    grandTotal = grandTotal + roundSum;
}

console.log("Grand total:", grandTotal);
