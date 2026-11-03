// Browser-safe entry: types and the HTTP client the UI uses to reach the
// local server. The server itself imports "@gymos/store/node".
export { createHttpStore, ServerUnreachableError, type StoreHttpClient } from "./http-client";
export type { SerializedError, SpikeNote, StoreApi, StoreInfo } from "./protocol";
export { RpcError } from "./rpc";
