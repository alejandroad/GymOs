import { serializeError, type SerializedError } from "./protocol";

/** A set of async methods that can be called across a message boundary. */
// any[] because argument lists differ per method; RpcClient.call restores the types.
export type Api = { [method: string]: (...args: any[]) => Promise<unknown> };

export interface RpcRequest {
  kind: "rpc-request";
  id: number;
  method: string;
  args: unknown[];
}

export type RpcResponse =
  | { kind: "rpc-response"; id: number; ok: true; result: unknown }
  | { kind: "rpc-response"; id: number; ok: false; error: SerializedError };

/** Anything with postMessage and message events: Worker, MessagePort, BroadcastChannel. */
export interface Endpoint {
  postMessage(message: unknown): void;
  addEventListener(type: "message", listener: (event: MessageEvent) => void): void;
  removeEventListener(type: "message", listener: (event: MessageEvent) => void): void;
}

/** An error thrown on the other side of the bridge, rethrown on this side. */
export class RpcError extends Error {
  constructor(error: SerializedError) {
    super(error.message);
    this.name = error.name;
  }
}

export interface RpcClient<A extends Api> {
  call<M extends keyof A & string>(
    method: M,
    ...args: Parameters<A[M]>
  ): Promise<Awaited<ReturnType<A[M]>>>;
  /** Rejects every pending call and stops listening. */
  dispose(reason?: string): void;
}

export function isRpcRequest(message: unknown): message is RpcRequest {
  return (message as RpcRequest | null)?.kind === "rpc-request";
}

function isRpcResponse(message: unknown): message is RpcResponse {
  return (message as RpcResponse | null)?.kind === "rpc-response";
}

/**
 * Calls methods of an Api served on the other end of `endpoint`. Each call
 * gets an ID, and its response settles exactly that call. Messages that
 * aren't RPC responses are ignored, so the endpoint can carry other traffic.
 */
export function createRpcClient<A extends Api>(endpoint: Endpoint): RpcClient<A> {
  let nextId = 1;
  const pending = new Map<number, { resolve(value: unknown): void; reject(error: Error): void }>();

  const onMessage = (event: MessageEvent) => {
    const message: unknown = event.data;
    if (!isRpcResponse(message)) return;
    const call = pending.get(message.id);
    if (!call) return;
    pending.delete(message.id);
    if (message.ok) call.resolve(message.result);
    else call.reject(new RpcError(message.error));
  };
  endpoint.addEventListener("message", onMessage);

  return {
    call(method, ...args) {
      const id = nextId++;
      const request: RpcRequest = { kind: "rpc-request", id, method, args };
      return new Promise((resolve, reject) => {
        pending.set(id, { resolve: resolve as (value: unknown) => void, reject });
        endpoint.postMessage(request);
      });
    },
    dispose(reason = "RPC client disposed") {
      endpoint.removeEventListener("message", onMessage);
      for (const call of pending.values()) call.reject(new RpcError({ name: "RpcDisposed", message: reason }));
      pending.clear();
    },
  };
}

/** The outcome of one call, independent of transport. */
export type RpcResult = { ok: true; result: unknown } | { ok: false; error: SerializedError };

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

/** Runs one request against `api` and builds its response. Never throws. */
export async function handleRpcRequest(api: Api, request: RpcRequest): Promise<RpcResponse> {
  return { kind: "rpc-response", id: request.id, ...(await handleRpcCall(api, request.method, request.args)) };
}

/** Serves `api` on `endpoint`. Returns a function that stops serving. */
export function serveRpc(endpoint: Endpoint, api: Api): () => void {
  const onMessage = (event: MessageEvent) => {
    const message: unknown = event.data;
    if (!isRpcRequest(message)) return;
    void handleRpcRequest(api, message).then((response) => endpoint.postMessage(response));
  };
  endpoint.addEventListener("message", onMessage);
  return () => endpoint.removeEventListener("message", onMessage);
}
