import { afterEach, describe, expect, it } from "vitest";

import { createRpcClient, RpcError, serveRpc, type Endpoint } from "./rpc";

// A type alias, not an interface: aliases satisfy Api's index signature.
type TestApi = {
  add(a: number, b: number): Promise<number>;
  slow(ms: number, value: string): Promise<string>;
  fail(message: string): Promise<never>;
};

const api: TestApi = {
  add: async (a, b) => a + b,
  slow: (ms, value) => new Promise((resolve) => setTimeout(() => resolve(value), ms)),
  fail: async (message) => {
    throw new TypeError(message);
  },
};

const channels: MessageChannel[] = [];

function connect() {
  const channel = new MessageChannel();
  channels.push(channel);
  channel.port1.start();
  channel.port2.start();
  serveRpc(channel.port2 as Endpoint, api);
  return createRpcClient<TestApi>(channel.port1 as Endpoint);
}

afterEach(() => {
  for (const { port1, port2 } of channels.splice(0)) {
    port1.close();
    port2.close();
  }
});

describe("rpc bridge", () => {
  it("returns the result of a call", async () => {
    await expect(connect().call("add", 2, 3)).resolves.toBe(5);
  });

  it("matches responses to calls by ID when they finish out of order", async () => {
    const client = connect();
    const results = await Promise.all([
      client.call("slow", 30, "first"),
      client.call("slow", 1, "second"),
      client.call("add", 1, 1),
    ]);
    expect(results).toEqual(["first", "second", 2]);
  });

  it("propagates the error name and message from the server", async () => {
    const error = await connect().call("fail", "bad input").catch((e: unknown) => e);
    expect(error).toBeInstanceOf(RpcError);
    expect(error).toMatchObject({ name: "TypeError", message: "bad input" });
  });

  it("rejects calls to unknown methods, including inherited properties", async () => {
    const call = connect().call as (method: string) => Promise<unknown>;
    await expect(call("missing")).rejects.toThrow("Unknown method: missing");
    await expect(call("toString")).rejects.toThrow("Unknown method: toString");
  });

  it("rejects pending calls on dispose", async () => {
    const client = connect();
    const call = client.call("slow", 50, "late");
    client.dispose("tab closed");
    await expect(call).rejects.toThrow("tab closed");
  });
});
