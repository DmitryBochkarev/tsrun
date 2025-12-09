// JSON Schema validation utilities

export type SchemaType = "string" | "number" | "boolean" | "object" | "array" | "null";

export interface SchemaDefinition {
    type: SchemaType | SchemaType[];
    properties?: { [key: string]: SchemaDefinition };
    items?: SchemaDefinition;
    required?: string[];
    minLength?: number;
    maxLength?: number;
    minimum?: number;
    maximum?: number;
    pattern?: string;
    enum?: any[];
}

export interface ValidationError {
    path: string;
    message: string;
    value: any;
}

export interface ValidationResult {
    valid: boolean;
    errors: ValidationError[];
}

// Get the type name of a value
function getTypeName(value: any): SchemaType {
    if (value === null) return "null";
    if (Array.isArray(value)) return "array";
    return typeof value as SchemaType;
}

// Check if value matches expected type(s)
function matchesType(value: any, schemaType: SchemaType | SchemaType[]): boolean {
    const actualType = getTypeName(value);
    if (Array.isArray(schemaType)) {
        return schemaType.includes(actualType);
    }
    return actualType === schemaType;
}

// Validate a value against a schema
export function validate(value: any, schema: SchemaDefinition, path: string = ""): ValidationResult {
    const errors: ValidationError[] = [];

    // Type check
    if (!matchesType(value, schema.type)) {
        const expectedType = Array.isArray(schema.type) ? schema.type.join(" | ") : schema.type;
        errors.push({
            path: path || "(root)",
            message: "Expected " + expectedType + " but got " + getTypeName(value),
            value: value
        });
        return { valid: false, errors: errors };
    }

    // Enum check
    if (schema.enum !== undefined) {
        let found = false;
        for (const enumValue of schema.enum) {
            if (value === enumValue) {
                found = true;
                break;
            }
        }
        if (!found) {
            errors.push({
                path: path || "(root)",
                message: "Value must be one of: " + JSON.stringify(schema.enum),
                value: value
            });
        }
    }

    // String validations
    if (typeof value === "string") {
        if (schema.minLength !== undefined && value.length < schema.minLength) {
            errors.push({
                path: path || "(root)",
                message: "String length must be at least " + schema.minLength,
                value: value
            });
        }
        if (schema.maxLength !== undefined && value.length > schema.maxLength) {
            errors.push({
                path: path || "(root)",
                message: "String length must be at most " + schema.maxLength,
                value: value
            });
        }
        if (schema.pattern !== undefined) {
            const regex = new RegExp(schema.pattern);
            if (!regex.test(value)) {
                errors.push({
                    path: path || "(root)",
                    message: "String must match pattern: " + schema.pattern,
                    value: value
                });
            }
        }
    }

    // Number validations
    if (typeof value === "number") {
        if (schema.minimum !== undefined && value < schema.minimum) {
            errors.push({
                path: path || "(root)",
                message: "Value must be >= " + schema.minimum,
                value: value
            });
        }
        if (schema.maximum !== undefined && value > schema.maximum) {
            errors.push({
                path: path || "(root)",
                message: "Value must be <= " + schema.maximum,
                value: value
            });
        }
    }

    // Object validations
    if (typeof value === "object" && value !== null && !Array.isArray(value)) {
        // Required properties check
        if (schema.required !== undefined) {
            for (const reqProp of schema.required) {
                if (!(reqProp in value)) {
                    errors.push({
                        path: path ? path + "." + reqProp : reqProp,
                        message: "Required property missing",
                        value: undefined
                    });
                }
            }
        }

        // Validate each property
        if (schema.properties !== undefined) {
            const keys = Object.keys(schema.properties);
            for (const key of keys) {
                if (key in value) {
                    const propSchema = schema.properties[key];
                    const propPath = path ? path + "." + key : key;
                    const propResult = validate(value[key], propSchema, propPath);
                    if (!propResult.valid) {
                        for (const err of propResult.errors) {
                            errors.push(err);
                        }
                    }
                }
            }
        }
    }

    // Array validations
    if (Array.isArray(value)) {
        if (schema.items !== undefined) {
            for (let i = 0; i < value.length; i++) {
                const itemPath = path ? path + "[" + i + "]" : "[" + i + "]";
                const itemResult = validate(value[i], schema.items, itemPath);
                if (!itemResult.valid) {
                    for (const err of itemResult.errors) {
                        errors.push(err);
                    }
                }
            }
        }
    }

    return { valid: errors.length === 0, errors: errors };
}

// Helper to create common schemas
export const Schemas = {
    string: (options?: { minLength?: number; maxLength?: number; pattern?: string }): SchemaDefinition => ({
        type: "string",
        ...(options || {})
    }),

    number: (options?: { minimum?: number; maximum?: number }): SchemaDefinition => ({
        type: "number",
        ...(options || {})
    }),

    boolean: (): SchemaDefinition => ({ type: "boolean" }),

    array: (items: SchemaDefinition): SchemaDefinition => ({
        type: "array",
        items: items
    }),

    object: (properties: { [key: string]: SchemaDefinition }, required?: string[]): SchemaDefinition => ({
        type: "object",
        properties: properties,
        required: required
    }),

    nullable: (schema: SchemaDefinition): SchemaDefinition => ({
        ...schema,
        type: Array.isArray(schema.type) ? [...schema.type, "null"] : [schema.type, "null"]
    }),

    oneOf: (...values: any[]): SchemaDefinition => ({
        type: typeof values[0] as SchemaType,
        enum: values
    })
};
