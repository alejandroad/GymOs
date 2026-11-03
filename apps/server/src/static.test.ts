import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { createApp } from "./http";
import { createStaticHandler } from "./static";
import { listen, type TestServer } from "./test-client";

let dir: string;
let server: TestServer;

function write(path: string, content: string): void {
  mkdirSync(dirname(join(dir, path)), { recursive: true });
  writeFileSync(join(dir, path), content);
}

beforeAll(async () => {
  dir = mkdtempSync(join(tmpdir(), "gymos-static-"));
  write("out/index.html", "home");
  write("out/spike.html", "spike page");
  write("out/spike/__next.spike/__PAGE__.txt", "rsc");
  write("out/docs/index.html", "docs index");
  write("out/404.html", "custom 404");
  write("out/_next/static/abc/app.js", "js");
  write("out/icons/icon.png", "png");
  write("secret.txt", "outside the web root");
  server = await listen(createApp({ api: {}, fallback: createStaticHandler(join(dir, "out")) }));
});

afterAll(async () => {
  await server.close();
  rmSync(dir, { recursive: true, force: true });
});

const get = (path: string, method = "GET") => server.request({ method, path });

describe("static UI", () => {
  it.each([
    ["/", "home"],
    ["/spike", "spike page"],
    ["/spike.html", "spike page"],
    ["/docs/", "docs index"],
    ["/docs", "docs index"],
    ["/spike/__next.spike/__PAGE__.txt", "rsc"],
  ])("serves %s with clean URLs", async (path, body) => {
    const res = await get(path);
    expect(res.status).toBe(200);
    expect(res.body).toBe(body);
  });

  it("sends the right content types", async () => {
    expect((await get("/")).headers["content-type"]).toBe("text/html; charset=utf-8");
    expect((await get("/_next/static/abc/app.js")).headers["content-type"]).toBe("text/javascript; charset=utf-8");
    expect((await get("/icons/icon.png")).headers["content-type"]).toBe("image/png");
  });

  it("caches only hashed build assets long-term", async () => {
    expect((await get("/_next/static/abc/app.js")).headers["cache-control"]).toBe("public, max-age=31536000, immutable");
    expect((await get("/spike")).headers["cache-control"]).toBe("no-cache");
    expect((await get("/icons/icon.png")).headers["cache-control"]).toBe("no-cache");
  });

  it("serves the export's 404 page for unknown paths", async () => {
    const res = await get("/nope");
    expect(res.status).toBe(404);
    expect(res.body).toBe("custom 404");
  });

  it.each([
    "/../secret.txt",
    "/%2e%2e/secret.txt",
    "/%2e%2e%2fsecret.txt",
    "/icons/..%2f..%2fsecret.txt",
    "/..%5csecret.txt",
    "/icons/%5c..%5c..%5csecret.txt",
  ])("never serves files outside the web root: %s", async (path) => {
    const res = await get(path);
    expect(res.body).not.toContain("outside the web root");
    expect([400, 404]).toContain(res.status);
  });

  it.each(["/%00", "/%E0%A4%A"])("refuses malformed paths: %s", async (path) => {
    expect((await get(path)).status).toBe(400);
  });

  it("answers HEAD without a body and refuses other methods", async () => {
    const head = await get("/spike", "HEAD");
    expect(head.status).toBe(200);
    expect(head.body).toBe("");
    const post = await get("/spike", "POST");
    expect(post.status).toBe(405);
    expect(post.headers.allow).toBe("GET, HEAD");
  });
});
