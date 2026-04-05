import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { ApplicationFormPage } from "../pages/application-form.page";
import { ApplicationSuccessPage } from "../pages/application-success.page";
import { loginViaKeycloak } from "../helpers/auth";
import {
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
    await adminApi.createTargetableRole(TEST_ROLE_NAME, TEST_ROLE_VALID_UNTIL);
  });

  test.afterEach(async ({ db }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
  });

  // This test needs a fresh login (no storageState) because it tests the new-user signup flow.
  test.use({ storageState: { cookies: [], origins: [] } });

  test("new user can log in and submit an application", async ({
    page,
    db,
  }) => {
    await loginViaKeycloak(page, TEST_USER_EMAIL, TEST_USER_PASSWORD);

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
    const loginRows = await db.query<{ count: string }>(
      `SELECT COUNT(*) as count FROM audit_log WHERE action = $1 AND actor_user_id IN (SELECT user_id FROM member WHERE email = $2)`,
      ["auth.login", TEST_USER_EMAIL],
    );
    expect(parseInt(loginRows[0].count, 10)).toBeGreaterThanOrEqual(1);

    const memberCreateRows = await db.query<{ count: string }>(
      `SELECT COUNT(*) as count FROM audit_log WHERE action = $1 AND actor_user_id IN (SELECT user_id FROM member WHERE email = $2)`,
      ["member.create", TEST_USER_EMAIL],
    );
    expect(parseInt(memberCreateRows[0].count, 10)).toBe(1);

    const appCreateRows = await db.query<{ count: string }>(
      `SELECT COUNT(*) as count FROM audit_log WHERE action = $1 AND actor_user_id IN (SELECT user_id FROM member WHERE email = $2)`,
      ["application.create", TEST_USER_EMAIL],
    );
    expect(parseInt(appCreateRows[0].count, 10)).toBe(1);
  });

  test("returning user does not see policies and municipality fields in application flow", async ({
    page,
    db,
    adminApi,
  }) => {
    // Need fresh login to create the member first
    await loginViaKeycloak(page, TEST_USER_EMAIL, TEST_USER_PASSWORD);

    // Complete profile via admin API
    const member = await db.getMemberByEmail(TEST_USER_EMAIL);
    expect(member).not.toBeNull();
    await adminApi.updateMember(member!.user_id as string, {
      first_name: member!.first_name as string,
      last_name: member!.last_name as string,
      home_municipality: "Helsinki",
      has_accepted_policies: true,
      email_notifications: false,
      language: "en",
    });

    // Navigate to apply page
    await page.goto("/apply");
    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();

    // Returning user with complete profile should NOT see these fields
    expect(await appForm.isMunicipalityFieldVisible()).toBe(false);
    expect(await appForm.isPoliciesFieldVisible()).toBe(false);
  });
});
