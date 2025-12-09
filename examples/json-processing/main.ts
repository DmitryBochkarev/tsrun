// JSON Processing Showcase
// Demonstrates JSON.parse, JSON.stringify, and data transformations

import { validate, Schemas, SchemaDefinition } from "./schema";
import {
    deepClone,
    deepMerge,
    pick,
    omit,
    flatten,
    unflatten,
    mapValues,
    filterObject,
    getPath,
    setPath
} from "./transform";

console.log("=== JSON Processing Showcase ===\n");

// --- Basic JSON Operations ---
console.log("--- Basic JSON Operations ---");

const user = {
    name: "Alice",
    age: 30,
    email: "alice@example.com",
    roles: ["admin", "user"],
    settings: {
        theme: "dark",
        notifications: true
    }
};

// Stringify with formatting
const jsonPretty = JSON.stringify(user, null, 2);
console.log("Pretty printed JSON:");
console.log(jsonPretty);

// Parse it back
const parsed = JSON.parse(jsonPretty);
console.log("\nParsed back:", parsed.name, "-", parsed.email);

// --- Manual Data Redaction ---
console.log("\n--- Manual Data Redaction ---");

// Since replacer functions aren't fully supported, use manual redaction
function redactSensitiveData(obj: any): any {
    const clone = deepClone(obj);
    if ("email" in clone) clone.email = "[REDACTED]";
    if ("password" in clone) clone.password = "[REDACTED]";
    return clone;
}

const redactedUser = redactSensitiveData(user);
console.log("Redacted:", JSON.stringify(redactedUser, null, 2));

// Select specific fields using pick
const publicFields = pick(user, ["name", "age"]);
console.log("\nPublic fields only:", JSON.stringify(publicFields));

// --- Deep Clone ---
console.log("\n--- Deep Clone ---");

const original = { a: 1, b: { c: 2, d: [3, 4] } };
const cloned = deepClone(original);
cloned.b.c = 999;
console.log("Original:", JSON.stringify(original));
console.log("Cloned (modified):", JSON.stringify(cloned));
console.log("Original unchanged:", original.b.c === 2);

// --- Deep Merge ---
console.log("\n--- Deep Merge ---");

const defaults = {
    server: { host: "localhost", port: 8080 },
    logging: { level: "info", format: "json" }
};

const overrides = {
    server: { port: 3000 },
    logging: { level: "debug" }
};

const merged = deepMerge(defaults, overrides);
console.log("Merged config:");
console.log(JSON.stringify(merged, null, 2));

// --- Pick and Omit ---
console.log("\n--- Pick and Omit ---");

const fullRecord = { id: 1, name: "Test", secret: "xxx", metadata: {} };

const picked = pick(fullRecord, ["id", "name"]);
console.log("Picked:", JSON.stringify(picked));

const omitted = omit(fullRecord, ["secret"]);
console.log("Omitted:", JSON.stringify(omitted));

// --- Flatten and Unflatten ---
console.log("\n--- Flatten and Unflatten ---");

const nested = {
    user: {
        name: "Bob",
        address: {
            city: "NYC",
            zip: "10001"
        }
    },
    active: true
};

const flattened = flatten(nested);
console.log("Flattened:");
console.log(JSON.stringify(flattened, null, 2));

const unflattened = unflatten(flattened);
console.log("Unflattened:");
console.log(JSON.stringify(unflattened, null, 2));

// --- Path-based Access ---
console.log("\n--- Path-based Access ---");

const config = {
    database: {
        primary: { host: "db1.local", port: 5432 },
        replica: { host: "db2.local", port: 5432 }
    }
};

console.log("Get path 'database.primary.host':", getPath(config, "database.primary.host"));
console.log("Get path 'database.backup.host':", getPath(config, "database.backup.host"));

const updated = setPath(config, "database.primary.port", 5433);
console.log("After setPath:", JSON.stringify(updated.database.primary));

// --- Map and Filter Objects ---
console.log("\n--- Map and Filter Objects ---");

const scores = { alice: 85, bob: 92, charlie: 78, diana: 95 };

const doubled = mapValues(scores, (v: number) => v * 2);
console.log("Doubled scores:", JSON.stringify(doubled));

const passing = filterObject(scores, (v: number) => v >= 80);
console.log("Passing scores:", JSON.stringify(passing));

// --- Schema Validation ---
console.log("\n--- Schema Validation ---");

const userSchema: SchemaDefinition = Schemas.object({
    name: Schemas.string({ minLength: 1, maxLength: 50 }),
    age: Schemas.number({ minimum: 0, maximum: 150 }),
    email: Schemas.string({ pattern: "^[^@]+@[^@]+\\.[^@]+$" }),
    role: Schemas.oneOf("admin", "user", "guest")
}, ["name", "email"]);

const validUser = { name: "Alice", age: 30, email: "alice@example.com", role: "admin" };
const invalidUser = { name: "", age: 200, email: "invalid", role: "superuser" };

console.log("\nValidating valid user:");
const validResult = validate(validUser, userSchema);
console.log("  Valid:", validResult.valid);

console.log("\nValidating invalid user:");
const invalidResult = validate(invalidUser, userSchema);
console.log("  Valid:", invalidResult.valid);
console.log("  Errors:");
for (const err of invalidResult.errors) {
    console.log("    - " + err.path + ": " + err.message);
}

// --- Nested Array Schema ---
console.log("\n--- Nested Array Schema ---");

const orderSchema: SchemaDefinition = Schemas.object({
    orderId: Schemas.string(),
    items: Schemas.array(Schemas.object({
        productId: Schemas.string(),
        quantity: Schemas.number({ minimum: 1 }),
        price: Schemas.number({ minimum: 0 })
    }, ["productId", "quantity"]))
}, ["orderId", "items"]);

const validOrder = {
    orderId: "ORD-001",
    items: [
        { productId: "P1", quantity: 2, price: 19.99 },
        { productId: "P2", quantity: 1, price: 9.99 }
    ]
};

const invalidOrder = {
    orderId: "ORD-002",
    items: [
        { productId: "P1", quantity: 0 }, // quantity < 1
        { quantity: 2, price: 10 } // missing productId
    ]
};

console.log("Valid order:", validate(validOrder, orderSchema).valid);

const orderResult = validate(invalidOrder, orderSchema);
console.log("Invalid order:", orderResult.valid);
for (const err of orderResult.errors) {
    console.log("  - " + err.path + ": " + err.message);
}

// --- Date Serialization ---
console.log("\n--- Date Serialization ---");

// Dates serialize to ISO strings automatically
const dataWithDate = {
    name: "Session",
    createdAt: new Date("2024-01-15T10:00:00Z")
};

const dateJson = JSON.stringify(dataWithDate, null, 2);
console.log("Date serialized:");
console.log(dateJson);

// Parse and manually convert dates
const parsedData = JSON.parse(dateJson);
console.log("Parsed createdAt:", parsedData.createdAt);
console.log("Is string:", typeof parsedData.createdAt === "string");

// Convert string back to Date manually
const dateFromString = new Date(parsedData.createdAt);
console.log("Converted back:", dateFromString instanceof Date);

// --- Building JSON Programmatically ---
console.log("\n--- Building JSON Programmatically ---");

function buildApiResponse(data: any, options: { status?: number; meta?: any } = {}): string {
    const response: any = {
        success: true,
        status: options.status || 200,
        data: data
    };

    if (options.meta) {
        response.meta = options.meta;
    }

    response.timestamp = new Date().toISOString();

    return JSON.stringify(response, null, 2);
}

const apiResponse = buildApiResponse(
    { users: [{ id: 1, name: "Alice" }] },
    { meta: { total: 1, page: 1 } }
);
console.log("API Response:");
console.log(apiResponse);

console.log("\n=== Demo Complete ===");
