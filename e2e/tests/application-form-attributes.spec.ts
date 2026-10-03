import type { Page } from "@playwright/test";
import { test, expect } from "../fixtures";
import type { AdminApiHelper } from "../fixtures/api.fixture";
import type { DatabaseHelper } from "../fixtures/db.fixture";
import { HomePage } from "../pages/home.page";
import { ApplicationFormPage } from "../pages/application-form.page";
import { ApplicationSuccessPage } from "../pages/application-success.page";
import { loginViaKeycloak } from "../helpers/auth";
import { TEST_ROLE_VALID_UNTIL } from "../helpers/constants";

// Per-worker attribute name so parallel workers don't collide on the
// AttributeDefinition PK.
const attributeName = (workerIndex: number) =>
  `e2e-major-subject-${workerIndex}`;

type AttributeShape = {
  allowed_values: string[];
  multiple?: boolean;
  allow_other?: boolean;
};

/**
 * Defines the worker's attribute and puts it on the test role's form.
 * Internal-only (sync_to_keycloak=false) so the test doesn't need a Keycloak
 * mapper round trip. Both editable so the form-time bypass and the
 * post-registration profile path both work. Required, so the form can't be
 * submitted until it's filled.
 */
async function setupFormAttribute(
  adminApi: AdminApiHelper,
  roleName: string,
  attr: string,
  shape: AttributeShape,
): Promise<void> {
  await adminApi.createAttributeDefinition({
    name: attr,
    editable_by: "both",
    required: true,
    ...shape,
  });
  await adminApi.createTargetableRole(
    roleName,
    TEST_ROLE_VALID_UNTIL,
    undefined,
    { form_attributes: [attr] },
  );
}

/** Logs in, opens the application form and picks the test role. */
async function openForm(
  page: Page,
  email: string,
  password: string,
  roleDisplayName: string,
): Promise<ApplicationFormPage> {
  await loginViaKeycloak(page, email, password);
  const home = new HomePage(page);
  await home.waitForLoaded();
  await home.clickApply();
  const appForm = new ApplicationFormPage(page);
  await appForm.waitForLoaded();
  await appForm.selectRole(roleDisplayName);
  return appForm;
}

/** Submits and returns the values stored for the member's attribute. */
async function submitAndReadValues(
  appForm: ApplicationFormPage,
  page: Page,
  db: DatabaseHelper,
  email: string,
  attr: string,
): Promise<string[]> {
  await appForm.fillApplicationText("E2E form-attribute test");
  await appForm.submit();
  // Land on success page (no payment link configured for this role).
  await page.waitForURL("**/apply/success");
  await new ApplicationSuccessPage(page).waitForLoaded();

  // Joining through member by email avoids embedding the user_id.
  const rows = await db.query<{ value_list: string[] }>(
    `SELECT ma.value_list
     FROM memberattribute ma
     JOIN member m ON ma.user_id = m.user_id
     WHERE m.email = $1 AND ma.attribute_name = $2`,
    [email, attr],
  );
  expect(rows).toHaveLength(1);
  return rows[0].value_list;
}

test.describe("Application form attributes", () => {
  test.beforeEach(async ({ db, adminApi, testUser, testRole }, testInfo) => {
    const attr = attributeName(testInfo.parallelIndex);

    await db.cleanupTestUser(testUser.email);
    await db.cleanupTestRole(testRole.name);
    // Attribute definition must be torn down separately — it has no CASCADE
    // path from role/member cleanup.
    await adminApi.deleteAttributeDefinition(attr);

    await adminApi.createRole(testRole.name);
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

  test("applicant fills a form attribute and the value lands in MemberAttribute", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }, testInfo) => {
    const attr = attributeName(testInfo.parallelIndex);
    await setupFormAttribute(adminApi, testRole.name, attr, {
      allowed_values: ["iem", "tuta"],
    });
    const appForm = await openForm(
      page,
      testUser.email,
      testUser.password,
      testRole.displayName,
    );

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

    // Application row exists and the value persists in MemberAttribute.
    const values = await submitAndReadValues(
      appForm,
      page,
      db,
      testUser.email,
      attr,
    );
    const dbApp = await db.getApplicationByUserEmail(testUser.email);
    expect(dbApp).not.toBeNull();
    expect(dbApp!.status).toBe("pending");
    expect(values).toEqual(["iem"]);
  });

  test("applicant types an answer of their own via the Other option", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }, testInfo) => {
    const attr = attributeName(testInfo.parallelIndex);
    await setupFormAttribute(adminApi, testRole.name, attr, {
      allowed_values: ["iem", "tuta"],
      allow_other: true,
    });
    const appForm = await openForm(
      page,
      testUser.email,
      testUser.password,
      testRole.displayName,
    );

    const submit = page.getByTestId("submit-application-button");
    await expect(submit).toBeDisabled();
    await appForm.setOtherAttribute(attr, "bioinformatics");
    await expect(submit).toBeEnabled();

    expect(
      await submitAndReadValues(appForm, page, db, testUser.email, attr),
    ).toEqual(["bioinformatics"]);
  });

  test("applicant picks several values plus an Other answer on a multichoice attribute", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }, testInfo) => {
    const attr = attributeName(testInfo.parallelIndex);
    await setupFormAttribute(adminApi, testRole.name, attr, {
      allowed_values: ["fi", "sv", "en"],
      multiple: true,
      allow_other: true,
    });
    const appForm = await openForm(
      page,
      testUser.email,
      testUser.password,
      testRole.displayName,
    );

    const submit = page.getByTestId("submit-application-button");
    await expect(submit).toBeDisabled();
    await appForm.chooseMany(attr, ["fi", "en"]);
    await expect(submit).toBeEnabled();
    await appForm.fillOther(attr, "kurdish");

    expect(
      await submitAndReadValues(appForm, page, db, testUser.email, attr),
    ).toEqual(["fi", "en", "kurdish"]);
  });
});
