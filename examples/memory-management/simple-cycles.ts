{
    const ITERATIONS: number = 10000;
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
