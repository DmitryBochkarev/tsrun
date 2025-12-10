// Memory Management Demo
// Demonstrates that objects are properly collected when they go out of scope
//
// Run with valgrind to verify no memory leaks:
//   cargo build --bin typescript-eval-runner
//   valgrind --leak-check=full ./target/debug/typescript-eval-runner examples/memory-management/main.ts
//
// Run with time to observe stable memory:
//   /usr/bin/time -v ./target/debug/typescript-eval-runner examples/memory-management/main.ts

import { testScopeCleanup } from "./scope-cleanup";
import { testClosureLifetime } from "./closure-lifetime";
import { testCircularReferences } from "./circular-refs";
import { testLargeObjectChurn } from "./object-churn";

console.log("=== Memory Management Demo ===\n");

// Test 1: Objects going out of scope
console.log("--- Test 1: Scope Cleanup ---");
const scopeResult = testScopeCleanup();
console.log("Scope cleanup iterations:", scopeResult.iterations);
console.log("Final sum:", scopeResult.sum);

// Test 2: Closures and captured environments
console.log("\n--- Test 2: Closure Lifetime ---");
const closureResult = testClosureLifetime();
console.log("Closure test iterations:", closureResult.iterations);
console.log("Final value:", closureResult.value);

// Test 3: Circular references (Rc handles these via drop)
console.log("\n--- Test 3: Circular References ---");
const circularResult = testCircularReferences();
console.log("Circular ref iterations:", circularResult.iterations);
console.log("Final count:", circularResult.count);

// Test 4: Large object allocation and deallocation
console.log("\n--- Test 4: Large Object Churn ---");
const churnResult = testLargeObjectChurn();
console.log("Object churn iterations:", churnResult.iterations);
console.log("Total elements processed:", churnResult.totalElements);

console.log("\n=== All Tests Complete ===");
console.log("If running with valgrind, check for 'All heap blocks were freed'");
console.log("If running with /usr/bin/time -v, check 'Maximum resident set size'");
