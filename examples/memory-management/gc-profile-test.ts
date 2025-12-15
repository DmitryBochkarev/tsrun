// Simple GC stress test
for (let i = 0; i < 10000; i++) {
    const obj = { x: i, y: i * 2, z: i * 3 };
    const arr = [i, i+1, i+2];
    const sum = obj.x + obj.y + obj.z + arr[0] + arr[1] + arr[2];
}
console.log("Done");
