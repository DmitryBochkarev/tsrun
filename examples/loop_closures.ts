// # Should show no leaks at exit
// valgrind --leak-check=full ./target/debug/typescript-eval-runner examples/algorithms/main.ts

// # Should show stable memory during execution (no accumulation)
let sum = 0;
for (let i = 0; i < 10000; i++) {
    let fn = () => i;
    sum = sum + fn();
}
sum

/*
 cargo build --bin typescript-eval-runner
 /usr/bin/time -v ./target/debug/typescript-eval-runner examples/loop_closures.ts
 valgrind ./target/debug/typescript-eval-runner examples/loop_closures.ts
*/
