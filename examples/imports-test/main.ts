// Import test with nested imports
// main.ts -> math.ts -> constants.ts
import { add, multiply, circleArea, PI, E } from "./math";
import { greet } from "./utils";

const result = {
    addition: add(2, 3),
    multiplication: multiply(4, 5),
    circleArea: circleArea(10),
    constants: { PI, E },
    greeting: greet("World")
};

JSON.stringify(result, null, 2);
