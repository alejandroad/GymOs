export { connectStore, type StoreClient } from "./client";
export type { Role, SerializedError, SpikeNote, StoreApi, StoreInfo, WorkerEvent } from "./protocol";
export { RpcError } from "./rpc";
export {
  estimate,
  persisted,
  requestPersistence,
  storageState,
  type StorageState,
} from "./durability";
