# Platform support and acceptance criteria

**Status:** active from E0-T6 (2026-11-02). **Owner decisions:**

- GymOs runs as a **local Node server** that owns a SQLite file on disk ([ADR 0001](adr/0001-local-node-server-with-on-disk-sqlite.md)).
- **Windows first, then macOS.** macOS-specific work waits until the owner calls Windows done.
- **Mobile is last.**

This page says where GymOs must run, and what "runs" means. A ticket that touches the toolchain, the store, the server or the build passes only when its criteria pass on every platform marked **required**.

## 1. Platforms

The runtime is **Node on the OS**: `apps/server` opens the database and serves the UI. Browsers are only the UI. They talk to the server at `http://127.0.0.1:<port>` and store nothing themselves.

### 1.1 Toolchain and runtime

| Platform | Status | How it is checked |
|---|---|---|
| Windows 10/11, x64 | **Required now** | CI (`windows-latest`) and the manual list in §3 on the owner's machine |
| macOS 14+, Apple Silicon | Required next | CI (`macos-latest`) now; the manual list in §3 once Windows is signed off |
| Linux, x64 | Required (CI only) | CI (`ubuntu-latest`) |

To run it, a user needs only Node from `.nvmrc` (26.x, Active LTS) and pnpm from `packageManager` (12.x). No compiler, Rust, admin rights or installer. `node:sqlite` is built into Node.

### 1.2 Browsers (UI only)

| OS | Browser | Status |
|---|---|---|
| Windows | Edge, Chrome | **Required**: current stable |
| Windows | Firefox | Required when installed: current stable |
| macOS | Safari, Chrome, Firefox | Required with macOS |
| iOS, Android | — | Deferred: a phone can't run the server (ADR 0001) |

Because browsers hold no data, browser storage policies (eviction, `persisted()`, private windows) no longer affect data safety.

## 2. Toolchain criteria (automated)

These run in CI on all three OSes for every PR. A PR is mergeable only when all three are green.

| ID | Check |
|---|---|
| T-1 | `pnpm install --frozen-lockfile` succeeds |
| T-2 | `pnpm lint` passes |
| T-3 | `pnpm typecheck` passes |
| T-4 | `pnpm test` passes, including the server's request-rejection tests and the store's on-disk tests |
| T-5 | `pnpm build` emits `apps/web/out/` and `apps/server/dist/server.mjs` |
| T-6 | `node apps/server/scripts/smoke.mjs` passes. It starts the built server on a free port with a temporary data dir, writes a note, kills the server hard and restarts it, then checks that the note and `integrity_check` survived, that a second server refuses the database, and that a foreign `Origin` or `Host` gets 403 |

Rules that keep this true:

- `package.json` scripts are cross-platform. No `rm -rf`, `cp`, `&&`-chained shell builtins, inline `VAR=x cmd` env prefixes or bash-only syntax. If a script needs logic, it goes in a Node script (`.mjs`).
- Root scripts run Turbo through `scripts/turbo.mjs`, which sets the Turbo and Next.js telemetry opt-outs.
- Paths in code and config use `node:path` or `URL`, never hard-coded `\` or `/` separators.
- Line endings are LF everywhere (`.gitattributes`). Tests never depend on `\r\n`.
- File names are case-correct. macOS and Windows filesystems are case-insensitive by default, Linux isn't.
- No native Node add-ons that need a compiler at install time (`allowBuilds` in `pnpm-workspace.yaml` stays deny-by-default).

## 3. Local-server criteria (manual, per OS)

Run these from a clean clone with `pnpm install` and then `pnpm start`. The server listens on `http://127.0.0.1:4321` (`GYMOS_PORT` overrides it).

The default data folder:

| OS | Folder |
|---|---|
| Windows | `%APPDATA%\GymOs` |
| macOS | `~/Library/Application Support/GymOs` |
| Linux | `$XDG_DATA_HOME/gymos` (falls back to `~/.local/share/gymos`) |

`GYMOS_DATA_DIR` overrides the folder.

| ID | Check | Pass when |
|---|---|---|
| L-1 | Fresh clone, then `pnpm install` and `pnpm start`, with only Node 26 and pnpm installed | It builds and starts with no Rust, compiler or admin rights |
| L-2 | Read the startup output | It prints the URL and the full database path. The file exists in the data folder, not in the repo |
| L-3 | Add notes on `/spike`, stop the server with Ctrl+C, then start it again | The notes are still there |
| L-4 | Hard-kill the server mid-burst of writes (Windows: `taskkill /F`; macOS: `kill -9`), then restart | `integrity_check` is `ok`, and every write that was acknowledged is present |
| L-5 | Write from two browser tabs concurrently | No errors, and both tabs show the same count after a refresh |
| L-6 | Run a second `pnpm start` (or point `GYMOS_DATA_DIR` at the same folder on another port) | It refuses to open the database with a clear message |
| L-7 | Check the listening sockets (Windows: `netstat -ano`; macOS: `lsof -iTCP -sTCP:LISTEN`) | The server listens on `127.0.0.1` only, never `0.0.0.0` or `::` |
| L-8 | Send a cross-origin `POST` (`curl` with a foreign `Origin` or `Host`, or a page on another port) | It gets 403, and nothing is written |
| L-9 | Delete the repo folder, clone it again, then `pnpm install` and `pnpm start` | The data is still there |
| L-10 | Open the UI in Edge and Chrome (and Firefox if installed) | It works, and no service worker is registered |
| L-11 | Start against a database from a newer app version, or one with a diverged migration history | Startup fails with a readable error, and the file is unchanged |

Record each run in the PR or the spike report with the OS version, Node version, browser versions and date: ✅ pass · ❌ fail (with a note) · N/A.

## 4. Measurements recorded per platform (informational)

These are recorded, not gated, until E1 sets budgets:

- server startup (module start to listening) and database open-and-migrate time;
- median insert and select latency over 50 operations from the `/spike` page, HTTP round trip included;
- `pnpm start` time: first (cold) build, and a cached run;
- build time.

## 5. Mobile (deferred)

A phone can't run the local server. Mobile returns as a separate runtime (a native shell or a PWA that syncs with this server, E14), with its own criteria.
