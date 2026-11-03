"use client";

import { useState, type FormEvent } from "react";

import { useSpikeStore } from "@/lib/store/use-spike-store";

const ms = (value: number | null) => (value === null ? "…" : `${value.toFixed(1)} ms`);
const mb = (bytes: number | null) => (bytes === null ? "n/a" : `${(bytes / 1024 / 1024).toFixed(2)} MB`);

/** E0-T3 spike: proves SQLite-WASM on OPFS in a worker. Not product UI. */
export default function SpikePage() {
  const { state, addNote, benchmark, integrityCheck } = useSpikeStore();
  const [body, setBody] = useState("");
  const [bench, setBench] = useState<string | null>(null);
  const [integrity, setIntegrity] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    const text = body.trim();
    if (!text) return;
    setBody("");
    await addNote(text);
  }

  async function runBenchmark() {
    setBench("running…");
    const result = await benchmark(50);
    setBench(
      `median of ${result.runs}: insert ${result.insertMedianMs.toFixed(2)} ms, select ${result.selectMedianMs.toFixed(2)} ms`,
    );
  }

  const isLeader = state.info !== null && state.info.leaderId === state.workerId;

  return (
    <main className="mx-auto max-w-2xl space-y-6 p-4">
      <h1 className="text-xl font-semibold">Local database spike</h1>

      {state.error && (
        <p role="alert" className="border border-red-600 p-3">
          Local storage is unavailable: {state.error}. GymOs needs OPFS, which some private or
          restricted browsing modes turn off.
        </p>
      )}

      <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-sm" data-testid="status">
        <dt>This tab</dt>
        <dd data-testid="role">
          {state.role === null ? "starting…" : isLeader ? "leader (owns the database)" : "follower (forwards to the leader)"}
        </dd>
        <dt>Engine</dt>
        <dd>{state.info ? `SQLite ${state.info.sqliteVersion} on ${state.info.vfs}, schema v${state.info.schemaVersion}` : "…"}</dd>
        <dt>Worker init (leader)</dt>
        <dd data-testid="init-ms">{ms(state.info?.initMs ?? null)}</dd>
        <dt>First answer (this tab)</dt>
        <dd data-testid="ready-ms">{ms(state.readyMs)}</dd>
        <dt>Last query</dt>
        <dd data-testid="last-query-ms">{ms(state.lastQueryMs)}</dd>
        <dt>Rows</dt>
        <dd data-testid="count">{state.count ?? "…"}</dd>
        <dt>Persisted</dt>
        <dd data-testid="persisted">{state.storage ? String(state.storage.persisted) : "…"}</dd>
        <dt>Storage</dt>
        <dd data-testid="estimate">
          {state.storage ? `${mb(state.storage.usage)} used of ${mb(state.storage.quota)}` : "…"}
        </dd>
      </dl>

      <form onSubmit={submit} className="flex gap-2">
        <input
          aria-label="Note"
          value={body}
          onChange={(event) => setBody(event.target.value)}
          className="flex-1 border px-2 py-1"
          placeholder="Write a note"
        />
        <button type="submit" className="border px-3 py-1">
          Add note
        </button>
      </form>

      <ol className="space-y-1 text-sm" data-testid="notes">
        {state.notes.map((note) => (
          <li key={note.id}>
            <time dateTime={new Date(note.createdAt).toISOString()}>
              {new Date(note.createdAt).toLocaleTimeString()}
            </time>{" "}
            {note.body}
          </li>
        ))}
      </ol>

      <p className="text-sm">
        <button type="button" onClick={runBenchmark} className="border px-3 py-1">
          Benchmark 50 inserts and selects
        </button>{" "}
        <span data-testid="bench">{bench}</span>
      </p>

      <p className="text-sm">
        <button type="button" onClick={async () => setIntegrity(await integrityCheck())} className="border px-3 py-1">
          Run integrity check
        </button>{" "}
        <span data-testid="integrity">{integrity}</span>
      </p>
    </main>
  );
}
