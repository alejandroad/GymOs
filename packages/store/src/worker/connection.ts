import type { Database, SqlValue as WasmSqlValue } from "@sqlite.org/sqlite-wasm";

import type { SqlConnection, SqlValue } from "../sql";

export function connectionFor(db: Database): SqlConnection {
  return {
    query: (sql, params = []) =>
      db.exec({
        sql,
        bind: [...params] as WasmSqlValue[],
        returnValue: "resultRows",
        rowMode: "array",
      }) as SqlValue[][],
    exec: (sql) => {
      db.exec(sql);
    },
  };
}
