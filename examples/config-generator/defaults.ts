// Default configuration values

export const DEFAULT_DATABASE_CONFIG = {
  host: "localhost",
  port: 5432,
  database: "app_db",
  username: "app_user",
  ssl: false,
  poolSize: 10,
};

export const DEFAULT_SERVER_CONFIG = {
  host: "0.0.0.0",
  port: 3000,
  cors: {
    enabled: true,
    origins: ["http://localhost:3000"],
  },
  rateLimit: {
    enabled: true,
    maxRequests: 100,
    windowMs: 60000,
  },
};

export const DEFAULT_LOGGING_CONFIG = {
  level: "info",
  format: "json",
  outputs: ["stdout"],
};

export const DEFAULT_FEATURES = {
  darkMode: true,
  analytics: false,
  betaFeatures: false,
};
