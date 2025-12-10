// Object Churn Test
// Allocates and deallocates many large objects to stress test memory management
// Memory usage should remain stable (not grow over time)

export interface ChurnResult {
    iterations: number;
    totalElements: number;
}

// Create and discard large arrays
export function testLargeObjectChurn(): ChurnResult {
    let totalElements: number = 0;
    const iterations: number = 500;

    for (let i = 0; i < iterations; i++) {
        // Create a large array
        const largeArray: number[] = [];
        for (let j = 0; j < 200; j++) {
            largeArray.push(j * i);
        }

        totalElements = totalElements + largeArray.length;

        // Array goes out of scope and should be freed
        // Memory should NOT accumulate across iterations
    }

    return { iterations, totalElements };
}

// Create and discard complex nested objects
export function testNestedObjectChurn(): number {
    let checksum: number = 0;

    for (let i = 0; i < 500; i++) {
        // Create a deeply nested structure
        const obj = {
            level1: {
                level2: {
                    level3: {
                        level4: {
                            value: i,
                            array: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
                        }
                    }
                }
            }
        };

        checksum = checksum + obj.level1.level2.level3.level4.value;

        // Entire nested structure goes out of scope
    }

    return checksum;
}

// Create and discard many small objects
export function testSmallObjectChurn(): number {
    let count: number = 0;

    for (let i = 0; i < 10000; i++) {
        const small = { x: i, y: i * 2 };
        count = count + small.x + small.y;
    }

    return count;
}

// Create and discard strings (which use Rc<str>)
export function testStringChurn(): number {
    let totalLength: number = 0;

    for (let i = 0; i < 5000; i++) {
        // Create temporary strings
        const str1: string = "Hello world " + i;
        const str2: string = str1 + " extra data";
        const str3: string = str2.toUpperCase();

        totalLength = totalLength + str3.length;

        // All strings go out of scope
    }

    return totalLength;
}

// Create and discard Maps and Sets
export function testCollectionChurn(): number {
    let totalSize: number = 0;

    for (let i = 0; i < 500; i++) {
        const map: Map<string, number> = new Map();
        const set: Set<number> = new Set();

        for (let j = 0; j < 50; j++) {
            map.set("key" + j, j);
            set.add(j);
        }

        totalSize = totalSize + map.size + set.size;

        // map and set go out of scope
    }

    return totalSize;
}

// Test function object churn
export function testFunctionChurn(): number {
    let sum: number = 0;

    for (let i = 0; i < 5000; i++) {
        // Create a new function each iteration
        const fn = (x: number): number => x * 2 + i;
        sum = sum + fn(i);

        // fn goes out of scope
    }

    return sum;
}

// Combined stress test
export function stressTest(): number {
    let result: number = 0;

    // Run multiple types of churn together
    for (let round = 0; round < 10; round++) {
        // Objects
        for (let i = 0; i < 100; i++) {
            const obj = { a: i, b: i * 2, c: [1, 2, 3] };
            result = result + obj.a;
        }

        // Arrays
        for (let i = 0; i < 100; i++) {
            const arr = [i, i + 1, i + 2, i + 3, i + 4];
            result = result + arr.reduce((a: number, b: number) => a + b, 0);
        }

        // Closures
        for (let i = 0; i < 100; i++) {
            const fn = (): number => i;
            result = result + fn();
        }

        // Strings
        for (let i = 0; i < 100; i++) {
            const s = "test" + i + "data";
            result = result + s.length;
        }
    }

    return result;
}
