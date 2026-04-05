import dotenv from "dotenv";
dotenv.config({ path: ".env.e2e" });

const REQUIRED_ENV_VARS = [
  "API_BASE_URL",
  "APP_BASE_URL",
  "E2E_USER_EMAIL",
  "E2E_USER_PASSWORD",
  "E2E_ADMIN_EMAIL",
  "E2E_ADMIN_PASSWORD",
  "DATABASE_URL",
] as const;

const missing = REQUIRED_ENV_VARS.filter((v) => !process.env[v]);
if (missing.length > 0) {
  throw new Error(`Missing required env vars in .env.e2e: ${missing.join(", ")}`);
}

export const API_BASE_URL = process.env.API_BASE_URL!;
export const APP_BASE_URL = process.env.APP_BASE_URL!;

export const TEST_USER_EMAIL = process.env.E2E_USER_EMAIL!;
export const TEST_USER_PASSWORD = process.env.E2E_USER_PASSWORD!;
export const ADMIN_EMAIL = process.env.E2E_ADMIN_EMAIL!;
export const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD!;

export const TEST_ROLE_NAME = "e2e-test-role";
export const TEST_ROLE_VALID_UNTIL = "2099-12-31";
