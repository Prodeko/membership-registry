import { type BrowserContext, type Page, chromium } from "@playwright/test";
import { KeycloakLoginPage } from "../pages/keycloak-login.page";
import { API_BASE_URL, TEST_USER_EMAIL, TEST_USER_PASSWORD } from "../helpers/constants";

let cachedContext: BrowserContext | null = null;
let cachedPage: Page | null = null;

/**
 * Perform Keycloak login and return an authenticated page.
 * Caches the browser context across calls within the same worker.
 */
export async function getAuthenticatedUserPage(): Promise<Page> {
  if (cachedPage && !cachedPage.isClosed()) {
    return cachedPage;
  }

  const browser = await chromium.launch();
  cachedContext = await browser.newContext();
  cachedPage = await cachedContext.newPage();

  // Navigate to login endpoint which redirects to Keycloak
  await cachedPage.goto(`${API_BASE_URL}/auth/login`);

  const keycloakLogin = new KeycloakLoginPage(cachedPage);
  await keycloakLogin.login(TEST_USER_EMAIL, TEST_USER_PASSWORD);

  // Wait for OAuth callback to complete and land on /home
  await cachedPage.waitForURL("**/home", { timeout: 30_000 });

  return cachedPage;
}

export async function cleanupAuthContext(): Promise<void> {
  if (cachedContext) {
    await cachedContext.close();
    cachedContext = null;
    cachedPage = null;
  }
}
