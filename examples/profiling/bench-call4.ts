// Benchmark: 4-arg function calls
function call4(a: number, b: number, c: number, d: number): number { return a + b + c + d; }
let sum = 0;
for (let i = 0; i < 200000; i++) { sum += call4(i, i+1, i+2, i+3); }
