import type { Page } from "@playwright/test";
import { KeycloakLoginPage } from "../pages/keycloak-login.page";
import { OnboardingPage } from "../pages/onboarding.page";
import { API_BASE_URL } from "./constants";

export interface LoginOptions {
  /**
   * If true (default), automatically completes the onboarding step (home
   * municipality) after login so tests can land on /home. Set to false when
   * the test wants to exercise the onboarding gate itself.
   */
  completeOnboarding?: boolean;
  /** Municipality to fill on the onboarding form (default: "Helsinki"). */
  onboardingMunicipality?: string;
}

export async function loginViaKeycloak(
  page: Page,
  email: string,
  password: string,
  options: LoginOptions = {},
): Promise<void> {
  const completeOnboarding = options.completeOnboarding ?? true;
  const municipality = options.onboardingMunicipality ?? "Helsinki";

  await page.goto(`${API_BASE_URL}/auth/login`);
  const keycloak = new KeycloakLoginPage(page);
  await keycloak.login(email, password);

  // First-login users land on /onboarding; returning users land on /home.
  await page.waitForURL(/\/(home|onboarding)(\?|$|#)/, { timeout: 30_000 });

  if (page.url().includes("/onboarding") && completeOnboarding) {
    const onboarding = new OnboardingPage(page);
    await onboarding.waitForLoaded();
    await onboarding.selectMunicipality(municipality);
    await onboarding.submit();
    await page.waitForURL("**/home", { timeout: 30_000 });
  }
}
