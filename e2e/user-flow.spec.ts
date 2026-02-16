import { test, expect } from "@playwright/test";
import { execSync } from "child_process";

// Full user journey: login → signup → apply for membership
//
// Prerequisites:
// - Backend + frontend + postgres running
// - Auth0 test users exist (Database Connection)
// - Credentials in .env.e2e

const API = process.env.API_BASE_URL!;
const DB = process.env.DATABASE_URL!;
const ROLE_NAME = "e2e-test-role";
const ROLE_VALID_UNTIL = "2099-12-31";

function sql(query: string) {
  execSync(`psql "${DB}" -c "${query}"`);
}

test.describe("User flow", () => {
  function cleanup() {
    const email = process.env.E2E_USER_EMAIL!;
    sql(
      `DELETE FROM application WHERE user_id IN (SELECT user_id FROM member WHERE email = '${email}')`,
    );
    sql(`DELETE FROM member WHERE email = '${email}'`);
    sql(
      `DELETE FROM applicationtargetablerole WHERE role_name = '${ROLE_NAME}'`,
    );
    sql(`DELETE FROM role WHERE name = '${ROLE_NAME}'`);
  }

  test.beforeAll(() => {
    cleanup();

    // Ensure the e2e test role and targetable role exist
    sql(
      `INSERT INTO role (name) VALUES ('${ROLE_NAME}') ON CONFLICT DO NOTHING`,
    );
    sql(
      `INSERT INTO applicationtargetablerole (role_name, valid_until, active) VALUES ('${ROLE_NAME}', '${ROLE_VALID_UNTIL}', true) ON CONFLICT DO NOTHING`,
    );
  });

  test.afterAll(() => {
    cleanup();
  });

  test("signup and submit application", async ({ page }) => {
    await page.goto(`${API}/auth/login`);
    await page
      .getByRole("textbox", { name: "Email address" })
      .fill(process.env.E2E_USER_EMAIL!);
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await page
      .getByRole("textbox", { name: "Password" })
      .fill(process.env.E2E_USER_PASSWORD!);
    await page.getByRole("button", { name: "Continue" }).click();

    await page.waitForURL("**/signup");

    await page.getByLabel("First name").fill("Test");
    await page.getByLabel("Last name").fill("User");
    await page.getByRole("combobox", { name: "Home municipality" }).click();
    await page.getByPlaceholder("Search region...").fill("helsin");
    await page.getByRole("option", { name: "Helsinki" }).click();
    await page
      .getByRole("checkbox", { name: "I have read and accept the" })
      .click();
    await page.getByRole("button", { name: "Submit" }).click();

    await page.waitForURL("**/apply");
    await page.getByRole("combobox", { name: "Membership type" }).click();
    await page.getByRole("option", { name: /E2e Test Role/ }).click();
    await page
      .getByRole("textbox", { name: "Application text" })
      .fill("E2E test application");
    await page.getByRole("button", { name: "Submit application" }).click();

    await page.waitForURL("**/apply/success");
    await expect(page.getByRole("heading")).toContainText("Success");
  });
});
