import { test as setup } from "@playwright/test";
import { KeycloakLoginPage } from "./pages/keycloak-login.page";
import { API_BASE_URL, ADMIN_EMAIL, ADMIN_PASSWORD } from "./helpers/constants";

const ADMIN_AUTH_FILE = ".auth/admin.json";

// Only the admin storage state is persisted: it's consumed by AdminApiHelper
// (fixtures/api.fixture.ts) to call admin endpoints from tests. Test users
// log in fresh in each test because the suite exercises first-login side
// effects and cleans up member rows in beforeEach.
//
// Transient Keycloak flakes (slow realm import, cold container) are handled
// by `retries: 2` on the setup project in playwright.config.ts.
setup("authenticate as admin", async ({ page }) => {
  await page.goto(`${API_BASE_URL}/auth/login`);
  const keycloak = new KeycloakLoginPage(page);
  await keycloak.login(ADMIN_EMAIL, ADMIN_PASSWORD);
  await page.waitForURL("**/home", { timeout: 30_000 });
  await page.context().storageState({ path: ADMIN_AUTH_FILE });
});
