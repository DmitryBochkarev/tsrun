// TypeScript Enum Patterns

// Basic numeric enum
export enum Direction {
    Up,
    Down,
    Left,
    Right
}

// Numeric enum with explicit values
export enum StatusCode {
    OK = 200,
    Created = 201,
    BadRequest = 400,
    Unauthorized = 401,
    NotFound = 404,
    InternalError = 500
}

// String enum
export enum LogLevel {
    Debug = "DEBUG",
    Info = "INFO",
    Warn = "WARN",
    Error = "ERROR"
}

// Heterogeneous enum (mixed types)
export enum Mixed {
    No = 0,
    Yes = "YES"
}

// Enum with computed values
export enum FileAccess {
    None = 0,
    Read = 1,
    Write = 2,
    ReadWrite = Read | Write,
    Execute = 4,
    All = Read | Write | Execute
}

// Using enums
export function getDirectionName(dir: Direction): string {
    switch (dir) {
        case Direction.Up: return "Up";
        case Direction.Down: return "Down";
        case Direction.Left: return "Left";
        case Direction.Right: return "Right";
        default: return "Unknown";
    }
}

export function isSuccessStatus(code: StatusCode): boolean {
    return code >= 200 && code < 300;
}

export function formatLogMessage(level: LogLevel, message: string): string {
    return "[" + level + "] " + message;
}

export function hasPermission(access: FileAccess, permission: FileAccess): boolean {
    return (access & permission) === permission;
}

// Enum iteration
export function getAllDirections(): Direction[] {
    return [Direction.Up, Direction.Down, Direction.Left, Direction.Right];
}

export function getAllLogLevels(): LogLevel[] {
    return [LogLevel.Debug, LogLevel.Info, LogLevel.Warn, LogLevel.Error];
}
