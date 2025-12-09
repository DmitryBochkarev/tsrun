// TypeScript Namespace Patterns

// Basic namespace
export namespace Geometry {
    export const PI: number = 3.14159265358979;

    export function circleArea(radius: number): number {
        return PI * radius * radius;
    }

    export function circleCircumference(radius: number): number {
        return 2 * PI * radius;
    }

    export function rectangleArea(width: number, height: number): number {
        return width * height;
    }

    export function rectanglePerimeter(width: number, height: number): number {
        return 2 * (width + height);
    }

    export function triangleArea(base: number, height: number): number {
        return 0.5 * base * height;
    }
}

// Nested namespaces
export namespace Validation {
    export namespace Strings {
        export function isNotEmpty(value: string): boolean {
            return value.length > 0;
        }

        export function minLength(value: string, min: number): boolean {
            return value.length >= min;
        }

        export function maxLength(value: string, max: number): boolean {
            return value.length <= max;
        }

        export function matches(value: string, pattern: RegExp): boolean {
            return pattern.test(value);
        }
    }

    export namespace Numbers {
        export function isPositive(value: number): boolean {
            return value > 0;
        }

        export function isNegative(value: number): boolean {
            return value < 0;
        }

        export function inRange(value: number, min: number, max: number): boolean {
            return value >= min && value <= max;
        }

        export function isInteger(value: number): boolean {
            return Math.floor(value) === value;
        }
    }

    export namespace Arrays {
        export function isNotEmpty<T>(arr: T[]): boolean {
            return arr.length > 0;
        }

        export function hasMinLength<T>(arr: T[], min: number): boolean {
            return arr.length >= min;
        }
    }
}

// Namespace with interfaces and classes
export namespace Models {
    export interface Entity {
        id: number;
        createdAt: Date;
    }

    export interface User extends Entity {
        name: string;
        email: string;
    }

    export function createUser(id: number, name: string, email: string): User {
        return {
            id: id,
            name: name,
            email: email,
            createdAt: new Date()
        };
    }

    export function formatUser(user: User): string {
        return user.name + " <" + user.email + "> (ID: " + user.id + ")";
    }
}

// Namespace for utilities
export namespace Utils {
    export function capitalize(s: string): string {
        if (s.length === 0) return s;
        return s.charAt(0).toUpperCase() + s.slice(1).toLowerCase();
    }

    export function truncate(s: string, maxLength: number): string {
        if (s.length <= maxLength) return s;
        return s.slice(0, maxLength - 3) + "...";
    }

    export function repeat(s: string, count: number): string {
        let result: string = "";
        for (let i: number = 0; i < count; i++) {
            result += s;
        }
        return result;
    }

    export function padLeft(s: string, length: number, char: string): string {
        while (s.length < length) {
            s = char + s;
        }
        return s;
    }

    export function padRight(s: string, length: number, char: string): string {
        while (s.length < length) {
            s = s + char;
        }
        return s;
    }
}

// Constants namespace
export namespace Constants {
    export const MAX_INT: number = 2147483647;
    export const MIN_INT: number = -2147483648;
    export const EPSILON: number = 0.0001;

    export namespace HTTP {
        export const GET: string = "GET";
        export const POST: string = "POST";
        export const PUT: string = "PUT";
        export const DELETE: string = "DELETE";
    }
}
