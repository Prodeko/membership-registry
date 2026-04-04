import dotenv from "dotenv";
dotenv.config({ path: ".env.e2e" });

export const API_BASE_URL = process.env.API_BASE_URL!;
export const APP_BASE_URL = process.env.APP_BASE_URL!;

export const TEST_USER_EMAIL = process.env.E2E_USER_EMAIL!;
export const TEST_USER_PASSWORD = process.env.E2E_USER_PASSWORD!;
export const ADMIN_EMAIL = process.env.E2E_ADMIN_EMAIL!;
export const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD!;

export const TEST_ROLE_NAME = "e2e-test-role";
export const TEST_ROLE_VALID_UNTIL = "2099-12-31";
