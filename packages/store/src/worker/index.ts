import { serializeError, type StoreApi, type StoreInfo, type WorkerEvent } from "../protocol";
import { serveRpc } from "../rpc";
import { openDatabase } from "./open";
import { workerScope } from "./scope";

const startedAt = performance.now();

function emit(event: WorkerEvent): void {
  workerScope.postMessage(event);
}

const ready: Promise<StoreInfo> = openDatabase().then(({ vfs, sqliteVersion }) => ({
  vfs,
  sqliteVersion,
  initMs: performance.now() - startedAt,
}));

const api: StoreApi = {
  info: () => ready,
};

serveRpc(workerScope, api);

ready.then(
  (info) => emit({ type: "ready", info }),
  (error: unknown) => emit({ type: "fatal", error: serializeError(error) }),
);
