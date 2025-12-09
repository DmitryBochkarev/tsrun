// Map/Set Collections Demo
// Demonstrates Map and Set data structures

import { Graph, buildGraph, bfs, dfs } from "./graph";
import { WordCounter, analyzeText } from "./counter";

console.log("=== Map/Set Collections Demo ===\n");

// --- Basic Map Operations ---
console.log("--- Basic Map Operations ---");

const map: Map<string, number> = new Map();
map.set("one", 1);
map.set("two", 2);
map.set("three", 3);

console.log("Map size:", map.size);
console.log("get('two'):", map.get("two"));
console.log("has('three'):", map.has("three"));
console.log("has('four'):", map.has("four"));

// Map initialization with array of entries
const fruitPrices: Map<string, number> = new Map([
    ["apple", 1.5],
    ["banana", 0.75],
    ["orange", 2.0],
    ["grape", 3.25]
]);
console.log("\nFruit prices map:");
fruitPrices.forEach((price, fruit) => {
    console.log("  " + fruit + ": $" + price);
});

// --- Map Iteration ---
console.log("\n--- Map Iteration ---");

console.log("\nMap entries:");
for (const [key, value] of fruitPrices.entries()) {
    console.log("  " + key + " => " + value);
}

console.log("\nMap keys:", JSON.stringify(Array.from(fruitPrices.keys())));
console.log("Map values:", JSON.stringify(Array.from(fruitPrices.values())));

// --- Map Operations ---
console.log("\n--- Map Operations ---");

fruitPrices.set("mango", 2.5);
console.log("After adding mango:", fruitPrices.size);

fruitPrices.delete("banana");
console.log("After deleting banana:", fruitPrices.size);
console.log("has('banana'):", fruitPrices.has("banana"));

// --- Basic Set Operations ---
console.log("\n--- Basic Set Operations ---");

const set: Set<number> = new Set();
set.add(1);
set.add(2);
set.add(3);
set.add(2);  // Duplicate - won't be added

console.log("Set size (after adding 1, 2, 3, 2):", set.size);
console.log("has(2):", set.has(2));
console.log("has(4):", set.has(4));

// Set initialization with array
const colors: Set<string> = new Set(["red", "green", "blue", "red", "yellow"]);
console.log("\nColors set size:", colors.size);
console.log("Colors:", JSON.stringify(Array.from(colors)));

// --- Set Iteration ---
console.log("\n--- Set Iteration ---");

console.log("Set values:");
for (const color of colors) {
    console.log("  " + color);
}

// forEach
console.log("\nUsing forEach:");
colors.forEach(color => {
    console.log("  Color: " + color);
});

// --- Set Operations (Union, Intersection, Difference) ---
console.log("\n--- Set Operations ---");

const setA: Set<number> = new Set([1, 2, 3, 4, 5]);
const setB: Set<number> = new Set([4, 5, 6, 7, 8]);

// Union
function union<T>(a: Set<T>, b: Set<T>): Set<T> {
    const result: Set<T> = new Set(a);
    for (const item of b) {
        result.add(item);
    }
    return result;
}

// Intersection
function intersection<T>(a: Set<T>, b: Set<T>): Set<T> {
    const result: Set<T> = new Set();
    for (const item of a) {
        if (b.has(item)) {
            result.add(item);
        }
    }
    return result;
}

// Difference (a - b)
function difference<T>(a: Set<T>, b: Set<T>): Set<T> {
    const result: Set<T> = new Set();
    for (const item of a) {
        if (!b.has(item)) {
            result.add(item);
        }
    }
    return result;
}

// Symmetric difference
function symmetricDifference<T>(a: Set<T>, b: Set<T>): Set<T> {
    return union(difference(a, b), difference(b, a));
}

console.log("Set A:", JSON.stringify(Array.from(setA)));
console.log("Set B:", JSON.stringify(Array.from(setB)));
console.log("Union:", JSON.stringify(Array.from(union(setA, setB))));
console.log("Intersection:", JSON.stringify(Array.from(intersection(setA, setB))));
console.log("Difference (A - B):", JSON.stringify(Array.from(difference(setA, setB))));
console.log("Symmetric Difference:", JSON.stringify(Array.from(symmetricDifference(setA, setB))));

// Subset check
function isSubset<T>(a: Set<T>, b: Set<T>): boolean {
    for (const item of a) {
        if (!b.has(item)) {
            return false;
        }
    }
    return true;
}

const setC: Set<number> = new Set([1, 2, 3]);
console.log("\nSet C:", JSON.stringify(Array.from(setC)));
console.log("Is C subset of A?", isSubset(setC, setA));
console.log("Is A subset of C?", isSubset(setA, setC));

// --- Using Map for Caching ---
console.log("\n--- Using Map for Caching ---");

const cache: Map<number, number> = new Map();
let computeCount: number = 0;

function expensiveCompute(n: number): number {
    if (cache.has(n)) {
        return cache.get(n)!;
    }
    computeCount++;
    // Simulate expensive computation
    const result: number = n * n + n;
    cache.set(n, result);
    return result;
}

console.log("Computing 5:", expensiveCompute(5));
console.log("Computing 10:", expensiveCompute(10));
console.log("Computing 5 (cached):", expensiveCompute(5));
console.log("Computing 10 (cached):", expensiveCompute(10));
console.log("Compute count (should be 2):", computeCount);
console.log("Cache size:", cache.size);

// --- Graph Example ---
console.log("\n--- Graph with Map<node, Set<neighbor>> ---");

const graph: Graph<string> = buildGraph();
console.log("\nGraph structure:");
graph.nodes.forEach((neighbors, node) => {
    console.log("  " + node + " -> " + JSON.stringify(Array.from(neighbors)));
});

console.log("\nBFS from A:", JSON.stringify(bfs(graph, "A")));
console.log("DFS from A:", JSON.stringify(dfs(graph, "A")));

// --- Word Frequency Counter ---
console.log("\n--- Word Frequency Counter ---");

const text: string = "the quick brown fox jumps over the lazy dog the fox is quick";
const counter: WordCounter = analyzeText(text);

console.log("\nWord frequencies:");
const sorted = Array.from(counter.frequencies.entries())
    .sort((a, b) => b[1] - a[1]);
for (const [word, count] of sorted) {
    console.log("  " + word + ": " + count);
}

console.log("\nUnique words:", counter.uniqueWords.size);
console.log("Total words:", counter.totalWords);

// --- Map with Object-like Keys ---
console.log("\n--- Map with Complex Keys ---");

// Note: JavaScript Maps can use objects as keys, but comparison is by reference
const pointMap: Map<string, string> = new Map();

// We use string keys to represent points since object keys use reference equality
function pointKey(x: number, y: number): string {
    return x + "," + y;
}

pointMap.set(pointKey(0, 0), "origin");
pointMap.set(pointKey(1, 0), "unit-x");
pointMap.set(pointKey(0, 1), "unit-y");
pointMap.set(pointKey(1, 1), "diagonal");

console.log("Point (0,0):", pointMap.get(pointKey(0, 0)));
console.log("Point (1,1):", pointMap.get(pointKey(1, 1)));

// --- Converting Between Collections ---
console.log("\n--- Converting Between Collections ---");

// Array to Set (removes duplicates)
const arr: number[] = [1, 2, 2, 3, 3, 3, 4, 4, 4, 4];
const uniqueSet: Set<number> = new Set(arr);
console.log("Array:", JSON.stringify(arr));
console.log("Unique values:", JSON.stringify(Array.from(uniqueSet)));

// Map to Object
const userMap: Map<string, number> = new Map([
    ["alice", 30],
    ["bob", 25],
    ["charlie", 35]
]);
const userObj: {[key: string]: number} = {};
userMap.forEach((value, key) => {
    userObj[key] = value;
});
console.log("\nMap to Object:", JSON.stringify(userObj));

// Object to Map
const configObj: {[key: string]: string} = {
    host: "localhost",
    port: "8080",
    protocol: "https"
};
const configMap: Map<string, string> = new Map(Object.entries(configObj));
console.log("Object to Map size:", configMap.size);
configMap.forEach((value, key) => {
    console.log("  " + key + " = " + value);
});

// --- Practical Example: Grouping ---
console.log("\n--- Grouping with Map ---");

interface Person {
    name: string;
    department: string;
    salary: number;
}

const employees: Person[] = [
    { name: "Alice", department: "Engineering", salary: 75000 },
    { name: "Bob", department: "Sales", salary: 60000 },
    { name: "Charlie", department: "Engineering", salary: 80000 },
    { name: "Diana", department: "Sales", salary: 65000 },
    { name: "Eve", department: "Marketing", salary: 55000 }
];

function groupBy<T>(items: T[], keyFn: (item: T) => string): Map<string, T[]> {
    const groups: Map<string, T[]> = new Map();
    for (const item of items) {
        const key: string = keyFn(item);
        if (!groups.has(key)) {
            groups.set(key, []);
        }
        groups.get(key)!.push(item);
    }
    return groups;
}

const byDepartment = groupBy(employees, e => e.department);
console.log("Employees by department:");
byDepartment.forEach((people, dept) => {
    const names: string = people.map(p => p.name).join(", ");
    console.log("  " + dept + ": " + names);
});

// --- Set for Deduplication and Filtering ---
console.log("\n--- Set for Deduplication ---");

const data: {id: number; value: string}[] = [
    { id: 1, value: "a" },
    { id: 2, value: "b" },
    { id: 1, value: "c" },  // duplicate id
    { id: 3, value: "d" },
    { id: 2, value: "e" }   // duplicate id
];

const seenIds: Set<number> = new Set();
const uniqueById: {id: number; value: string}[] = [];

for (const item of data) {
    if (!seenIds.has(item.id)) {
        seenIds.add(item.id);
        uniqueById.push(item);
    }
}

console.log("Original data length:", data.length);
console.log("Unique by id length:", uniqueById.length);
console.log("Unique items:", JSON.stringify(uniqueById));

console.log("\n=== Demo Complete ===");
