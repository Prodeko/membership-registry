import type { Page } from "@playwright/test";

export class ApplicationSuccessPage {
  constructor(private page: Page) {}

  async waitForLoaded(): Promise<void> {
    await this.page.getByTestId("success-title").waitFor();
  }

  async getTitle(): Promise<string> {
    return (await this.page.getByTestId("success-title").textContent()) ?? "";
  }

  async clickGoHome(): Promise<void> {
    await this.page.getByTestId("success-go-home").click();
  }
}
