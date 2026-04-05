import { defineConfig, devices } from "@playwright/test";
import dotenv from "dotenv";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
dotenv.config({ path: path.resolve(__dirname, ".env.e2e"), quiet: true });

const workers = parseInt(process.env.E2E_WORKERS ?? "1", 10);

export default defineConfig({
  testDir: "./tests",
  testMatch: "**/*.spec.ts",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers,
  reporter: "html",
  timeout: 60_000,
  expect: {
    timeout: 10_000,
  },
  projects: [
    {
      name: "setup",
      testDir: ".",
      testMatch: "auth.setup.ts",
      retries: 2,
    },
    {
      name: "tests",
      dependencies: ["setup"],
      use: {
        ...devices["Desktop Chrome"],
        baseURL: "http://127.0.0.1:5173",
        trace: "on-first-retry",
      },
    },
  ],
});
