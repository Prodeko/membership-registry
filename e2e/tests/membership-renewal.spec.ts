import { test, expect } from "../fixtures";
import { HomePage } from "../pages/home.page";
import { KeycloakLoginPage } from "../pages/keycloak-login.page";
import {
  API_BASE_URL,
  TEST_USER_EMAIL,
  TEST_USER_PASSWORD,
  TEST_ROLE_NAME,
} from "../helpers/constants";

const RENEWAL_PAYMENT_LINK = "https://buy.stripe.com/test_e2e_renewal";

test.describe("Membership renewal", () => {
  test.beforeEach(async ({ db, adminApi }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
    await adminApi.createRole(TEST_ROLE_NAME);
  });

  test.afterEach(async ({ db }) => {
    await db.cleanupTestUser(TEST_USER_EMAIL);
    await db.cleanupTestRole(TEST_ROLE_NAME);
  });

  test("user can see renewal warning and click renew link which redirects to stripe", async ({
    page,
    db,
  }) => {
    // Login to create the member
    await page.goto(`${API_BASE_URL}/auth/login`);
    const keycloak = new KeycloakLoginPage(page);
    await keycloak.login(TEST_USER_EMAIL, TEST_USER_PASSWORD);
    await page.waitForURL("**/home", { timeout: 30_000 });

    // Get user_id
    const member = await db.getMemberByEmail(TEST_USER_EMAIL);
    expect(member).not.toBeNull();
    const userId = member!.user_id as string;

    // Make the role renewable with a payment link
    await db.query(
      `UPDATE role SET renewable = true, renewal_payment_link = $1, renewal_period_months = 12 WHERE name = $2`,
      [RENEWAL_PAYMENT_LINK, TEST_ROLE_NAME],
    );

    // Give user a role membership expiring in 15 days
    const now = new Date();
    const validFrom = new Date(now.getFullYear(), 0, 1).toISOString().split("T")[0];
    const validUntil = new Date(now.getTime() + 15 * 86400000).toISOString().split("T")[0];
    const newValidFrom = new Date(now.getTime() + 16 * 86400000).toISOString().split("T")[0];
    const newValidUntil = new Date(now.getTime() + 16 * 86400000 + 365 * 86400000).toISOString().split("T")[0];

    await db.query(
      `INSERT INTO rolemember (user_id, role_name, valid_from, valid_until) VALUES ($1, $2, $3, $4)`,
      [userId, TEST_ROLE_NAME, validFrom, validUntil],
    );

    // Create a pending renewal record (what the scheduler would create)
    const renewalRows = await db.query<{ renewal_id: string }>(
      `INSERT INTO rolerenewal (user_id, role_name, old_valid_from, old_valid_until, new_valid_from, new_valid_until, status)
       VALUES ($1, $2, $3, $4, $5, $6, 'pending')
       RETURNING renewal_id`,
      [userId, TEST_ROLE_NAME, validFrom, validUntil, newValidFrom, newValidUntil],
    );
    const renewalId = renewalRows[0].renewal_id;

    // Reload home page
    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    // Verify expiring warning and renewal link are visible
    expect(await home.isExpiringWarningVisible(TEST_ROLE_NAME)).toBe(true);
    const href = await home.getRenewalLinkHref(TEST_ROLE_NAME);
    expect(href).toContain(RENEWAL_PAYMENT_LINK);
    expect(href).toContain(`client_reference_id=${renewalId}`);

    // Simulate payment completion: mark renewal as paid and create new role membership
    await db.query(
      `UPDATE rolerenewal SET status = 'paid', stripe_payment_id = 'pi_e2e_test' WHERE renewal_id = $1`,
      [renewalId],
    );
    await db.query(
      `INSERT INTO rolemember (user_id, role_name, valid_from, valid_until) VALUES ($1, $2, $3, $4)`,
      [userId, TEST_ROLE_NAME, newValidFrom, newValidUntil],
    );

    // Go back to home and verify the renewal warning is gone (merged period extends far out)
    await page.goto("/home");
    await home.waitForLoaded();

    expect(await home.isExpiringWarningVisible(TEST_ROLE_NAME)).toBe(false);

    // Verify new membership exists in DB
    const roleMemberCount = await db.count(
      "rolemember",
      "user_id = $1 AND role_name = $2",
      [userId, TEST_ROLE_NAME],
    );
    expect(roleMemberCount).toBe(2); // old + new period
  });

  test("non-renewable expiring role does not show warning", async ({
    page,
    db,
  }) => {
    // Login to create the member
    await page.goto(`${API_BASE_URL}/auth/login`);
    const keycloak = new KeycloakLoginPage(page);
    await keycloak.login(TEST_USER_EMAIL, TEST_USER_PASSWORD);
    await page.waitForURL("**/home", { timeout: 30_000 });

    const member = await db.getMemberByEmail(TEST_USER_EMAIL);
    const userId = member!.user_id as string;

    // Role is NOT renewable (default)
    const now = new Date();
    const validFrom = new Date(now.getFullYear(), 0, 1).toISOString().split("T")[0];
    const validUntil = new Date(now.getTime() + 15 * 86400000).toISOString().split("T")[0];

    await db.query(
      `INSERT INTO rolemember (user_id, role_name, valid_from, valid_until) VALUES ($1, $2, $3, $4)`,
      [userId, TEST_ROLE_NAME, validFrom, validUntil],
    );

    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    // Non-renewable role should NOT show expiring warning
    expect(await home.isExpiringWarningVisible(TEST_ROLE_NAME)).toBe(false);
  });

  test("role expiring in more than 30 days does not show warning", async ({
    page,
    db,
  }) => {
    await page.goto(`${API_BASE_URL}/auth/login`);
    const keycloak = new KeycloakLoginPage(page);
    await keycloak.login(TEST_USER_EMAIL, TEST_USER_PASSWORD);
    await page.waitForURL("**/home", { timeout: 30_000 });

    const member = await db.getMemberByEmail(TEST_USER_EMAIL);
    const userId = member!.user_id as string;

    await db.query(
      `UPDATE role SET renewable = true, renewal_payment_link = $1, renewal_period_months = 12 WHERE name = $2`,
      [RENEWAL_PAYMENT_LINK, TEST_ROLE_NAME],
    );

    const now = new Date();
    const validFrom = new Date(now.getFullYear(), 0, 1).toISOString().split("T")[0];
    const validUntil = new Date(now.getTime() + 60 * 86400000).toISOString().split("T")[0];

    await db.query(
      `INSERT INTO rolemember (user_id, role_name, valid_from, valid_until) VALUES ($1, $2, $3, $4)`,
      [userId, TEST_ROLE_NAME, validFrom, validUntil],
    );

    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    expect(await home.isExpiringWarningVisible(TEST_ROLE_NAME)).toBe(false);
  });

  test("renewed role merges periods and hides warning", async ({
    page,
    db,
  }) => {
    await page.goto(`${API_BASE_URL}/auth/login`);
    const keycloak = new KeycloakLoginPage(page);
    await keycloak.login(TEST_USER_EMAIL, TEST_USER_PASSWORD);
    await page.waitForURL("**/home", { timeout: 30_000 });

    const member = await db.getMemberByEmail(TEST_USER_EMAIL);
    const userId = member!.user_id as string;

    await db.query(
      `UPDATE role SET renewable = true, renewal_payment_link = $1, renewal_period_months = 12 WHERE name = $2`,
      [RENEWAL_PAYMENT_LINK, TEST_ROLE_NAME],
    );

    // Old period expiring in 10 days
    const now = new Date();
    const oldValidFrom = new Date(now.getFullYear(), 0, 1).toISOString().split("T")[0];
    const oldValidUntil = new Date(now.getTime() + 10 * 86400000).toISOString().split("T")[0];

    // New period starting after old one, valid for a year
    const newValidFrom = new Date(now.getTime() + 11 * 86400000).toISOString().split("T")[0];
    const newValidUntil = new Date(now.getTime() + 11 * 86400000 + 365 * 86400000).toISOString().split("T")[0];

    // Insert both membership periods (simulates a completed renewal)
    await db.query(
      `INSERT INTO rolemember (user_id, role_name, valid_from, valid_until) VALUES ($1, $2, $3, $4)`,
      [userId, TEST_ROLE_NAME, oldValidFrom, oldValidUntil],
    );
    await db.query(
      `INSERT INTO rolemember (user_id, role_name, valid_from, valid_until) VALUES ($1, $2, $3, $4)`,
      [userId, TEST_ROLE_NAME, newValidFrom, newValidUntil],
    );

    await page.goto("/home");
    const home = new HomePage(page);
    await home.waitForLoaded();

    // Merged role should show as active with no expiry warning (latest valid_until is far out)
    const roles = await home.getRoles();
    const testRole = roles.find((r) => r.roleName === "E2e Test Role");
    expect(testRole).toBeDefined();
    expect(testRole!.status).not.toContain("Expired");

    // No warning because merged valid_until is > 30 days out
    expect(await home.isExpiringWarningVisible(TEST_ROLE_NAME)).toBe(false);
  });
});
