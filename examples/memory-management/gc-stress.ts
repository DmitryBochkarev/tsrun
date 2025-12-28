// GC Stress Test Suite
// Tests various scenarios to verify garbage collection is working properly
//
// Run with:
//   /usr/bin/time -v ./target/release/tsrun examples/memory-management/gc-stress.ts
//
// Key metrics to watch:
// - Maximum resident set size: should stay bounded (< 10MB typical)
// - Exit status: should be 0

console.log("=== GC Stress Test Suite ===\n");

// ============================================================================
// Test 1: Simple object allocation and deallocation
// ============================================================================
console.log("Test 1: Simple object allocation/deallocation");
{
    const ITERATIONS: number = 10000;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const obj = { value: i, data: "test_" + i };
        sum = sum + obj.value;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 2: Nested object structures
// ============================================================================
console.log("\nTest 2: Nested object structures");
{
    const ITERATIONS: number = 5000;
    let total: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const deep = {
            level1: {
                level2: {
                    level3: {
                        level4: {
                            value: i
                        }
                    }
                }
            }
        };
        total = total + deep.level1.level2.level3.level4.value;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Total:", total);
}

// ============================================================================
// Test 3: Array allocation and filling
// ============================================================================
console.log("\nTest 3: Array allocation and filling");
{
    const ITERATIONS: number = 1000;
    const ARRAY_SIZE: number = 100;
    let arraySum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const arr: number[] = [];
        for (let j = 0; j < ARRAY_SIZE; j++) {
            arr.push(j);
        }
        arraySum = arraySum + arr.length;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Array size:", ARRAY_SIZE);
    console.log("  Total elements:", arraySum);
}

// ============================================================================
// Test 4: Function closures capturing environment
// ============================================================================
console.log("\nTest 4: Function closures");
{
    const ITERATIONS: number = 5000;
    let closureSum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const capturedValue = i;
        const capturedObj = { x: i * 2 };

        const closure = (): number => {
            return capturedValue + capturedObj.x;
        };

        closureSum = closureSum + closure();
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Closure sum:", closureSum);
}

// ============================================================================
// Test 5: Map creation and population
// ============================================================================
console.log("\nTest 5: Map operations");
{
    const ITERATIONS: number = 1000;
    const MAP_SIZE: number = 50;
    let mapTotal: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const map: Map<string, number> = new Map();

        for (let j = 0; j < MAP_SIZE; j++) {
            map.set("key_" + j, j);
        }

        mapTotal = mapTotal + map.size;
        // map goes out of scope here
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Map size each:", MAP_SIZE);
    console.log("  Total entries:", mapTotal);
}

// ============================================================================
// Test 6: Set creation and population
// ============================================================================
console.log("\nTest 6: Set operations");
{
    const ITERATIONS: number = 1000;
    const SET_SIZE: number = 50;
    let setTotal: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const set: Set<number> = new Set();

        for (let j = 0; j < SET_SIZE; j++) {
            set.add(j + i * SET_SIZE);
        }

        setTotal = setTotal + set.size;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Set size each:", SET_SIZE);
    console.log("  Total elements:", setTotal);
}

// ============================================================================
// Test 7: Self-referencing objects (simple cycles)
// ============================================================================
console.log("\nTest 7: Self-referencing objects");
{
    const ITERATIONS: number = 5000;
    let selfRefSum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const obj: { value: number; self: any } = { value: i, self: null };
        obj.self = obj; // Self-reference

        selfRefSum = selfRefSum + obj.value;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", selfRefSum);
}

// ============================================================================
// Test 8: Two-object cycles (A -> B -> A)
// ============================================================================
console.log("\nTest 8: Two-object cycles");
{
    const ITERATIONS: number = 3000;
    let cycleSum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const a: { id: number; ref: any } = { id: i, ref: null };
        const b: { id: number; ref: any } = { id: i + 1, ref: null };

        a.ref = b;
        b.ref = a;

        cycleSum = cycleSum + a.id + b.id;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", cycleSum);
}

// ============================================================================
// Test 9: Longer reference chains
// ============================================================================
console.log("\nTest 9: Long reference chains");
{
    const ITERATIONS: number = 1000;
    const CHAIN_LENGTH: number = 20;
    let chainSum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        interface Node { value: number; next: Node | null; }

        let head: Node = { value: 0, next: null };
        let current: Node = head;

        for (let j = 1; j < CHAIN_LENGTH; j++) {
            const newNode: Node = { value: j, next: null };
            current.next = newNode;
            current = newNode;
        }

        // Traverse and sum
        let node: Node | null = head;
        while (node !== null) {
            chainSum = chainSum + node.value;
            node = node.next;
        }
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Chain length:", CHAIN_LENGTH);
    console.log("  Sum:", chainSum);
}

// ============================================================================
// Test 10: Circular linked lists
// ============================================================================
console.log("\nTest 10: Circular linked lists");
{
    const ITERATIONS: number = 1000;
    const LIST_SIZE: number = 10;
    let circularSum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        interface CircNode { id: number; next: CircNode | null; }

        const nodes: CircNode[] = [];
        for (let j = 0; j < LIST_SIZE; j++) {
            nodes.push({ id: j, next: null });
        }

        // Link them in a circle
        for (let j = 0; j < LIST_SIZE; j++) {
            nodes[j].next = nodes[(j + 1) % LIST_SIZE];
        }

        // Traverse the circle once
        let current: CircNode | null = nodes[0];
        for (let j = 0; j < LIST_SIZE; j++) {
            if (current) {
                circularSum = circularSum + current.id;
                current = current.next;
            }
        }
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  List size:", LIST_SIZE);
    console.log("  Sum:", circularSum);
}

// ============================================================================
// Test 11: Tree structures with parent references
// ============================================================================
console.log("\nTest 11: Trees with parent back-references");
{
    const ITERATIONS: number = 500;
    let treeSum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        interface TreeNode {
            value: number;
            parent: TreeNode | null;
            children: TreeNode[];
        }

        const root: TreeNode = { value: i, parent: null, children: [] };

        // Create children with back-references to parent
        for (let j = 0; j < 3; j++) {
            const child: TreeNode = { value: j, parent: root, children: [] };
            root.children.push(child);

            // Add grandchildren
            for (let k = 0; k < 2; k++) {
                const grandchild: TreeNode = { value: k, parent: child, children: [] };
                child.children.push(grandchild);
            }
        }

        // Sum all values
        treeSum = treeSum + root.value;
        for (const child of root.children) {
            treeSum = treeSum + child.value;
            for (const gc of child.children) {
                treeSum = treeSum + gc.value;
            }
        }
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", treeSum);
}

// ============================================================================
// Test 12: Objects with array properties
// ============================================================================
console.log("\nTest 12: Objects with array properties");
{
    const ITERATIONS: number = 2000;
    let arrPropSum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const obj = {
            id: i,
            numbers: [1, 2, 3, 4, 5],
            strings: ["a", "b", "c"],
            nested: {
                inner: [10, 20, 30]
            }
        };

        arrPropSum = arrPropSum + obj.numbers.length + obj.strings.length + obj.nested.inner.length;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", arrPropSum);
}

// ============================================================================
// Test 13: Closures with captured arrays
// ============================================================================
console.log("\nTest 13: Closures with captured arrays");
{
    const ITERATIONS: number = 2000;
    let capturedArraySum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const arr = [1, 2, 3, 4, 5];
        const obj = { value: i };

        const compute = (): number => {
            let sum = 0;
            for (const n of arr) {
                sum = sum + n + obj.value;
            }
            return sum;
        };

        capturedArraySum = capturedArraySum + compute();
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", capturedArraySum);
}

// ============================================================================
// Test 14: Mixed Map and object cycles
// ============================================================================
console.log("\nTest 14: Map with cyclic object values");
{
    const ITERATIONS: number = 500;
    let mapCycleCount: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const map: Map<string, any> = new Map();

        const objA = { id: "a", partner: null as any };
        const objB = { id: "b", partner: null as any };

        objA.partner = objB;
        objB.partner = objA;

        map.set("a", objA);
        map.set("b", objB);

        mapCycleCount = mapCycleCount + map.size;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Total entries:", mapCycleCount);
}

// ============================================================================
// Test 15: Stress test - many small allocations
// ============================================================================
console.log("\nTest 15: Many small allocations");
{
    const ITERATIONS: number = 50000;
    let smallSum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const tiny = { v: i };
        smallSum = smallSum + tiny.v;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", smallSum);
}

// ============================================================================
// Final Summary
// ============================================================================
console.log("\n=== All GC Stress Tests Complete ===");
console.log("Check 'Maximum resident set size' with /usr/bin/time -v");
console.log("Memory should stay bounded regardless of iteration counts.");
