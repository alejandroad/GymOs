# Handoff: PR — SQLite store (DD-04, DD-15; PROMPT-2 §10 "2b")

**Date:** 2026-11-19 · **Owner:** Alejandro · **For:** the implementing agent
**Branch:** `feat/sqlite-store` off `main` (at `8e7570c`, PR #12 merged) · **Size:** one PR, about 8–9 commits
**Status:** ready. Every §4 decision was settled by the owner on 2026-11-19 (all as recommended, including the "Also answer" items).

---

## 1. Why this PR, and why now

PR #12 built the engine against ports and proved it on an in-memory fake: commands, `last_time`, `session_verdicts`, and a store contract in `gymos-testkit/src/scenarios.rs`. Nothing persists yet. Every later milestone needs real storage:
- **Records** are materialized and need stored history (DD-14).
- **The Tauri shell** needs something to construct `GymOs` with.
- **Sync** needs the D-7 columns from schema v1.

This PR builds `gymos-store`, the only crate that knows SQLite exists:
- `SqliteStore`, implementing `Store`, `ReadTx` and `WriteTx`;
- the first migration;
- open → backup → migrate, refusing an unknown schema;
- row mappers that re-validate everything through domain constructors.

At the end, **the same contract scenarios and engine stories that pass on `FakeStore` pass on real SQLite**, in memory and on disk.

**Alternatives considered:**
- **Records engine first, on the fake.** Records must be materialized "inside the same `write` as `LogSet`". Proving that against a real transaction first is the cheaper order, and the records PR also brings edit and delete commands (§10), which need the tombstone decision made here.
- **Tauri shell first.** The owner placed it last, and it would have nothing to persist into.

## 2. Before you start

1. **Prerequisite:** PR #12 is merged into `main` (`8e7570c`).
2. **Decisions:** every §4 decision must be marked *Decided*. If one isn't, stop and ask.
3. Read, in order:
   - `docs/design.md`: DD-03 (including "Port shapes"), DD-04, DD-07, DD-09, DD-10, DD-11, DD-15
   - `docs/research/kernel-architecture.md`: §5.2, §5.3, §5.5, §6.4, §7, §8, §9
   - `docs/prompts/PROMPT-2-engine-ports.md`: §4 (D-1 to D-7) and §10
   - the engine ports: `crates/gymos-engine/src/ports/{store,clock,ids}.rs`, `error.rs`, `command.rs`, `commands.rs`
   - the oracle you must satisfy: `crates/gymos-testkit/src/scenarios.rs`, then `fake_store.rs` (the reference behaviour) and `gym.rs`
   - the domain types you will map: `crates/gymos-domain/src/{session,set,exercise,units,effort,time,provenance,text,ids}.rs`, `progression/scheme.rs`
4. Run `cargo test --workspace` and confirm it's green: **223 tests** before you start.
5. **Don't commit, push or open PRs.** The owner does that. §8 describes the intended commit boundaries, so keep each step self-contained and green.

## 3. Scope

### In
- **`SqliteStore`:**
  - `Store` impl with one connection owner (S-1);
  - `SqliteRead<'t>` and `SqliteWrite<'t>` wrapping `rusqlite::Transaction<'t>`;
  - the `ReadTx` and `WriteTx` impls.
- **Opening a database:**
  - `SqliteStore::open(StoreConfig)` and `SqliteStore::in_memory()`;
  - pragmas (S-8);
  - a backup before any pending migration (S-6);
  - forward-only migrations;
  - refusing a schema newer than this build knows (DD-04), with a typed `OpenError`.
- **Schema v1** (`migrations/0001_initial.sql`): exercises, sessions, session entries, sets and applied commands, with the D-7 sync-ready columns (S-2, S-3).
- **Row mappers** (`rows/`): domain → row and row → domain. Reading always goes back through the domain's fallible constructors; a rejected row is `RepoError::Corrupt`, never a panic (S-5).
- **Error mapping:** `rusqlite::Error` → `RepoError` in one place, `error.rs` (DD-07).
- **Replayed-command outcomes** (S-7), if decided: the engine change, the fake's change and the SQLite implementation.
- **Tests (`crates/gymos-store/tests/`):**
  - **`contract.rs`:** every `gymos_testkit::scenarios` function on SQLite, in memory and on a temp file.
  - **`engine.rs`:** the walking skeleton and a representative set of `training_history` stories, run through `GymOs` over `SqliteStore` (S-9).
  - **`open.rs`:**
    - a fresh database;
    - reopening a database;
    - a backup is taken only when a migration is pending;
    - a newer schema is refused and the file is left byte-for-byte untouched;
    - a corrupt row is reported as `Corrupt`.
  - **`upgrade.rs`:** the committed v1 fixture (S-10) migrates to the latest and the contract invariants hold on it.
  - **`roundtrip.rs`:** a property test that any valid session survives save → load unchanged (S-11).
- **Docs:** a DD-04 amendment recording the §4 answers.

### Out
- Record (PR) detection and materialization: the next PR (§10).
- Edit and delete commands, and changing a scheme or name: the next PR. The schema must be ready for them (S-3), but no new engine command lands here.
- HLC generation and any sync logic. The columns exist; their values are placeholders (S-2).
- Templates as stored entities: `template_id` is a plain column, with no `templates` table yet.
- The Tauri shell, the IPC error codes and resolving the database path. The shell passes a path in.
- Activities, measurements and the profile. Their tables come with their features.

## 4. Decisions (owner to settle)

Each decision lists options and a recommendation. Replace *Proposed* with *Decided: X* before handoff.

### S-1. Who owns the connection · *Decided: A (2026-11-19)*
| Option | Shape | Trade-off |
|---|---|---|
| **A. `Mutex<Connection>`** *(recommended; DD-04, kernel-architecture §5.2)* | `write` locks, opens an `IMMEDIATE` transaction and runs the closure. `read` locks and opens a deferred transaction, so all reads in one closure see one snapshot. | Simplest, and fully synchronous, matching the kernel. Reads queue behind writes, which doesn't matter for one user. |
| B. Dedicated DB thread with a channel | Messages carry closures. | Needs `Send + 'static` closures and boxing. A lot of machinery for no measurable gain. |
| C. Connection pool, with readers in WAL mode | Concurrent reads. | Adds a dependency (r2d2) and more failure modes. Revisit only if profiling asks for it. |

**Also answer:** a poisoned mutex is reported as `RepoError::Backend`, not recovered from. The fake can recover because it swaps state; SQLite can't prove the connection is clean. *Recommended: yes.*

### S-2. Sync-ready columns (D-7) before sync exists · *Decided: A (2026-11-19)*
D-7 decided the columns ship in v1. This decides what goes in them now.

| Option | Shape | Trade-off |
|---|---|---|
| **A. Columns present, written with placeholders** *(recommended)* | Every synced table gets `hlc_updated TEXT NOT NULL DEFAULT ''`, `deleted INTEGER NOT NULL DEFAULT 0 CHECK (deleted IN (0,1))` and `schema_version INTEGER NOT NULL`. Per-field clocks are one `field_clocks TEXT NOT NULL DEFAULT '{}'` column per table (S-4 explains why not one column per field). The store writes `''` / `'{}'` until sync defines the format. | No HLC design is pulled into this PR, and no migration over every table later. The placeholder is easy to detect and backfill. |
| B. Implement an HLC port now | `Hlc` in the engine, generated on every write. | It's the right end state, but it needs node identity, a clock-skew policy and an encoding: a sync design, not a storage one. |

### S-3. How `save_session` writes the aggregate, and what "removed" means · *Decided: A (2026-11-19)*
The `WriteTx::save_session` contract (scenario `saving_a_session_replaces_it`): anything stored for the session but missing from the new value is gone from reads.

| Option | Shape | Trade-off |
|---|---|---|
| **A. Upsert by ID, tombstone what's missing** *(recommended)* | Entries and sets have stable IDs (D-3, `SetId`). `INSERT … ON CONFLICT(id) DO UPDATE` each row, then set `deleted = 1` on child rows of this session whose IDs aren't in the aggregate. Every read filters `deleted = 0`. | Sync-ready: a deletion is a fact that can be replicated. Rows keep their identity, so edits in the next PR are plain upserts. Costs a filter on every read; covered by partial indexes. |
| B. Delete children and reinsert | Simplest SQL. | Hard deletes leave nothing to sync, and every save churns every set row. Contradicts D-7. |
| C. Diff in the engine with per-row store methods | Precise writes. | Per-row setters on the store: kernel-architecture §5.3 option B, rejected in DD-03. |

**Also answer:**
- **Order:** entries and sets get an explicit `position INTEGER NOT NULL`, written from the aggregate's order on every save. Order is data, and occurrence matching (D-1) depends on it. *Recommended: yes.*
- **Re-adding an ID:** a tombstoned row whose ID appears again is revived (`deleted = 0`). *Recommended: yes*, because replaying a sync can legitimately do this.

### S-4. Column encodings · *Decided: all recommended (2026-11-19)*
| Value | Recommended encoding | Why |
|---|---|---|
| UUIDs (every ID, `CommandId`) | `BLOB` of 16 bytes, `CHECK (length(id) = 16)` | Compact; v7 sorts bytewise, which matches `SessionId` tie-breaking (DD-15). |
| `Timestamp` | `*_utc_ms INTEGER` + `*_offset_min INTEGER` | DD-04: UTC plus the original offset. Order by the UTC column only. |
| `Load` | `*_hundredths INTEGER` + `*_unit TEXT CHECK (unit IN ('kg','lb'))` + `*_ng INTEGER` (normalized nanograms) | DD-04: as entered, plus normalized. Equality and display use what was entered; queries can sort on `_ng`. On read, `_ng` is re-derived and checked against the stored value: a mismatch is `Corrupt`. |
| `SetMetrics` | One `sets` table with a `mode TEXT` discriminant and nullable columns per field, plus a `CHECK` per mode requiring exactly that mode's fields | One table keeps "sets of a session" a single indexed query. The `CHECK`s make an impossible row unwritable even by hand-written SQL. |
| `Effort` | `effort_scale TEXT CHECK (… IN ('rpe','rir'))` + `effort_value INTEGER` (RPE in half-points, RIR in reps), both null or both set | Lossless; scales stay distinct (DD-13 compares them on one scale at read time). |
| `Source` | `source_kind TEXT` + `import_format TEXT NULL` + `importer_version INTEGER NULL`, with a `CHECK` tying them together | Data-model rule 4: provenance. |
| `ProgressionScheme` | `scheme_min_reps`, `scheme_max_reps` and the increment as a `Load` triple, all null or all set | One column per fact, and queryable. |
| Enums (`TrackingMode`, `SetType`, `WeightUnit`, …) | Stable lowercase `TEXT` codes, with a `CHECK (… IN (…))` | Readable in a DB browser and safe to reorder in Rust. **Never store a Rust discriminant or a `Debug` string.** The mapping lives in one function per enum, with an exhaustive `match` both ways. |
| Text (`Name`, `Note`) | `TEXT` | Re-validated on read (control characters, length). |

**Why `field_clocks` is JSON and not a column per field (S-2):** the per-field set changes whenever a field is added, and sync owns its format. It is opaque to the store.

### S-5. A row the domain rejects · *Decided: A (2026-11-19)*
| Option | Shape | Trade-off |
|---|---|---|
| **A. Fail the read with `RepoError::Corrupt(detail)`** *(recommended)* | The detail names the table, the ID and the rejected field, never the value. | Honest. The shell can offer "restore the backup". Matches DD-08: the database is just another input boundary. |
| B. Skip bad rows | | A silent data loss that would also make verdicts wrong. |

### S-6. Backup before migrating · *Decided: A (2026-11-19)*
| Option | Shape | Trade-off |
|---|---|---|
| **A. SQLite online backup (`rusqlite::backup`), or `VACUUM INTO`, into `<db>.<label>.bak`, only when a migration is pending** *(recommended)* | The `label` comes from `StoreConfig` (the shell passes a timestamp string). The store never reads the clock (clippy's `disallowed-methods`). Existing backups are never overwritten: an existing target is an `OpenError`. | Safe under WAL. A plain file copy can miss pages that are still in the `-wal` file. |
| B. Copy the file | DD-04's literal wording. | Unsafe with WAL unless checkpointed first. If chosen, checkpoint (`wal_checkpoint(TRUNCATE)`) first. |

**Also answer:** how many backups are kept? *Recommended: all, for now.* Pruning is a shell setting later; deleting user data is not the store's call.

### S-7. A replayed command returns its original outcome · *Decided: A (2026-11-19)*
Today a retried `StartSession` gets `AlreadyApplied` with no `SessionId`, so a UI whose first response was lost can't continue (PR #12 review, finding 4).

| Option | Shape | Trade-off |
|---|---|---|
| **A. Store the outcome with the command** *(recommended)* | `applied_commands(id, kind, outcome_kind, outcome_id NULL, applied_utc_ms)`. `WriteTx::mark_applied` becomes `begin_command(CommandId) -> Option<CommandOutcome>` (`Some` = already applied, here is what it did), plus `finish_command(CommandId, &CommandOutcome)` inside the same transaction. `CommandOutcome::AlreadyApplied` becomes `Replayed(Box<…>)` or the engine returns the stored outcome directly; pick the smaller API and document it. | Retries become truly idempotent. One small engine and testkit change, in its own commit, before the store implements it. |
| B. Keep `AlreadyApplied` | No change. | The UI must invent recovery, and every importer has to reason about lost responses. |
| C. Derive entity IDs from the `CommandId` | No outcome table. | Couples ID generation to commands and breaks the `IdGen` port's meaning. |

**Also answer:** detecting a reused `CommandId` with a *different* payload (a payload hash column). *Recommended: defer*, and note it in DD-15.

### S-8. Connection settings · *Decided: all recommended (2026-11-19)*
- `PRAGMA foreign_keys = ON`, checked after setting (it is silently ignored inside a transaction).
- `PRAGMA journal_mode = WAL` for files. In-memory databases keep their default.
- `PRAGMA synchronous = FULL`. Writes are tiny, and losing a logged set is the worst outcome this app has.
- `busy_timeout` of 5 s, which protects against an external DB browser holding a lock.
- `rusqlite` with the `bundled` feature (DD-04). Confirm the version pairing at implementation time: `rusqlite 0.43.x` with a `rusqlite_migration` release that supports it (2.x at the time of writing). Each dependency goes in its own `build(store): …` commit.

### S-9. Running engine scenarios on SQLite · *Decided: A (2026-11-19)*
| Option | Shape | Trade-off |
|---|---|---|
| **A. Make `TestGym` generic over its store** *(recommended)* | `TestGym<S: Store = FakeStore>`, with `TestGym::with_store(s)`, plus a `SqlitePorts` bundle in `gymos-store/tests/`. Engine stories are then written once against `TestGym<S>`. | The strongest proof that the fake matches SQLite. Needs `FakeStore`'s snapshot API behind a small trait, or engine-only stories that compare verdicts rather than state. |
| B. Re-run only the contract scenarios | Zero testkit change. | The contract checks storage, not orchestration. A subtle ordering or transaction bug would only show up in the app. |

### S-10. The v1 fixture database · *Decided: A (2026-11-19)*
DD-04 requires a committed fixture per *released* schema version. None is released yet.
- **A (recommended):** commit `tests/fixtures/schema_v1.sqlite` now, generated by a `#[test] #[ignore]` generator that seeds a rich history (every mode and set type, a deload, templates, an import), with the generator's source in the repo. `upgrade.rs` opens a copy, so the fixture itself is never modified. This puts the upgrade harness in place before it's needed.
- B: wait for the first release. The harness then arrives under release pressure.

### S-11. Strategies for property tests · *Decided: A (2026-11-19)*
- **A (recommended):** add `gymos-testkit/src/strategies.rs` (planned in kernel-architecture §6.5). It contains `proptest` strategies for valid `Load`, `Reps`, `Effort`, `SetMetrics` per mode, and `Session`s with 0–5 entries of 0–6 sets each. `proptest` becomes a testkit dependency, which is fine because testkit is dev-only. The round-trip property is `load(save(s)) == s`.
- B: hand-written cases only.

## 5. Target layout

```
crates/gymos-store/
├─ Cargo.toml          deps: gymos-engine, gymos-domain, rusqlite (bundled), rusqlite_migration, thiserror
│                      dev: gymos-testkit, tempfile, proptest
├─ migrations/0001_initial.sql      forward-only; never edited once released
├─ src/
│  ├─ lib.rs           curated pub use: SqliteStore, StoreConfig, OpenError
│  ├─ sqlite_store.rs  SqliteStore { conn: Mutex<Connection> }, impl Store
│  ├─ open.rs          open → pragmas → (backup if pending) → migrate → refuse unknown schema
│  ├─ backup.rs
│  ├─ migrations.rs    the embedded migration list (include_str!)
│  ├─ tx.rs            SqliteRead<'t>, SqliteWrite<'t>, impl ReadTx / WriteTx
│  ├─ rows.rs          facade; rows/{exercise,session,entry,set,command,codes}.rs
│  └─ error.rs         the ONLY place rusqlite::Error is named; From<rusqlite::Error> for RepoError
└─ tests/{contract,engine,open,upgrade,roundtrip}.rs, tests/fixtures/schema_v1.sqlite
```

## 6. Design: what you must follow

### 6.1 Boundaries
- `gymos-store` depends on `gymos-engine` and `gymos-domain`, never the reverse. **No `rusqlite` type appears in any `pub` signature** (`cargo-deny` wrappers, kernel-architecture §9).
- The store does **not** apply business rules. It never filters deloads, unfinished sessions or warm-ups; that's the engine (D-1, DD-15 rule 5). `sessions_containing` returns every live (`deleted = 0`) session with a live entry for the exercise.
- The store never reads a clock, the environment or a path of its own. Everything comes through `StoreConfig`.

### 6.2 Transactions
- `write`: `BEGIN IMMEDIATE` → closure → `COMMIT` on `Ok`, `ROLLBACK` on `Err` *and on panic* (the guard's `Drop` rolls back). A failed `COMMIT` is `RepoError::Backend`, and the closure's result is discarded.
- `read`: a deferred transaction that is always rolled back, so reads can never write.
- Prepared statements are cached (`prepare_cached`). **No SQL is built with `format!`.** Every value is a bound parameter (OWASP A03). Table and column names are literals.

### 6.3 Mapping
- Encoding and decoding each enum lives in `rows/codes.rs`, both directions, exhaustive. A unit test round-trips every variant.
- Decoding goes through the domain constructors: `Load::new`, `Reps::new`, `Name::new`, `Session::start` + `finish` + `add_exercise` + `push_set`, and so on. If the domain refuses, the row is `Corrupt` (S-5). A row is never trusted because the store wrote it.
- `save_session` writes in one pass, parent then children by `position`, then tombstones what's missing (S-3).

### 6.4 Indexes (for the queries the engine actually makes)
- `session_entries(exercise_id) WHERE deleted = 0`, for `sessions_containing`.
- `sessions(started_utc_ms, id)`.
- `sets(entry_id, position) WHERE deleted = 0`.
- A `PRIMARY KEY` on every `id`. Foreign keys from entries to sessions, sets to entries, and entries to exercises.

## 7. Tests: the oracle

The contract and the engine behaviour already exist and are the oracle. **Don't change an expected value in an existing test to make SQLite pass.** If SQLite and the fake disagree, the fake is the reference unless the contract is wrong; if you believe the contract is wrong, stop and report it.

Required:
- **Contract:** every function in `gymos_testkit::scenarios` passes on `SqliteStore::in_memory()` and on a temp-file store, including `every_field_round_trips`, `a_failed_write_changes_nothing` and `saving_a_session_replaces_it`.
- **Engine on SQLite** (S-9): the walking skeleton matches golden example A exactly. Also required, at minimum, from `training_history.rs`:
  - the double-progression cycle;
  - the deload week;
  - importing an older session that moves a later baseline;
  - rotating templates;
  - replayed commands, and with S-7 the replay now returns the original outcome.
- **Rollback under a real transaction:** a command that fails after `begin_command` leaves no row in any table, including `applied_commands`.
- **Panic safety:** a closure that panics inside `write` leaves the database unchanged and the store usable afterwards (catch the panic in the test).
- **Open:**
  - A fresh file gets v1, and reopening it applies nothing.
  - A backup is written only when a migration is pending.
  - A database with `user_version` above the latest known is refused with `OpenError::NewerSchema { found, supported }`, and the file's bytes are unchanged.
- **Corruption:** for each of an unknown enum code, an out-of-range rep count, a `Load` with mismatched `_ng`, and a `NULL` in a required mode column, raw SQL plants the bad value and the read returns `RepoError::Corrupt`. Seeding with raw SQL is allowed only in tests.
- **Upgrade:** a copy of the v1 fixture opens, migrates (a no-op today) and passes the read-only contract checks.
- **Round-trip property** (S-11): at least 256 cases. The shrunk failure must be readable.
- **Determinism:** `sessions_containing` returns the same set as the fake for a seeded history (compare sorted).

## 8. Engineering rules (non-negotiable)

These are the same as PROMPT-2 §8:
- `/** */` and `/*! */` doc style;
- `-D warnings`;
- no `unwrap`, `expect` or `panic` outside tests;
- no `_ =>` on domain enums;
- `#[expect(.., reason)]` for every suppression;
- `BTreeMap` only;
- every public item documented, with `# Errors` (and `# Panics` in testkit).

Also:
- **Lessons from PR #12.** Apply them up front:
  - Test files use one file-level `#![expect(clippy::unwrap_used, …, reason = "…")]` listing only lints that actually fire.
  - Build test sets with `vec![…; n]` (`Note` isn't `Copy`).
  - Keep helpers under clippy's `too_many_lines` by splitting by concern, not by suppressing.
  - Prefer a generic `fn` over a closure taking `&mut dyn Iterator`.
  - **Check every number in this prompt against the oracle before relying on it**: PROMPT-2 had golden A's baseline wrong.
- **Edit files with the editor tools**, not scripted slicing. A scripted edit in PR #12 duplicated tests silently.
- `gymos-store` is not part of the wasm32 check (it can't be). The domain and engine checks must stay green.
- `cargo deny check` must pass with the new tree. `rusqlite` and `libsqlite3-sys` are MIT; the bundled SQLite is public domain. If a license isn't in `deny.toml`, stop and ask; don't add it.

**Intended commit boundaries** (one logical change each, about 200 lines or fewer, excluding the fixture and lockfile):
1. `feat(engine): return the original outcome for replayed commands`, with the testkit fake updated (if S-7 = A)
2. `build(store): add rusqlite and rusqlite_migration`
3. `feat(store): add schema v1 migration`
4. `feat(store): open, back up and migrate a database`
5. `feat(store): map rows to and from domain values`
6. `feat(store): implement read and write transactions`
7. `test(store): run the store contract and engine stories on sqlite` (with the generic `TestGym` if S-9 = A; the testkit part may be its own `refactor(testkit)` commit)
8. `test(store): add v1 fixture, upgrade and round-trip tests` (the `proptest` strategies may be their own `feat(testkit)` commit)
9. `docs(design): record store decisions in DD-04`

## 9. Definition of done

- [ ] Every §7 test passes. All 223 existing tests still pass, unchanged except where S-7 deliberately changes an `AlreadyApplied` assertion.
- [ ] `cargo fmt --check`, clippy `-D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc`, `cargo deny check` and the wasm32 check (domain and engine) pass.
- [ ] Every §4 decision is implemented as recorded, and DD-04 is amended in its own `docs(design):` commit.
- [ ] No `rusqlite` type in a public signature, and no `format!`-built SQL (`grep -rn "format!(" crates/gymos-store/src | grep -i -E "select|insert|update|delete"` is empty).
- [ ] No changes outside `crates/gymos-store`, `crates/gymos-testkit`, the S-7 change in `crates/gymos-engine`, `Cargo.toml` / `Cargo.lock` and `docs/design.md`.
- [ ] **A self-review:** before handing back, spawn a review subagent with §3–§9 of this prompt. Fix what it confirms, and list what you declined and why.

## 10. After this PR

1. **The records PR.** It brings:
   - record detection, materialized in the same `write` as `LogSet`, reusing `progression::e1rm`;
   - edit and delete commands: `EditSet`, `DeleteSet`, `RemoveExercise`, `DeleteSession`, `UpdateExercise` for name and scheme, each recomputing the records it affects in the same transaction.

   Questions it must settle first, carried over from PR #12:
   - **A `Quantity` for volume.** `BestSetVolume` and `BestSessionVolume` can't be stored today: `Mass` caps at 1,000 kg through `Load`, and the progression code uses a separate `Volume` (kg·reps).
   - **Cross-unit records.** Do records use the 0.5% kg/lb tolerance that verdicts use? Today 220.5 lb "beats" 100 kg by 18 g.
   - **Adding an RPE where there was none.** It raises the e1RM (unrated sets are treated as RPE 10), so it reads as progress.
2. **The Tauri shell:**
   - `Desktop` ports and `SystemClock`;
   - a UUIDv7 `IdGen`;
   - resolving the database path and the backup label;
   - `IpcError` codes, with `spawn_blocking` around engine calls.
