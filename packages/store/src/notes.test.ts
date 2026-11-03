import { newId } from "@gymos/schema";
import sqlite3InitModule, { type Database } from "@sqlite.org/sqlite-wasm";
import { afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";

import { createDrizzle, type StoreDb } from "./drizzle";
import { runMigrations } from "./migrate";
import { migrations } from "./migrations.generated";
import { addNote, countNotes, listNotes } from "./notes";
import type { SpikeNote } from "./schema";
import { connectionFor } from "./sql";

function note(body: string): SpikeNote {
  return { id: newId(), body, createdAt: Date.now() };
}

let sqlite3: Awaited<ReturnType<typeof sqlite3InitModule>>;
let raw: Database;
let db: StoreDb;

beforeAll(async () => {
  sqlite3 = await sqlite3InitModule();
});

beforeEach(() => {
  raw = new sqlite3.oo1.DB(":memory:");
  const conn = connectionFor(raw);
  runMigrations(conn, migrations);
  db = createDrizzle(conn);
});

afterEach(() => raw.close());

describe("spike notes through drizzle", () => {
  it("starts empty after the generated migrations run", async () => {
    expect(await countNotes(db)).toBe(0);
    expect(await listNotes(db, 10)).toEqual([]);
  });

  it("stores notes with UUIDv7 IDs and lists them newest first", async () => {
    await addNote(db, note("first"));
    const second = await addNote(db, note("second"));

    expect(await countNotes(db)).toBe(2);
    expect((await listNotes(db, 10)).map((n) => n.body)).toEqual(["second", "first"]);
    expect(await listNotes(db, 1)).toEqual([second]);
  });

  it("inserts a retried note only once", async () => {
    const retried = note("sent twice");
    await addNote(db, retried);
    await addNote(db, retried);

    expect(await countNotes(db)).toBe(1);
  });
});
