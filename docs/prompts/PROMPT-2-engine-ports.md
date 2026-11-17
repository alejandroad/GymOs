# Handoff: PR — Engine ports and walking skeleton (DD-03, DD-13 baseline)

**Date:** 2026-11-16 · **Owner:** Alejandro · **For:** the implementing agent
**Branch:** `feat/engine-ports` off `main` · **Size:** one PR, about 6–8 commits
**Status:** ready. Every §4 decision was settled by the owner on 2026-11-16.

---

## 1. Why this PR, and why now

PR #11 added the overload rules (`gymos_domain::progression`). Nothing can use them yet. `exercise_verdict` needs a baseline, and finding "last time" while skipping deloads (DD-13 gap 2) needs history, which makes it an engine query. `gymos-engine` and `gymos-testkit` are still empty doc comments.

This PR builds the engine half of DD-03:
- the ports (`Store`, `Clock`, `IdGen`);
- the `GymOs` struct, with the commands needed to log a workout;
- the `last_time` and `session_verdicts` queries;
- in-memory fakes, so all of it is tested without a database.

At the end, a test can create an exercise, log two sessions and read back a verdict. That is the walking skeleton.

**Alternatives considered:**
- **The engine and the SQLite store in one PR.** It retires more risk at once, but it's about 15 commits, more than DD-10 allows. The store also needs schema decisions (open question 2) that shouldn't hold up the engine. The store is the **next** PR (§10).
- **The PR/records engine first.** It needs persisted history to detect records against, so it comes after the store.
- **The Tauri shell.** The owner placed it last.

## 2. Before you start

1. **Prerequisite:** PR #11 (overload verdicts) is merged into `main`.
2. **Decisions:** every §4 decision is settled. Build exactly what §4 records. D-7 applies to the store PR, not this one.
3. Read, in order:
   - `docs/design.md` DD-03, DD-06, DD-07, DD-09, DD-10, DD-13 (gap 2 and "Placement")
   - `docs/research/kernel-architecture.md` §5.1–5.6, §6.3, §6.5, §8 and §9 (port shapes, transactions, anti-patterns)
   - `docs/product.md` §1.3, §3.2 and §3.3
   - `crates/gymos-domain/src/{session,exercise,ids,time,progression}.rs`
4. Run `cargo test --workspace` and confirm it's green: **116 tests** before you start.
5. **Don't commit.** The owner makes the commits. §8's sequence describes the intended commit boundaries, so keep each step self-contained and green.

## 3. Scope

### In
- **Ports:** `Ports` bundle, `Store` with closure-scoped `read`/`write`, `ReadTx`/`WriteTx`, `Clock`, `IdGen` (kernel-architecture §5.1, §5.3).
- **Errors:** `EngineError` and `RepoError` (DD-07).
- **Commands:**
  - `CreateExercise`
  - `StartSession`
  - `AddExercise` (adds an exercise to a session)
  - `LogSet`
  - `FinishSession`
  - `SetDeload`

  All writes go through one `Command` enum carrying a `CommandId`, and are idempotent (D-5).
- **Queries:**
  - `last_time(session, entry)`: the baseline resolution (D-1)
  - `session_verdicts(session)`: an `ExerciseVerdict` per entry, plus the `SessionVerdict`
- **`gymos-testkit`:**
  - `FakeStore`, an in-memory store with real rollback;
  - `FixedClock` / `SteppingClock`;
  - `SeqIds`, a deterministic ID generator;
  - builders for sessions and sets.
- **Tests:** engine orchestration tests (`gymos-engine/tests/`). Store-generic contract scenarios in `gymos-testkit/src/scenarios.rs`, run against the fake only for now.
- **Domain:** whatever small, additive change D-3 requires.

### Out
- SQLite, migrations and `gymos-store`: next PR.
- Record (PR) detection and materialization.
- Jaccard session similarity, and the user overriding the baseline (D-1).
- Editing and deleting sets or sessions, and recomputing derived data.
- The outbox table, sync and the IPC layer.
- Live per-set feedback (`beat` / `matched` / `below`) while logging.

## 4. Decisions (settled 2026-11-16)

The owner accepted the recommended option for D-1 to D-6, sub-questions included, and chose to include the sync-ready columns for D-7. The other options stay listed for the record. **2a** is this PR. **2b** is the SQLite store PR.

### D-1. How "last time" is resolved for verdicts · blocks 2a
**Decided: A.** No unfinished sessions as a baseline. Order by `started_at`, then `SessionId`. Duplicate entries match by occurrence; if the current session has more entries than the baseline, use the baseline's last entry.

Product §3.2 describes two ways to find the baseline:
- **Same workout type:** if the session came from a template, use the last session from that template. Otherwise, use the most similar session by Jaccard similarity.
- **Same exercise:** use the most recent session containing exercise X.

DD-13 adds one rule: deloads are never a baseline.

| Option | Rule | Trade-off |
|---|---|---|
| **A. Template first, then same exercise** *(recommended)* | If the current session has a template, use the most recent finished, non-deload session from that template that contains the exercise. Otherwise, or if there's none, use the most recent finished, non-deload session containing the exercise. | Deterministic, explainable, cheap. Jaccard is left out until it can be specified properly. |
| B. Full §3.2, including Jaccard | As A, but sessions without a template match on exercise-set similarity of at least a threshold. | Closer to the product spec. Needs a versioned threshold and golden tests, which makes this PR too big. |
| C. Same exercise only | Most recent non-deload session containing the exercise. | Simplest. A "Push A" bench press gets compared with a "Push B" bench press. |

**Questions you also need to answer:**
- **Unfinished sessions:** should an in-progress or abandoned session (no `ended_at`) ever be a baseline? Recommendation: no.
- **Order:** "most recent" goes by `started_at` (UTC millis), with ties broken by `SessionId`, so the result is deterministic.
- **Duplicate entries:** if the baseline session has the same exercise twice, which entry is the baseline? This depends on D-3. Recommendation: match by occurrence, so the 1st entry compares with the 1st and the 2nd with the 2nd. If the current session has more entries than the baseline, use the baseline's last entry.

### D-2. Where progression schemes live · blocks 2a
**Decided: A.** No default scheme until the user sets one (no `ExerciseKind` for now). Verdicts always use the current scheme.

`session_verdicts` has to load a `ProgressionScheme`, otherwise `LoadStep` never fires. Product §3.2 says the user picks a scheme "for each exercise or template", and new exercises default to double progression.

| Option | Shape | Trade-off |
|---|---|---|
| **A. Per exercise now, template override later** *(recommended)* | `Exercise` gains `scheme: Option<ProgressionScheme>`, or the store keeps it beside the exercise. `None` means `LoadStep` isn't evaluated. | Covers the main case. A template-level override can be added later without migrating existing data. |
| B. Exercise and template, resolved template → exercise → none | Needs a template entity, which doesn't exist yet. | It's the full product rule, but it pulls templates into this PR. |
| C. Per template only | Exercises logged without a template never get `LoadStep`. | Contradicts "new exercises default to double progression". |

**Questions you also need to answer:**
- **Default rep range:** what is a new exercise's default? Product suggests ranges by exercise kind (compound 5–8, accessory 8–12, isolation 10–15), but `Exercise` has no kind field. Recommendation: no scheme until the user sets one, and the UI prompts for it. The other option is to add an `ExerciseKind` now.
- **Scheme history:** if the user changes a scheme, are old verdicts recomputed with the new one? Recommendation: verdicts are computed on read (D-6), so they always use the current scheme. Document that.

### D-3. How a session's exercise entry is identified · blocks 2a
**Decided: A.** Add `SessionExerciseId`.

`LogSet` has to name *which* entry in a session it adds to. `Session::add_exercise` allows the same exercise more than once, and `SessionExercise` has no ID.

| Option | Shape | Trade-off |
|---|---|---|
| **A. Add `SessionExerciseId`** *(recommended)* | A new ID type in `ids.rs`, generated by `IdGen` and stored on `SessionExercise`. | Stays stable when entries are reordered or deleted, gives sync a stable key later, and allows a clean `EditSet` later. Small additive domain change. |
| B. Position in the session | `LogSet { session, position, .. }` | No domain change. Reordering or deleting an entry silently re-targets later commands, which breaks idempotent replay. |

### D-4. What happens if an exercise's tracking mode changes · blocks 2a
**Decided: A.** Changing the mode is forbidden once sets exist. A domain `ModeMismatch` means stored data is inconsistent, and it is reported as an error.

`exercise.rs` already says *"a mode can't change once sets exist", so the engine enforces it*. `exercise_verdict` returns `ModeMismatch` if one slips through.

| Option | Rule | Trade-off |
|---|---|---|
| **A. Forbid it** *(recommended)* | No `ChangeMode` command in this PR. If one is added later, it is refused once any set exists (`EngineError::Conflict`). A `ModeMismatch` from the domain then means stored data is inconsistent, and it is reported as an error. | Matches the existing doc comment. It's simple and honest. |
| B. Allow it, starting a new comparison history | `last_time` ignores sessions logged under the old mode, so the first session after the change is `InsufficientData`. | Friendlier, but it needs a mode on every stored entry and more rules. |

### D-5. Command model and idempotency (design.md open question 4) · blocks 2a
**Decided: A.** A `Command` enum with a caller-generated `CommandId`. The outbox table is deferred.

| Option | Shape | Trade-off |
|---|---|---|
| **A. `Command` enum with a `CommandId` now, outbox later** *(recommended, per kernel-architecture §5.3)* | Every write is a `Command` and runs inside one `store.write`. `WriteTx::mark_applied(CommandId)` returns `false` if the command already ran, and the engine then returns `AlreadyApplied`. | Importers and sync get one write path. Adding the outbox later is one extra call inside the closure. |
| B. Plain methods now, commands with sync | `os.log_set(..)` with no IDs. | Fewer types now. Every importer and the sync path would have to be retrofitted later. |

**Also confirm:** the **caller** generates the `CommandId` (the UI, an importer, or a test), not the engine, so a retried call reuses the same ID.

### D-6. Whether verdicts are computed on read or stored · blocks 2a
**Decided: A.** Verdicts are computed on read.

| Option | Shape | Trade-off |
|---|---|---|
| **A. Compute on read** *(recommended)* | `session_verdicts` loads the session and its baselines, then calls `exercise_verdict`. Nothing is stored. | Never stale: an edit, a deload toggle or a scheme change shows up immediately. The cost is a few lookups per exercise. |
| B. Store them when a session finishes | Fast reads. | Needs invalidation on every edit, deload toggle, scheme change and `ALGORITHM_VERSION` bump. Records need that machinery anyway, but verdicts don't yet. |

### D-7. Schema v1: sync-ready columns (design.md open question 2) · blocks 2b only
**Decided: include them** (owner, 2026-11-16). Schema v1 gets `hlc_updated`, per-field clocks, a `deleted` tombstone and `schema_version` on every synced table. `SessionExerciseId` (D-3) is a primary key there. The store PR records this answer to design.md open question 2.

The first migration either includes `hlc_updated`, per-field clocks, a `deleted` tombstone and `schema_version`, or leaves them for the sync work. design.md leans towards including them, because adding columns to every table later costs more. It doesn't affect this PR; it shapes the store PR's first migration.

## 5. Target layout

```
crates/gymos-engine/
├─ Cargo.toml              deps: gymos-domain, thiserror · dev: gymos-testkit
├─ src/
│  ├─ lib.rs               curated pub use
│  ├─ gymos.rs             GymOs<P: Ports>: execute(Command), query entry points
│  ├─ ports.rs             Ports bundle trait (facade for ports/)
│  ├─ ports/{store,clock,ids}.rs
│  ├─ command.rs           enum Command, CommandId, CommandOutcome
│  ├─ commands/            one file per command; each is one store.write(|tx| …)
│  ├─ queries/{last_time,session_verdicts}.rs
│  └─ error.rs             EngineError, RepoError
└─ tests/commands.rs       GymOs<TestPorts>: orchestration, idempotency, rollback
crates/gymos-testkit/src/
├─ lib.rs  fake_store.rs  fixed_clock.rs  seq_ids.rs  builders.rs  scenarios.rs
```

Create folders as you need them, because git doesn't track empty ones.

## 6. Design: what you must follow

### 6.1 Ports (kernel-architecture §5.1–5.3)
- **One `Ports` bundle trait with associated types**, so `GymOs<P: Ports>` has a single type parameter. No trait objects on the write path.
- **The store is a closure-scoped unit of work.** `write(|tx| …)` commits on `Ok` and rolls back on `Err`. `read(|tx| …)` gets a read-only view. Every command makes exactly one `write` call.
- **Writes work on the whole aggregate.** `WriteTx` saves and loads a whole `Session`, and the domain type checks its own rules. Don't add per-field setters to the store, because business sequencing would then leak into the adapter (§5.3 option B).
- **Clock and IDs are injected.** `Clock::now() -> Timestamp`. `IdGen` returns a fresh `Uuid`, and the engine wraps it in the right ID newtype. The domain never creates "now" or random values.
- **Everything is synchronous** (§5.2). Methods take `&self`. Interior mutability belongs inside the store.

### 6.2 Commands
- `execute(Command) -> Result<CommandOutcome, EngineError>`.
- **Each command does, in order:** `mark_applied`, load, construct domain values, decide (pure), persist.
- **Errors:**
  - Validation errors are domain errors, wrapped in `EngineError::Domain(..)`.
  - A missing entity is `NotFound`.
  - A rule that needs history (such as a refused mode change, D-4) is `Conflict`.
- **`LogSet` accepts sets on a finished session** (decided): product §3.5 says past sessions stay editable.

### 6.3 Queries
- **`last_time`** implements D-1 exactly and returns `Option<…>` with the baseline session and entry. It never returns a deload or an unfinished session.
- **`session_verdicts`** returns a view model: one `ExerciseVerdict` per entry, in the session's order, plus the `SessionVerdict` tally. Pass `current_is_deload` from the session. A domain `VerdictError` becomes `EngineError::Domain`, except `UnsupportedMode` (decided): it is an expected outcome, so it gets its own view-model variant per entry instead of failing the whole session.

### 6.4 Testkit fake
- `FakeStore::write` runs the closure against a **clone** of its state, and swaps the clone in only on `Ok`. That gives real rollback in engine tests.
- `BTreeMap` only (clippy determinism rule).
- The fake must pass the same `scenarios.rs` contract that the SQLite store will run in the next PR.

## 7. Tests: the oracle

**Orchestration (`gymos-engine/tests/commands.rs`):**
- **Walking skeleton:** create bench → session 1 (100×8 ×3) → session 2 (100×8, 100×9, 100×7) → `session_verdicts(s2)` equals golden example A.
- **Deloads are skipped:** S1 normal, S2 deload, S3 normal → S3's baseline is S1, and S2's verdict is `Deload`.
- **Template first (D-1):** S1 from "Push A", S2 from "Push B", S3 from "Push A" → S3's baseline is S1, not S2.
- **Unfinished sessions are never a baseline.**
- **Idempotency:** the same `CommandId` twice → `AlreadyApplied`, and the state is unchanged.
- **Rollback:** a command that fails after a partial write leaves the store byte-for-byte unchanged.
- **First session:** `InsufficientData`.
- **Mode guard (D-4):** an inconsistent stored mode surfaces as an error.

**Contract (`gymos-testkit/src/scenarios.rs`):** the scenarios are store-generic functions, run here against `FakeStore`.

## 8. Engineering rules (non-negotiable)

These are the same as PROMPT-1 §7: `/** */` and `/*! */` doc style; `-D warnings`; no `unwrap`, `expect` or `panic` outside tests; no `_ =>` on domain enums; `#[expect(.., reason)]` for every suppression; `BTreeMap` only; every public item documented with `# Errors`.

Also:
- `gymos-engine` depends only on `gymos-domain` and `thiserror` (DD-03; `cargo-deny` checks this).
- The wasm32 check covers `gymos-engine` and must stay green.
- **Don't commit, push or open PRs.** The owner does that.

**Intended commit boundaries** (for the owner; one logical change each, about 200 lines or fewer):
1. `feat(domain): add session exercise id` (if D-3 = A)
2. `feat(engine): add engine and repository errors`
3. `feat(engine): add store, clock and id ports`
4. `feat(testkit): add fake store, fixed clock and sequential ids`
5. `feat(engine): add command model with idempotency`
6. `feat(engine): add session logging commands`
7. `feat(engine): add last-time baseline query`
8. `feat(engine): add session verdicts query`

If D-2 changes `Exercise` or a dependency is added, give each its own commit.

## 9. Definition of done

- [ ] Every §7 test passes, including the walking skeleton matching golden example A
- [ ] `cargo fmt --check`, clippy `-D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc`, `cargo deny check` and the wasm32 check pass
- [ ] Every public item is documented, including `# Errors` sections
- [ ] Each §4 decision is implemented as recorded
- [ ] DD-03 is amended with the port shapes (`Ports` bundle, closure-scoped unit of work, sync kernel; kernel-architecture §10.1) in its own `docs(design):` change
- [ ] No changes outside `crates/gymos-engine`, `crates/gymos-testkit`, the small D-3 change in `crates/gymos-domain`, `Cargo.toml`/`Cargo.lock` and `docs/design.md`

## 10. After this PR

1. **The SQLite store (2b):**
   - `SqliteStore` implementing the ports;
   - `0001_initial.sql`, with the sync-ready columns (D-7);
   - backup before migrating, and refusing an unknown schema (DD-04);
   - row mappers, the same `scenarios.rs` contract run on real SQLite, and fixture upgrade tests.
2. **The PR/records engine,** reusing `progression::e1rm`, materialized inside the same `write` as `LogSet`.
3. **The Tauri shell.**
