import dotenv from "dotenv";
dotenv.config({ path: ".env.e2e" });

const REQUIRED_ENV_VARS = [
  "API_BASE_URL",
  "APP_BASE_URL",
  "E2E_USER_EMAIL_0",
  "E2E_USER_PASSWORD_0",
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

export const ADMIN_EMAIL = process.env.E2E_ADMIN_EMAIL!;
export const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD!;

/**
 * Number of parallel Playwright workers. Each worker needs its own
 * pre-provisioned Keycloak user (see getWorkerTestUser below). Defaults
 * to 1 so the suite runs without extra Keycloak provisioning.
 */
export const PARALLEL_WORKERS = parseInt(process.env.E2E_WORKERS ?? "1", 10);

/**
 * Returns the Keycloak test user credentials for a given worker index.
 * Each parallel Playwright worker needs its own distinct Keycloak user
 * because tests clean up member rows in beforeEach — sharing would race.
 */
export function getWorkerTestUser(workerIndex: number): {
  email: string;
  password: string;
} {
  const email = process.env[`E2E_USER_EMAIL_${workerIndex}`];
  const password = process.env[`E2E_USER_PASSWORD_${workerIndex}`];
  if (!email || !password) {
    throw new Error(
      `Missing E2E_USER_EMAIL_${workerIndex}/E2E_USER_PASSWORD_${workerIndex} ` +
        `env vars. Each parallel worker needs its own Keycloak user. ` +
        `Either provision one in Keycloak and set these vars, or lower E2E_WORKERS.`,
    );
  }
  return { email, password };
}

/**
 * Per-worker test role. Role slug is worker-scoped so parallel tests
 * don't fight over the same row in the `role` table. The display name
 * mirrors the frontend's kebab-case → title-case formatting (see
 * frontend/src/lib/utils.ts: kebabCaseToTitleCase).
 */
export function getWorkerTestRole(workerIndex: number): {
  name: string;
  displayName: string;
} {
  const name = `e2e-test-role-${workerIndex}`;
  const displayName = name
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
    .join(" ");
  return { name, displayName };
}

export const TEST_ROLE_VALID_UNTIL = "2099-12-31";
