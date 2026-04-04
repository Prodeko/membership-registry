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

export const KEYCLOAK_URL = process.env.KEYCLOAK_URL!;
export const KEYCLOAK_REALM = process.env.KEYCLOAK_REALM!;
export const KEYCLOAK_ADMIN_CLIENT_ID = process.env.KEYCLOAK_ADMIN_CLIENT_ID!;
export const KEYCLOAK_ADMIN_CLIENT_SECRET = process.env.KEYCLOAK_ADMIN_CLIENT_SECRET!;
