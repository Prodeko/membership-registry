import { test as base } from "@playwright/test";
import { DatabaseHelper } from "./db.fixture";
import { AdminApiHelper } from "./api.fixture";
import { closePool } from "../helpers/db-client";

type Fixtures = {
  db: DatabaseHelper;
  adminApi: AdminApiHelper;
};

export const test = base.extend<Fixtures>({
  db: async ({}, use) => {
    const db = new DatabaseHelper();
    await use(db);
  },

  adminApi: async ({}, use) => {
    const api = new AdminApiHelper();
    await use(api);
  },
});

test.afterAll(async () => {
  await closePool();
});

export { expect } from "@playwright/test";
