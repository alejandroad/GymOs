import { isWorkerEvent, type StoreApi, type StoreInfo, type WorkerEvent } from "./protocol";
import { createRpcClient, RpcError, type RpcClient } from "./rpc";

export interface StoreClient extends Pick<RpcClient<StoreApi>, "call"> {
  /** Settles once the worker has opened the database (or failed to). */
  ready: Promise<StoreInfo>;
  /** Subscribes to worker events. Returns an unsubscribe function. */
  onEvent(listener: (event: WorkerEvent) => void): () => void;
  /** Stops the worker and rejects pending calls. */
  close(): void;
}

/**
 * Starts the store worker and returns a typed client for it. The worker
 * owns the database; the UI never touches SQLite directly.
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

  const ready = new Promise<StoreInfo>((resolve, reject) => {
    worker.addEventListener("message", (event: MessageEvent) => {
      const message: unknown = event.data;
      if (!isWorkerEvent(message)) return;
      if (message.type === "ready") resolve(message.info);
      else if (message.type === "fatal") reject(new RpcError(message.error));
      for (const listener of listeners) listener(message);
    });
    worker.addEventListener("error", (event) =>
      reject(new RpcError({ name: "WorkerError", message: event.message || "Store worker failed to load" })),
    );
  });

  return {
    ready,
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
