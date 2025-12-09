// Test loop without closures for memory comparison
let sum = 0;
for (let i = 0; i < 10000; i++) {
    sum = sum + i;
}
sum
