import type { Database, SqlValue } from "@sqlite.org/sqlite-wasm";

/**
 * The one synchronous SQL primitive the store builds on. The migration
 * runner and the Drizzle driver both use it, and tests implement it over
 * an in-memory database.
 */
export interface SqlConnection {
  /** Runs one statement and returns its rows as positional arrays. */
  query(sql: string, params?: readonly SqlValue[]): SqlValue[][];
  /** Runs one or more statements, discarding any rows. */
  exec(sql: string): void;
}

export function connectionFor(db: Database): SqlConnection {
  return {
    query: (sql, params = []) =>
      db.exec({ sql, bind: [...params], returnValue: "resultRows", rowMode: "array" }),
    exec: (sql) => {
      db.exec(sql);
    },
  };
}
