import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { ApplicationFormPage } from "../pages/application-form.page";
import { ApplicationSuccessPage } from "../pages/application-success.page";
import { loginViaKeycloak } from "../helpers/auth";
import { TEST_ROLE_VALID_UNTIL } from "../helpers/constants";

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
    // bypass and the post-registration profile path both work.
    await adminApi.createAttributeDefinition({
      name: attr,
      allowed_values: ["iem", "other"],
      editable_by: "both",
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
    await adminApi.deleteAttributeDefinition(attributeName(testInfo.parallelIndex));
  });

  // Fresh login — beforeEach removed the test user.
  test.use({ storageState: { cookies: [], origins: [] } });

  test("applicant fills a form attribute and the value lands in MemberAttribute", async ({
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

    await appForm.setAttribute(attr, "iem");
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
  });
});
