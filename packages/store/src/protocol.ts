import type { RpcRequest, RpcResponse } from "./rpc";
import type { SpikeNote } from "./schema";

export type { SpikeNote };

/** Facts about the opened database, reported by the leader. */
export interface StoreInfo {
  vfs: string;
  sqliteVersion: string;
  /** Number of migrations applied to the database, i.e. its schema version. */
  schemaVersion: number;
  /** Milliseconds from the leader worker's start until the database was open and migrated. */
  initMs: number;
  /** ID of the worker that owns the database (the leader). */
  leaderId: string;
}

export type Role = "leader" | "follower";

/**
 * The store's RPC surface. Every UI call goes through the worker; the UI
 * never touches SQLite. A type alias so it satisfies rpc's Api constraint.
 */
export type StoreApi = {
  info(): Promise<StoreInfo>;
  /** Idempotent: the caller generates the note's UUIDv7. */
  addNote(note: SpikeNote): Promise<SpikeNote>;
  listNotes(limit: number): Promise<SpikeNote[]>;
  countNotes(): Promise<number>;
};

/** Messages the worker sends to its tab outside of request/response. */
export type WorkerEvent =
  /** This tab's worker is the leader (owns the database) or a follower. */
  | { type: "role"; role: Role; workerId: string }
  /** Data changed in some tab; re-run queries. */
  | { type: "changed" }
  /** The leader couldn't open or migrate the database. */
  | { type: "fatal"; error: SerializedError };

/** Messages between tabs' workers on the BroadcastChannel. */
export type BusMessage =
  /** A follower asks the leader to run its tab's request. */
  | { kind: "forward"; from: string; request: RpcRequest }
  /** The leader's answer, addressed to one follower. */
  | { kind: "forward-response"; to: string; response: RpcResponse }
  /** A worker took the writer lock; followers resend unanswered requests. */
  | { kind: "leader"; leaderId: string }
  /** The leader committed a write. */
  | { kind: "changed" };

/** Worker events share the channel with RPC traffic, which has no `type`. */
export function isWorkerEvent(message: unknown): message is WorkerEvent {
  return typeof message === "object" && message !== null && "type" in message;
}

export interface SerializedError {
  name: string;
  message: string;
}

export function serializeError(error: unknown): SerializedError {
  if (error instanceof Error) return { name: error.name, message: error.message };
  return { name: "Error", message: String(error) };
}
