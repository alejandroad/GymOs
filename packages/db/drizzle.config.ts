import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { loadEnvFile } from "node:process";

import { defineConfig } from "drizzle-kit";

// drizzle-kit runs outside Next.js, so it can't use apps/web/src/env.ts.
// Locally it reads the same .env.local the app uses; in CI or on a host,
// DATABASE_URL comes from the process environment. Paths are relative to
// the package root, where the db:* scripts run (drizzle-kit loads this
// file as CommonJS, so import.meta.dirname is unavailable).
const appEnvFile = resolve("../../apps/web/.env.local");
if (!process.env.DATABASE_URL && existsSync(appEnvFile)) {
  loadEnvFile(appEnvFile);
}

export default defineConfig({
  dialect: "postgresql",
  schema: "./src/schema/index.ts",
  out: "./migrations",
  dbCredentials: {
    url: process.env.DATABASE_URL ?? "",
  },
  strict: true,
  verbose: true,
});
