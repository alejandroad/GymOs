/**
 * The parts of DedicatedWorkerGlobalScope the store worker uses.
 *
 * The package compiles against the DOM lib so the UI-side code type-checks,
 * and the DOM and WebWorker libs can't be combined in one program. This
 * narrow interface keeps the worker typed without pulling in the WebWorker lib.
 */
export interface WorkerScope {
  postMessage(message: unknown): void;
  addEventListener(type: "message", listener: (event: MessageEvent) => void): void;
}

export const workerScope = globalThis as unknown as WorkerScope;
