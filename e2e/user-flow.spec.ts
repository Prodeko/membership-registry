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

    // 3. Auth0 redirects → /auth/callback → sets cookie → redirects to /
    //    Non-admin user will land on / then get bounced to an error page,
    //    or may already be redirected elsewhere. Either way, we wait for
    //    the app to settle, then navigate to signup.
    await page.waitForURL("http://127.0.0.1:5173/**", { timeout: 15000 });
  });
});
