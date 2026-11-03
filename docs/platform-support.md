# Platform support and acceptance criteria

**Status:** active from E0 (2026-11-02). **Owner decision:** Windows and macOS come first. Mobile (iOS and Android) is the final integration step. It is not the E0 gate.

This page says where GymOs must run, and what "runs" means. A ticket that touches the toolchain, the store, the service worker or the build passes only when its criteria pass on every platform marked **required**.

## 1. Platforms

### 1.1 Development toolchain

| Platform | Status | How it is checked |
|---|---|---|
| Windows 10/11, x64 | **Required** | CI (`windows-latest`) and the owner's machine |
| macOS 14+, Apple Silicon | **Required** | CI (`macos-latest`) and a manual pass on a Mac |
| Linux, x64 | Required (CI only) | CI (`ubuntu-latest`) |

Toolchain: Node from `.nvmrc` (26.x, Active LTS) and pnpm from `packageManager` (12.x).

### 1.2 Runtime: browsers running the static app

| OS | Browser | Status | Minimum |
|---|---|---|---|
| Windows | Chrome | **Required** | current stable and the one before it |
| Windows | Edge | **Required** | current stable and the one before it |
| Windows | Firefox | **Required** | current stable and ESR |
| macOS | Safari | **Required** | 17+ (`opfs-sahpool` needs 16.4+) |
| macOS | Chrome | **Required** | current stable |
| macOS | Firefox | **Required** | current stable |
| iOS / iPadOS | Safari tab and Home Screen PWA | Deferred: final integration | 17+ |
| Android | Chrome tab and installed PWA | Deferred: final integration | current stable |

Out of scope: Internet Explorer, legacy Edge, and any browser without OPFS sync access handles in workers.

## 2. Toolchain criteria (automated)

These run in CI on all three OSes for every PR. A PR is mergeable only when all three are green.

| ID | Check |
|---|---|
| T-1 | `pnpm install --frozen-lockfile` succeeds |
| T-2 | `pnpm lint` passes |
| T-3 | `pnpm typecheck` passes |
| T-4 | `pnpm test` passes |
| T-5 | `pnpm build` emits `apps/web/out/` |

Rules that keep this true:

- `package.json` scripts are cross-platform. No `rm -rf`, `cp`, `&&`-chained shell builtins, inline `VAR=x cmd` env prefixes or bash-only syntax. If a script needs logic, it goes in a Node script (`.mjs`).
- Paths in code and config use `node:path` or `URL`, never hard-coded `\` or `/` separators.
- Line endings are LF everywhere (`.gitattributes`). Tests never depend on `\r\n`.
- File names are case-correct. macOS and Windows filesystems are case-insensitive by default, Linux isn't.
- No native Node add-ons that need a compiler at install time (`allowBuilds` in `pnpm-workspace.yaml` stays deny-by-default).

## 3. Runtime criteria (manual, per browser)

Run these against `pnpm build && pnpm preview` (http://localhost:3000). `localhost` counts as a secure context, so OPFS and service workers work without HTTPS. Never run them against `next dev`.

| ID | Check | Pass when |
|---|---|---|
| R-1 | Open `/spike` | The worker starts, the VFS shows `opfs-sahpool`, and no console errors appear |
| R-2 | Add 3 notes | They list newest first, and the count is 3 |
| R-3 | Reload | The 3 notes are still there |
| R-4 | Quit the browser completely (every window), then reopen `/spike` | The notes are still there |
| R-5 | Open `/spike` in a second tab | One tab shows **leader** and the other **follower** |
| R-6 | Add notes from both tabs, alternating | Both tabs show the same rows and count after a refresh; no errors |
| R-7 | Close the leader tab, then add a note in the remaining tab | It takes over as leader within about 1 s, and the write succeeds |
| R-8 | After one online visit, set DevTools offline (or disconnect the network) and reload `/` and `/spike` | Both load, and `/spike` reads and writes |
| R-9 | Install the app (Chrome/Edge: install icon; Safari: File → Add to Dock). Firefox desktop has no install, so it's N/A there | The installed app opens, and it passes R-2, R-3 and R-8 |
| R-10 | Read the storage panel on `/spike` | `persisted()` and the estimate are recorded in the matrix (informational, not a gate) |
| R-11 | Open `/spike` in a private window | Either it works, or it shows a clear "storage unavailable" message. Never a blank or broken page |

Record each result in the browser matrix in `docs/research/stack-spike.md` with the browser version, OS version and date: ✅ pass · ❌ fail (with a note) · N/A.

## 4. Measurements recorded per platform (informational)

These are recorded, not gated, until E1 sets budgets:

- worker cold init (ms) and warm init with a cached service worker;
- median insert and select latency over 50 operations;
- build time;
- the `persisted()` result and storage estimate.

## 5. Mobile: final integration (deferred)

The iPhone and Android checklists from Prompt 0 v2 (E0-T6 §3) still apply, but run as a separate final-integration pass after the desktop matrix is green and a deploy exists (OPFS needs HTTPS on a phone). The main open risk, iOS storage eviction, is tracked in the spike report until then.
