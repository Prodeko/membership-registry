import type { Page } from "@playwright/test";

export class ApplicationFormPage {
  constructor(private page: Page) {}

  async goto(): Promise<void> {
    await this.page.goto("/apply");
  }

  async waitForLoaded(): Promise<void> {
    await this.page.getByTestId("submit-application-button").waitFor();
  }

  async selectRole(roleName: string): Promise<void> {
    await this.page.getByTestId("role-select").click();
    // Select items are rendered in a portal — use role-based locator
    await this.page
      .getByRole("option", { name: new RegExp(roleName, "i") })
      .click();
  }

  async fillApplicationText(text: string): Promise<void> {
    await this.page.getByTestId("application-text").fill(text);
  }

  async submit(): Promise<void> {
    await this.page.getByTestId("submit-application-button").click();
  }
}
