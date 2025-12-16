// Nested import - math.ts imports from constants.ts
import { PI, E } from "./constants";

export function add(a: number, b: number): number {
    return a + b;
}

export function multiply(a: number, b: number): number {
    return a * b;
}

export function circleArea(radius: number): number {
    return PI * radius * radius;
}

export { PI, E };
