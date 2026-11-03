"use client";

import { newId } from "@gymos/schema";
import { createHttpStore, type LocalStoreInfo, type SpikeNote } from "@gymos/store";
import { useCallback, useEffect, useState } from "react";

const LIST_LIMIT = 20;

/** The local server that served this page also owns the database. */
const store = createHttpStore();

/** Times one HTTP round trip to the local server. */
async function timed<T>(run: () => Promise<T>): Promise<{ value: T; ms: number }> {
  const start = performance.now();
  const value = await run();
  return { value, ms: performance.now() - start };
}

export interface SpikeState {
  info: LocalStoreInfo | null;
  /** UI-measured: the first round trip to the server, from mount. */
  readyMs: number | null;
  notes: SpikeNote[];
  count: number | null;
  lastQueryMs: number | null;
  error: string | null;
}

const initial: SpikeState = {
  info: null,
  readyMs: null,
  notes: [],
  count: null,
  lastQueryMs: null,
  error: null,
};

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function useSpikeStore() {
  const [state, setState] = useState<SpikeState>(initial);

  const refresh = useCallback(async () => {
    const [{ value: notes, ms }, count] = await Promise.all([
      timed(() => store.call("listNotes", LIST_LIMIT)),
      store.call("countNotes"),
    ]);
    setState((prev) => ({ ...prev, notes, count, lastQueryMs: ms, error: null }));
  }, []);

  useEffect(() => {
    let active = true;
    const fail = (error: unknown) => active && setState((prev) => ({ ...prev, error: message(error) }));

    void timed(() => store.call("info"))
      .then(({ value: info, ms }) => {
        if (active) setState((prev) => ({ ...prev, info, readyMs: ms }));
        return refresh();
      })
      .catch(fail);

    // Other tabs write through the same server; catch up when this tab is shown again.
    const onShow = () => document.visibilityState === "visible" && void refresh().catch(fail);
    window.addEventListener("focus", onShow);
    document.addEventListener("visibilitychange", onShow);
    return () => {
      active = false;
      window.removeEventListener("focus", onShow);
      document.removeEventListener("visibilitychange", onShow);
    };
  }, [refresh]);

  const addNote = useCallback(
    async (body: string) => {
      const { ms } = await timed(() => store.call("addNote", { id: newId(), body, createdAt: Date.now() }));
      setState((prev) => ({ ...prev, lastQueryMs: ms }));
      await refresh();
    },
    [refresh],
  );

  /** Median latency of `runs` inserts and `runs` selects, HTTP round trips included. */
  const benchmark = useCallback(async (runs = 50) => {
    const insert: number[] = [];
    const select: number[] = [];
    for (let i = 0; i < runs; i++) {
      insert.push((await timed(() => store.call("addNote", { id: newId(), body: `bench ${i}`, createdAt: Date.now() }))).ms);
    }
    for (let i = 0; i < runs; i++) {
      select.push((await timed(() => store.call("listNotes", LIST_LIMIT))).ms);
    }
    await refresh();
    return { runs, insertMedianMs: median(insert), selectMedianMs: median(select) };
  }, [refresh]);

  const integrityCheck = useCallback(() => store.call("integrityCheck"), []);

  return { state, addNote, benchmark, integrityCheck };
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid]! : (sorted[mid - 1]! + sorted[mid]!) / 2;
}
