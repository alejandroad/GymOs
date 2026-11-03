export { connectStore, type StoreClient } from "./client";
export { createHttpStore, ServerUnreachableError, type StoreHttpClient } from "./http-client";
export type { LocalStoreApi, LocalStoreInfo } from "./protocol";
export type { Role, SerializedError, SpikeNote, StoreApi, StoreInfo, WorkerEvent } from "./protocol";
export { RpcError } from "./rpc";
export {
  estimate,
  persisted,
  requestPersistence,
  storageState,
  type StorageState,
} from "./durability";
