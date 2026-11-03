import type { StoreApi } from "./protocol";
import { RpcError, type RpcResult } from "./rpc";

/** The local server didn't answer: it isn't running, or it stopped. */
export class ServerUnreachableError extends Error {
  override name = "ServerUnreachableError";
}

export interface StoreHttpClient {
  call<M extends keyof StoreApi & string>(
    method: M,
    ...args: Parameters<StoreApi[M]>
  ): Promise<Awaited<ReturnType<StoreApi[M]>>>;
}

/**
 * Calls the local server's store API. The UI is served by the same server,
 * so the endpoint is same-origin, which its Origin check requires.
 *
 * Errors thrown by the store come back as RpcError with their original
 * name; a server that is down gives ServerUnreachableError.
 */
export function createHttpStore(endpoint = "/api/rpc"): StoreHttpClient {
  return {
    async call(method, ...args) {
      let response: Response;
      try {
        response = await fetch(endpoint, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ method, args }),
        });
      } catch (error) {
        throw new ServerUnreachableError("Can't reach the GymOs server. Is it still running?", { cause: error });
      }
      if (!response.ok) {
        throw new RpcError({ name: "HttpError", message: `${response.status}: ${(await response.text()).trim()}` });
      }
      const result = (await response.json()) as RpcResult;
      if (!result.ok) throw new RpcError(result.error);
      return result.result as never;
    },
  };
}
