import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { createApp } from "./http";
import { listen, rpcHeaders, type TestServer } from "./test-client";

const api = {
  echo: async (...args: unknown[]) => args,
  fail: async () => {
    throw new TypeError("boom");
  },
};

let server: TestServer;

beforeAll(async () => {
  server = await listen(createApp({ api, maxBodyBytes: 256 }));
});

afterAll(() => server.close());

const call = (body: unknown, headers: Record<string, string> = rpcHeaders(server.origin)) =>
  server.request({ method: "POST", path: "/api/rpc", headers, body: JSON.stringify(body) });

describe("POST /api/rpc", () => {
  it("runs a same-origin call", async () => {
    const res = await call({ method: "echo", args: [1, "two"] });
    expect(res.status).toBe(200);
    expect(res.headers["content-type"]).toBe("application/json; charset=utf-8");
    expect(res.headers["cache-control"]).toBe("no-store");
    expect(JSON.parse(res.body)).toEqual({ ok: true, result: [1, "two"] });
  });

  it("accepts localhost as well as 127.0.0.1", async () => {
    const origin = `http://localhost:${server.port}`;
    const res = await call({ method: "echo", args: [] }, { ...rpcHeaders(origin), host: `localhost:${server.port}` });
    expect(res.status).toBe(200);
  });

  it("returns API errors as results, not HTTP errors", async () => {
    const res = await call({ method: "fail", args: [] });
    expect(res.status).toBe(200);
    expect(JSON.parse(res.body)).toEqual({ ok: false, error: { name: "TypeError", message: "boom" } });
  });

  it.each(["missing", "constructor", "toString", "__proto__", "hasOwnProperty"])(
    "treats %j as an unknown method",
    async (method) => {
      const res = await call({ method, args: [] });
      expect(JSON.parse(res.body)).toEqual({
        ok: false,
        error: { name: "Error", message: `Unknown method: ${method}` },
      });
    },
  );
});

describe("rejections", () => {
  it.each([
    ["an attacker's hostname (DNS rebinding)", "attacker.example"],
    ["the right address on another port", "127.0.0.1:1"],
    ["the LAN address", "192.168.1.20"],
  ])("refuses a Host of %s", async (_, hostValue) => {
    const host = hostValue.includes(":") ? hostValue : `${hostValue}:${server.port}`;
    const res = await call({ method: "echo", args: [] }, { ...rpcHeaders(server.origin), host });
    expect(res.status).toBe(403);
    expect((await server.request({ path: "/", headers: { host } })).status).toBe(403);
  });

  it.each([
    ["missing", undefined],
    ["another site", "https://attacker.example"],
    ["another local port", "http://127.0.0.1:1"],
    ["opaque (null)", "null"],
    ["https on this host", `https://127.0.0.1:PORT`],
  ])("refuses an Origin that is %s", async (_, originValue) => {
    const headers = rpcHeaders(server.origin);
    if (originValue === undefined) delete headers.origin;
    else headers.origin = originValue.replace("PORT", String(server.port));
    const res = await call({ method: "echo", args: [] }, headers);
    expect(res.status).toBe(403);
  });

  it("refuses a request the browser labels cross-site", async () => {
    const res = await call({ method: "echo", args: [] }, { ...rpcHeaders(server.origin), "sec-fetch-site": "cross-site" });
    expect(res.status).toBe(403);
  });

  it.each(["text/plain", "application/x-www-form-urlencoded", "multipart/form-data; boundary=x"])(
    "refuses Content-Type %s (no CORS-simple requests)",
    async (contentType) => {
      const res = await call({ method: "echo", args: [] }, { ...rpcHeaders(server.origin), "content-type": contentType });
      expect(res.status).toBe(415);
    },
  );

  it("refuses an oversized body", async () => {
    const res = await call({ method: "echo", args: ["x".repeat(300)] });
    expect(res.status).toBe(413);
  });

  it("refuses an oversized body sent without Content-Length", async () => {
    const res = await server.request({
      method: "POST",
      path: "/api/rpc",
      headers: { ...rpcHeaders(server.origin), "transfer-encoding": "chunked" },
      body: JSON.stringify({ method: "echo", args: ["x".repeat(300)] }),
    });
    expect(res.status).toBe(413);
  });

  it.each([
    ["not JSON", "{"],
    ["not a call", JSON.stringify({ method: "echo" })],
    ["a non-string method", JSON.stringify({ method: 1, args: [] })],
  ])("refuses a body that is %s", async (_, body) => {
    const res = await server.request({ method: "POST", path: "/api/rpc", headers: rpcHeaders(server.origin), body });
    expect(res.status).toBe(400);
  });

  it.each(["GET", "PUT", "OPTIONS"])("refuses %s and never sends CORS headers", async (method) => {
    const res = await server.request({
      method,
      path: "/api/rpc",
      headers: { origin: "https://attacker.example", "access-control-request-method": "POST" },
    });
    expect(res.status).toBe(405);
    expect(res.headers.allow).toBe("POST");
    expect(Object.keys(res.headers).filter((name) => name.startsWith("access-control-"))).toEqual([]);
  });

  it("answers unknown API paths with 404", async () => {
    expect((await server.request({ path: "/api/other" })).status).toBe(404);
  });

  it("refuses absolute-form request targets", async () => {
    expect((await server.request({ path: "http://attacker.example/" })).status).toBe(400);
  });

  it("sends security headers on every response", async () => {
    const res = await server.request({ path: "/api/other", headers: { host: "attacker.example" } });
    expect(res.headers["x-content-type-options"]).toBe("nosniff");
    expect(res.headers["x-frame-options"]).toBe("DENY");
    expect(res.headers["cross-origin-resource-policy"]).toBe("same-origin");
  });
});
