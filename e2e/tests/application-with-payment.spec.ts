import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { ApplicationFormPage } from "../pages/application-form.page";
import { loginViaKeycloak } from "../helpers/auth";
import {
  TEST_USER_EMAIL,
  TEST_USER_PASSWORD,
  TEST_ROLE_NAME,
  TEST_ROLE_VALID_UNTIL,
} from "../helpers/constants";

const TEST_PAYMENT_LINK = "https://buy.stripe.com/test_e2e_fake";

test.describe("Application with payment", () => {
  test.beforeEach(async ({ db, adminApi }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
    await adminApi.createRole(TEST_ROLE_NAME);
    await adminApi.createTargetableRole(
      TEST_ROLE_NAME,
      TEST_ROLE_VALID_UNTIL,
      TEST_PAYMENT_LINK,
    );
  });

  test.afterEach(async ({ db }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
  });

  // Fresh login needed — beforeEach cleans up the test user
  test.use({ storageState: { cookies: [], origins: [] } });

  test("application redirects to stripe with correct client_reference_id", async ({
    page,
    db,
    adminApi,
  }) => {
    await loginViaKeycloak(page, TEST_USER_EMAIL, TEST_USER_PASSWORD);

    // Complete profile via admin API so we skip signup fields
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

    // Navigate to apply
    const home = new HomePage(page);
    await home.waitForLoaded();
    await home.clickApply();

    // Fill and submit application
    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();
    await appForm.selectRole("E2e Test Role");
    await appForm.fillApplicationText("E2E payment test");
    await appForm.submit();

    // Should redirect to Stripe payment link with client_reference_id
    await page.waitForURL(`${TEST_PAYMENT_LINK}**`, { timeout: 10_000 });
    const url = new URL(page.url());
    expect(url.origin + url.pathname).toBe(TEST_PAYMENT_LINK);
    const clientRefId = url.searchParams.get("client_reference_id");
    expect(clientRefId).toBeTruthy();

    // Verify application exists in DB with unpaid status
    const dbApp = await db.getApplicationByUserEmail(TEST_USER_EMAIL);
    expect(dbApp).not.toBeNull();
    expect(dbApp!.status).toBe("unpaid");
    expect(dbApp!.application_id).toBe(clientRefId);
  });

  test("admin can approve unpaid application after payment", async ({
    page,
    db,
    adminApi,
  }) => {
    await loginViaKeycloak(page, TEST_USER_EMAIL, TEST_USER_PASSWORD);

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

    // Create application via UI
    const home = new HomePage(page);
    await home.waitForLoaded();
    await home.clickApply();

    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();
    await appForm.selectRole("E2e Test Role");
    await appForm.fillApplicationText("E2E payment flow test");
    await appForm.submit();

    // Wait for Stripe redirect, then simulate payment + approval
    await page.waitForURL(`${TEST_PAYMENT_LINK}**`, { timeout: 10_000 });

    const dbApp = await db.getApplicationByUserEmail(TEST_USER_EMAIL);
    expect(dbApp).not.toBeNull();
    await adminApi.approveApplication(dbApp!.application_id as string);

    // Go home and verify the application shows as approved
    await page.goto("/home");
    await home.waitForLoaded();

    const apps = await home.getApplications();
    expect(apps.length).toBe(1);

    const updatedApp = await db.getApplicationByUserEmail(TEST_USER_EMAIL);
    expect(updatedApp!.status).toBe("approved");
  });
});
