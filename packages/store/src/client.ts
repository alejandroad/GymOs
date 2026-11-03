/**
 * Starts the store worker. The worker owns the database; the UI never
 * touches SQLite directly.
 *
 * The `new Worker(new URL(..., import.meta.url))` form is what bundlers
 * (Turbopack and webpack) recognize to emit the worker as its own chunk.
 */
export function startStoreWorker(): Worker {
  return new Worker(new URL("./worker/index.ts", import.meta.url), {
    type: "module",
    name: "gymos-store",
  });
}
