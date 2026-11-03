import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import type { IncomingMessage, ServerResponse } from "node:http";
import { extname, join, resolve, sep } from "node:path";

import { sendText } from "./http";

const MIME_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  // Next's RSC payloads for client navigation.
  ".txt": "text/plain; charset=utf-8",
  ".webmanifest": "application/manifest+json",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".woff2": "font/woff2",
  ".wasm": "application/wasm",
};

/** Content-hashed build assets never change at the same URL. */
const IMMUTABLE_PREFIX = "/_next/static/";

/**
 * Serves the Next.js static export in `root` (apps/web/out).
 *
 * - Clean URLs, as the export expects: `/spike` serves `spike.html`, and
 *   `/docs/` serves `docs/index.html`.
 * - Paths are resolved and must stay inside `root`, so `..` (raw, encoded
 *   or backslashed) can't reach other files.
 * - HTML and everything else revalidate on every load (`no-cache`), so a
 *   `git pull` and restart shows the new UI at once. Only hashed assets
 *   under `/_next/static/` are cached long-term.
 */
export function createStaticHandler(root: string) {
  const base = resolve(root);

  return async (req: IncomingMessage, res: ServerResponse, pathname: string): Promise<void> => {
    if (req.method !== "GET" && req.method !== "HEAD") {
      res.setHeader("Allow", "GET, HEAD");
      return sendText(res, 405, "Method not allowed.");
    }

    let decoded: string;
    try {
      decoded = decodeURIComponent(pathname);
    } catch {
      return sendText(res, 400, "Bad request.");
    }
    if (decoded.includes("\0")) return sendText(res, 400, "Bad request.");

    const target = resolve(base, `.${decoded}`);
    if (target !== base && !target.startsWith(base + sep)) return sendText(res, 404, "Not found.");

    const candidates = decoded.endsWith("/")
      ? [join(target, "index.html")]
      : [target, `${target}.html`, join(target, "index.html")];
    for (const file of candidates) {
      const size = await fileSize(file);
      if (size !== null) return sendFile(req, res, file, size, 200, cacheControl(pathname));
    }

    const notFound = join(base, "404.html");
    const size = await fileSize(notFound);
    if (size !== null) return sendFile(req, res, notFound, size, 404, "no-cache");
    return sendText(res, 404, "Not found.");
  };
}

function cacheControl(pathname: string): string {
  return pathname.startsWith(IMMUTABLE_PREFIX) ? "public, max-age=31536000, immutable" : "no-cache";
}

/** The size of `path` if it is a regular file, otherwise null. */
async function fileSize(path: string): Promise<number | null> {
  try {
    const stats = await stat(path);
    return stats.isFile() ? stats.size : null;
  } catch {
    return null;
  }
}

function sendFile(
  req: IncomingMessage,
  res: ServerResponse,
  file: string,
  size: number,
  status: number,
  cache: string,
): Promise<void> {
  res.writeHead(status, {
    "Content-Type": MIME_TYPES[extname(file).toLowerCase()] ?? "application/octet-stream",
    "Content-Length": size,
    "Cache-Control": cache,
  });
  if (req.method === "HEAD") {
    res.end();
    return Promise.resolve();
  }
  return new Promise((done) => {
    const stream = createReadStream(file);
    stream.on("error", () => res.destroy());
    res.on("close", () => {
      stream.destroy();
      done();
    });
    stream.pipe(res);
  });
}
