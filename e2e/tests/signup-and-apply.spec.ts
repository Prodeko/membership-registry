import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { ApplicationFormPage } from "../pages/application-form.page";
import { ApplicationSuccessPage } from "../pages/application-success.page";
import { KeycloakLoginPage } from "../pages/keycloak-login.page";
import {
  API_BASE_URL,
  TEST_USER_EMAIL,
  TEST_USER_PASSWORD,
  TEST_ROLE_NAME,
  TEST_ROLE_VALID_UNTIL,
} from "../helpers/constants";

test.describe("Signup and apply", () => {
  test.beforeEach(async ({ db, adminApi }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
    await adminApi.createRole(TEST_ROLE_NAME);
    await db.ensureTargetableRole(TEST_ROLE_NAME, TEST_ROLE_VALID_UNTIL, true);
  });

  test.afterEach(async ({ db }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
  });

  test("new user can log in and submit an application", async ({
    page,
    db,
  }) => {
    // Login via Keycloak
    await page.goto(`${API_BASE_URL}/auth/login`);
    const keycloak = new KeycloakLoginPage(page);
    await keycloak.login(TEST_USER_EMAIL, TEST_USER_PASSWORD);

    // OAuth callback redirects to /home — backend auto-creates member
    await page.waitForURL("**/home", { timeout: 30_000 });
    const home = new HomePage(page);
    await home.waitForLoaded();

    // New user should see the apply button
    await expect(page.getByTestId("apply-button")).toBeVisible();
    await home.clickApply();

    // Fill the application form (includes signup fields for new user)
    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();

    // New user should see municipality and policies fields
    expect(await appForm.isMunicipalityFieldVisible()).toBe(true);
    expect(await appForm.isPoliciesFieldVisible()).toBe(true);

    await appForm.selectMunicipality("Helsinki");
    await appForm.acceptPolicies();
    await appForm.selectRole("E2e Test Role");
    await appForm.fillApplicationText("E2E test application");
    await appForm.submit();

    // Should land on success page
    await page.waitForURL("**/apply/success");
    const success = new ApplicationSuccessPage(page);
    await success.waitForLoaded();

    // Go back to home and verify the application shows
    await success.clickGoHome();
    await page.waitForURL("**/home");
    await home.waitForLoaded();

    const apps = await home.getApplications();
    expect(apps.length).toBe(1);

    // Verify application status in DB (UI text is locale-dependent)
    const dbApp = await db.getApplicationByUserEmail(TEST_USER_EMAIL);
    expect(dbApp).not.toBeNull();
    expect(dbApp!.status).toBe("pending");

    // Verify audit log entries
    const loginCount = await db.count(
      "audit_log",
      "action = $1 AND actor_user_id IN (SELECT user_id FROM member WHERE email = $2)",
      ["auth.login", TEST_USER_EMAIL],
    );
    expect(loginCount).toBeGreaterThanOrEqual(1);

    const memberCreateCount = await db.count(
      "audit_log",
      "action = $1 AND actor_user_id IN (SELECT user_id FROM member WHERE email = $2)",
      ["member.create", TEST_USER_EMAIL],
    );
    expect(memberCreateCount).toBe(1);

    const appCreateCount = await db.count(
      "audit_log",
      "action = $1 AND actor_user_id IN (SELECT user_id FROM member WHERE email = $2)",
      ["application.create", TEST_USER_EMAIL],
    );
    expect(appCreateCount).toBe(1);
  });

  test("returning user does not see policies and municipality fields in application flow", async ({
    page,
    db,
  }) => {
    // Seed: create member with complete profile by logging in first, then updating
    await page.goto(`${API_BASE_URL}/auth/login`);
    const keycloak = new KeycloakLoginPage(page);
    await keycloak.login(TEST_USER_EMAIL, TEST_USER_PASSWORD);
    await page.waitForURL("**/home", { timeout: 30_000 });

    // Set municipality and policies via DB
    await db.query(
      `UPDATE member SET home_municipality = 'Helsinki', has_accepted_policies = true WHERE email = $1`,
      [TEST_USER_EMAIL],
    );

    // Navigate to apply page
    await page.goto("/apply");
    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();

    // Returning user with complete profile should NOT see these fields
    expect(await appForm.isMunicipalityFieldVisible()).toBe(false);
    expect(await appForm.isPoliciesFieldVisible()).toBe(false);
  });
});
