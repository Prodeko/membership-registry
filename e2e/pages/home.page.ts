import type { Page } from "@playwright/test";

export class HomePage {
  constructor(private page: Page) {}

  async goto(): Promise<void> {
    await this.page.goto("/home");
  }

  async waitForLoaded(): Promise<void> {
    await this.page.getByTestId("home-profile-card").waitFor();
  }

  // --- Applications ---

  async getApplications(): Promise<{ roleName: string; status: string }[]> {
    // The applications card is only rendered when the user has applications,
    // so we wait for it with a short timeout and return [] if it never appears.
    const card = this.page.getByTestId("home-applications-card");
    try {
      await card.waitFor({ timeout: 5_000 });
    } catch {
      return [];
    }

    const rows = card.locator("[data-testid^='application-row-']");
    const count = await rows.count();
    const results: { roleName: string; status: string }[] = [];

    for (let i = 0; i < count; i++) {
      const row = rows.nth(i);
      const roleName =
        (await row
          .locator("[data-testid='application-role-name']")
          .textContent()) ?? "";
      const statusBadge = row.locator("[data-testid^='application-status-']");
      const status = (await statusBadge.textContent()) ?? "";
      results.push({ roleName: roleName.trim(), status: status.trim() });
    }

    return results;
  }

  async clickWithdraw(applicationId: string): Promise<void> {
    await this.page.getByTestId(`withdraw-button-${applicationId}`).click();
  }

  async confirmWithdraw(): Promise<void> {
    await this.page.getByTestId("withdraw-confirm-button").click();
  }

  async cancelWithdraw(): Promise<void> {
    await this.page.getByTestId("withdraw-cancel-button").click();
  }

  // --- Roles ---

  async getRoles(): Promise<{ roleName: string; status: string }[]> {
    const card = this.page.getByTestId("home-roles-card");
    const rows = card.locator("[data-testid^='role-row-']");
    const count = await rows.count();
    const results: { roleName: string; status: string }[] = [];

    for (let i = 0; i < count; i++) {
      const row = rows.nth(i);
      const roleName =
        (await row.locator("[data-testid='role-name']").textContent()) ?? "";
      const statusBadge = row.locator("[data-testid^='role-status-']");
      const status = (await statusBadge.textContent()) ?? "";
      results.push({ roleName: roleName.trim(), status: status.trim() });
    }

    return results;
  }

  async isExpiringWarningVisible(roleName: string): Promise<boolean> {
    return this.page
      .getByTestId(`role-expiring-warning-${roleName}`)
      .isVisible();
  }

  async getRenewalLinkHref(roleName: string): Promise<string | null> {
    const link = this.page.getByTestId(`role-renewal-link-${roleName}`);
    if (!(await link.isVisible())) return null;
    return link.getAttribute("href");
  }

  // --- Profile ---

  async getProfileName(): Promise<string> {
    return (
      (await this.page.getByTestId("profile-name-value").textContent()) ?? ""
    );
  }

  async getProfileMunicipality(): Promise<string> {
    return (
      (await this.page
        .getByTestId("profile-municipality-value")
        .textContent()) ?? ""
    );
  }

  async clickEditProfile(): Promise<void> {
    await this.page.getByTestId("edit-profile-button").click();
  }

  // --- Apply ---

  async clickApply(): Promise<void> {
    await this.page.getByTestId("apply-button").click();
  }

  async isApplyButtonVisible(): Promise<boolean> {
    return this.page.getByTestId("apply-button").isVisible();
  }
}
