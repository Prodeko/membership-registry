import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { ApplicationFormPage } from "../pages/application-form.page";
import { ApplicationSuccessPage } from "../pages/application-success.page";
import { loginViaKeycloak } from "../helpers/auth";
import { APP_BASE_URL, TEST_ROLE_VALID_UNTIL } from "../helpers/constants";

// Written by auth.setup.ts.
const ADMIN_AUTH_FILE = ".auth/admin.json";

// Per-worker attribute name so parallel workers don't collide on the
// AttributeDefinition PK.
const attributeName = (workerIndex: number) =>
  `e2e-major-subject-${workerIndex}`;

test.describe("Application form attributes", () => {
  test.beforeEach(async ({ db, adminApi, testUser, testRole }, testInfo) => {
    const attr = attributeName(testInfo.parallelIndex);

    await db.cleanupTestUser(testUser.email);
    await db.cleanupTestRole(testRole.name);
    // Attribute definition must be torn down separately — it has no CASCADE
    // path from role/member cleanup.
    await adminApi.deleteAttributeDefinition(attr);

    await adminApi.createRole(testRole.name);
    // Internal-only attribute (sync_to_keycloak=false) so the test doesn't
    // need a Keycloak mapper round trip. Both editable so the form-time
    // bypass and the post-registration profile path both work. Required, so
    // the form can't be submitted until it's filled.
    await adminApi.createAttributeDefinition({
      name: attr,
      allowed_values: ["iem", "other"],
      editable_by: "both",
      required: true,
    });
    await adminApi.createTargetableRole(
      testRole.name,
      TEST_ROLE_VALID_UNTIL,
      undefined,
      { form_attributes: [attr] },
    );
  });

  test.afterEach(async ({ db, adminApi, testUser, testRole }, testInfo) => {
    await db.cleanupTestUser(testUser.email);
    await db.cleanupTestRole(testRole.name);
    await adminApi.deleteAttributeDefinition(
      attributeName(testInfo.parallelIndex),
    );
  });

  // Fresh login — beforeEach removed the test user.
  test.use({ storageState: { cookies: [], origins: [] } });

  test("applicant fills a form attribute, the value lands in MemberAttribute and admin sees it on the application and in the list", async ({
    browser,
    page,
    db,
    testUser,
    testRole,
  }, testInfo) => {
    const attr = attributeName(testInfo.parallelIndex);

    await loginViaKeycloak(page, testUser.email, testUser.password);

    const home = new HomePage(page);
    await home.waitForLoaded();
    await home.clickApply();

    const appForm = new ApplicationFormPage(page);
    await appForm.waitForLoaded();
    await appForm.selectRole(testRole.displayName);

    // Selecting the role must reveal the attribute field — this is the
    // load-bearing UI behavior the targetable-role's form_attributes drives.
    await expect(page.getByTestId(`application-attr-${attr}`)).toBeVisible();

    // Required and still empty: submit is blocked until it's filled.
    const submit = page.getByTestId("submit-application-button");
    await expect(submit).toBeDisabled();
    await expect(
      page.getByTestId("application-required-missing"),
    ).toBeVisible();

    await appForm.setAttribute(attr, "iem");
    await expect(submit).toBeEnabled();
    await appForm.fillApplicationText("E2E form-attribute test");
    await appForm.submit();

    // Land on success page (no payment link configured for this role).
    await page.waitForURL("**/apply/success");
    const success = new ApplicationSuccessPage(page);
    await success.waitForLoaded();

    // Application row exists.
    const dbApp = await db.getApplicationByUserEmail(testUser.email);
    expect(dbApp).not.toBeNull();
    expect(dbApp!.status).toBe("pending");

    // Submitted attribute value persists in MemberAttribute. Joining
    // through member by email avoids embedding the user_id in the test.
    const attrRows = await db.query<{ value: string }>(
      `SELECT ma.value
       FROM memberattribute ma
       JOIN member m ON ma.user_id = m.user_id
       WHERE m.email = $1 AND ma.attribute_name = $2`,
      [testUser.email, attr],
    );
    expect(attrRows).toHaveLength(1);
    expect(attrRows[0].value).toBe("iem");

    // Admin reviewing the application sees the submitted value. Separate
    // context so the applicant's session doesn't leak into the admin one.
    const adminContext = await browser.newContext({
      storageState: ADMIN_AUTH_FILE,
    });
    try {
      const adminPage = await adminContext.newPage();
      await adminPage.goto(
        `${APP_BASE_URL}/applications/${dbApp!.application_id}`,
      );
      await expect(
        adminPage.getByTestId("application-attributes"),
      ).toBeVisible();
      await expect(
        adminPage.getByTestId(`application-attr-value-${attr}`),
      ).toHaveText("iem");

      // The application list has a column for the form attribute, and the
      // applicant's row shows the submitted value in it.
      await adminPage.goto(`${APP_BASE_URL}/applications?status=pending`);
      await expect(
        adminPage.getByRole("columnheader", { name: attr }),
      ).toBeVisible();
      const row = adminPage.getByRole("row").filter({
        has: adminPage.locator(
          `a[href="/applications/${dbApp!.application_id}"]`,
        ),
      });
      await expect(row).toContainText("iem");
    } finally {
      await adminContext.close();
    }
  });
});
