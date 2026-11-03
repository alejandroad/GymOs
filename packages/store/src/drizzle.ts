import type { SqlValue } from "@sqlite.org/sqlite-wasm";
import { drizzle } from "drizzle-orm/sqlite-proxy";

import * as schema from "./schema";
import type { SqlConnection } from "./sql";

/**
 * Drizzle over the worker's own SQLite connection, through the sqlite-proxy
 * driver: Drizzle builds SQL and maps rows, and the callback runs it in
 * process. Drizzle has no SQLite-WASM driver; the proxy driver is its
 * documented route for custom SQLite connections. It runs inside the
 * worker, so queries never cross the UI boundary.
 */
export function createDrizzle(conn: SqlConnection) {
  return drizzle(
    async (sql, params, method) => {
      const rows = conn.query(sql, params as SqlValue[]);
      // "get" expects the single row itself (or undefined), not a list.
      return { rows: method === "get" ? (rows[0] as unknown as unknown[]) : rows };
    },
    { schema },
  );
}

export type StoreDb = ReturnType<typeof createDrizzle>;
