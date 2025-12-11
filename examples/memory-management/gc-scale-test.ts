// GC Scale Test
// Tests memory behavior at different scales to verify GC is collecting cycles
//
// Run and compare memory usage:
//   /usr/bin/time -v ./target/release/typescript-eval-runner examples/memory-management/gc-scale-test.ts
//
// Key question: Does memory grow proportionally with TOTAL_CYCLES?
// If GC works: Memory stays ~constant regardless of TOTAL_CYCLES
// If GC broken: Memory grows linearly with TOTAL_CYCLES

// Adjust this to test different scales
const TOTAL_CYCLES: number = 50000;

console.log("=== GC Scale Test ===");
console.log("Total cycles to create:", TOTAL_CYCLES);
console.log("");

let totalSum: number = 0;

// Create many cycles
for (let i = 0; i < TOTAL_CYCLES; i++) {
    // Create a 3-node cycle
    const a: { id: number; next: any } = { id: i * 3, next: null };
    const b: { id: number; next: any } = { id: i * 3 + 1, next: null };
    const c: { id: number; next: any } = { id: i * 3 + 2, next: null };

    // Link: a -> b -> c -> a
    a.next = b;
    b.next = c;
    c.next = a;

    // Use values
    totalSum = totalSum + a.id + b.id + c.id;

    // All three go out of scope here
}

console.log("Total sum:", totalSum);
console.log("Test complete.");
console.log("");
console.log("If GC is working properly:");
console.log("- Memory should NOT grow proportionally with TOTAL_CYCLES");
console.log("- Running with 50000 vs 100000 cycles should use similar peak memory");
