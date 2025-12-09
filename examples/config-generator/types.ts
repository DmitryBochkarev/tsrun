// TypeScript interfaces for configuration
// These are parsed but stripped at runtime - serve as documentation

export interface DatabaseConfig {
  host: string;
  port: number;
  database: string;
  username: string;
  password?: string;
  ssl: boolean;
  poolSize: number;
}

export interface ServerConfig {
  host: string;
  port: number;
  cors: {
    enabled: boolean;
    origins: string[];
  };
  rateLimit: {
    enabled: boolean;
    maxRequests: number;
    windowMs: number;
  };
}

export interface LoggingConfig {
  level: "debug" | "info" | "warn" | "error";
  format: "json" | "text";
  outputs: string[];
}

export interface AppConfig {
  name: string;
  version: string;
  environment: "development" | "staging" | "production";
  database: DatabaseConfig;
  server: ServerConfig;
  logging: LoggingConfig;
  features: {
    [key: string]: boolean;
  };
}
