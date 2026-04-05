import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { ApplicationFormPage } from "../pages/application-form.page";
import { loginViaKeycloak } from "../helpers/auth";
import { completeMemberProfile } from "../helpers/profile";
import { findKeycloakUserByEmail, getUserRealmRoles } from "../helpers/keycloak-api";
import { clearCapturedEmails, getCapturedEmailsForRecipient } from "../helpers/email-api";
import {
  TEST_USER_EMAIL,
  TEST_USER_PASSWORD,
  TEST_ROLE_NAME,
  TEST_ROLE_VALID_UNTIL,
} from "../helpers/constants";

const TEST_PAYMENT_LINK = "https://buy.stripe.com/test_e2e_fake";
const TEST_EMAIL_TEMPLATE = "e2e-approval-template";

test.describe("Application with payment", () => {
  test.beforeEach(async ({ db, adminApi }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
    await clearCapturedEmails();

    await adminApi.createRole(TEST_ROLE_NAME);

    // Set up email template for approval notifications
    await adminApi.createEmailTemplate(TEST_EMAIL_TEMPLATE);
    await adminApi.upsertEmailTranslation(
      TEST_EMAIL_TEMPLATE,
      "en",
      "Your application has been approved",
      "<p>Congratulations, your application for {{role_name}} has been approved.</p>",
    );

    await adminApi.createTargetableRole(
      TEST_ROLE_NAME,
      TEST_ROLE_VALID_UNTIL,
      TEST_PAYMENT_LINK,
      { approved_email_template: TEST_EMAIL_TEMPLATE },
    );
  });

  test.afterEach(async ({ db, adminApi }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
    await adminApi.deleteEmailTemplate(TEST_EMAIL_TEMPLATE);
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
    await completeMemberProfile(db, adminApi, TEST_USER_EMAIL);

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

  test("approval assigns keycloak role and sends notification email", async ({
    page,
    db,
    adminApi,
  }) => {
    await loginViaKeycloak(page, TEST_USER_EMAIL, TEST_USER_PASSWORD);

    await completeMemberProfile(db, adminApi, TEST_USER_EMAIL);

    // Create application via UI
    const home = new HomePage(page);
    await home.waitForLoaded();
    await home.clickApply();

    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();
    await appForm.selectRole("E2e Test Role");
    await appForm.fillApplicationText("E2E payment flow test");
    await appForm.submit();

    // Wait for Stripe redirect, then approve
    await page.waitForURL(`${TEST_PAYMENT_LINK}**`, { timeout: 10_000 });

    const dbApp = await db.getApplicationByUserEmail(TEST_USER_EMAIL);
    expect(dbApp).not.toBeNull();
    await adminApi.approveApplication(dbApp!.application_id as string);

    // Verify application status
    const updatedApp = await db.getApplicationByUserEmail(TEST_USER_EMAIL);
    expect(updatedApp!.status).toBe("approved");

    // Verify Keycloak role was assigned
    const kcUserId = await findKeycloakUserByEmail(TEST_USER_EMAIL);
    expect(kcUserId).not.toBeNull();
    const kcRoles = await getUserRealmRoles(kcUserId!);
    expect(kcRoles).toContain(TEST_ROLE_NAME);

    // Verify approval email was sent (email send is async, poll until it arrives)
    await expect
      .poll(
        async () =>
          (await getCapturedEmailsForRecipient(TEST_USER_EMAIL)).length,
        { timeout: 10_000 },
      )
      .toBeGreaterThan(0);
    const emails = await getCapturedEmailsForRecipient(TEST_USER_EMAIL);
    expect(emails.length).toBe(1);
    expect(emails[0].subject).toContain("approved");

    // Verify UI reflects the approved state
    await page.goto("/home");
    await home.waitForLoaded();

    const apps = await home.getApplications();
    expect(apps.length).toBe(1);
  });
});
