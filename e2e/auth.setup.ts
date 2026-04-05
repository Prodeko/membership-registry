import { test as setup } from "@playwright/test";
import { KeycloakLoginPage } from "./pages/keycloak-login.page";
import {
  API_BASE_URL,
  TEST_USER_EMAIL,
  TEST_USER_PASSWORD,
  ADMIN_EMAIL,
  ADMIN_PASSWORD,
} from "./helpers/constants";

const USER_AUTH_FILE = ".auth/user.json";
const ADMIN_AUTH_FILE = ".auth/admin.json";

setup("authenticate as test user", async ({ page }) => {
  await page.goto(`${API_BASE_URL}/auth/login`);
  const keycloak = new KeycloakLoginPage(page);
  await keycloak.login(TEST_USER_EMAIL, TEST_USER_PASSWORD);
  await page.waitForURL("**/home", { timeout: 30_000 });
  await page.context().storageState({ path: USER_AUTH_FILE });
});

setup("authenticate as admin", async ({ page }) => {
  await page.goto(`${API_BASE_URL}/auth/login`);
  const keycloak = new KeycloakLoginPage(page);
  await keycloak.login(ADMIN_EMAIL, ADMIN_PASSWORD);
  await page.waitForURL("**/home", { timeout: 30_000 });
  await page.context().storageState({ path: ADMIN_AUTH_FILE });
});
