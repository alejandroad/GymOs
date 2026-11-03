// Server-side entry: the on-disk database through node:sqlite. Browser code
// imports the package root instead.
export { MigrationError } from "./migrate";
export { connectionFor } from "./node/connection";
export { openStore, StoreLockedError, type LocalStore, type OpenStoreOptions } from "./node/open";
export type { StoreApi, StoreInfo } from "./protocol";
export { handleRpcCall, type Api, type RpcResult } from "./rpc";
