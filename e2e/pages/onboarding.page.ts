import type { Page } from "@playwright/test";

export class OnboardingPage {
  constructor(private page: Page) {}

  async waitForLoaded(): Promise<void> {
    await this.page.getByTestId("onboarding-submit-button").waitFor();
  }

  async selectMunicipality(name: string): Promise<void> {
    await this.page.getByTestId("municipality-combobox-trigger").click();
    await this.page.getByTestId("municipality-search-input").fill(name);
    await this.page.getByRole("option", { name }).click();
  }

  async submit(): Promise<void> {
    await this.page.getByTestId("onboarding-submit-button").click();
  }
}
