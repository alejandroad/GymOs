"use client";

import { newId } from "@gymos/schema";
import {
  connectStore,
  requestPersistence,
  storageState,
  type Role,
  type SpikeNote,
  type StorageState,
  type StoreClient,
  type StoreInfo,
} from "@gymos/store";
import { useCallback, useEffect, useState } from "react";

const LIST_LIMIT = 20;

/** One store worker per tab, created on first use in the browser. */
let client: StoreClient | null = null;
let connectedAt = 0;

function store(): StoreClient {
  if (!client) {
    connectedAt = performance.now();
    client = connectStore();
  }
  return client;
}

/** Times one round trip through the worker (and the leader tab, for followers). */
async function timed<T>(run: () => Promise<T>): Promise<{ value: T; ms: number }> {
  const start = performance.now();
  const value = await run();
  return { value, ms: performance.now() - start };
}

export interface SpikeState {
  role: Role | null;
  workerId: string | null;
  info: StoreInfo | null;
  /** UI-measured: from creating the worker until the first answer from the database. */
  readyMs: number | null;
  notes: SpikeNote[];
  count: number | null;
  lastQueryMs: number | null;
  storage: StorageState | null;
  error: string | null;
}

const initial: SpikeState = {
  role: null,
  workerId: null,
  info: null,
  readyMs: null,
  notes: [],
  count: null,
  lastQueryMs: null,
  storage: null,
  error: null,
};

export function useSpikeStore() {
  const [state, setState] = useState<SpikeState>(initial);

  const refresh = useCallback(async () => {
    const s = store();
    const [{ value: notes, ms }, count, storage] = await Promise.all([
      timed(() => s.call("listNotes", LIST_LIMIT)),
      s.call("countNotes"),
      storageState(),
    ]);
    setState((prev) => ({ ...prev, notes, count, storage, lastQueryMs: ms }));
  }, []);

  useEffect(() => {
    const s = store();
    let active = true;
    const fail = (error: unknown) => {
      if (active) setState((prev) => ({ ...prev, error: error instanceof Error ? error.message : String(error) }));
    };

    const unsubscribe = s.onEvent((event) => {
      if (!active) return;
      if (event.type === "role") {
        setState((prev) => ({ ...prev, role: event.role, workerId: event.workerId }));
        // The leader may have changed: re-read which worker owns the database.
        void s.call("info").then((info) => active && setState((prev) => ({ ...prev, info })), fail);
      } else if (event.type === "changed") {
        void refresh().catch(fail);
      } else if (event.type === "fatal") {
        fail(new Error(`${event.error.name}: ${event.error.message}`));
      }
    });

    void s.started.then(async ({ role, workerId }) => {
      if (!active) return;
      setState((prev) => ({ ...prev, role, workerId }));
      const info = await s.call("info");
      const readyMs = performance.now() - connectedAt;
      if (active) setState((prev) => ({ ...prev, info, readyMs: prev.readyMs ?? readyMs }));
      await requestPersistence();
      await refresh();
    }).catch(fail);

    return () => {
      active = false;
      unsubscribe();
    };
  }, [refresh]);

  const addNote = useCallback(
    async (body: string) => {
      const { ms } = await timed(() =>
        store().call("addNote", { id: newId(), body, createdAt: Date.now() }),
      );
      setState((prev) => ({ ...prev, lastQueryMs: ms }));
    },
    [],
  );

  /** Median latency of `runs` inserts and `runs` selects, for the spike report. */
  const benchmark = useCallback(async (runs = 50) => {
    const s = store();
    const insert: number[] = [];
    const select: number[] = [];
    for (let i = 0; i < runs; i++) {
      insert.push((await timed(() => s.call("addNote", { id: newId(), body: `bench ${i}`, createdAt: Date.now() }))).ms);
    }
    for (let i = 0; i < runs; i++) {
      select.push((await timed(() => s.call("listNotes", LIST_LIMIT))).ms);
    }
    return { runs, insertMedianMs: median(insert), selectMedianMs: median(select) };
  }, []);

  return { state, addNote, benchmark };
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid]! : (sorted[mid - 1]! + sorted[mid]!) / 2;
}
