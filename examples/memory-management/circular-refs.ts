// Circular Reference Test
// Tests that circular references don't prevent memory cleanup
//
// Note: Rust's Rc uses reference counting which can leak circular references.
// However, when the entire cycle goes out of scope together, it should be freed.
// This test verifies that circular structures created and dropped together
// are properly cleaned up.

export interface CircularResult {
    iterations: number;
    count: number;
}

interface Node {
    id: number;
    next: Node | null;
    prev: Node | null;
}

// Test doubly-linked list style circular references
export function testCircularReferences(): CircularResult {
    let count: number = 0;
    const iterations: number = 1000;

    for (let i = 0; i < iterations; i++) {
        // Create a circular structure
        const a: Node = { id: 1, next: null, prev: null };
        const b: Node = { id: 2, next: null, prev: null };
        const c: Node = { id: 3, next: null, prev: null };

        // Link them in a circle: a -> b -> c -> a
        a.next = b;
        b.next = c;
        c.next = a;

        // And backwards: a <- b <- c <- a
        a.prev = c;
        b.prev = a;
        c.prev = b;

        // Traverse the circle
        let current: Node | null = a;
        for (let j = 0; j < 6; j++) {
            if (current) {
                count = count + current.id;
                current = current.next;
            }
        }

        // a, b, c all go out of scope together
        // The entire circular structure should be collected
    }

    return { iterations, count };
}

// Test self-referencing objects
export function testSelfReference(): number {
    let sum: number = 0;

    for (let i = 0; i < 1000; i++) {
        const obj: { value: number; self: any } = {
            value: i,
            self: null
        };
        obj.self = obj; // Self-reference

        sum = sum + obj.value;

        // obj goes out of scope - should be collected despite self-reference
    }

    return sum;
}

// Test parent-child circular references
interface TreeNode {
    value: number;
    parent: TreeNode | null;
    children: TreeNode[];
}

export function testParentChildCircular(): number {
    let total: number = 0;

    for (let i = 0; i < 500; i++) {
        // Create a tree with parent back-references
        const root: TreeNode = { value: i, parent: null, children: [] };

        const child1: TreeNode = { value: i + 1, parent: root, children: [] };
        const child2: TreeNode = { value: i + 2, parent: root, children: [] };

        root.children.push(child1);
        root.children.push(child2);

        const grandchild: TreeNode = { value: i + 3, parent: child1, children: [] };
        child1.children.push(grandchild);

        // Sum all values
        total = total + root.value + child1.value + child2.value + grandchild.value;

        // Entire tree goes out of scope
    }

    return total;
}

// Test Map with circular values
export function testMapCircular(): number {
    let count: number = 0;

    for (let i = 0; i < 500; i++) {
        const map: Map<string, any> = new Map();

        const objA = { id: "a", ref: null as any };
        const objB = { id: "b", ref: null as any };

        objA.ref = objB;
        objB.ref = objA;

        map.set("a", objA);
        map.set("b", objB);

        count = count + map.size;

        // map and both objects go out of scope
    }

    return count;
}
