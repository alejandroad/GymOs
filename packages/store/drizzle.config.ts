import { defineConfig } from "drizzle-kit";

// Generates SQL only. Migrations are applied in the browser by the store
// worker (src/migrate.ts), never by drizzle-kit against a server.
export default defineConfig({
  dialect: "sqlite",
  schema: "./src/schema.ts",
  out: "./drizzle",
});
