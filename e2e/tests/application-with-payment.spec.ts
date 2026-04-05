import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { ApplicationFormPage } from "../pages/application-form.page";
import { loginViaKeycloak } from "../helpers/auth";
import { completeMemberProfile } from "../helpers/profile";
import { findKeycloakUserByEmail, getUserRealmRoles } from "../helpers/keycloak-api";
import {
  clearCapturedEmailsForRecipient,
  getCapturedEmailsForRecipient,
} from "../helpers/email-api";
import { TEST_ROLE_VALID_UNTIL } from "../helpers/constants";

const TEST_PAYMENT_LINK = "https://buy.stripe.com/test_e2e_fake";

test.describe("Application with payment", () => {
  // Per-worker email template name so parallel workers don't collide
  // on the email_template PK.
  const emailTemplateName = (workerIndex: number) =>
    `e2e-approval-template-${workerIndex}`;

  test.beforeEach(async ({ db, adminApi, testUser, testRole }, testInfo) => {
    const templateName = emailTemplateName(testInfo.parallelIndex);

    await db.cleanupTestUser(testUser.email);
    await db.cleanupTestRole(testRole.name);
    await clearCapturedEmailsForRecipient(testUser.email);

    await adminApi.createRole(testRole.name);

    // Set up email template for approval notifications
    await adminApi.createEmailTemplate(templateName);
    await adminApi.upsertEmailTranslation(
      templateName,
      "en",
      "Your application has been approved",
      "<p>Congratulations, your application for {{role_name}} has been approved.</p>",
    );

    await adminApi.createTargetableRole(
      testRole.name,
      TEST_ROLE_VALID_UNTIL,
      TEST_PAYMENT_LINK,
      { approved_email_template: templateName },
    );
  });

  test.afterEach(async ({ db, adminApi, testUser, testRole }, testInfo) => {
    await db.cleanupTestUser(testUser.email);
    await db.cleanupTestRole(testRole.name);
    await adminApi.deleteEmailTemplate(emailTemplateName(testInfo.parallelIndex));
  });

  // Fresh login needed — beforeEach cleans up the test user
  test.use({ storageState: { cookies: [], origins: [] } });

  test("application redirects to stripe with correct client_reference_id", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }) => {
    await loginViaKeycloak(page, testUser.email, testUser.password);

    // Complete profile via admin API so we skip signup fields
    await completeMemberProfile(db, adminApi, testUser.email);

    // Navigate to apply
    const home = new HomePage(page);
    await home.waitForLoaded();
    await home.clickApply();

    // Fill and submit application
    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();
    await appForm.selectRole(testRole.displayName);
    await appForm.fillApplicationText("E2E payment test");
    await appForm.submit();

    // Should redirect to Stripe payment link with client_reference_id
    await page.waitForURL(`${TEST_PAYMENT_LINK}**`, { timeout: 10_000 });
    const url = new URL(page.url());
    expect(url.origin + url.pathname).toBe(TEST_PAYMENT_LINK);
    const clientRefId = url.searchParams.get("client_reference_id");
    expect(clientRefId).toBeTruthy();

    // Verify application exists in DB with unpaid status
    const dbApp = await db.getApplicationByUserEmail(testUser.email);
    expect(dbApp).not.toBeNull();
    expect(dbApp!.status).toBe("unpaid");
    expect(dbApp!.application_id).toBe(clientRefId);
  });

  test("approval assigns keycloak role and sends notification email", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }) => {
    await loginViaKeycloak(page, testUser.email, testUser.password);

    await completeMemberProfile(db, adminApi, testUser.email);

    // Create application via UI
    const home = new HomePage(page);
    await home.waitForLoaded();
    await home.clickApply();

    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();
    await appForm.selectRole(testRole.displayName);
    await appForm.fillApplicationText("E2E payment flow test");
    await appForm.submit();

    // Wait for Stripe redirect, then approve
    await page.waitForURL(`${TEST_PAYMENT_LINK}**`, { timeout: 10_000 });

    const dbApp = await db.getApplicationByUserEmail(testUser.email);
    expect(dbApp).not.toBeNull();
    await adminApi.approveApplication(dbApp!.application_id as string);

    // Verify application status
    const updatedApp = await db.getApplicationByUserEmail(testUser.email);
    expect(updatedApp!.status).toBe("approved");

    // Verify Keycloak role was assigned
    const kcUserId = await findKeycloakUserByEmail(testUser.email);
    expect(kcUserId).not.toBeNull();
    const kcRoles = await getUserRealmRoles(kcUserId!);
    expect(kcRoles).toContain(testRole.name);

    // Verify approval email was sent (email send is async, poll until it arrives)
    await expect
      .poll(
        async () =>
          (await getCapturedEmailsForRecipient(testUser.email)).length,
        { timeout: 10_000 },
      )
      .toBeGreaterThan(0);
    const emails = await getCapturedEmailsForRecipient(testUser.email);
    expect(emails.length).toBe(1);
    expect(emails[0].subject).toContain("approved");

    // Verify UI reflects the approved state
    await page.goto("/home");
    await home.waitForLoaded();

    const apps = await home.getApplications();
    expect(apps.length).toBe(1);
  });
});
