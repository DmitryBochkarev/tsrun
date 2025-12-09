// TypeScript Enum and Namespace Demo
// Demonstrates TypeScript-specific features

import {
    Direction,
    StatusCode,
    LogLevel,
    FileAccess,
    getDirectionName,
    isSuccessStatus,
    formatLogMessage,
    hasPermission,
    getAllDirections,
    getAllLogLevels
} from "./enums";

import {
    Geometry,
    Validation,
    Models,
    Utils,
    Constants
} from "./namespaces";

console.log("=== TypeScript Enum and Namespace Demo ===\n");

// --- Basic Numeric Enum ---
console.log("--- Basic Numeric Enum (Direction) ---");

console.log("Direction.Up =", Direction.Up);
console.log("Direction.Down =", Direction.Down);
console.log("Direction.Left =", Direction.Left);
console.log("Direction.Right =", Direction.Right);

console.log("\nAll directions:");
const directions: Direction[] = getAllDirections();
for (const dir of directions) {
    console.log("  " + dir + " -> " + getDirectionName(dir));
}

// --- Enum with Explicit Values ---
console.log("\n--- Enum with Explicit Values (StatusCode) ---");

console.log("StatusCode.OK =", StatusCode.OK);
console.log("StatusCode.NotFound =", StatusCode.NotFound);
console.log("StatusCode.InternalError =", StatusCode.InternalError);

const codes: StatusCode[] = [StatusCode.OK, StatusCode.Created, StatusCode.BadRequest, StatusCode.NotFound];
console.log("\nStatus check:");
for (const code of codes) {
    const status: string = isSuccessStatus(code) ? "success" : "error";
    console.log("  " + code + " is " + status);
}

// --- String Enum ---
console.log("\n--- String Enum (LogLevel) ---");

console.log("LogLevel.Debug =", LogLevel.Debug);
console.log("LogLevel.Info =", LogLevel.Info);
console.log("LogLevel.Warn =", LogLevel.Warn);
console.log("LogLevel.Error =", LogLevel.Error);

console.log("\nFormatted messages:");
const levels: LogLevel[] = getAllLogLevels();
for (const level of levels) {
    console.log("  " + formatLogMessage(level, "Sample message"));
}

// --- Bitwise/Flag Enum ---
console.log("\n--- Bitwise/Flag Enum (FileAccess) ---");

console.log("FileAccess.None =", FileAccess.None);
console.log("FileAccess.Read =", FileAccess.Read);
console.log("FileAccess.Write =", FileAccess.Write);
console.log("FileAccess.ReadWrite =", FileAccess.ReadWrite);
console.log("FileAccess.Execute =", FileAccess.Execute);
console.log("FileAccess.All =", FileAccess.All);

console.log("\nPermission checks:");
const userAccess: FileAccess = FileAccess.ReadWrite;
console.log("User has ReadWrite (" + userAccess + "):");
console.log("  Has Read?", hasPermission(userAccess, FileAccess.Read));
console.log("  Has Write?", hasPermission(userAccess, FileAccess.Write));
console.log("  Has Execute?", hasPermission(userAccess, FileAccess.Execute));

// --- Geometry Namespace ---
console.log("\n--- Geometry Namespace ---");

console.log("Geometry.PI =", Geometry.PI);
console.log("Circle (radius=5):");
console.log("  Area:", Geometry.circleArea(5).toFixed(4));
console.log("  Circumference:", Geometry.circleCircumference(5).toFixed(4));

console.log("Rectangle (4x6):");
console.log("  Area:", Geometry.rectangleArea(4, 6));
console.log("  Perimeter:", Geometry.rectanglePerimeter(4, 6));

console.log("Triangle (base=8, height=5):");
console.log("  Area:", Geometry.triangleArea(8, 5));

// --- Nested Namespaces (Validation) ---
console.log("\n--- Nested Namespaces (Validation) ---");

console.log("Validation.Strings:");
console.log("  isNotEmpty('hello'):", Validation.Strings.isNotEmpty("hello"));
console.log("  isNotEmpty(''):", Validation.Strings.isNotEmpty(""));
console.log("  minLength('hello', 3):", Validation.Strings.minLength("hello", 3));
console.log("  maxLength('hello', 3):", Validation.Strings.maxLength("hello", 3));

console.log("\nValidation.Numbers:");
console.log("  isPositive(5):", Validation.Numbers.isPositive(5));
console.log("  isNegative(-3):", Validation.Numbers.isNegative(-3));
console.log("  inRange(7, 1, 10):", Validation.Numbers.inRange(7, 1, 10));
console.log("  isInteger(3.5):", Validation.Numbers.isInteger(3.5));
console.log("  isInteger(4):", Validation.Numbers.isInteger(4));

console.log("\nValidation.Arrays:");
console.log("  isNotEmpty([1,2,3]):", Validation.Arrays.isNotEmpty([1, 2, 3]));
console.log("  isNotEmpty([]):", Validation.Arrays.isNotEmpty([]));
console.log("  hasMinLength([1,2,3], 2):", Validation.Arrays.hasMinLength([1, 2, 3], 2));

// --- Models Namespace ---
console.log("\n--- Models Namespace ---");

const user1 = Models.createUser(1, "Alice", "alice@example.com");
const user2 = Models.createUser(2, "Bob", "bob@example.com");

console.log("Created users:");
console.log("  " + Models.formatUser(user1));
console.log("  " + Models.formatUser(user2));

// --- Utils Namespace ---
console.log("\n--- Utils Namespace ---");

console.log("capitalize('hELLO'):", Utils.capitalize("hELLO"));
console.log("truncate('Hello World!', 8):", Utils.truncate("Hello World!", 8));
console.log("repeat('ab', 4):", Utils.repeat("ab", 4));
console.log("padLeft('42', 5, '0'):", Utils.padLeft("42", 5, "0"));
console.log("padRight('Hi', 5, '.'):", Utils.padRight("Hi", 5, "."));

// --- Constants Namespace ---
console.log("\n--- Constants Namespace ---");

console.log("Constants.MAX_INT =", Constants.MAX_INT);
console.log("Constants.MIN_INT =", Constants.MIN_INT);
console.log("Constants.EPSILON =", Constants.EPSILON);

console.log("\nConstants.HTTP:");
console.log("  GET:", Constants.HTTP.GET);
console.log("  POST:", Constants.HTTP.POST);
console.log("  PUT:", Constants.HTTP.PUT);
console.log("  DELETE:", Constants.HTTP.DELETE);

// --- Combining Enums and Namespaces ---
console.log("\n--- Combining Enums and Namespaces ---");

// Simulate a simple request handler
interface Request {
    method: string;
    path: string;
}

interface Response {
    status: StatusCode;
    body: string;
}

function handleRequest(req: Request): Response {
    const logMsg: string = formatLogMessage(LogLevel.Info, req.method + " " + req.path);
    console.log(logMsg);

    if (req.method === Constants.HTTP.GET && req.path === "/users") {
        return {
            status: StatusCode.OK,
            body: JSON.stringify([user1, user2])
        };
    } else if (req.method === Constants.HTTP.GET && req.path === "/health") {
        return {
            status: StatusCode.OK,
            body: JSON.stringify({ status: "healthy" })
        };
    } else {
        return {
            status: StatusCode.NotFound,
            body: JSON.stringify({ error: "Not found" })
        };
    }
}

const requests: Request[] = [
    { method: Constants.HTTP.GET, path: "/users" },
    { method: Constants.HTTP.GET, path: "/health" },
    { method: Constants.HTTP.POST, path: "/unknown" }
];

console.log("\nProcessing requests:");
for (const req of requests) {
    const resp: Response = handleRequest(req);
    const statusType: string = isSuccessStatus(resp.status) ? "OK" : "ERROR";
    console.log("  -> " + resp.status + " (" + statusType + ")");
}

// --- Enum as Object Pattern ---
console.log("\n--- Enum as Object Pattern ---");

// Enums compile to objects, so we can iterate over them
console.log("Direction enum object:");
console.log("  Keys:", JSON.stringify(Object.keys(Direction)));
console.log("  Values:", JSON.stringify(Object.values(Direction)));

console.log("\nLogLevel enum object:");
console.log("  Keys:", JSON.stringify(Object.keys(LogLevel)));
console.log("  Values:", JSON.stringify(Object.values(LogLevel)));

console.log("\n=== Demo Complete ===");
