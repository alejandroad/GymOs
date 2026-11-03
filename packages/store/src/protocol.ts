/** Facts about the opened database, reported once the worker is ready. */
export interface StoreInfo {
  vfs: string;
  sqliteVersion: string;
  /** Milliseconds from worker start until the database was open. */
  initMs: number;
}

/**
 * The store's RPC surface. Every UI call goes through the worker; the UI
 * never touches SQLite. A type alias so it satisfies rpc's Api constraint.
 */
export type StoreApi = {
  info(): Promise<StoreInfo>;
};

/** Messages the worker sends to its tab outside of request/response. */
export type WorkerEvent =
  | { type: "ready"; info: StoreInfo }
  | { type: "fatal"; error: SerializedError };

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
