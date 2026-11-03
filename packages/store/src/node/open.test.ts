import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";

import { newId } from "@gymos/schema";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MigrationError } from "../migrate";
import { migrations } from "../migrations.generated";
import { openStore, StoreLockedError, type LocalStore } from "./open";

let dir: string;
let dbPath: string;
let opened: LocalStore[] = [];

function open(): LocalStore {
  const store = openStore({ dbPath, busyTimeoutMs: 50 });
  opened.push(store);
  return store;
}

function closeAll(): void {
  for (const store of opened) store.close();
  opened = [];
}

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "gymos-store-"));
  dbPath = join(dir, "nested", "gymos.sqlite3");
});

afterEach(() => {
  closeAll();
  rmSync(dir, { recursive: true, force: true });
});

describe("openStore", () => {
  it("creates the folder and the file, migrates, and reports facts about it", async () => {
    const { api, info } = open();
    expect(info.dbPath).toBe(dbPath);
    expect(info.schemaVersion).toBe(migrations.length);
    expect(info.sqliteVersion).toMatch(/^3\.\d+\.\d+$/);
    expect(await api.countNotes()).toBe(0);
    expect(await api.integrityCheck()).toBe("ok");
  });

  it("keeps notes across close and reopen", async () => {
    const note = { id: newId(), body: "kept", createdAt: Date.now() };
    await open().api.addNote(note);
    closeAll();

    expect(await open().api.listNotes(10)).toEqual([note]);
  });

  it("refuses a second opener while the first holds the file", () => {
    open();
    expect(() => openStore({ dbPath, busyTimeoutMs: 50 })).toThrow(StoreLockedError);
  });

  it("refuses a database from a newer app version and leaves its history alone", () => {
    open();
    closeAll();
    const raw = new DatabaseSync(dbPath);
    raw.exec("INSERT INTO __migrations (idx, tag, applied_at) VALUES (999, '9999_from_the_future', 0)");
    raw.close();

    expect(() => open()).toThrow(MigrationError);

    const check = new DatabaseSync(dbPath);
    const tags = check.prepare("SELECT tag FROM __migrations ORDER BY idx").all().map((row) => row.tag);
    check.close();
    expect(tags).toEqual([...migrations.map((m) => m.tag), "9999_from_the_future"]);
  });

  it("refuses a file that isn't a SQLite database", () => {
    const junk = join(dir, "junk.sqlite3");
    writeFileSync(junk, "plain text, long enough to fill where a SQLite header would be ".repeat(4));
    expect(() => openStore({ dbPath: junk })).toThrow();
  });
});
