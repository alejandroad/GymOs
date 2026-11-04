# GymOs: Design Decisions (Rust + Tauri)

**Status:** Accepted · **Date:** 2026-11-03 · **Owner:** Alejandro
**Supersedes:** the stack, platform and workflow parts of `docs/HANDOFF.md` v1.5 (D6, D11, D15, D23, rules 13 and 15, the E0 tickets) and ADR 0001 (local Node server with `node:sqlite`).
**Unchanged:** the product. The principles (§1.2), the core loop (§1.3) and the product spec (§3) in `docs/HANDOFF.md` still define what GymOs does. This document defines how it is built.

> Each section below becomes its own ADR in `docs/adr/`. This file is the single summary; the ADRs carry the alternatives and consequences in full.

---

## 1. Goals and constraints

- **Target:** a desktop application for **Windows and macOS** first. **Phone logging ships last**, after desktop is out and functional. It stays on the roadmap.
- **Local-first:** data lives on the device. No account, no network needed for any feature.
- **Portfolio and growth:** the project should expand the owner's skills beyond the TypeScript he writes at work, and be explainable in an interview.
- **Engineering principles:** single responsibility, dependency inversion, small reviewable commits, OWASP-informed security from the start.
- **Hard rule that follows from "phone last":** the core must not assume it runs on a desktop. No file paths, OS APIs or UI concepts inside the domain or engine.

## 2. Decision summary

| # | Area | Decision |
|---|---|---|
| DD-01 | Core language | **Rust** |
| DD-02 | App shell | **Tauri 2** (desktop now; its iOS/Android targets later) |
| DD-03 | Architecture | Layered Cargo workspace: domain → engine ← store; desktop shell; one UI layer |
| DD-04 | Storage | **rusqlite** (`bundled`), one connection owner, `rusqlite_migration`, forward-only |
| DD-05 | UI | **React + TypeScript + Vite** inside the Tauri web view; TS types generated from Rust |
| DD-06 | IPC | Thin Tauri commands; least-privilege capabilities; no HTTP |
| DD-07 | Errors | Typed `thiserror` errors per crate; one mapping to stable codes at the IPC boundary; no panics in libraries |
| DD-08 | Domain modelling | Newtypes and enums; "parse, don't validate" |
| DD-09 | Testing | Per-layer strategy; property and golden tests; e2e option (c) |
| DD-10 | Workflow | Trunk-based on `main`; strict small-commit rules |
| DD-11 | CI and security | GitHub Actions on Windows and macOS; `clippy`, `rustfmt`, `cargo-deny`; Tauri hardening |
| DD-12 | Repo reset | Tag the TS/Node spike, clear it in commits, keep history |

---

## 3. Decisions in detail

### DD-01 · Rust as the core language
**Why:**
- The domain is made of sum types (tracking modes, set types, verdicts). Rust enums with exhaustive `match` make the compiler check every rule against every case. Go has no sum types.
- Go's strengths (network services, concurrency) don't matter for a single-user desktop diary. Go remains a fine choice for a future sync server.
- Rust is the larger skill expansion, and its ownership and error model carries back into TypeScript work.

**Accepted cost:** about 3–6 weeks before full productivity. The January M1 date in the handoff will slip; dates are re-planned after the scaffold.

**Rejected:** Go + Wails, C# + Avalonia, Swift or Kotlin Compose (single-platform or less growth), Rust native GUIs (Iced, Slint, egui: charting too immature).

### DD-02 · Tauri 2 as the shell
- The web view is the only UI layer; everything else is Rust.
- The capability model gives an explicit allowlist for every command the UI can call, which is a concrete OWASP control.
- Tauri 2 has iOS and Android targets, which keeps "phone last" on a path that reuses the same core. **Fallback** if Tauri mobile is not good enough when we get there: expose the core to native Swift/Kotlin apps through UniFFI. The business logic is still written once.

### DD-03 · Layered architecture

```
 ui/ (React + TS)                     ← the only layer that renders anything
   │  Tauri IPC (generated TS types)
   ▼
 gymos-desktop  (Tauri shell)          ← thin commands: validate → call engine → map error
   │  constructs GymOs with a concrete store
   ▼
 gymos-engine   (the GymOs "engine")   ← GymOs struct: commands + queries; defines repository traits
   │                     ▲
   ▼                     │ implements the traits
 gymos-domain            gymos-store   ← SQLite (rusqlite); the "Impl" layer
 (pure business logic)
```

| Owner's term | Rust shape | Crate | May depend on | Must not know about |
|---|---|---|---|---|
| Helper classes (business logic) | Pure functions and types in modules | `gymos-domain` | `std`, `thiserror`, small pure crates | I/O, SQLite, Tauri, time sources, the OS |
| GymOs main / engine | `GymOs` struct with command and query methods; repository traits | `gymos-engine` | `gymos-domain` | SQLite, Tauri |
| Impl layer | Structs implementing the engine's traits | `gymos-store` | `gymos-engine`, `gymos-domain`, `rusqlite` | Tauri, UI |
| Endpoints | `#[tauri::command]` functions | `gymos-desktop` | all crates above | business rules |
| UI | React components | `ui/` | generated types, IPC client | the database, business rules |

**Rules:**
- **Dependencies point inward.** The engine defines the traits; the store implements them (dependency inversion). The engine never knows SQLite exists, so tests use an in-memory fake and the phone build can swap the store.
- **The engine is called by endpoints, never the reverse.**
- **Endpoints contain no logic.** One engine call each.
- **Progression and PR rules live in `gymos-domain`** as pure functions, so they're testable without any infrastructure.
- **Time and IDs are injected.** The engine takes a `Clock` and an ID generator through traits, so rules stay deterministic in tests.
- Rust has no classes. Single responsibility is expressed through small modules, focused structs and narrow traits.

### DD-04 · Storage: rusqlite
- **`bundled` feature:** ships its own SQLite, so Windows and macOS behave identically.
- **One connection owner:** `Connection` is not `Sync`, so the store owns it behind a `Mutex` or on a dedicated DB thread. Hidden inside `gymos-store`.
- **Migrations:** `rusqlite_migration` (confirm the version at scaffold time). Forward-only. Applied at startup, before the UI can issue commands.
- **Safety:** copy the database file to a timestamped backup before any migration runs. A newer or unknown schema is refused with a readable error and left untouched.
- **Upgrade tests:** a committed fixture database for every released schema version, migrated to the latest in CI.
- **Location:** the OS app-data directory, resolved by the shell and passed into the store (not hard-coded in the core).
- Carried over from the handoff: UUIDv7 IDs generated in the app, loads stored as entered value + entered unit + normalized kg, UTC plus original local offset, provenance columns, and derived data that is recomputable and never exported.

### DD-05 · UI: React + TypeScript + Vite
- **One hard new thing at a time.** Rust takes the learning budget; the UI uses known tools.
- **Charts are the showcase surface:** TradingView Lightweight Charts, ECharts only where needed.
- **Rust is the source of truth for types.** TS types are generated from Rust structs and enums (specta or ts-rs; pick at scaffold time). The UI can't drift from the domain model.
- **Revisit:** Leptos (an all-Rust UI) only for a possible v2 UI.

### DD-06 · IPC boundary
- Communication is Tauri IPC (`invoke`). There is no HTTP server and no open port.
- Each command: deserialize → construct domain newtypes (validation) → one engine call → map the result or error.
- **Least privilege:** Tauri capabilities allowlist only the commands the UI needs. A strict Content Security Policy in the Tauri config. No remote content in the web view.

### DD-07 · Error handling

| Layer | Error type | Rule |
|---|---|---|
| `gymos-domain` | Small `thiserror` enums per area (e.g. `SetError::RepsOutOfRange`) | No I/O errors can exist here |
| `gymos-engine` | `EngineError`, wrapping domain errors and an engine-defined `RepoError` | Never mentions rusqlite |
| `gymos-store` | Converts `rusqlite::Error` into `RepoError` | SQLite details stay in the Impl layer |
| `gymos-desktop` | Maps `EngineError` once into a serializable `IpcError { code, message }` | Stable codes; no internals reach the UI |
| `ui/` | Switches on `code` (generated type) | Never parses message strings |

- `anyhow` only in the binary and in tests, never in library crates.
- No panics in library crates: `clippy::unwrap_used` and `clippy::expect_used` are denied there.
- Full error detail is logged locally; only stable codes cross to the UI (OWASP: no information leakage).
- **Rejected:** one global error enum (couples every crate), `anyhow` everywhere, `Box<dyn Error>`.

### DD-08 · Domain modelling: parse, don't validate
- Every domain value is a newtype with a fallible constructor: `Reps`, `LoadKg`, `Rpe`, `DistanceM`, `DurationS` and so on, built through `TryFrom`.
- Invalid states are unrepresentable, so the engine never re-checks them.
- Variants (tracking modes, set types, verdicts) are enums. A set is an enum over tracking modes, not a struct full of optional fields.
- Consequence: every value entering the system goes through one constructor. That is the input-validation boundary.

### DD-09 · Testing

| Layer | Test type | Tools | Proves |
|---|---|---|---|
| `gymos-domain` | Unit, property, golden | `#[test]`, `proptest`, `insta` | Rules are correct (unit round-trips exact; full recompute = incremental; no orphan PRs after delete) |
| `gymos-engine` | Unit with in-memory fake repo | Hand-written fake implementing the traits | Orchestration is correct without a database |
| `gymos-store` | Integration on real SQLite | `:memory:` and temp files | SQL, mappers and migrations are correct; fixture DB upgrades |
| `gymos-desktop` | Handler tests + contract check | Plain tests; regenerate TS types and fail on diff | IPC mapping is right; UI types can't drift |
| `ui/` | Component | Vitest + React Testing Library, IPC mocked | UI states and errors render correctly |
| Whole app | **Option (c)** | Playwright on the UI with mocked IPC (every PR, all OSes) **and** Tauri WebDriver e2e (Windows CI) + a manual macOS checklist | It actually launches and works |

- **The rule spec is the test oracle.** Worked examples in the progression and PR rule spec become golden files before any rule code exists.
- **Runner:** `cargo-nextest`.
- **Coverage:** `cargo-llvm-cov`. Target 100% line/region coverage on rule modules. Branch coverage needs a nightly toolchain, so property tests cover branch-level confidence.

### DD-10 · Workflow
**Branching: trunk-based.**
- `main` is the only long-lived branch. `dev` is retired.
- One short-lived branch per ticket → PR into `main` → **rebase-merge** so every commit survives.
- Milestones are tags (`m1`, `m2`). Branch protection: PR required, linear history, CI required.

**Commit rules:**
1. One logical change per commit. If the message needs "and", split it.
2. Target about 200 changed lines or fewer, excluding lockfiles and generated code.
3. One crate per commit where possible; the scope names it: `feat(domain): add Reps newtype`.
4. Kinds never mix: dependency additions, generated code, formatting, refactors and behaviour changes are separate commits.
5. Every commit builds and passes tests. Check with `git rebase -x "cargo test"` before pushing.
6. The body explains why and links the ADR or ticket.
7. A PR is one ticket, about 3–8 commits.
8. CI enforces the Conventional Commit format; size is a review habit.

### DD-11 · CI and security baseline
**Pipeline (GitHub Actions; required to merge):**

| Job | Runs on | Checks |
|---|---|---|
| Format and lint | Linux | `cargo fmt --check`; `clippy -D warnings` (+ `unwrap_used`, `expect_used` denied in libraries); ESLint; `tsc` |
| Rust tests | Windows, macOS | `cargo nextest`; coverage via `cargo-llvm-cov` |
| Type drift | Linux | Regenerate TS types; fail on diff |
| UI tests | Linux | Vitest; Playwright against mocked IPC |
| App build | Windows, macOS | Full Tauri build |
| End-to-end | Windows | Tauri WebDriver smoke test |
| Commit format | Linux | Conventional Commits check |

**Security:**
- `cargo-deny`: vulnerable crates (RustSec advisories), licenses not compatible with MIT, unknown sources.
- `pnpm audit` for the UI; Dependabot for both ecosystems.
- GitHub secret scanning with push protection; CodeQL for TypeScript.
- Tauri: strict CSP and least-privilege capabilities.
- Pinned toolchains: `rust-toolchain.toml`, a pinned Node version; both lockfiles committed.
- An OWASP ASVS verification pass once real features exist.

### DD-12 · Repository reset
- Tag the current TypeScript/Node spike as `archive/ts-node-spike` and push the tag.
- Remove the old code in small commits on `main`, **keeping history**. A visible, documented pivot is a portfolio signal.
- ADR 0001 stays, marked **Superseded**. `docs/HANDOFF.md` keeps the product definition and gets a note pointing here for the stack.
- Retire `dev`.

---

## 4. Target repository layout

```
GymOs/
├─ Cargo.toml                 # workspace
├─ rust-toolchain.toml
├─ deny.toml                  # cargo-deny policy
├─ crates/
│  ├─ gymos-domain/           # pure business logic
│  ├─ gymos-engine/           # GymOs struct, commands/queries, repository traits
│  └─ gymos-store/            # rusqlite implementation + migrations
├─ apps/
│  └─ desktop/
│     ├─ src-tauri/           # gymos-desktop crate (Tauri shell, IPC commands)
│     ├─ src/                 # React + TS UI
│     ├─ package.json
│     └─ vite.config.ts
├─ docs/
│  ├─ design.md               # this file
│  ├─ HANDOFF.md              # product definition (stack sections superseded)
│  ├─ workflow.md
│  ├─ adr/
│  └─ research/
├─ .github/workflows/
├─ LICENSE  README.md  .editorconfig  .gitattributes  .gitignore
```

Small deviations are fine if the official Tauri scaffold prefers them.

## 5. Deferred decisions

| Item | When |
|---|---|
| Code signing and macOS notarization | Distribution decision |
| Distribution and installers | Before the first public release |
| Multi-device sync (topology, protocol) | After desktop M1 |
| Phone app (Tauri mobile vs UniFFI + native) | Final milestone |
| Leptos UI | Possible v2 only |
| OWASP ASVS pass | Once real features exist |

## 6. Open questions (decide at scaffold or schema time)

1. **Type generation:** specta vs ts-rs.
2. **Sync-ready columns in schema v1:** ship `hlc_updated`, per-field clocks, a `deleted` tombstone and `schema_version` now, or add them with sync. Leaning: ship now; a migration of every table later costs more.
3. **UI libraries:** component kit (shadcn/ui?) and data fetching over IPC (TanStack Query?).
4. **Command model:** keep the handoff's command + outbox write model in the engine from day one, or add the outbox with sync.
5. **Timeline:** re-plan M1 after the scaffold, based on actual Rust velocity.
