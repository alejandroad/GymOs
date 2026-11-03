import { describe, expect, it } from "vitest";

import { handleRpcCall } from "./rpc";

const api = {
  add: async (a: number, b: number) => a + b,
  fail: async (message: string) => {
    throw new TypeError(message);
  },
};

describe("handleRpcCall", () => {
  it("returns the result of a call", async () => {
    expect(await handleRpcCall(api, "add", [2, 3])).toEqual({ ok: true, result: 5 });
  });

  it("returns a thrown error's name and message instead of throwing", async () => {
    expect(await handleRpcCall(api, "fail", ["bad input"])).toEqual({
      ok: false,
      error: { name: "TypeError", message: "bad input" },
    });
  });

  it.each(["missing", "toString", "constructor", "__proto__"])("refuses unknown method %j", async (method) => {
    expect(await handleRpcCall(api, method, [])).toEqual({
      ok: false,
      error: { name: "Error", message: `Unknown method: ${method}` },
    });
  });
});
