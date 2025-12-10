// Scope Cleanup Test
// Objects created inside a block should be freed when the block exits

export interface ScopeResult {
    iterations: number;
    sum: number;
}

export function testScopeCleanup(): ScopeResult {
    let sum: number = 0;
    const iterations: number = 10000;

    for (let i = 0; i < iterations; i++) {
        // These objects are created and should be freed each iteration
        const obj = {
            id: i,
            data: "some data " + i,
            nested: {
                value: i * 2,
                array: [1, 2, 3, 4, 5]
            }
        };

        // Use the object so it's not optimized away
        sum = sum + obj.nested.value;

        // obj goes out of scope here - should be collected
    }

    return { iterations, sum };
}

// Test with arrays going out of scope
export function testArrayCleanup(): number {
    let total: number = 0;

    for (let i = 0; i < 5000; i++) {
        // Create a temporary array
        const arr: number[] = [];
        for (let j = 0; j < 100; j++) {
            arr.push(j * i);
        }

        // Sum it up
        for (const val of arr) {
            total = total + val;
        }

        // arr goes out of scope - should be collected
    }

    return total;
}

// Test with nested function scopes
export function testNestedScopes(): number {
    let result: number = 0;

    function outer(n: number): number {
        const outerData = { value: n * 10 };

        function inner(): number {
            const innerData = { value: outerData.value + 5 };
            return innerData.value;
        }

        result = result + inner();
        // innerData freed when inner() returns
        // outerData freed when outer() returns
        return result;
    }

    for (let i = 0; i < 1000; i++) {
        outer(i);
    }

    return result;
}
