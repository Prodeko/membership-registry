import type { Page } from "@playwright/test";

export class KeycloakLoginPage {
  constructor(private page: Page) {}

  async login(email: string, password: string): Promise<void> {
    // Step 1: Enter username and submit
    await this.page.locator("#username").fill(email);
    await this.page.locator("#kc-login").click();

    // Step 2: Enter password and submit
    await this.page.locator("#password").waitFor();
    await this.page.locator("#password").fill(password);
    await this.page.locator("#kc-login").click();
  }
}
