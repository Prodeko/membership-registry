import { test, expect } from "@playwright/test";

// Full user journey: login → signup → apply for membership
//
// Prerequisites:
// - Backend + frontend + postgres running
// - Auth0 test user exists (Database Connection, no admin role)
// - Credentials in .env.e2e

test.describe("User flow", () => {
  test("signup and submit application", async ({ page }) => {
    await page.goto(`${process.env.API_BASE_URL!}/auth/login`);

    // 2. Authenticate with Auth0
    await page
      .getByRole("textbox", { name: "Email address" })
      .fill(process.env.E2E_USER_EMAIL!);
    await page.getByRole("button", { name: "Continue", exact: true }).click();
    await page
      .getByRole("textbox", { name: "Password" })
      .fill(process.env.E2E_USER_PASSWORD!);
    await page.getByRole("button", { name: "Continue" }).click();

    // 3. Auth0 redirects → /auth/callback → backend checks member record
    //    → no member exists → redirects to /signup
    await page.waitForURL("**/signup");

    await page.getByRole("combobox", { name: "Home municipality" }).click();
    await page.getByPlaceholder("Search region...").fill("helsin");
    await page.getByRole("option", { name: "Helsinki" }).click();
    await page
      .getByRole("checkbox", { name: "I have read and accept the" })
      .click();
    await page.getByRole("button", { name: "Submit" }).click();

    await page.waitForURL("**/application-form");
    await page.getByRole("combobox", { name: "Membership type" }).click();
    await page
      .getByRole("option", { name: "Test (Valid until 2/17/2026)" })
      .click();
    await page.getByRole("textbox", { name: "Application text" }).click();
    await page
      .getByRole("textbox", { name: "Application text" })
      .fill("Haluaisin kovasti osaksi tätä yhteisöä");
    await page.getByRole("button", { name: "Submit application" }).click();

    await page.waitForURL("**/application-form/success");
    await expect(page.getByRole("heading")).toContainText("Success");
  });
});
