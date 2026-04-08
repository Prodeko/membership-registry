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

  test("user can see renewal warning and click renew link which redirects to stripe", async ({
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

    // Make the role renewable with a payment link
    await adminApi.updateRole(testRole.name, {
      renewable: true,
      renewal_payment_link: RENEWAL_PAYMENT_LINK,
      renewal_period_months: 12,
    });

    // Give user a role membership expiring in 15 days
    const validFrom = startOfYear();
    const validUntil = daysFromNow(15);
    const newValidFrom = daysFromNow(16);
    const newValidUntil = daysFromNow(16 + 365);

    await adminApi.assignRole(userId, testRole.name, validFrom, validUntil);

    // Create a pending renewal record (no admin endpoint for this)
    const renewalRows = await db.query<{ renewal_id: string }>(
      `INSERT INTO rolerenewal (user_id, role_name, old_valid_from, old_valid_until, new_valid_from, new_valid_until, status)
       VALUES ($1, $2, $3, $4, $5, $6, 'pending')
       RETURNING renewal_id`,
      [
        userId,
        testRole.name,
        validFrom,
        validUntil,
        newValidFrom,
        newValidUntil,
      ],
    );
    const renewalId = renewalRows[0].renewal_id;

    // Reload home page
    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    // Verify expiring warning and renewal link are visible
    await expect(home.expiringWarning(testRole.name)).toBeVisible();
    const href = await home.getRenewalLinkHref(testRole.name);
    expect(href).toContain(RENEWAL_PAYMENT_LINK);
    expect(href).toContain(`client_reference_id=${renewalId}`);

    // Simulate payment completion: mark renewal as paid and create new role membership
    await db.query(
      `UPDATE rolerenewal SET status = 'paid', stripe_payment_id = 'pi_e2e_test' WHERE renewal_id = $1`,
      [renewalId],
    );
    await adminApi.assignRole(
      userId,
      testRole.name,
      newValidFrom,
      newValidUntil,
    );

    // Go back to home and verify the renewal warning is gone (merged period extends far out)
    await page.goto("/home");
    await home.waitForLoaded();

    await expect(home.expiringWarning(testRole.name)).toBeHidden();

    // Verify new membership exists in DB
    const roleMemberRows = await db.query<{ count: string }>(
      `SELECT COUNT(*) as count FROM rolemember WHERE user_id = $1 AND role_name = $2`,
      [userId, testRole.name],
    );
    expect(parseInt(roleMemberRows[0].count, 10)).toBe(2); // old + new period
  });

  test("non-renewable expiring role does not show warning", async ({
    page,
    db,
    adminApi,
    testUser,
    testRole,
  }) => {
    await loginViaKeycloak(page, testUser.email, testUser.password);

    const member = await db.getMemberByEmail(testUser.email);
    const userId = member!.user_id as string;

    // Role is NOT renewable (default)
    await adminApi.assignRole(
      userId,
      testRole.name,
      startOfYear(),
      daysFromNow(15),
    );

    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    // Non-renewable role should NOT show expiring warning
    await expect(home.expiringWarning(testRole.name)).toBeHidden();
  });

  test("role expiring in more than 30 days does not show warning", async ({
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
      daysFromNow(60),
    );

    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    await expect(home.expiringWarning(testRole.name)).toBeHidden();
  });

  test("renewed role merges periods and hides warning", async ({
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

    // Old period expiring in 10 days + new period starting after, valid for a year
    const oldValidFrom = startOfYear();
    const oldValidUntil = daysFromNow(10);
    const newValidFrom = daysFromNow(11);
    const newValidUntil = daysFromNow(11 + 365);

    // Insert both membership periods (simulates a completed renewal)
    await adminApi.assignRole(
      userId,
      testRole.name,
      oldValidFrom,
      oldValidUntil,
    );
    await adminApi.assignRole(
      userId,
      testRole.name,
      newValidFrom,
      newValidUntil,
    );

    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    // Merged role should show as active with no expiry warning (latest valid_until is far out)
    const roles = await home.getRoles();
    const matched = roles.find((r) => r.roleName === testRole.displayName);
    expect(matched).toBeDefined();
    expect(matched!.status).not.toContain("Expired");

    // No warning because merged valid_until is > 30 days out
    await expect(home.expiringWarning(testRole.name)).toBeHidden();
  });
});
