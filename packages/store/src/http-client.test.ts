import { afterEach, describe, expect, it, vi } from "vitest";

import { createHttpStore, ServerUnreachableError } from "./http-client";
import { RpcError } from "./rpc";

function respond(status: number, body: unknown): void {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => new Response(typeof body === "string" ? body : JSON.stringify(body), { status })),
  );
}

afterEach(() => vi.unstubAllGlobals());

describe("createHttpStore", () => {
  it("posts {method, args} as JSON to the endpoint and returns the result", async () => {
    respond(200, { ok: true, result: 3 });
    expect(await createHttpStore().call("countNotes")).toBe(3);

    const [url, init] = vi.mocked(fetch).mock.calls[0]!;
    expect(url).toBe("/api/rpc");
    expect(init).toMatchObject({ method: "POST", headers: { "Content-Type": "application/json" } });
    expect(JSON.parse(init!.body as string)).toEqual({ method: "countNotes", args: [] });
  });

  it("rethrows store errors with their name", async () => {
    respond(200, { ok: false, error: { name: "MigrationError", message: "nope" } });
    const error = await createHttpStore().call("countNotes").catch((e: unknown) => e);
    expect(error).toBeInstanceOf(RpcError);
    expect(error).toMatchObject({ name: "MigrationError", message: "nope" });
  });

  it("turns refused requests into HTTP errors", async () => {
    respond(403, "Forbidden: cross-origin request.\n");
    await expect(createHttpStore().call("countNotes")).rejects.toMatchObject({
      name: "HttpError",
      message: "403: Forbidden: cross-origin request.",
    });
  });

  it("reports a server that is down", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => Promise.reject(new TypeError("Failed to fetch"))));
    await expect(createHttpStore().call("countNotes")).rejects.toBeInstanceOf(ServerUnreachableError);
  });
});
