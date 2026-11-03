"use client";

import { useState, type FormEvent } from "react";

import { useSpikeStore } from "@/lib/store/use-spike-store";

const ms = (value: number | null) => (value === null ? "…" : `${value.toFixed(1)} ms`);

/** E0 spike: proves the local server's on-disk SQLite database. Not product UI. */
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
      `median of ${result.runs} over HTTP: insert ${result.insertMedianMs.toFixed(2)} ms, select ${result.selectMedianMs.toFixed(2)} ms`,
    );
  }

  return (
    <main className="mx-auto max-w-2xl space-y-6 p-4">
      <h1 className="text-xl font-semibold">Local database spike</h1>

      {state.error && (
        <p role="alert" className="border border-red-600 p-3">
          {state.error} Start it with <code>pnpm start</code> in the GymOs folder, then reload this page.
        </p>
      )}

      <dl className="grid grid-cols-[max-content_1fr] gap-x-4 gap-y-1 text-sm" data-testid="status">
        <dt>Engine</dt>
        <dd data-testid="engine">
          {state.info ? `SQLite ${state.info.sqliteVersion} (node:sqlite), schema v${state.info.schemaVersion}` : "…"}
        </dd>
        <dt>Database file</dt>
        <dd data-testid="db-path" className="break-all font-mono">
          {state.info?.dbPath ?? "…"}
        </dd>
        <dt>Open and migrate (server)</dt>
        <dd data-testid="init-ms">{ms(state.info?.initMs ?? null)}</dd>
        <dt>First answer (this tab)</dt>
        <dd data-testid="ready-ms">{ms(state.readyMs)}</dd>
        <dt>Last query</dt>
        <dd data-testid="last-query-ms">{ms(state.lastQueryMs)}</dd>
        <dt>Rows</dt>
        <dd data-testid="count">{state.count ?? "…"}</dd>
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
