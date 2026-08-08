import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { loginViaKeycloak } from "../helpers/auth";

const RENEWAL_PAYMENT_LINK = "https://buy.stripe.com/test_e2e_renewal";

function daysFromNow(days: number): string {
  return new Date(Date.now() + days * 86_400_000).toISOString().split("T")[0];
}

function startOfYear(): string {
  return new Date(new Date().getFullYear(), 0, 1).toISOString().split("T")[0];
}

test.describe("Membership renewal", () => {
  test.beforeEach(async ({ db, adminApi, testUser, testRole }) => {
    await db.cleanupTestUser(testUser.email);
    await db.cleanupTestRole(testRole.name);
    await adminApi.createRole(testRole.name);
  });

  test.afterEach(async ({ db, testUser, testRole }) => {
    await db.cleanupTestUser(testUser.email);
    await db.cleanupTestRole(testRole.name);
  });

  // Fresh login needed — beforeEach cleans up the test user
  test.use({ storageState: { cookies: [], origins: [] } });

  test("member renews from the home banner and lands on the payment link", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }) => {
    await loginViaKeycloak(page, testUser.email, testUser.password);

    const member = await db.getMemberByEmail(testUser.email);
    expect(member).not.toBeNull();
    const userId = member!.user_id as string;

    await adminApi.updateRole(testRole.name, {
      renewable: true,
      renewal_payment_link: RENEWAL_PAYMENT_LINK,
      renewal_period_months: 12,
    });
    await adminApi.assignRole(
      userId,
      testRole.name,
      startOfYear(),
      daysFromNow(15),
    );

    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    await expect(home.renewalBanner(testRole.name)).toBeVisible();

    // Intercept the Stripe navigation so the test stays offline
    await page.route("**/test_e2e_renewal*", (route) =>
      route.fulfill({
        status: 200,
        contentType: "text/html",
        body: "<html>stripe checkout stub</html>",
      }),
    );
    const [request] = await Promise.all([
      page.waitForRequest((req) => req.url().includes("test_e2e_renewal")),
      home.clickRenew(testRole.name),
    ]);

    const renewalId = new URL(request.url()).searchParams.get(
      "client_reference_id",
    );
    expect(renewalId).toBeTruthy();

    const renewalRows = await db.query<{ status: string }>(
      `SELECT status FROM rolerenewal WHERE renewal_id = $1 AND user_id = $2 AND role_name = $3`,
      [renewalId, userId, testRole.name],
    );
    expect(renewalRows).toHaveLength(1);
    expect(renewalRows[0].status).toBe("pending");

    // Clicking again reuses the same pending renewal
    await page.goto("/home");
    await home.waitForLoaded();
    const [secondRequest] = await Promise.all([
      page.waitForRequest((req) => req.url().includes("test_e2e_renewal")),
      home.clickRenew(testRole.name),
    ]);
    expect(
      new URL(secondRequest.url()).searchParams.get("client_reference_id"),
    ).toBe(renewalId);

    // Simulate payment completion: banner disappears
    await db.query(
      `UPDATE rolerenewal SET status = 'paid', stripe_payment_id = 'pi_e2e_test' WHERE renewal_id = $1`,
      [renewalId],
    );
    await adminApi.assignRole(
      userId,
      testRole.name,
      daysFromNow(16),
      daysFromNow(16 + 365),
    );

    await page.goto("/home");
    await home.waitForLoaded();
    await expect(home.renewalBanner(testRole.name)).toBeHidden();
  });

  test("configurable window controls when the banner appears", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }) => {
    await loginViaKeycloak(page, testUser.email, testUser.password);
    const member = await db.getMemberByEmail(testUser.email);
    const userId = member!.user_id as string;

    // Expires in 60 days: outside the default 30-day window
    await adminApi.updateRole(testRole.name, {
      renewable: true,
      renewal_payment_link: RENEWAL_PAYMENT_LINK,
      renewal_period_months: 12,
    });
    await adminApi.assignRole(
      userId,
      testRole.name,
      startOfYear(),
      daysFromNow(60),
    );

    const home = new HomePage(page);
    await page.goto("/home");
    await home.waitForLoaded();
    await expect(home.renewalBanner(testRole.name)).toBeHidden();

    // Widening the window to 90 days makes it due
    await adminApi.updateRole(testRole.name, {
      renewable: true,
      renewal_payment_link: RENEWAL_PAYMENT_LINK,
      renewal_period_months: 12,
      renewal_window_days: 90,
    });

    await page.goto("/home");
    await home.waitForLoaded();
    await expect(home.renewalBanner(testRole.name)).toBeVisible();
  });

  test("grace period keeps renewal open after expiry", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }) => {
    await loginViaKeycloak(page, testUser.email, testUser.password);
    const member = await db.getMemberByEmail(testUser.email);
    const userId = member!.user_id as string;

    await adminApi.updateRole(testRole.name, {
      renewable: true,
      renewal_payment_link: RENEWAL_PAYMENT_LINK,
      renewal_period_months: 12,
      grace_period_days: 30,
    });
    // Expired 5 days ago
    await adminApi.assignRole(
      userId,
      testRole.name,
      startOfYear(),
      daysFromNow(-5),
    );

    const home = new HomePage(page);
    await page.goto("/home");
    await home.waitForLoaded();
    await expect(home.renewalBanner(testRole.name)).toBeVisible();
  });

  test("expired role without grace shows no banner", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }) => {
    await loginViaKeycloak(page, testUser.email, testUser.password);
    const member = await db.getMemberByEmail(testUser.email);
    const userId = member!.user_id as string;

    await adminApi.updateRole(testRole.name, {
      renewable: true,
      renewal_payment_link: RENEWAL_PAYMENT_LINK,
      renewal_period_months: 12,
    });
    await adminApi.assignRole(
      userId,
      testRole.name,
      startOfYear(),
      daysFromNow(-5),
    );

    const home = new HomePage(page);
    await page.goto("/home");
    await home.waitForLoaded();
    await expect(home.renewalBanner(testRole.name)).toBeHidden();
  });

  test("non-renewable expiring role shows no banner", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }) => {
    await loginViaKeycloak(page, testUser.email, testUser.password);
    const member = await db.getMemberByEmail(testUser.email);
    const userId = member!.user_id as string;

    await adminApi.assignRole(
      userId,
      testRole.name,
      startOfYear(),
      daysFromNow(15),
    );

    const home = new HomePage(page);
    await page.goto("/home");
    await home.waitForLoaded();
    await expect(home.renewalBanner(testRole.name)).toBeHidden();
  });
});
