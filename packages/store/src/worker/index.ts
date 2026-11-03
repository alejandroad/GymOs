import { serializeError, type WorkerEvent } from "../protocol";
import { openDatabase } from "./open";
import { workerScope } from "./scope";

const startedAt = performance.now();

function emit(event: WorkerEvent): void {
  workerScope.postMessage(event);
}

openDatabase().then(
  ({ vfs, sqliteVersion }) =>
    emit({ type: "ready", info: { vfs, sqliteVersion, initMs: performance.now() - startedAt } }),
  (error: unknown) => emit({ type: "fatal", error: serializeError(error) }),
);
