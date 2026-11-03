import { request, type IncomingMessage, type ServerResponse } from "node:http";
import { connect, type Socket } from "node:net";

import { allowedHost, sendText } from "./http";

/**
 * Dev mode only: forwards UI requests and the HMR websocket to `next dev`.
 * Headers pass through unchanged, so Next sees the browser's own Host and
 * Origin (this server's), and treats the requests as same-origin.
 */
export function createDevProxy(target: URL) {
  const host = target.hostname;
  const port = Number(target.port);

  function handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
    return new Promise((done) => {
      const upstream = request({ host, port, method: req.method, path: req.url, headers: req.headers }, (up) => {
        res.writeHead(up.statusCode ?? 502, up.headers);
        up.pipe(res);
        res.on("close", () => done());
      });
      upstream.on("error", () => {
        if (!res.headersSent) sendText(res, 502, `next dev isn't answering at ${target.origin} yet. Reload in a moment.`);
        else res.destroy();
        done();
      });
      req.pipe(upstream);
    });
  }

  /** Tunnels a websocket upgrade (Next's HMR) to `next dev`. */
  function upgrade(req: IncomingMessage, socket: Socket, head: Buffer): void {
    const hostHeader = allowedHost(req);
    const origin = req.headers.origin;
    if (!hostHeader || (origin !== undefined && origin !== `http://${hostHeader}`)) {
      socket.destroy();
      return;
    }
    const upstream = connect(port, host, () => {
      const lines = [`${req.method} ${req.url} HTTP/${req.httpVersion}`];
      for (let i = 0; i < req.rawHeaders.length; i += 2) lines.push(`${req.rawHeaders[i]}: ${req.rawHeaders[i + 1]}`);
      upstream.write(`${lines.join("\r\n")}\r\n\r\n`);
      if (head.length) upstream.write(head);
      socket.pipe(upstream).pipe(socket);
    });
    upstream.on("error", () => socket.destroy());
    socket.on("error", () => upstream.destroy());
  }

  return { handle, upgrade };
}
