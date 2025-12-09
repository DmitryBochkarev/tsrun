// Config Generator Example
// Demonstrates: object literals, interfaces, spread operator, template literals, default values

import { AppConfig } from "./types";
import {
  DEFAULT_DATABASE_CONFIG,
  DEFAULT_SERVER_CONFIG,
  DEFAULT_LOGGING_CONFIG,
  DEFAULT_FEATURES,
} from "./defaults";

// Environment-specific overrides
const ENV = "production";

const envOverrides: Record<string, Partial<AppConfig>> = {
  development: {
    database: {
      ...DEFAULT_DATABASE_CONFIG,
      database: "app_dev",
    },
    logging: {
      ...DEFAULT_LOGGING_CONFIG,
      level: "debug",
      format: "text",
    },
  },
  staging: {
    database: {
      ...DEFAULT_DATABASE_CONFIG,
      host: "staging-db.example.com",
      database: "app_staging",
      ssl: true,
    },
    server: {
      ...DEFAULT_SERVER_CONFIG,
      cors: {
        enabled: true,
        origins: ["https://staging.example.com"],
      },
    },
  },
  production: {
    database: {
      ...DEFAULT_DATABASE_CONFIG,
      host: "prod-db.example.com",
      database: "app_prod",
      ssl: true,
      poolSize: 50,
    },
    server: {
      ...DEFAULT_SERVER_CONFIG,
      cors: {
        enabled: true,
        origins: ["https://example.com", "https://www.example.com"],
      },
      rateLimit: {
        enabled: true,
        maxRequests: 1000,
        windowMs: 60000,
      },
    },
    logging: {
      ...DEFAULT_LOGGING_CONFIG,
      level: "warn",
      outputs: ["stdout", "file"],
    },
    features: {
      ...DEFAULT_FEATURES,
      analytics: true,
    },
  },
};

// Build the final configuration
function buildConfig(env: string): AppConfig {
  const overrides = envOverrides[env] || {};

  return {
    name: "MyApp",
    version: "1.0.0",
    environment: env as "development" | "staging" | "production",
    database: {
      ...DEFAULT_DATABASE_CONFIG,
      ...(overrides.database || {}),
    },
    server: {
      ...DEFAULT_SERVER_CONFIG,
      ...(overrides.server || {}),
    },
    logging: {
      ...DEFAULT_LOGGING_CONFIG,
      ...(overrides.logging || {}),
    },
    features: {
      ...DEFAULT_FEATURES,
      ...(overrides.features || {}),
    },
  };
}

// Generate the configuration
const config = buildConfig(ENV);

// Output as JSON
JSON.stringify(config, null, 2);
