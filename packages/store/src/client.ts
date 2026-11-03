import { isWorkerEvent, type Role, type StoreApi, type WorkerEvent } from "./protocol";
import { createRpcClient, RpcError, type RpcClient } from "./rpc";

export interface StoreClient extends Pick<RpcClient<StoreApi>, "call"> {
  /** Resolves with this tab's first known role, once the worker is running. */
  started: Promise<{ role: Role; workerId: string }>;
  /** Subscribes to worker events. Returns an unsubscribe function. */
  onEvent(listener: (event: WorkerEvent) => void): () => void;
  /** Stops the worker and rejects pending calls. */
  close(): void;
}

/**
 * Starts this tab's store worker and returns a typed client for it. The
 * worker (or, in another tab, the leader it forwards to) owns the database;
 * the UI never touches SQLite directly.
 */
export function connectStore(): StoreClient {
  // `new Worker(new URL(..., import.meta.url))` is the form bundlers
  // (Turbopack and webpack) recognize to emit the worker as its own chunk.
  const worker = new Worker(new URL("./worker/index.ts", import.meta.url), {
    type: "module",
    name: "gymos-store",
  });
  const rpc = createRpcClient<StoreApi>(worker);
  const listeners = new Set<(event: WorkerEvent) => void>();

  const started = new Promise<{ role: Role; workerId: string }>((resolve, reject) => {
    worker.addEventListener("message", (event: MessageEvent) => {
      const message: unknown = event.data;
      if (!isWorkerEvent(message)) return;
      if (message.type === "role") resolve({ role: message.role, workerId: message.workerId });
      for (const listener of listeners) listener(message);
    });
    worker.addEventListener("error", (event) =>
      reject(new RpcError({ name: "WorkerError", message: event.message || "Store worker failed to load" })),
    );
  });

  return {
    started,
    call: rpc.call,
    onEvent(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    close() {
      rpc.dispose("Store closed");
      worker.terminate();
    },
  };
}
