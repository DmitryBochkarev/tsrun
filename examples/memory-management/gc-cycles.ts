// GC Cycle Collection Test
// Specifically tests that circular references are properly collected
//
// Run with different scales to verify memory doesn't grow proportionally:
//   /usr/bin/time -v ./target/release/tsrun examples/memory-management/gc-cycles.ts
//
// Memory should stay roughly constant regardless of SCALE multiplier

const SCALE: number = 1; // Increase to 2, 5, 10 to test scalability

console.log("=== GC Cycle Collection Test ===");
console.log("Scale:", SCALE);
console.log("");

// ============================================================================
// Test 1: Simple two-node cycles
// ============================================================================
console.log("Test 1: Two-node cycles");
{
    const ITERATIONS: number = 10000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const a: { id: number; other: any } = { id: i, other: null };
        const b: { id: number; other: any } = { id: i + 1, other: null };

        // Create cycle: a <-> b
        a.other = b;
        b.other = a;

        sum = sum + a.id + b.id;

        // Both go out of scope together - should be collected
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 2: Triangle cycles (A -> B -> C -> A)
// ============================================================================
console.log("\nTest 2: Triangle cycles");
{
    const ITERATIONS: number = 5000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const a: { v: number; next: any } = { v: 1, next: null };
        const b: { v: number; next: any } = { v: 2, next: null };
        const c: { v: number; next: any } = { v: 3, next: null };

        // Create cycle: a -> b -> c -> a
        a.next = b;
        b.next = c;
        c.next = a;

        sum = sum + a.v + b.v + c.v;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 3: Ring cycles of varying sizes
// ============================================================================
console.log("\nTest 3: Ring cycles (variable size)");
{
    const ITERATIONS: number = 1000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const ringSize: number = 5 + (i % 10); // Rings of size 5-14
        interface RingNode { id: number; next: RingNode | null; }

        const nodes: RingNode[] = [];
        for (let j = 0; j < ringSize; j++) {
            nodes.push({ id: j, next: null });
        }

        // Link into a ring
        for (let j = 0; j < ringSize; j++) {
            nodes[j].next = nodes[(j + 1) % ringSize];
        }

        // Sum values
        for (let j = 0; j < ringSize; j++) {
            sum = sum + nodes[j].id;
        }
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 4: Bidirectional doubly-linked cycles
// ============================================================================
console.log("\nTest 4: Doubly-linked cycles");
{
    const ITERATIONS: number = 2000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        interface DLNode { id: number; prev: DLNode | null; next: DLNode | null; }

        const a: DLNode = { id: 1, prev: null, next: null };
        const b: DLNode = { id: 2, prev: null, next: null };
        const c: DLNode = { id: 3, prev: null, next: null };
        const d: DLNode = { id: 4, prev: null, next: null };

        // Forward links: a -> b -> c -> d -> a
        a.next = b; b.next = c; c.next = d; d.next = a;

        // Backward links: a <- b <- c <- d <- a
        b.prev = a; c.prev = b; d.prev = c; a.prev = d;

        sum = sum + a.id + b.id + c.id + d.id;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 5: Self-referencing objects
// ============================================================================
console.log("\nTest 5: Self-referencing objects");
{
    const ITERATIONS: number = 20000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const obj: { value: number; self: any } = { value: i, self: null };
        obj.self = obj;

        sum = sum + obj.value;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 6: Complex graph with multiple cycles
// ============================================================================
console.log("\nTest 6: Complex graphs with multiple cycles");
{
    const ITERATIONS: number = 1000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        interface GraphNode { id: number; edges: GraphNode[]; }

        // Create a small graph with multiple cycles
        const n1: GraphNode = { id: 1, edges: [] };
        const n2: GraphNode = { id: 2, edges: [] };
        const n3: GraphNode = { id: 3, edges: [] };
        const n4: GraphNode = { id: 4, edges: [] };
        const n5: GraphNode = { id: 5, edges: [] };

        // Create a complex cycle pattern:
        // n1 <-> n2, n2 <-> n3, n3 <-> n4, n4 <-> n5
        // n1 -> n3, n2 -> n4, n3 -> n5, n5 -> n1
        n1.edges.push(n2, n3);
        n2.edges.push(n1, n3, n4);
        n3.edges.push(n2, n4, n5);
        n4.edges.push(n3, n5);
        n5.edges.push(n4, n1);

        sum = sum + n1.id + n2.id + n3.id + n4.id + n5.id;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 7: Cycles through arrays
// ============================================================================
console.log("\nTest 7: Cycles through arrays");
{
    const ITERATIONS: number = 3000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        interface ArrayNode { value: number; refs: ArrayNode[]; }

        const a: ArrayNode = { value: 1, refs: [] };
        const b: ArrayNode = { value: 2, refs: [] };
        const c: ArrayNode = { value: 3, refs: [] };

        // Create cycles through array properties
        a.refs.push(b, c);
        b.refs.push(c, a);
        c.refs.push(a, b);

        sum = sum + a.value + b.value + c.value;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 8: Cycles through Map values
// ============================================================================
console.log("\nTest 8: Cycles through Maps");
{
    const ITERATIONS: number = 1000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const map: Map<string, any> = new Map();

        const objA = { name: "A", ref: null as any };
        const objB = { name: "B", ref: null as any };

        // Create cycle through map values
        objA.ref = objB;
        objB.ref = objA;

        map.set("a", objA);
        map.set("b", objB);

        sum = sum + map.size;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 9: Closure cycles (closure captures object that references closure)
// ============================================================================
console.log("\nTest 9: Closure cycles");
{
    const ITERATIONS: number = 5000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        const holder: { fn: (() => number) | null } = { fn: null };

        // Closure captures holder, holder.fn references closure
        const fn = (): number => {
            return i + (holder.fn ? 1 : 0);
        };

        holder.fn = fn;

        sum = sum + holder.fn();
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Test 10: Tree structures with parent back-references
// ============================================================================
console.log("\nTest 10: Trees with parent cycles");
{
    const ITERATIONS: number = 2000 * SCALE;
    let sum: number = 0;

    for (let i = 0; i < ITERATIONS; i++) {
        interface TreeNode {
            value: number;
            parent: TreeNode | null;
            left: TreeNode | null;
            right: TreeNode | null;
        }

        const root: TreeNode = { value: 1, parent: null, left: null, right: null };
        const left: TreeNode = { value: 2, parent: root, left: null, right: null };
        const right: TreeNode = { value: 3, parent: root, left: null, right: null };
        const leftLeft: TreeNode = { value: 4, parent: left, left: null, right: null };
        const leftRight: TreeNode = { value: 5, parent: left, left: null, right: null };

        root.left = left;
        root.right = right;
        left.left = leftLeft;
        left.right = leftRight;

        sum = sum + root.value + left.value + right.value + leftLeft.value + leftRight.value;
    }

    console.log("  Iterations:", ITERATIONS);
    console.log("  Sum:", sum);
}

// ============================================================================
// Summary
// ============================================================================
console.log("\n=== GC Cycle Collection Test Complete ===");
console.log("Run with SCALE=1, 2, 5, 10 to verify memory stays bounded.");
console.log("Memory usage should NOT grow proportionally with SCALE.");
