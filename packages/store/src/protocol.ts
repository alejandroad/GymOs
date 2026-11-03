/** Facts about the opened database, reported once the worker is ready. */
export interface StoreInfo {
  vfs: string;
  sqliteVersion: string;
  /** Milliseconds from worker start until the database was open. */
  initMs: number;
}

/** Messages the worker sends to its tab outside of request/response. */
export type WorkerEvent =
  | { type: "ready"; info: StoreInfo }
  | { type: "fatal"; error: SerializedError };

export interface SerializedError {
  name: string;
  message: string;
}

export function serializeError(error: unknown): SerializedError {
  if (error instanceof Error) return { name: error.name, message: error.message };
  return { name: "Error", message: String(error) };
}
