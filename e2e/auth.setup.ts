import { test as setup } from "@playwright/test";
import { loginViaKeycloak } from "./helpers/auth";
import { ADMIN_EMAIL, ADMIN_PASSWORD } from "./helpers/constants";

const ADMIN_AUTH_FILE = ".auth/admin.json";

// Only the admin storage state is persisted: it's consumed by AdminApiHelper
// (fixtures/api.fixture.ts) to call admin endpoints from tests. Test users
// log in fresh in each test because the suite exercises first-login side
// effects and cleans up member rows in beforeEach.
//
// Transient Keycloak flakes (slow realm import, cold container) are handled
// by `retries: 2` on the setup project in playwright.config.ts.
setup("authenticate as admin", async ({ page }) => {
  // Admins are gated by the same onboarding requirement as regular users.
  // loginViaKeycloak auto-completes onboarding so the admin lands on /home.
  await loginViaKeycloak(page, ADMIN_EMAIL, ADMIN_PASSWORD);
  await page.context().storageState({ path: ADMIN_AUTH_FILE });
});
