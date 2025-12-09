// Generator Functions Demo
// Demonstrates: function*, yield, yield*, for...of, lazy evaluation
import { range, rangeInclusive, countdown, take, skip } from "./range";
import { fibonacci, fibonacciUpto, fibonacciN, lucas, tribonacci } from "./fibonacci";
import { createNode, preorder, postorder, levelOrder, leaves, flatten } from "./tree";

// Helper to collect generator values into array
function collect<T>(gen: Generator<T>): T[] {
  const result: T[] = [];
  for (const value of gen) {
    result.push(value);
  }
  return result;
}

// Demo 1: Basic range generator
function rangeDemo(): { basic: number[]; stepped: number[]; inclusive: number[] } {
  return {
    basic: collect(range(0, 5)),           // [0, 1, 2, 3, 4]
    stepped: collect(range(0, 10, 2)),     // [0, 2, 4, 6, 8]
    inclusive: collect(rangeInclusive(1, 5)), // [1, 2, 3, 4, 5]
  };
}

// Demo 2: Countdown generator
function countdownDemo(): number[] {
  return collect(countdown(5)); // [5, 4, 3, 2, 1]
}

// Demo 3: Take and skip combinators
function takeSkipDemo(): { taken: number[]; skipped: number[]; combined: number[] } {
  return {
    taken: collect(take(range(0, 100), 5)),     // [0, 1, 2, 3, 4]
    skipped: collect(skip(range(0, 10), 5)),    // [5, 6, 7, 8, 9]
    combined: collect(take(skip(range(0, 100), 10), 5)), // [10, 11, 12, 13, 14]
  };
}

// Demo 4: Fibonacci sequences
function fibonacciDemo(): { first10: number[]; upTo100: number[]; lucas5: number[]; tribonacci8: number[] } {
  return {
    first10: collect(fibonacciN(10)),           // [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]
    upTo100: collect(fibonacciUpto(100)),       // [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]
    lucas5: collect(take(lucas(), 5)),          // [2, 1, 3, 4, 7]
    tribonacci8: collect(take(tribonacci(), 8)), // [0, 0, 1, 1, 2, 4, 7, 13]
  };
}

// Demo 5: Infinite generator with take
function infiniteDemo(): number[] {
  // Take first 15 fibonacci numbers from infinite generator
  return collect(take(fibonacci(), 15));
}

// Demo 6: Tree traversal
function treeDemo(): { preorder: number[]; postorder: number[]; levelOrder: number[]; leaves: number[] } {
  // Build a tree:
  //        1
  //       /|\
  //      2 3 4
  //     /|   |
  //    5 6   7
  const tree = createNode(1, [
    createNode(2, [
      createNode(5),
      createNode(6),
    ]),
    createNode(3),
    createNode(4, [
      createNode(7),
    ]),
  ]);

  return {
    preorder: collect(preorder(tree)),    // [1, 2, 5, 6, 3, 4, 7]
    postorder: collect(postorder(tree)),  // [5, 6, 2, 3, 7, 4, 1]
    levelOrder: collect(levelOrder(tree)), // [1, 2, 3, 4, 5, 6, 7]
    leaves: collect(leaves(tree)),         // [5, 6, 3, 7]
  };
}

// Demo 7: Flatten nested arrays
function flattenDemo(): { simple: number[]; deep: number[] } {
  return {
    simple: collect(flatten([1, [2, 3], 4, [5, 6]])),
    deep: collect(flatten([1, [2, [3, [4, 5]]]])),
  };
}

// Demo 8: Generator composition with yield*
function* composedGenerator(): Generator<string> {
  yield "start";
  yield* ["a", "b", "c"];
  yield "middle";
  yield* ["x", "y", "z"];
  yield "end";
}

function compositionDemo(): string[] {
  return collect(composedGenerator());
}

// Demo 9: Generator with early termination
function* generateUntil(max: number): Generator<number> {
  let n = 0;
  while (true) {
    if (n > max) return;
    yield n;
    n++;
  }
}

function earlyTerminationDemo(): number[] {
  return collect(generateUntil(5)); // [0, 1, 2, 3, 4, 5]
}

// Demo 10: Generator state machine
function* stateMachine(): Generator<string> {
  yield "idle";
  yield "loading";
  yield "processing";
  yield "complete";
}

function stateMachineDemo(): string[] {
  return collect(stateMachine());
}

// Run all demos
function runAllDemos(): {
  range: { basic: number[]; stepped: number[]; inclusive: number[] };
  countdown: number[];
  takeSkip: { taken: number[]; skipped: number[]; combined: number[] };
  fibonacci: { first10: number[]; upTo100: number[]; lucas5: number[]; tribonacci8: number[] };
  infinite: number[];
  tree: { preorder: number[]; postorder: number[]; levelOrder: number[]; leaves: number[] };
  flatten: { simple: number[]; deep: number[] };
  composition: string[];
  earlyTermination: number[];
  stateMachine: string[];
} {
  return {
    range: rangeDemo(),
    countdown: countdownDemo(),
    takeSkip: takeSkipDemo(),
    fibonacci: fibonacciDemo(),
    infinite: infiniteDemo(),
    tree: treeDemo(),
    flatten: flattenDemo(),
    composition: compositionDemo(),
    earlyTermination: earlyTerminationDemo(),
    stateMachine: stateMachineDemo(),
  };
}

const results = runAllDemos();
JSON.stringify(results, null, 2);
