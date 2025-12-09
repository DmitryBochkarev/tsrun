// Validation functions that throw errors

export class ValidationError extends Error {
    field: string;
    constraint: string;

    constructor(field: string, constraint: string, message: string) {
        super(message);
        this.name = "ValidationError";
        this.field = field;
        this.constraint = constraint;
    }
}

export function validateRequired(value: any, fieldName: string): void {
    if (value === undefined || value === null || value === "") {
        throw new ValidationError(fieldName, "required", fieldName + " is required");
    }
}

export function validateString(value: any, fieldName: string): void {
    if (typeof value !== "string") {
        throw new TypeError(fieldName + " must be a string, got " + typeof value);
    }
}

export function validateNumber(value: any, fieldName: string): void {
    if (typeof value !== "number" || Number.isNaN(value)) {
        throw new TypeError(fieldName + " must be a valid number");
    }
}

export function validateMinLength(value: string, minLength: number, fieldName: string): void {
    if (value.length < minLength) {
        throw new ValidationError(
            fieldName,
            "minLength",
            fieldName + " must be at least " + minLength + " characters"
        );
    }
}

export function validateMaxLength(value: string, maxLength: number, fieldName: string): void {
    if (value.length > maxLength) {
        throw new ValidationError(
            fieldName,
            "maxLength",
            fieldName + " must be at most " + maxLength + " characters"
        );
    }
}

export function validateRange(value: number, min: number, max: number, fieldName: string): void {
    if (value < min || value > max) {
        throw new RangeError(fieldName + " must be between " + min + " and " + max);
    }
}

export function validatePositive(value: number, fieldName: string): void {
    if (value <= 0) {
        throw new RangeError(fieldName + " must be positive");
    }
}

export function validateEmail(email: string): void {
    // Simple email validation
    if (!email.includes("@") || !email.includes(".")) {
        throw new ValidationError("email", "format", "Invalid email format");
    }
}

export function validateUrl(url: string): void {
    if (!url.startsWith("http://") && !url.startsWith("https://")) {
        throw new ValidationError("url", "format", "URL must start with http:// or https://");
    }
}

// Compound validator that collects all errors
export interface ValidationResult {
    valid: boolean;
    errors: string[];
}

export function validateObject(
    obj: any,
    rules: { field: string; validators: ((value: any) => void)[] }[]
): ValidationResult {
    const errors: string[] = [];

    for (const rule of rules) {
        const value = obj[rule.field];
        for (const validator of rule.validators) {
            try {
                validator(value);
            } catch (e) {
                if (e instanceof Error) {
                    errors.push(e.message);
                }
            }
        }
    }

    return {
        valid: errors.length === 0,
        errors: errors
    };
}
