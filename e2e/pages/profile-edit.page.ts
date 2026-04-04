import type { Page } from "@playwright/test";

export class ProfileEditPage {
  constructor(private page: Page) {}

  async goto(): Promise<void> {
    await this.page.goto("/profile/edit");
  }

  async waitForLoaded(): Promise<void> {
    await this.page.getByTestId("profile-save-button").waitFor();
  }

  async fillFirstName(name: string): Promise<void> {
    await this.page.getByTestId("profile-first-name").clear();
    await this.page.getByTestId("profile-first-name").fill(name);
  }

  async fillLastName(name: string): Promise<void> {
    await this.page.getByTestId("profile-last-name").clear();
    await this.page.getByTestId("profile-last-name").fill(name);
  }

  async selectMunicipality(name: string): Promise<void> {
    await this.page.getByTestId("municipality-combobox-trigger").click();
    await this.page.getByTestId("municipality-search-input").fill(name);
    await this.page.getByRole("option", { name }).click();
  }

  async selectLanguage(lang: "fi" | "en"): Promise<void> {
    await this.page.getByTestId("profile-language-select").click();
    await this.page.getByRole("option", { name: lang === "fi" ? "Suomi" : "English" }).click();
  }

  async toggleNotifications(): Promise<void> {
    await this.page.getByTestId("profile-notifications-switch").click();
  }

  async save(): Promise<void> {
    await this.page.getByTestId("profile-save-button").click();
  }

  async cancel(): Promise<void> {
    await this.page.getByTestId("profile-cancel-button").click();
  }
}
