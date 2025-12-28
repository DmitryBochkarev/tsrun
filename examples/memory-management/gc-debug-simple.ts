// Simple test to verify GC is working
// Run with: cargo run --release --bin tsrun -- examples/memory-management/gc-debug-simple.ts

console.log("Starting GC debug test...");

for (let i = 0; i < 5; i++) {
    const a = { id: i, data: "test" };
    const b = { id: i + 1, data: "test" };

    // Create cycle
    (a as any).other = b;
    (b as any).other = a;

    console.log("Iteration", i, "created objects");
}

console.log("Loop complete");
