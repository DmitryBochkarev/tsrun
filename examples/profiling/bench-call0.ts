// Benchmark: 0-arg function calls
function call0(): number { return 0; }
let sum = 0;
for (let i = 0; i < 200000; i++) { sum += call0(); }
