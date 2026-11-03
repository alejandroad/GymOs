import { describe, expect, it } from "vitest";

import { newId } from "./id";

const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

/** Reads the 48-bit Unix millisecond timestamp from the front of a UUIDv7. */
function timestampOf(id: string): number {
  return Number.parseInt(id.replaceAll("-", "").slice(0, 12), 16);
}

describe("newId", () => {
  it("returns a lowercase RFC 9562 UUIDv7", () => {
    expect(newId()).toMatch(UUID_V7);
  });

  it("sets the version nibble to 7 and the RFC 9562 variant bits to 10", () => {
    const hex = newId().replaceAll("-", "");

    expect(Number.parseInt(hex[12]!, 16)).toBe(7);
    expect(Number.parseInt(hex[16]!, 16) >> 2).toBe(0b10);
  });

  it("embeds the current time", () => {
    const before = Date.now();
    const id = newId();
    const after = Date.now();

    expect(timestampOf(id)).toBeGreaterThanOrEqual(before);
    expect(timestampOf(id)).toBeLessThanOrEqual(after);
  });

  it("is strictly increasing within a process, even in the same millisecond", () => {
    const ids = Array.from({ length: 10_000 }, () => newId());

    expect(new Set(ids).size).toBe(ids.length);
    expect([...ids].sort()).toEqual(ids);
  });

  it("orders IDs from different milliseconds by time", async () => {
    const first = newId();
    await new Promise((resolve) => setTimeout(resolve, 2));
    const second = newId();

    expect(timestampOf(second)).toBeGreaterThan(timestampOf(first));
    expect(second > first).toBe(true);
  });
});
