import type { DatabaseSync, SQLInputValue, StatementSync } from "node:sqlite";

import type { SqlConnection, SqlValue } from "../sql";

/**
 * A SqlConnection over a `node:sqlite` database.
 *
 * Rows come back as positional arrays (`setReturnArrays`), which is what
 * Drizzle's sqlite-proxy driver expects, and which keeps duplicate column
 * names in joins. Prepared statements are cached by SQL text: Drizzle and
 * the store reuse a small, fixed set of parameterized statements.
 */
export function connectionFor(db: DatabaseSync): SqlConnection {
  const statements = new Map<string, StatementSync>();

  function prepare(sql: string): StatementSync {
    let statement = statements.get(sql);
    if (!statement) {
      statement = db.prepare(sql);
      statement.setReturnArrays(true);
      statements.set(sql, statement);
    }
    return statement;
  }

  return {
    query: (sql, params = []) =>
      prepare(sql).all(...(params as SQLInputValue[])) as unknown as SqlValue[][],
    exec: (sql) => db.exec(sql),
  };
}
