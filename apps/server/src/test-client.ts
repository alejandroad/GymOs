// Test helper: raw HTTP requests with full control over Host and Origin,
// which fetch() doesn't allow.
import { createServer, request as httpRequest, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import type { RequestHandler } from "./http";

export interface TestResponse {
  status: number;
  headers: Record<string, string | string[] | undefined>;
  body: string;
}

export interface TestServer {
  port: number;
  origin: string;
  request(options: {
    method?: string;
    path?: string;
    headers?: Record<string, string>;
    body?: string | Buffer;
  }): Promise<TestResponse>;
  close(): Promise<void>;
}

export async function listen(handler: RequestHandler): Promise<TestServer> {
  const server: Server = createServer((req, res) => void handler(req, res));
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as AddressInfo;
  const origin = `http://127.0.0.1:${port}`;

  return {
    port,
    origin,
    request: ({ method = "GET", path = "/", headers = {}, body }) =>
      new Promise((resolve, reject) => {
        const req = httpRequest(
          { host: "127.0.0.1", port, method, path, headers: { host: `127.0.0.1:${port}`, ...headers } },
          (res) => {
            const chunks: Buffer[] = [];
            res.on("data", (chunk: Buffer) => chunks.push(chunk));
            res.on("end", () =>
              resolve({ status: res.statusCode ?? 0, headers: res.headers, body: Buffer.concat(chunks).toString("utf8") }),
            );
          },
        );
        // The server may refuse and close before the whole body is sent.
        req.on("error", (error: NodeJS.ErrnoException) => (error.code === "ECONNRESET" || error.code === "EPIPE" ? undefined : reject(error)));
        req.end(body);
      }),
    close: () =>
      new Promise((resolve) => {
        server.closeAllConnections();
        server.close(() => resolve());
      }),
  };
}

/** Headers of a well-formed same-origin RPC call from the UI. */
export function rpcHeaders(origin: string): Record<string, string> {
  return { origin, "content-type": "application/json", "sec-fetch-site": "same-origin" };
}
