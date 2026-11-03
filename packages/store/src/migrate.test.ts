import sqlite3InitModule, { type Database } from "@sqlite.org/sqlite-wasm";
import { afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";

import { MigrationError, runMigrations, type Migration } from "./migrate";
import { connectionFor, type SqlConnection } from "./sql";

const m0: Migration = {
  tag: "0000_create_a",
  sql: "CREATE TABLE a (id TEXT PRIMARY KEY);\n--> statement-breakpoint\nCREATE INDEX a_id ON a (id);",
};
const m1: Migration = { tag: "0001_create_b", sql: "CREATE TABLE b (id TEXT PRIMARY KEY);" };
const m2: Migration = { tag: "0002_add_b_note", sql: "ALTER TABLE b ADD COLUMN note TEXT;" };

let sqlite3: Awaited<ReturnType<typeof sqlite3InitModule>>;
let db: Database;
let conn: SqlConnection;

beforeAll(async () => {
  sqlite3 = await sqlite3InitModule();
});

beforeEach(() => {
  db = new sqlite3.oo1.DB(":memory:");
  conn = connectionFor(db);
});

afterEach(() => db.close());

function tables(): string[] {
  return conn
    .query("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
    .map(([name]) => String(name));
}

function history(): string[] {
  return conn.query("SELECT tag FROM __migrations ORDER BY idx").map(([tag]) => String(tag));
}

describe("runMigrations", () => {
  it("applies migrations in order and records each one", () => {
    expect(runMigrations(conn, [m0, m1, m2])).toEqual([m0.tag, m1.tag, m2.tag]);
    expect(tables()).toEqual(["__migrations", "a", "b"]);
    expect(history()).toEqual([m0.tag, m1.tag, m2.tag]);
  });

  it("is idempotent: a second run applies nothing and keeps data", () => {
    runMigrations(conn, [m0, m1, m2]);
    conn.query("INSERT INTO b (id, note) VALUES (?, ?)", ["x", "kept"]);

    expect(runMigrations(conn, [m0, m1, m2])).toEqual([]);
    expect(history()).toEqual([m0.tag, m1.tag, m2.tag]);
    expect(conn.query("SELECT id, note FROM b")).toEqual([["x", "kept"]]);
  });

  it("applies only the new migrations when the app adds some", () => {
    runMigrations(conn, [m0]);
    expect(runMigrations(conn, [m0, m1, m2])).toEqual([m1.tag, m2.tag]);
  });

  it("rolls back a failing migration completely and keeps the earlier ones", () => {
    const broken: Migration = {
      tag: "0001_broken",
      sql: "CREATE TABLE c (id TEXT);\n--> statement-breakpoint\nNOT VALID SQL;",
    };
    const error = (() => {
      try {
        runMigrations(conn, [m0, broken]);
      } catch (e) {
        return e;
      }
    })();

    expect(error).toBeInstanceOf(MigrationError);
    expect((error as Error).message).toContain("0001_broken");
    expect(tables()).toEqual(["__migrations", "a"]);
    expect(history()).toEqual([m0.tag]);
  });

  it("refuses a database written by a newer app version", () => {
    runMigrations(conn, [m0, m1]);
    expect(() => runMigrations(conn, [m0])).toThrow(/newer version/);
  });

  it("refuses a database whose history diverges", () => {
    runMigrations(conn, [m0, m1]);
    const renamed = { ...m1, tag: "0001_something_else" };
    expect(() => runMigrations(conn, [m0, renamed])).toThrow(/diverges at position 1/);
  });
});
