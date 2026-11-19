# GymOs: Design Decisions (Rust + Tauri)

**Status:** Accepted · **Date:** 2026-11-03 · **Updated:** 2026-11-16 (DD-13 implemented; DD-15 added and implemented; DD-03 port shapes; open questions 2 and 4 resolved) · **Owner:** Alejandro
**Supersedes:** the stack, platform and workflow parts of `docs/HANDOFF.md` v1.5 (D6, D11, D15, D23, rules 13 and 15, the E0 tickets) and ADR 0001 (local Node server with `node:sqlite`).
**Unchanged:** the product. The principles (§1.2), the core loop (§1.3) and the product spec (§3), now in [`docs/product.md`](product.md), still define what GymOs does. This document defines how it is built.

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
| DD-13 | Overload verdicts | Verdict + signal list (bool derived); exact integer e1RM; `LoadStep` for double progression; deloads never a baseline |
| DD-14 | Profile and records | Stats page is a view; measurements are time series; custom metrics; one `Record` type: open subjects × closed kinds |
| DD-15 | Engine writes and baselines | Every write is an idempotent `Command`; "last time" is template-first, then same exercise; verdicts are computed on read |

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

**Port shapes** (amended 2026-11-16, engine-ports PR; from `docs/research/kernel-architecture.md` §5.1–5.3):
- **One `Ports` bundle, generics over trait objects.** `trait Ports { type Store: Store; type Clock: Clock; type Ids: IdGen; }`, so the engine is `GymOs<P: Ports>` with one type parameter and static dispatch. Only the shell (and tests, through `gymos-testkit`'s `TestPorts`) names concrete types.
- **The store is a closure-scoped unit of work.** `Store::write(|tx| …)` commits when the closure returns `Ok` and rolls back on `Err`; `Store::read(|tx| …)` gets a read-only view. The engine decides *what* belongs in one transaction, the store decides *how* it is atomic. Every command makes exactly one `write` call.
- **Writes work on whole aggregates.** `WriteTx` saves a whole `Exercise` or `Session` and records applied `CommandId`s. There are no per-field setters, so business sequencing never leaks into an adapter. Choosing among stored rows ("last time", DD-15) is the engine's job; the store only lists candidates.
- **Sync kernel, async shell.** Every port and engine method is synchronous and takes `&self`; interior mutability lives inside the store. The shell runs engine calls off the UI thread (`spawn_blocking`).
- **Clock and IDs.** `Clock::now() -> Timestamp`; `IdGen::next_id() -> Uuid`, wrapped by the engine in the right ID type. The domain re-exports `Uuid`, so the engine needs no direct `uuid` dependency.

### DD-04 · Storage: rusqlite
- **`bundled` feature:** ships its own SQLite, so Windows and macOS behave identically.
- **One connection owner:** `Connection` is not `Sync`, so the store owns it behind a `Mutex` or on a dedicated DB thread. Hidden inside `gymos-store`.
- **Migrations:** `rusqlite_migration` (confirm the version at scaffold time). Forward-only. Applied at startup, before the UI can issue commands.
- **Safety:** copy the database file to a timestamped backup before any migration runs. A newer or unknown schema is refused with a readable error and left untouched.
- **Upgrade tests:** a committed fixture database for every released schema version, migrated to the latest in CI.
- **Sync-ready from schema v1** (decided 2026-11-16; resolves open question 2): every table that will sync gets `hlc_updated`, per-field clocks, a `deleted` tombstone and `schema_version` in the first migration. Adding them to every table later would cost a migration of the whole database. Entities that sync need stable IDs, so session entries get a `SessionExerciseId` (DD-15).
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
- ADR 0001 was marked **Superseded**, then removed along with `docs/HANDOFF.md` in `368e165`. Both remain in git history. The product definition (handoff §1 and §3) now lives in [`docs/product.md`](product.md).
- Retire `dev`.

### DD-13 · Progressive overload verdicts
**Refines** `product.md` §3.3. The product rules stay the same; this decision fixes how they are computed and closes three gaps found while researching them.

**Status:** implemented for `WeightReps` in `gymos_domain::progression` (PR #11, 2026-11-16). Baseline selection is DD-15, in the engine.

**Shape: a verdict plus a list of signals, not a boolean.**
- Each exercise in a session gets an `ExerciseVerdict`: one `Verdict` (`Progressed | Matched | Regressed | InsufficientData | Deload`), **every** `Signal` that fired (positive and negative, strongest first) and the `algorithm_version`.
- Each signal carries its before and after values, so the UI renders *"+1 rep at 100 kg on set 2 (8→9)"* without recomputing anything.
- The yes/no answer is derived: `ExerciseVerdict::is_progressive_overload()` is true exactly when the verdict is `Progressed`. A stored boolean alone would lose first sessions, deloads, matched sessions and mixed results.

**Verdict rules, evaluated in order:**
1. A tracking mode with no rules yet → `VerdictError::UnsupportedMode` (only `WeightReps` has rules so far).
2. The session is marked as a deload → `Deload` (signals are still listed).
3. No baseline, or no counted (non-warm-up) sets on either side → `InsufficientData`.
4. Any positive signal (a `LoadStep` is always positive) → `Progressed` ("any one counts", §3.3).
5. Otherwise, any negative signal → `Regressed`.
6. Otherwise → `Matched`.

Comparing a baseline from a different exercise or tracking mode is a caller bug, not a verdict: `VerdictError::ExerciseMismatch` / `ModeMismatch`.

**Signals** (`WeightReps`; other tracking modes will follow the same pattern with fewer signals), in output order, strongest first. Within one kind, signals are ordered by set:

| Signal | Fires when |
|---|---|
| `E1rm` | the best counted set's estimated 1RM changed (Epley, RPE-adjusted via RIR = 10 − RPE) |
| `LoadStep` | a scheme is set, the top pairable load increased, **and** the fewest reps at that new load are still at or above the scheme's minimum (gap 1) |
| `LoadAtReps` | a paired set used more or less load at the same reps |
| `RepsAtLoad` | a paired set did more or fewer reps at the same load |
| `WorkingSets` | the pairable set count changed, every pair is identical (at least one pair), **and** every extra or missing set repeats the other side's load × reps |
| `VolumeLoad` | Σ load × reps over counted sets changed. `Down` is always listed. `Up` only when the e1RM didn't drop, and not when only one side has an e1RM (a drop can't be ruled out) |
| `Effort` | the same load and reps on a pair at a different effort; RPE and RIR compare on one scale (RIR n = RPE 10 − n), easier is `Up` |

Rules that can't be evaluated are listed as `Skipped` (today only `E1rmAboveRepCap`), so the explanation stays honest.

**Gaps closed:**
- **Gap 1: double progression must read as progress.** Under double progression (the default scheme, §3.2), reaching the top of the rep range means adding load, and reps fall back to the bottom: 100 kg × 8 → 102.5 kg × 6. Every §3.3 signal reads that as a drop (e1RM −3%, volume down). The `LoadStep` signal makes the verdict scheme-aware. `ProgressionScheme` (a checked rep range and a non-zero increment) is modelled in the domain. Schemes are stored per exercise (DD-15), and without one `LoadStep` is not evaluated. Any load increase counts: the increment is the engine's suggested step, not a threshold.
- **Gap 2: deloads are never a baseline.** "Last time" selection skips sessions marked as a deload. Otherwise the session after a deload "progresses" on every signal. The full selection rule is in DD-15.
- **Gap 3: set pairing is defined.** `TopSet`, `Working` and `Failure` sets are paired by order. `Drop` and `Backoff` sets are excluded from pairing but count toward volume. Warm-ups are excluded everywhere (`SetType::counts_by_default`).
  - A `TopSet` is the heaviest planned set, so a top set pairs with the previous session's top set first. A session with no top set falls back to plain order.
  - This keeps "top set 100 → 102.5 kg" comparing like with like, even when the number of working sets before it changes.

**Arithmetic and edge rules:**
- **Formula: Epley**, `e1RM = w × (1 + r/30)`, for algorithm version 1. Why Epley over Brzycki (`w × 36 / (37 − r)`):
  - It's linear in reps and defined for every rep count. Brzycki has a pole at 37 reps.
  - The two agree exactly at 10 reps and differ by at most about 4% between 2 and 12 reps (Epley is higher below 10, lower above). A verdict compares the same formula on both sides, so the choice rarely flips one.
  - It's the formula lifters most often recognise, which helps the "every number explains itself" principle.

  Switching formulas later is a new `ALGORITHM_VERSION`, recomputed locally (§3.5).
- **A single rep is the 1RM.** Epley gives 1.033 × w at one rep, so a true single would beat itself. For `r = 1`, e1RM is `w`.
- **Exact comparisons.** Epley is a ratio, so "did e1RM go up" is decided by cross-multiplying integer `Mass` values in `i128`: `w₁·k(r₁)` vs `w₂·k(r₂)`, where `k(r) = 30 + r`, and `k(1) = 30` for the single-rep rule. RPE adjustment uses half-reps to stay integral (`2·r_eff = 2r + 20 − rpe_half_points`). Floats are used only to display values such as "124.0 → 126.7 kg".
- **e1RM above 12 reps is skipped.** The formulas lose accuracy past about 10–12 reps. The skip is recorded so the explanation stays honest, and reps and volume still apply.
- **Cross-unit "same load".** 100 kg and 220.46 lb differ by about 1 g. With exact comparison, a unit switch would read as a false `E1rm` drop and a `Regressed` verdict. A blanket tolerance would also be wrong: micro-plates (+0.25 kg on 100 kg = 0.25%) are real progress.
  - Loads entered in the **same unit** compare exactly.
  - Loads entered in **different units** count as equal within 0.5%, checked in integers: `|a − b| × 200 ≤ max(a, b)`.
  - The same rule applies to e1RM and volume deltas when the two sides' best sets (for e1RM) or any counted sets (for volume) were entered in different units.
  - The threshold is versioned with `ALGORITHM_VERSION`, and it lives in one place (`progression/compare.rs`).
  - **Known effects, accepted as spec-literal.** The tolerance isn't transitive. A set logged in the other unit at the *old* load can count as "at" the new top load and suppress a `LoadStep`. When a session mixes units, the best set's unit alone decides whether the e1RM comparison is tolerant, so it can hide a same-unit micro-plate gain. Both only happen with mixed units inside one session. Revisit them with a new `ALGORITHM_VERSION` if they show up in real logs.
- **Only the e1RM is skipped above the cap.** If one side has counted sets but none within the cap, the e1RM isn't compared and `E1rmAboveRepCap` is recorded. A side with no counted sets at all is `InsufficientData` instead.

**Placement** (follows DD-03):
- Pure rules in `gymos-domain/src/progression/`, behind the public `progression` module:
  - `scheme.rs`: `ProgressionScheme`, `RepRange`
  - `e1rm.rs`: exact Epley with effort adjustment
  - `compare.rs`: the cross-unit tolerance
  - `pairing.rs`: set pairing (it reads set types only, so other modes can reuse it)
  - `signals.rs`: signals and detection
  - `verdict.rs`: `exercise_verdict`, `SessionVerdict`

  `ALGORITHM_VERSION` stays in `lib.rs`.
- Baseline selection needs history, so it is an engine query (`gymos-engine/src/queries/last_time.rs`, DD-15).

**Testing:**
- **Golden:** examples A–M in `gymos-domain/tests/golden.rs`, written by hand from the spec. Each asserts the verdict, every signal in order, the skipped rules and the algorithm version. Extra cases cover warm-ups, drop sets, extra sets at another load, `is_progressive_overload` and `SessionVerdict` counts.
- **Properties:** `gymos-domain/tests/properties.rs` checks that the e1RM comparison is antisymmetric and transitive, agrees with the displayed kilograms, and never drops when one rep is added at the same load.

### DD-14 · Profile, measurements, activities and records
**Implements** `product.md` §3.5–3.6 in `gymos-domain`.

**The stats and medals page is a view, not a struct.** It is an engine query that combines three sources:

| Source | What it holds | Stored? |
|---|---|---|
| `Profile` | display name, `UnitPreferences`, optional `BirthYear` and `ReferencePopulation` | yes, one per device (D7) |
| `Measurement` | time series: body weight, VO2 max, resting HR, custom metrics | yes |
| `Record` | every PR across exercises, activities and measurements | derived, recomputable (§3.5) |

The profile holds **no stats**:
- "Current body weight" is the latest body-weight measurement, so it can't go stale. The history it keeps drives full-range charts (§1.2.2).
- Bodyweight exercises resolve body weight *on the session date* with `body_weight_at` (§3.6).

**Measurements.** `MeasurementValue` is an enum with one variant per kind (`BodyWeight`, `Vo2Max`, `RestingHeartRate`, `Custom`), the same pattern as `SetMetrics` (DD-08). Each kind knows which direction is better:
- VO2 max: higher.
- Resting HR: lower.
- Body weight: **neither**. Up or down depends on the person's goal, so body weight gets a chart and no "best" record.

**Custom metrics are the extension point.** A `CustomMetric { name, unit, better, decimals }` lets the athlete track anything (grip strength, vertical jump, waist size). Values are exact thousandths (`CustomValue`). Each metric automatically gets a chart and a best-value record.

**Activities.** `Activity` is one struct with optional `ActivityDetails`, because activity kinds share almost every field. Fields stay private because rules tie them together:
- distance only where the kind tracks one
- moving time ≤ elapsed time
- max HR ≥ average HR

`ElapsedTime` allows up to 7 days for multi-day events, without loosening the 24 h cap on set durations.

- **22 activity kinds,** grouped into endurance, gym/classes and sports. Each kind declares whether it tracks distance.
- **Indoor vs outdoor is a detail, not a kind.** `Environment::{Indoor, Outdoor}` is an optional field. A treadmill run is a `Run` that took place `Indoor`, which avoids doubling the enum.
- **Strength training never duplicates sets.** `ActivityKind::StrengthTraining` holds a summary (often a watch's HR and energy). It can link to the logged `Session` through `linked_session`, and only that kind may link.
- **Activities carry no overload verdict.** Verdicts are derived data computed per strength exercise (DD-13). A linked strength activity's verdicts are its session's verdicts.

**Records: one generic type, a closed set of kinds.** `Record { subject, kind, value, better, achieved_at, source, previous, algorithm_version }`.
- **Subject is open:** any exercise, any activity kind, any measurement series (custom metrics included), or `Overall`. A new exercise or metric gets records with no code change.
- **`RecordKind` is closed:** each kind is a rule with its own detector and explanation, and the compiler checks every rule handles every kind. A free-form `String` → `f64` map was rejected: it would lose exactness (D10), compile-time checking, and the "every number explains itself" output (§1.2.7).
- **`Quantity`** is a typed value (`Mass`, `Reps`, `Elapsed`, `Distance`, `Count`, …). Comparison only ranks like with like (`compare` returns `None` across variants), and a tie never counts as a new record.

**Provenance on every user record.** Sessions, measurements and activities carry `Source::Manual | Import { format, importer_version }` (data-model rule 4). A bad importer version's rows can then be found and re-imported.

**Privacy.** Birth year and reference population are optional and local, and only select a labelled percentile table later (D8). A year, not a date, keeps less data.

**Scope of this decision:** types only. The record detectors arrive with the PR engine, and the stats view with the engine's `profile_stats` query.

### DD-15 · Engine writes, baselines and derived verdicts
**Status:** accepted 2026-11-16; implemented in the engine-ports PR ([`docs/prompts/PROMPT-2-engine-ports.md`](prompts/PROMPT-2-engine-ports.md), which also gives the rejected options) over the in-memory fake. That PR also amended DD-03 with the port shapes.

**Every write is a `Command`** (resolves open question 4).
- One `Command` enum carries a `CommandId` generated by the **caller** (the UI, an importer or a test), so a retried call reuses the same ID.
- Each command runs inside one `store.write` transaction. It first records its `CommandId`, and a command that was already applied returns `AlreadyApplied` without changing anything.
- Importers and sync get one write path. The sync outbox table is deferred: adding it later is one extra call inside the same transaction.

**"Last time" for verdicts** refines `product.md` §3.2 and DD-13 gap 2:
1. If the current session came from a template: the most recent **finished, non-deload** session from that template that contains the exercise.
2. Otherwise, or if there is none: the most recent finished, non-deload session containing the exercise.
3. Unfinished sessions are never a baseline. Only sessions before the current one count, so a past session's baseline doesn't move when later sessions are logged. "Before" and "most recent" order by `started_at` (UTC), with ties broken by `SessionId` (the larger, later-generated ID wins).
4. If an exercise appears more than once in a session, entries match by occurrence (1st with 1st). An extra current entry uses the baseline's last entry.
5. A session only counts as containing the exercise if that matched entry has at least one counted (non-warm-up) set (decided 2026-11-16). An empty or warm-up-only entry can't be compared, so the search moves on to the next older session instead of hiding it.

Jaccard similarity for sessions without a template, and a baseline the user picks, are deferred (§5).

**Session entries have identity.** `SessionExercise` gets a `SessionExerciseId`, so a command names *which* entry it changes even when an exercise appears twice. A position index was rejected: reordering or deleting an entry would silently re-target replayed commands. The ID is also the stable key sync needs (DD-04).

**Progression schemes are stored per exercise.**
- An exercise has an optional `ProgressionScheme`. There is no default until the user sets one, and without one `LoadStep` is not evaluated.
- A per-template override and kind-based default rep ranges (`ExerciseKind`) are deferred (§5).
- Verdicts always use the exercise's current scheme.

**A tracking mode can't change once sets exist** (as `exercise.rs` already says). No command changes a mode yet; when one is added it refuses the change with `EngineError::Conflict` (the variant arrives with that command). A domain `ModeMismatch` therefore means stored data is inconsistent, and it is reported as an error, never as a verdict.

**Verdicts are computed on read, never stored.** The user must be able to open any past workout and see whether it was productive, and that answer must stay right after edits, late deload marks, imports of older sessions, scheme changes and rule upgrades. A stored verdict would need invalidating on every one of those; a computed one is correct on the next read. Logging new sessions never changes an old session's verdict, because its baseline is always earlier.
- The `session_verdicts` query loads the session and each entry's baseline, then calls `progression::exercise_verdict`. It returns one result per entry plus the `SessionVerdict` counts.
- An entry whose mode has no rules yet gets its own "unsupported" result instead of failing the whole session.
- Computing on read means an edit, a deload toggle, a scheme change or an `ALGORITHM_VERSION` bump shows up immediately, with no invalidation. Records (DD-14) are different: they are materialized and recomputable.

**Past sessions stay editable** (`product.md` §3.5), so sets can be logged into a finished session.

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
│  ├─ gymos-store/            # rusqlite implementation + migrations
│  └─ gymos-testkit/          # dev-only: in-memory fakes, builders, contract scenarios
├─ apps/
│  └─ desktop/
│     ├─ src-tauri/           # gymos-desktop crate (Tauri shell, IPC commands)
│     ├─ src/                 # React + TS UI
│     ├─ package.json
│     └─ vite.config.ts
├─ docs/
│  ├─ design.md               # this file
│  ├─ product.md              # product definition (what GymOs does)
│  ├─ prompts/                # implementation handoffs, one per PR
│  ├─ research/               # spikes behind the decisions here
│  └─ adr/
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
| Overload signals for bodyweight modes (DD-13) | After the body-weight measurement model |
| Weight-dependent 1RM formula as a new `ALGORITHM_VERSION` | Only if peer-reviewed evidence holds up |
| Milestones and achievements beyond records (DD-14) | After M1, checked against the no-dark-patterns principle (§1.2.6) |
| Per-exercise unit override (D10) | With the exercise catalog |
| Jaccard session matching and a user-picked baseline (DD-15) | When template-less baselines prove too coarse; needs a versioned threshold and golden tests |
| Per-template scheme override and kind-based default rep ranges (DD-15) | With templates and an `ExerciseKind` |
| Sync outbox table (DD-15) | With multi-device sync (M2) |

## 6. Open questions (decide at scaffold or schema time)

1. **Type generation:** specta vs ts-rs.
2. ~~**Sync-ready columns in schema v1.**~~ **Resolved 2026-11-16:** they ship in schema v1 (DD-04).
3. **UI libraries:** component kit (shadcn/ui?) and data fetching over IPC (TanStack Query?).
4. ~~**Command model.**~~ **Resolved 2026-11-16:** `Command` + `CommandId` idempotency from day one; the outbox table comes with sync (DD-15).
5. **Timeline:** re-plan M1 after the scaffold, based on actual Rust velocity.
6. **Session-level overload boolean (DD-13):** the spec only gives counts ("4 of 6 exercises progressed"). If a single session yes/no is wanted, define it explicitly, for example "at least one exercise progressed and none regressed". This is a product decision, not something the engine infers. Until then, `SessionVerdict` holds counts only.
7. **Indoor vs outdoor records (DD-14):** should a treadmill 5 km and a road 5 km share one "fastest 5 km" record? Indoor distances are machine estimates. Leaning: separate records when the environment is known. Decide with the record detectors.