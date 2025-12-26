// Benchmark: 4-arg method calls (Op::CallMethod)
const obj = {
    method4(a: number, b: number, c: number, d: number): number { return a + b + c + d; }
};
let sum = 0;
for (let i = 0; i < 200000; i++) { sum += obj.method4(i, i+1, i+2, i+3); }
