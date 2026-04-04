import { test as base } from "@playwright/test";
import { DatabaseHelper } from "./db.fixture";
import { AdminApiHelper } from "./api.fixture";
import {
  getAuthenticatedUserPage,
  cleanupAuthContext,
} from "./auth.fixture";
import { closePool } from "../helpers/db-client";
import type { Page } from "@playwright/test";

type Fixtures = {
  db: DatabaseHelper;
  adminApi: AdminApiHelper;
  userPage: Page;
};

export const test = base.extend<Fixtures>({
  db: async ({}, use) => {
    const db = new DatabaseHelper();
    await use(db);
  },

  adminApi: async ({}, use) => {
    const api = new AdminApiHelper();
    await use(api);
    await api.cleanup();
  },

  userPage: async ({}, use) => {
    const page = await getAuthenticatedUserPage();
    await use(page);
  },
});

// Cleanup after all tests in a worker
test.afterAll(async () => {
  await cleanupAuthContext();
  await closePool();
});

export { expect } from "@playwright/test";
