---
status: "accepted"
date: 2026-11-02
decision-makers: "Alejandro (owner)"
---

# Run GymOs as a local Node server that owns an on-disk SQLite file

## Context and Problem Statement

Handoff v1.5 (D18, D23) made GymOs local-first. The E0 spike (tickets T2–T5) built it as a browser PWA: SQLite-WASM persisted to OPFS inside a dedicated worker, with Web Locks leader election across tabs and a Serwist service worker for offline use. The spike worked in Windows Edge and Chrome. But browser storage reported `navigator.storage.persisted() === false`, which means the browser may evict the origin's data under storage pressure. "Clear browsing data" also deletes it. For an app whose pitch is "years of your training history, owned by you", data the browser can silently delete is disqualifying.

Where should the source-of-truth database live, and what runs it, so that a user's data is durable and fully under their control?

## Decision Drivers

* **Durable, user-owned data.** The database is a real file the user can see, copy and back up. No browser or OS policy can evict it.
* **Clone and run.** A person with Node and pnpm clones the repo, runs `pnpm install` and `pnpm start`, and has a working app. No installer, signing, admin rights or native toolchain.
* **Windows first, then macOS.** Linux stays green in CI. Mobile comes last.
* **Minimal toolchain and dependencies.** Supply-chain rules: exact pins, release-age gating, and install scripts denied by default.
* **Keep the investment.** The Next.js static UI, Drizzle schema, migration runner and RPC surface should carry over.

## Considered Options

* Keep the browser PWA (SQLite-WASM on OPFS, service worker)
* Local Node server with `node:sqlite` on disk, serving the static UI to any browser on `127.0.0.1`
* Tauri desktop app wrapping the same UI
* Electron desktop app wrapping the same UI

## Decision Outcome

Chosen option: **"Local Node server with `node:sqlite` on disk"**. It is the only option that gives durable, user-visible data without an installer or a native toolchain, and it reuses almost all of the spike's code.

Shape:

```
browser (any)  --HTTP-->  apps/server (Node 26, 127.0.0.1 only)
  Next static UI            ├─ serves apps/web/out/ (static files)
  fetch('/api/rpc')         ├─ POST /api/rpc → handleRpcRequest(storeApi, req)
                            └─ node:sqlite DatabaseSync → <data dir>/gymos.sqlite3
                               runMigrations() at startup, before listening
```

* **Database file.**
  * Lives in the OS app-data folder, not the repo:
    * Windows: `%APPDATA%\GymOs`;
    * macOS: `~/Library/Application Support/GymOs`;
    * Linux: `$XDG_DATA_HOME/gymos`, falling back to `~/.local/share/gymos`.
  * `GYMOS_DATA_DIR` overrides it.
  * The full path is printed at startup.
* **Pragmas:** `journal_mode=WAL`, `synchronous=FULL`, `foreign_keys=ON` and a busy timeout.
* **One process, one connection, one writer.**
  * `PRAGMA locking_mode=EXCLUSIVE` makes SQLite hold an OS file lock for the life of the process. A second server on the same file therefore fails at startup with a clear message.
  * The OS releases the lock when the process dies, even on a hard kill, so there is no stale lock file to clean up.
* **Network surface.**
  * The server binds to `127.0.0.1` only.
  * Every request must carry a `Host` of `127.0.0.1:<port>` or `localhost:<port>`, which blocks DNS rebinding.
  * `POST /api/rpc` requires a same-origin `Origin` and `Content-Type: application/json`. The server sends no CORS headers, so other websites' cross-origin requests are refused.
  * The request body is capped.
* **No service worker.** A service worker caching `localhost` would serve a stale UI after `git pull`.
* **Defaults:** port `4321` (`GYMOS_PORT`). `pnpm dev` uses port `4322` and a separate `-dev` data folder, so it never touches real data.

### Consequences

* Good, because the data is an ordinary SQLite file in a well-known folder. It survives browser data clearing, recloning the repo and hard kills (WAL with `synchronous=FULL`), and the user can copy it as a backup.
* Good, because the browser-only machinery disappears: WASM loading, OPFS pool retries, leader election, request forwarding between tabs, the service worker and its cache strategy. Tabs are just HTTP clients of one process.
* Good, because `node:sqlite` is built into Node 26. It adds no native add-on and no install script, and it ships a current SQLite (3.54.0 on Node 26.13).
* Good, because the store's RPC surface (`StoreApi`), migrations and Drizzle schema are unchanged. Only the transport changed, from `postMessage` to `fetch`.
* Bad, because GymOs now needs Node and pnpm and a terminal to run. That is fine for developers and early users, and it is a barrier for everyone else (see the parked Tauri option).
* Bad, because a phone can't run it. Mobile has to come back later as a different runtime (a native shell or a revived PWA, both E14). The phone acceptance work in E0 (iOS go/no-go) no longer applies.
* Bad, because a local HTTP server is reachable by other software on the same machine. Binding to loopback and the Host and Origin checks stop remote and cross-site access, but not local malware, which could read the file directly anyway.
* Neutral, because the "deployed public demo" in handoff v1.5 (M1 DoD, never-cut list) has no host in this model. The owner has to decide what replaces it (§ More Information).

### Confirmation

* Unit tests in `packages/store` run the migration runner and queries on `node:sqlite`.
* `apps/server` tests cover every request rejection: a foreign `Host`, a missing or foreign `Origin`, a wrong content type, an oversized body, an unknown method and path traversal.
* A CI smoke test (`apps/server/scripts/smoke.mjs`) runs on Windows, macOS and Linux:
  1. start the built server on a random port with a temporary data dir;
  2. write a note;
  3. restart the server;
  4. assert that the data and the integrity check survived;
  5. assert that a foreign `Origin` gets a 403.
* The manual Windows checks L-1…L-11 in `docs/platform-support.md`.

## Pros and Cons of the Options

### Keep the browser PWA

* Good, because nothing needs installing: open a URL, and it runs on phones too.
* Good, because it was already built and working in Chromium on Windows.
* Bad, because `persisted()` was `false`. The browser may evict the data, "clear browsing data" deletes it, and on iOS WebKit's 7-day eviction policy applies to non-installed sites. This fails the main driver.
* Bad, because of its complexity: the sqlite-wasm patch for Turbopack, the OPFS SyncAccessHandle pool, leader election and forwarding across tabs, and service-worker update semantics.

### Local Node server with `node:sqlite` on disk (chosen)

* Good, because the data is durable and user-visible, as described above.
* Good, because it is clone-and-run with only Node 26 and pnpm, the same toolchain the repo already requires.
* Good, because the UI stays a static Next.js export, so a future PWA or native shell can reuse it.
* Bad, because it needs a terminal, and it doesn't run on phones.

### Tauri desktop app (parked)

* Good, because it is a real desktop app with a small binary, an OS webview and native file storage.
* Bad, because shipping it needs an installer and code signing (Windows SmartScreen, macOS notarization), plus a Rust toolchain for contributors.
* Neutral, because it can wrap the same static UI and talk to the same store later. It is the likely path if non-developers should use GymOs. Parked, not rejected.

### Electron desktop app

* Good, because it bundles Node, so `node:sqlite` and this server code would run inside it largely unchanged.
* Bad, because it is a large download, it brings its own installer and signing story like Tauri, and it adds a big dependency tree to audit. It has no advantage over the plain local server at this stage.

## More Information

* **This supersedes:**
  * handoff v1.5 D15 (static deploy on Vercel Hobby) for the app runtime;
  * D23 (SQLite-WASM and OPFS) and rule 13's browser-specific durability items (`navigator.storage.persist()`, the iOS install prompt, single-writer leader election);
  * R0.1 and the E0-T6 Vercel deploy;
  * the PWA offline shell from E0-T4.
* **This keeps** D18 (local-first: the device is the source of truth) and D22 (sync design for M2). The sync server can still be self-hosted, and this local server is a natural sync client.
* **Revisit when:**
  * GymOs should reach non-developers (Tauri, then this ADR's packaging section changes);
  * mobile work starts (E14);
  * `node:sqlite` or Drizzle gains a first-party driver that makes the proxy unnecessary.
* **Open owner questions** (not decided here): what replaces the "deployed public demo" (for example, a demo video or a seeded demo mode run locally), and whether the iOS go/no-go is dropped or moved to the mobile phase.
