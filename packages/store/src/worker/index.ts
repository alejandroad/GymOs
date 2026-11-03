import { createDrizzle, type StoreDb } from "../drizzle";
import { runMigrations } from "../migrate";
import { migrations } from "../migrations.generated";
import { addNote, countNotes, listNotes } from "../notes";
import { serializeError, type StoreApi, type StoreInfo, type WorkerEvent } from "../protocol";
import { serveRpc } from "../rpc";
import { connectionFor } from "../sql";
import { openDatabase } from "./open";
import { workerScope } from "./scope";

const startedAt = performance.now();

function emit(event: WorkerEvent): void {
  workerScope.postMessage(event);
}

/** Opens the database and migrates it before any query is served. */
async function start(): Promise<{ db: StoreDb; info: StoreInfo }> {
  const { db: raw, vfs, sqliteVersion } = await openDatabase();
  const conn = connectionFor(raw);
  runMigrations(conn, migrations);
  return {
    db: createDrizzle(conn),
    info: { vfs, sqliteVersion, schemaVersion: migrations.length, initMs: performance.now() - startedAt },
  };
}

const started = start();

const api: StoreApi = {
  info: async () => (await started).info,
  addNote: async (body) => addNote((await started).db, body),
  listNotes: async (limit) => listNotes((await started).db, limit),
  countNotes: async () => countNotes((await started).db),
};

serveRpc(workerScope, api);

started.then(
  ({ info }) => emit({ type: "ready", info }),
  (error: unknown) => emit({ type: "fatal", error: serializeError(error) }),
);
