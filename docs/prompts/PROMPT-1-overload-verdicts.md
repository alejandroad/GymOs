# Handoff: PR — Progressive overload verdicts (DD-13)

**Date:** 2026-11-16 · **Owner:** Alejandro · **For:** the implementing agent
**Branch:** `feat/overload-verdicts` off `main` · **Size:** one PR, about 6–8 commits

---

## 1. Why this PR, and why now

GymOs's main showcase is the deterministic progression engine (product §1.1, D14). The domain model it needs is finished: sets, set types, effort, sessions, the deload flag and exact masses. The rules are fully specified in `docs/design.md` **DD-13**. Nothing else blocks this work. It's pure functions in `gymos-domain`, testable without a database or UI.

**Alternatives considered:**
- **Engine ports + SQLite store + walking skeleton.** This retires infrastructure risk, but it has no product value until the rules exist. It's the **next** PR (§9).
- **The Tauri shell.** The owner explicitly placed this last.
- **The PR/records engine.** It shares e1RM code with this PR, so it should come after it.

## 2. Before you start

1. **Prerequisite:** the `data-models` branch (measurements, activities, profile, records, DD-14) must be merged into `main`. Don't start until it is.
2. Read these, in order:
   - `docs/design.md` DD-03, DD-07, DD-08, DD-09, DD-10 and **DD-13** (the spec for this PR)
   - `docs/product.md` §3.1–3.3
   - `crates/gymos-domain/src/{set,session,units,effort}.rs`
3. Run `cargo test --workspace` and confirm it's green: 48 tests before you start.

## 3. Scope

### In
- `ProgressionScheme` (rep range + increment). It's needed by the `LoadStep` signal.
- e1RM (Epley) with exact integer comparison.
- Set pairing.
- Signal detection for **`TrackingMode::WeightReps` only**.
- `Verdict`, `ExerciseVerdict` and the session roll-up.
- Golden tests from §6 and property tests.
- A small DD-13 amendment (§5.6).

### Out
- Other tracking modes. They return a typed error (§5.5).
- Finding the baseline session. That's an engine query, next PR.
- Where schemes are stored (exercise vs template).
- The UI and the IPC layer.
- The record detectors.

## 4. Target layout

```
crates/gymos-domain/src/
├─ progression.rs              facade: pub use of the items below
└─ progression/
   ├─ scheme.rs                ProgressionScheme, RepRange, SchemeError
   ├─ e1rm.rs                  epley factor, exact compare, RPE adjustment, display value
   ├─ pairing.rs               which sets pair with which (TopSet first, then order)
   ├─ signals.rs               Signal, Direction, detection for WeightReps
   └─ verdict.rs               Verdict, ExerciseVerdict, VerdictInput, SessionVerdict, VerdictError
crates/gymos-domain/tests/
├─ golden.rs                   examples A–L from §6, as hand-written expected values
└─ properties.rs               proptest: e1RM ordering properties
```

Create `crates/gymos-domain/src/progression/` (git does not track empty folders, so it won't exist in a fresh clone). `ALGORITHM_VERSION` already exists in `lib.rs`; reuse it, don't redefine it.

## 5. Design: the decisions you must follow

### 5.1 Public API (shape; names may be refined)

```rust
pub struct RepRange { min: Reps, max: Reps }             // min ≤ max, checked
pub struct ProgressionScheme { pub reps: RepRange, pub increment: Load }

pub struct VerdictInput<'a> {
    pub baseline: Option<&'a SessionExercise>,  // None → InsufficientData
    pub current: &'a SessionExercise,
    pub current_is_deload: bool,
    pub scheme: Option<&'a ProgressionScheme>,  // None → LoadStep is not evaluated
}

pub fn exercise_verdict(input: &VerdictInput<'_>) -> Result<ExerciseVerdict, VerdictError>;

pub enum Verdict { Progressed, Matched, Regressed, InsufficientData, Deload }

pub struct ExerciseVerdict {
    pub verdict: Verdict,
    pub signals: Vec<Signal>,     // strongest first (§5.4)
    pub skipped: Vec<Skipped>,    // e.g. e1RM skipped above 12 reps, so the explanation stays honest
    pub algorithm_version: u32,   // = ALGORITHM_VERSION
}
impl ExerciseVerdict { pub fn is_progressive_overload(&self) -> bool } // verdict == Progressed

pub struct SessionVerdict { pub progressed: u16, pub matched: u16, pub regressed: u16,
                            pub insufficient: u16, pub deload: u16 }   // counts only (open question 6)
```

`VerdictError::UnsupportedMode(TrackingMode)` covers anything other than `WeightReps`.

### 5.2 e1RM: Epley, exact
- `k(r) = 30 + r`, with `k(1) = 30` (a single rep *is* the 1RM). e1RM = `w·k(r)/30`.
- **Comparisons are exact.** Compare `w₁·k(r₁)` with `w₂·k(r₂)` using `Mass::nanograms()` as `i128`. Floats appear only in a display helper that returns kg.
- **RPE adjustment:** reps to failure = `reps + RIR`, where RIR = 10 − RPE.
  - Work in half-reps to stay integral: `2·r_eff = 2·reps + (20 − rpe_half_points)`. The comparison factor becomes `60 + 2·r_eff`, over 60.
  - `Effort::Rir(n)` maps to `r_eff = reps + n`.
  - No effort logged → `r_eff = reps`.
- **Cap:** if `r_eff > 12`, the set gives no e1RM. If no set in a session qualifies, skip the E1rm signal and push `Skipped::E1rmAboveRepCap`.
- The best set is the set with the highest e1RM among **counted** sets (not warm-ups).

### 5.3 Pairing (DD-13 gap 3)
- **Pairable sets:** `TopSet`, `Working` and `Failure`.
- **`Drop` and `Backoff` sets** are never paired, but they count toward volume. **`Warmup`** is excluded from everything.
- **Pair `TopSet`s first,** in order: the n-th top set with the n-th top set. Then pair the remaining pairable sets by order.
- **Unequal counts:** extra sets are unpaired. They feed `WorkingSets` and `VolumeLoad` only.

### 5.4 Signals for `WeightReps`, in output order (strongest first)

| # | Signal | Fires when |
|---|---|---|
| 1 | `E1rm { direction, before, after }` | best-set e1RM changed (§5.2) |
| 2 | `LoadStep { before, after, reps }` | scheme present **and** the top pairable load increased **and** the minimum reps among current sets at that load ≥ `scheme.reps.min` |
| 3 | `LoadAtReps { direction, set_index, reps, before, after }` | a pair has the same reps and a different load |
| 4 | `RepsAtLoad { direction, set_index, load, before, after }` | a pair has the same load and different reps |
| 5 | `WorkingSets { direction, before, after }` | pairable counts differ **and** every paired set is identical (same load and reps) |
| 6 | `VolumeLoad { direction, before, after }` | Σ load × reps over counted sets changed. **Up** only if the e1RM didn't drop; **Down** always |
| 7 | `Effort { direction, set_index, before, after }` | a pair has the same load and reps, both have an effort, and RPE differs (lower = Up) |

`set_index` is the 0-based index within the current session's pairable sets. The UI shows it 1-based. Volume uses an exact `i128` of nanograms × reps.

### 5.5 Verdict rules, evaluated in order (DD-13)
1. `current.mode() != WeightReps` → `Err(UnsupportedMode)`.
2. `current_is_deload` → `Deload`. Still compute and list the signals if there's a baseline.
3. No baseline, or no counted sets on either side → `InsufficientData`.
4. Any `Up` signal (or `LoadStep`) → `Progressed`.
5. Otherwise any `Down` → `Regressed`.
6. Otherwise → `Matched`.

**Baseline selection** (skip deloads, template matching) is **not** in this PR. Document on `VerdictInput` that the caller must pass a non-deload baseline.

### 5.6 Amendment you must make to DD-13: unit switches
Found while writing these examples: 100 kg and 220.46 lb differ by about 1 g. With exact comparison, switching units would produce a false `E1rm Down` and `Regressed` verdict (example J). Micro-plates (+0.25 kg on 100 kg = 0.25%) are real progress, so a blanket tolerance would also be wrong.

**Rule:**
- Two loads entered in the **same unit** compare exactly.
- Loads entered in **different units** count as equal when they're within 0.5% (exact integer check: `|a − b|·200 ≤ max(a, b)`).
- The same rule applies to e1RM and volume deltas when the two sides' best sets (or any counted sets) differ in unit.

Update the DD-13 "Cross-unit same load" bullet in `docs/design.md` to say this, in its own `docs(design):` commit.

## 6. Golden examples: the test oracle (DD-09)

All examples use `WeightReps` and kg unless noted. "×" means reps. Expected e1RM is shown in kg, rounded to 2 decimals for display. Volume is in kg·reps. Write these as hand-written assertions on the full `ExerciseVerdict` (verdict, signals in order, skipped), **not** as auto-accepted snapshots.

| # | Baseline | Current | Expected signals, in order | Verdict |
|---|---|---|---|---|
| A | 100×8, 100×8, 100×7 | 100×8, 100×9, 100×7 | E1rm Up 126.67→130.00 · RepsAtLoad Up set 1 (8→9) · VolumeLoad Up 2300→2400 | Progressed |
| B | 100×8 ×3 sets | 102.5×6 ×3 sets, scheme 6–8 / 2.5 kg | E1rm Down 126.67→123.00 · LoadStep 100→102.5 @6 · VolumeLoad Down 2400→1845 | **Progressed** |
| B′ | same as B | same as B, **no scheme** | E1rm Down · VolumeLoad Down | Regressed |
| C | 100×8 @RPE 9 | 100×8 @RPE 8 | E1rm Up 130.00→133.33 (RPE-adjusted) · Effort Up set 0 (9→8) | Progressed |
| D | 100×8 ×3 | 100×8 ×3 | none | Matched |
| E | 100×8 ×3 | 100×8, 100×7, 100×6 | RepsAtLoad Down set 1 (8→7) · RepsAtLoad Down set 2 (8→6) · VolumeLoad Down 2400→2100 | Regressed |
| F | none | 100×8 | none | InsufficientData |
| G | 100×8 ×3 | 80×8 ×3, session is a deload | E1rm Down 126.67→101.33 · LoadAtReps Down ×3 · VolumeLoad Down 2400→1920 | Deload |
| H | 100×8 ×3 | 100×9, 100×8, 100×6 | E1rm Up 126.67→130.00 · RepsAtLoad Up set 0 · RepsAtLoad Down set 2 · VolumeLoad Down 2400→2300 | Progressed |
| I | 100×8 ×3 | 100×8 ×4 | WorkingSets Up 3→4 · VolumeLoad Up 2400→3200 | Progressed |
| J | 100 kg×8 | 220.46 lb×8 | none (within 0.5%, different units) | Matched |
| K | 60×15, 60×15 | 60×16, 60×15 | RepsAtLoad Up set 0 · VolumeLoad Up 1800→1860; skipped: E1rmAboveRepCap | Progressed |
| L | TopSet 100×5, Backoff 90×8 | TopSet 102.5×5, Backoff 90×8 | E1rm Up 116.67→119.58 · LoadAtReps Up set 0 (100→102.5 @5) · VolumeLoad Up 1220→1232.5 | Progressed |
| M | 100×8 | a `Duration` exercise | — | `Err(UnsupportedMode(Duration))` |

**Also test:**
- warm-ups are ignored, e.g. adding `Warmup 60×10` to both sides of A changes nothing;
- `Drop` sets count toward volume but are never paired;
- `is_progressive_overload()` is true only for `Progressed`;
- `SessionVerdict` counts.

**Property tests** (`proptest`):
- the e1RM comparison is antisymmetric and transitive;
- the integer comparison agrees with the f64 display values whenever they differ by more than 1e-9;
- adding a rep at the same load never lowers the e1RM.

## 7. Engineering rules (non-negotiable)

- **Docs style:** `/** */` and `/*! */` blocks, as in the existing code.
- **Lints:** the workspace lints must pass with `-D warnings`. No `unwrap`, `expect` or `panic` outside tests. No `_ =>` arm on domain enums. Any suppression is `#[expect(lint, reason = "...")]`.
- **Purity:** no serde, no clock, no `HashMap` (use `BTreeMap`; the determinism rule in `clippy.toml`), no floats in comparisons.
- **Dependencies:** add `proptest` as a workspace dev-dependency, in its own `build(domain):` commit. Nothing else.
- **Commits** (DD-10): one logical change each, about 200 lines or fewer. Use Conventional Commits with **no space before the colon**. Every commit builds and passes tests (`git rebase -x "cargo test"`).
- **Suggested commit sequence:**
  1. `build(domain): add proptest dev-dependency`
  2. `feat(domain): add progression scheme and rep range`
  3. `feat(domain): add exact epley e1rm with rpe adjustment`
  4. `feat(domain): add set pairing for overload comparison`
  5. `feat(domain): add overload signals for weight-reps sets`
  6. `feat(domain): add exercise and session verdicts`
  7. `test(domain): add golden overload examples a–m`
  8. `docs(design): define cross-unit tolerance in dd-13`

## 8. Definition of done

- [ ] Examples A–M and the extra cases in §6 pass as golden tests
- [ ] Property tests pass
- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS=-D warnings cargo doc`, `cargo deny check` and the wasm32 check all pass
- [ ] Every public item is documented, including `# Errors` sections
- [ ] DD-13 is amended (§5.6)
- [ ] The PR description lists the signals, the verdict rules and the unit-switch decision, and links DD-13
- [ ] No changes outside `crates/gymos-domain`, `Cargo.toml`/`Cargo.lock` (proptest) and `docs/design.md`

## 9. After this PR

1. **Engine ports + store + walking skeleton.** `Store`/`ReadTx`/`WriteTx`, `Clock` and `IdGen` ports, a `LogSet` command, the `last_time` baseline query (skips deloads), a first SQLite migration, and `gymos-testkit` fakes.
2. **The PR/records engine.** Record detectors that reuse `progression::e1rm`.
3. **The Tauri shell.**

## 10. Housekeeping noticed (not part of this PR)

- The remote still has retired branches (`dev`, `e0/*`, `rust-setup`). Delete them per DD-12 once confirmed unused.
- Recent commits like `lint : …`, `added TopSet…` and `Updated design.md…` don't follow Conventional Commits. The CI commit check will reject them on future PRs.
- The owner's local checkout has empty scaffold folders (`crates/*/src/{commands,ports,queries,rows,…}`). Git doesn't track empty folders, so they aren't in the repo. Create folders as modules need them; don't rely on them existing.
