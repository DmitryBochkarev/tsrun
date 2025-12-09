// JSON transformation utilities

// Deep clone an object
export function deepClone<T>(obj: T): T {
    return JSON.parse(JSON.stringify(obj));
}

// Deep merge objects (target values take precedence for conflicting keys)
export function deepMerge(source: any, target: any): any {
    const result = deepClone(source);

    const targetKeys = Object.keys(target);
    for (const key of targetKeys) {
        const sourceValue = result[key];
        const targetValue = target[key];

        if (isPlainObject(sourceValue) && isPlainObject(targetValue)) {
            result[key] = deepMerge(sourceValue, targetValue);
        } else {
            result[key] = deepClone(targetValue);
        }
    }

    return result;
}

// Check if value is a plain object (not array, null, etc.)
function isPlainObject(value: any): boolean {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}

// Pick specific keys from an object
export function pick<T>(obj: T, keys: string[]): Partial<T> {
    const result: any = {};
    for (const key of keys) {
        if (key in (obj as any)) {
            result[key] = (obj as any)[key];
        }
    }
    return result;
}

// Omit specific keys from an object
export function omit<T>(obj: T, keys: string[]): Partial<T> {
    const result: any = {};
    const objKeys = Object.keys(obj as any);
    for (const key of objKeys) {
        let shouldOmit = false;
        for (const omitKey of keys) {
            if (key === omitKey) {
                shouldOmit = true;
                break;
            }
        }
        if (!shouldOmit) {
            result[key] = deepClone((obj as any)[key]);
        }
    }
    return result;
}

// Flatten a nested object to dot notation
export function flatten(obj: any, prefix: string = ""): { [key: string]: any } {
    const result: { [key: string]: any } = {};

    const objKeys = Object.keys(obj);
    for (const key of objKeys) {
        const value = obj[key];
        const newKey = prefix ? prefix + "." + key : key;

        if (isPlainObject(value)) {
            const nested = flatten(value, newKey);
            const nestedKeys = Object.keys(nested);
            for (const nk of nestedKeys) {
                result[nk] = nested[nk];
            }
        } else {
            result[newKey] = value;
        }
    }

    return result;
}

// Unflatten a dot-notation object to nested structure
export function unflatten(obj: { [key: string]: any }): any {
    const result: any = {};

    const objKeys = Object.keys(obj);
    for (const key of objKeys) {
        const parts = key.split(".");
        let current = result;

        for (let i = 0; i < parts.length - 1; i++) {
            const part = parts[i];
            if (!(part in current)) {
                current[part] = {};
            }
            current = current[part];
        }

        const lastPart = parts[parts.length - 1];
        current[lastPart] = obj[key];
    }

    return result;
}

// Map over object values (like array.map but for objects)
export function mapValues<T, U>(obj: { [key: string]: T }, fn: (value: T, key: string) => U): { [key: string]: U } {
    const result: { [key: string]: U } = {};
    const keys = Object.keys(obj);
    for (const key of keys) {
        result[key] = fn(obj[key], key);
    }
    return result;
}

// Filter object entries
export function filterObject<T>(obj: { [key: string]: T }, predicate: (value: T, key: string) => boolean): { [key: string]: T } {
    const result: { [key: string]: T } = {};
    const keys = Object.keys(obj);
    for (const key of keys) {
        if (predicate(obj[key], key)) {
            result[key] = obj[key];
        }
    }
    return result;
}

// Get a nested value by path (dot notation)
export function getPath(obj: any, path: string): any {
    const parts = path.split(".");
    let current = obj;
    for (const part of parts) {
        if (current === undefined || current === null) {
            return undefined;
        }
        current = current[part];
    }
    return current;
}

// Set a nested value by path (dot notation)
export function setPath(obj: any, path: string, value: any): any {
    const result = deepClone(obj);
    const parts = path.split(".");
    let current = result;

    for (let i = 0; i < parts.length - 1; i++) {
        const part = parts[i];
        if (!(part in current) || !isPlainObject(current[part])) {
            current[part] = {};
        }
        current = current[part];
    }

    const lastPart = parts[parts.length - 1];
    current[lastPart] = value;
    return result;
}

// JSON reviver that converts date strings to Date objects
export function dateReviver(key: string, value: any): any {
    if (typeof value === "string") {
        // ISO date pattern: YYYY-MM-DDTHH:mm:ss.sssZ
        const datePattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/;
        if (datePattern.test(value)) {
            return new Date(value);
        }
    }
    return value;
}

// JSON replacer that handles special types
export function typePreservingReplacer(key: string, value: any): any {
    if (value instanceof Date) {
        return { __type: "Date", __value: value.toISOString() };
    }
    if (value instanceof Map) {
        return { __type: "Map", __value: Array.from(value.entries()) };
    }
    if (value instanceof Set) {
        return { __type: "Set", __value: Array.from(value.values()) };
    }
    return value;
}

// JSON reviver that restores special types
export function typePreservingReviver(key: string, value: any): any {
    if (typeof value === "object" && value !== null && "__type" in value) {
        switch (value.__type) {
            case "Date":
                return new Date(value.__value);
            case "Map":
                return new Map(value.__value);
            case "Set":
                return new Set(value.__value);
        }
    }
    return value;
}
