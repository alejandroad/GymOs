import { createDrizzle, type StoreDb } from "../drizzle";
import { runMigrations } from "../migrate";
import { migrations } from "../migrations.generated";
import { addNote, countNotes, listNotes } from "../notes";
import {
  serializeError,
  type BusMessage,
  type StoreApi,
  type StoreInfo,
  type WorkerEvent,
} from "../protocol";
import { handleRpcRequest, isRpcRequest, type RpcRequest, type RpcResponse } from "../rpc";
import { connectionFor } from "../sql";
import { openDatabase } from "./open";
import { workerScope } from "./scope";

/**
 * The store worker. One runs per tab, but only one tab's worker (the leader)
 * opens the database: opfs-sahpool allows a single opener per origin.
 *
 * - Leader election: every worker queues for the same Web Lock. The holder
 *   is the leader until its tab closes, then the next in line takes over.
 * - Followers forward their tab's requests to the leader over a
 *   BroadcastChannel and resend unanswered ones when a new leader appears.
 *   Writes are idempotent (client-generated IDs), so a resend is safe.
 */

const WRITER_LOCK = "gymos-store-writer";
const BUS_NAME = "gymos-store";

const startedAt = performance.now();
const workerId = crypto.randomUUID();
const bus = new BroadcastChannel(BUS_NAME);

/** Set once this worker holds the writer lock. */
let started: Promise<{ db: StoreDb; info: StoreInfo }> | null = null;
/** This tab's requests waiting on another tab's leader, by request ID. */
const forwarded = new Map<number, RpcRequest>();

function emit(event: WorkerEvent): void {
  workerScope.postMessage(event);
}

function post(message: BusMessage): void {
  bus.postMessage(message);
}

async function open(): Promise<{ db: StoreDb; info: StoreInfo }> {
  const { db: raw, vfs, sqliteVersion } = await openDatabase();
  const conn = connectionFor(raw);
  runMigrations(conn, migrations);
  const initMs = performance.now() - startedAt;
  return {
    db: createDrizzle(conn),
    info: { vfs, sqliteVersion, schemaVersion: migrations.length, initMs, leaderId: workerId },
  };
}

function owned(): Promise<{ db: StoreDb; info: StoreInfo }> {
  if (!started) throw new Error("This tab doesn't own the database");
  return started;
}

/** Runs only in the leader. */
const api: StoreApi = {
  info: async () => (await owned()).info,
  addNote: async (note) => {
    const saved = await addNote((await owned()).db, note);
    post({ kind: "changed" });
    emit({ type: "changed" });
    return saved;
  },
  listNotes: async (limit) => listNotes((await owned()).db, limit),
  countNotes: async () => countNotes((await owned()).db),
};

function answer(response: RpcResponse): void {
  forwarded.delete(response.id);
  workerScope.postMessage(response);
}

async function route(request: RpcRequest): Promise<void> {
  if (started) {
    answer(await handleRpcRequest(api, request));
  } else {
    forwarded.set(request.id, request);
    post({ kind: "forward", from: workerId, request });
  }
}

workerScope.addEventListener("message", (event) => {
  const message: unknown = event.data;
  if (isRpcRequest(message)) void route(message);
});

bus.addEventListener("message", (event: MessageEvent<BusMessage>) => {
  const message = event.data;
  switch (message.kind) {
    case "forward":
      if (started) {
        void handleRpcRequest(api, message.request).then((response) =>
          post({ kind: "forward-response", to: message.from, response }),
        );
      }
      break;
    case "forward-response":
      // A resent request can be answered twice; only the first answer counts.
      if (message.to === workerId && forwarded.has(message.response.id)) answer(message.response);
      break;
    case "leader":
      for (const request of forwarded.values()) post({ kind: "forward", from: workerId, request });
      break;
    case "changed":
      emit({ type: "changed" });
      break;
  }
});

emit({ type: "role", role: "follower", workerId });

void navigator.locks.request(WRITER_LOCK, () => {
  started = open();
  emit({ type: "role", role: "leader", workerId });
  post({ kind: "leader", leaderId: workerId });
  started.catch((error: unknown) => emit({ type: "fatal", error: serializeError(error) }));
  // Requests this tab forwarded to the previous leader now run here.
  const orphaned = [...forwarded.values()];
  forwarded.clear();
  for (const request of orphaned) void route(request);
  // Hold the lock for the worker's lifetime; it's released when the tab closes.
  return new Promise<never>(() => {});
});
