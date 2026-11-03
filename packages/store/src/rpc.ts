import { serializeError, type SerializedError } from "./protocol";

/** A set of async methods that can be called across a process boundary. */
// any[] because argument lists differ per method; the HTTP client restores the types.
export type Api = { [method: string]: (...args: any[]) => Promise<unknown> };

/** The outcome of one call, as the server sends it back. */
export type RpcResult = { ok: true; result: unknown } | { ok: false; error: SerializedError };

/** An error thrown on the server, rethrown in the UI with its original name. */
export class RpcError extends Error {
  constructor(error: SerializedError) {
    super(error.message);
    this.name = error.name;
  }
}

/**
 * Runs `method` on `api`. Never throws. Only the API's own methods are
 * callable: inherited names such as `constructor` or `toString` count as
 * unknown.
 */
export async function handleRpcCall(api: Api, method: string, args: unknown[]): Promise<RpcResult> {
  try {
    const fn = Object.hasOwn(api, method) ? api[method] : undefined;
    if (typeof fn !== "function") throw new Error(`Unknown method: ${method}`);
    return { ok: true, result: await fn(...args) };
  } catch (error) {
    return { ok: false, error: serializeError(error) };
  }
}
