// Benchmark: 8-arg function calls
function call8(a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number): number {
    return a + b + c + d + e + f + g + h;
}
let sum = 0;
for (let i = 0; i < 200000; i++) { sum += call8(i, i+1, i+2, i+3, i+4, i+5, i+6, i+7); }
