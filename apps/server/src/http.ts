import type { IncomingMessage, ServerResponse } from "node:http";

import { handleRpcCall, type Api } from "@gymos/store/node";

/** Largest accepted RPC request body. */
export const MAX_BODY_BYTES = 1024 * 1024;

export interface AppOptions {
  api: Api;
  /** Serves everything outside /api/. Without it those paths are 404. */
  fallback?: (req: IncomingMessage, res: ServerResponse, pathname: string) => Promise<void> | void;
  maxBodyBytes?: number;
}

export type RequestHandler = (req: IncomingMessage, res: ServerResponse) => Promise<void>;

/**
 * The local server's request handler.
 *
 * The server listens on 127.0.0.1 only, but any website the user visits can
 * still send requests to localhost. So:
 * - Every request must name this server in `Host` (`127.0.0.1:<port>` or
 *   `localhost:<port>`). That defeats DNS rebinding, where an attacker's
 *   hostname resolves to 127.0.0.1.
 * - `POST /api/rpc` also requires a same-origin `Origin` and a JSON content
 *   type. A cross-site JSON POST needs a CORS preflight, and this server
 *   never sends CORS headers, so browsers never let one through.
 */
export function createApp({ api, fallback, maxBodyBytes = MAX_BODY_BYTES }: AppOptions): RequestHandler {
  async function rpc(req: IncomingMessage, res: ServerResponse, host: string): Promise<void> {
    if (req.method !== "POST") {
      res.setHeader("Allow", "POST");
      return reject(res, 405, "Method not allowed.");
    }
    if (req.headers.origin !== `http://${host}`) return reject(res, 403, "Forbidden: cross-origin request.");
    const site = req.headers["sec-fetch-site"];
    if (site !== undefined && site !== "same-origin") return reject(res, 403, "Forbidden: cross-site request.");
    const mediaType = req.headers["content-type"]?.split(";")[0]?.trim().toLowerCase();
    if (mediaType !== "application/json") return reject(res, 415, "Content-Type must be application/json.");

    const body = await readBody(req, maxBodyBytes);
    if (body === null) return reject(res, 413, "Request body too large.");

    let call: unknown;
    try {
      call = JSON.parse(body.toString("utf8"));
    } catch {
      return sendText(res, 400, "Request body must be JSON.");
    }
    if (!isCall(call)) return sendText(res, 400, 'Request body must be {"method": string, "args": array}.');

    sendJson(res, 200, await handleRpcCall(api, call.method, call.args));
  }

  return async (req, res) => {
    setSecurityHeaders(res);
    try {
      const host = allowedHost(req);
      if (!host) return sendText(res, 403, "Forbidden: this server only answers to 127.0.0.1 and localhost.");
      if (!req.url?.startsWith("/")) return sendText(res, 400, "Bad request.");

      const { pathname } = new URL(req.url, `http://${host}`);
      if (pathname === "/api/rpc") return await rpc(req, res, host);
      if (pathname === "/api" || pathname.startsWith("/api/")) return sendText(res, 404, "Not found.");
      if (fallback) return await fallback(req, res, pathname);
      return sendText(res, 404, "Not found.");
    } catch (error) {
      console.error(error);
      if (!res.headersSent) sendText(res, 500, "Internal server error.");
      else res.destroy();
    }
  };
}

/** The request's Host if it names this server, lowercased; otherwise null. */
export function allowedHost(req: IncomingMessage): string | null {
  const port = req.socket.localPort;
  const host = req.headers.host?.toLowerCase();
  if (port === undefined || !host) return null;
  return host === `127.0.0.1:${port}` || host === `localhost:${port}` ? host : null;
}

function isCall(value: unknown): value is { method: string; args: unknown[] } {
  const call = value as { method?: unknown; args?: unknown } | null;
  return typeof call === "object" && call !== null && typeof call.method === "string" && Array.isArray(call.args);
}

/** The body, or null when it exceeds `limit` bytes. */
async function readBody(req: IncomingMessage, limit: number): Promise<Buffer | null> {
  const declared = Number(req.headers["content-length"]);
  if (declared > limit) return null;
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of req as AsyncIterable<Buffer>) {
    size += chunk.length;
    if (size > limit) return null;
    chunks.push(chunk);
  }
  return Buffer.concat(chunks);
}

export function setSecurityHeaders(res: ServerResponse): void {
  res.setHeader("X-Content-Type-Options", "nosniff");
  res.setHeader("X-Frame-Options", "DENY");
  res.setHeader("Referrer-Policy", "no-referrer");
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Resource-Policy", "same-origin");
}

/** Refuses a request without reading its body, and drops the connection. */
function reject(res: ServerResponse, status: number, message: string): void {
  res.setHeader("Connection", "close");
  sendText(res, status, message);
}

export function sendText(res: ServerResponse, status: number, message: string): void {
  const body = Buffer.from(`${message}\n`);
  res.writeHead(status, {
    "Content-Type": "text/plain; charset=utf-8",
    "Content-Length": body.length,
    "Cache-Control": "no-store",
  });
  res.end(body);
}

function sendJson(res: ServerResponse, status: number, value: unknown): void {
  const body = Buffer.from(JSON.stringify(value));
  res.writeHead(status, {
    "Content-Type": "application/json; charset=utf-8",
    "Content-Length": body.length,
    "Cache-Control": "no-store",
  });
  res.end(body);
}
