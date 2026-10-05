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

  /**
   * Set a form-attribute value. Allowed-values attributes render a Radix
   * Select (clickable trigger + portal'd options); free-text attributes
   * render an Input. Falls through to a fill if the trigger isn't a
   * combobox.
   */
  async setAttribute(name: string, value: string): Promise<void> {
    const field = this.page.getByTestId(`application-attr-${name}`);
    const role = await field.getAttribute("role");
    if (role === "combobox") {
      await field.click();
      await this.page.getByRole("option", { name: value }).click();
    } else {
      await field.fill(value);
    }
  }

  /**
   * Pick "Other…" on a single-valued attribute's Select and type a free-text
   * answer. The answer is committed when the input loses focus.
   */
  async setOtherAttribute(name: string, text: string): Promise<void> {
    await this.page.getByTestId(`application-attr-${name}`).click();
    await this.page.getByRole("option", { name: /^(Other|Muu)…$/ }).click();
    await this.fillOther(name, text);
  }

  /** Pick each of `values` in a multichoice attribute's selector. */
  async chooseMany(name: string, values: string[]): Promise<void> {
    const input = this.page
      .getByTestId(`application-attr-${name}`)
      .getByRole("combobox");
    for (const value of values) {
      await input.click();
      await this.page.getByRole("option", { name: value, exact: true }).click();
    }
    await input.press("Escape");
  }

  /** Type into an attribute's "other" input and commit it by blurring. */
  async fillOther(name: string, text: string): Promise<void> {
    const other = this.page.getByTestId(`application-attr-${name}-other`);
    await other.fill(text);
    await other.blur();
  }

  async submit(): Promise<void> {
    await this.page.getByTestId("submit-application-button").click();
  }
}
