# GymOs: Product Specification

**Status:** Accepted · **Source:** `docs/HANDOFF.md` v1.5 (2026-11-02), §1 and §3, carried over unchanged. Section numbers are kept so existing `§` references still resolve; §2 (prior context) is omitted.
**Scope:** this document defines **what** GymOs does. [`design.md`](design.md) defines **how** it is built and wins wherever the two disagree on technology.

> **Reading note.** This text was written before the Rust + Tauri pivot. A few phrases refer to the old web stack: "installable PWA" (now a Tauri desktop app), "phone-first" logging (phone ships last, per `design.md` §1), epic numbers (E5, E6, E14) and milestones (M1, M2). The product intent behind them still holds. Milestone dates get re-planned after the scaffold.
>
> The full handoff, including its superseded stack and work-plan sections, is in git history: `git show 368e165~1:docs/HANDOFF.md`.

---

## 1. Mission

### 1.1 What GymOs is
**GymOs** is a **local-first, open-source lifting analytics platform**. It is a smart lifting diary that makes progressive overload visible and actionable, with a trading-platform-style dashboard. It runs on your own device, it's free, and it shows your whole history.

**Pitch:** "Offline-first lifting analytics. Your data lives on your devices, syncs between them if you want, and nobody paywalls your own history."

**Why it exists (the owner's motivation):**
- Strava's free tier shows only the past 12 weeks of your own running history. Everything older sits behind a subscription, even though the data is already on Strava's servers.
- Garmin Connect has the full history, but nobody uses it to interact.
- Analytics are scattered across apps.

GymOs puts a person's training analytics in one place, on their own machine, with no gates.

**The name:** GymOs is the operating layer for your training data: a local data store, engines that run on it, an open format and pluggable importers.

It is also a **portfolio piece for FAANG-level applications**. A deployed, working, well-engineered and well-tested system matters more than feature count. **The two engineering showcases are local-first sync and the deterministic progression engine.**

### 1.2 Principles (non-negotiable; treat them as constraints)
1. **Local-first.** The device is the source of truth. Every feature works offline and without an account. The network is an enhancement.
2. **No paywalled history.** Every chart can cover any range, back to the first record. No feature is limited by time range.
3. **No required account.** An account exists only to sync between your own devices.
4. **Your data, in an open format.** Full export at any time, in a documented and versioned format, with full round-trip import.
5. **No surveillance.** No ads, no third-party trackers, no telemetry by default. Any diagnostics are opt-in and documented.
6. **No dark patterns.** No streak guilt, no engagement bait, no upsells.
7. **Every number explains itself.** Verdicts, PRs, trends and scores show the rule and inputs behind them. They are deterministic and versioned, and no LLM decides them.
8. **Self-hostable.** The sync server runs as one container that anyone can host. The public instance is a convenience, not a dependency.

**"Decentralized" here means user sovereignty, not peer-to-peer.** Your data is on your device; sync is optional and goes through a server you choose (the public one, your own, or none); and the format is open. True P2P sync (WebRTC with no server) is deferred to E14. It needs both devices online at the same time plus NAT traversal, and it adds little for the user.

### 1.3 The core loop (the product; everything else supports it)

```
 Start workout ──► see LAST TIME (same exercise / same workout) + target to beat
      │
      ▼
 Log sets (fast, phone-first, offline)  ──►  live feedback: "matched", "beat it", "new 5RM 🏆"
      │
      ▼
 Finish ──► session verdict (what progressed, what stalled, why)
      │
      ▼
 Dashboard ──► progress RATE per lift, PR board, full history (any range)
```

**Feature priority:**
1. Strength workout logging with **last-time comparison** (M1).
2. **Progression engine**: per-set, per-exercise and per-session overload verdicts, progress rate and acceleration, plateau detection (M1).
3. **PR and records engine** for every effort type (M1).
4. Analytics dashboard: trading-style charts of e1RM, volume, PRs and progress rate, over **any range** (M1).
5. Body-weight tracking; the manual activity form; manual measurements (VO2 max, resting HR) (M1).
6. **Durable local data:** a persistent local store, an installable PWA, and archive export/import as backup and device transfer (M1).
7. **Multi-device sync:** an optional account and a self-hostable server (M2).
8. File imports: **the Garmin Connect activities CSV first** (the "your whole history" demo), then Strong/Hevy CSV and the Garmin VO2 max report.
9. Open-source release: user and developer docs, a contribution guide, a self-host guide and published packages.
10. Reference percentiles.

## 3. Product core: specification

E5 and E6 implement this section. **Every rule is deterministic, versioned and explainable, and every engine runs locally.**

### 3.1 Exercise tracking modes
Each catalog exercise has a tracking mode. **The schema, the PR types and the overload rules all depend on it** (an E1 decision).

| Mode | Fields per set | Examples |
|---|---|---|
| `weight_reps` | load, reps | Bench press, squat, curl |
| `bodyweight_reps` | reps (body weight snapshot) | Push-up, pull-up |
| `weighted_bodyweight` | added load, reps | Weighted pull-up, dip |
| `assisted_bodyweight` | assist load (negative), reps | Assisted pull-up |
| `duration` | seconds (optional load) | Plank, dead hang |
| `distance_load` | distance, load (optional time) | Farmer's carry, sled push |
| `reps_only` / `time_reps` | reps, or reps in time | Burpees, kettlebell swings (EMOM) |

Every set also carries a **set type** (`warmup`, `working`, `drop`, `failure`, `backoff`), plus optional RPE or RIR, tempo and notes. **Warm-up sets are excluded from verdicts and PRs by default.**

### 3.2 "Last time": comparable-context resolution
- **Same exercise:** the most recent session containing exercise X. Its working sets appear inline next to each set row as the user logs.
- **Same workout type:** if the session started from a **template or routine** (for example "Push A"), "last time" means the last session from that template. Otherwise the app matches on exercise-set similarity (Jaccard ≥ a threshold), and the user can pick a different comparison session.
- Each exercise shows:
  - **last time:** sets × reps @ load, plus RPE;
  - **best ever:** the relevant PRs;
  - **target to beat:** a deterministic suggestion from the progression scheme **the user chooses** for each exercise or template (double progression, linear, RPE-based, or none). New exercises default to double progression. Rep ranges and load increments are user input. Suggested defaults: compounds 5–8 reps, accessories 8–12, isolation 10–15, with the smallest available plate or dumbbell step as the increment.
- **Live set feedback while logging:** `beat`, `matched` or `below`, compared with the corresponding set last time, plus an instant 🏆 when a set is a new PR. All of it is computed locally.

### 3.3 Overload verdicts
Verdicts are computed **per exercise per session** against the comparable previous session, then rolled up to the session.

**Progress signals.** Any one of these counts. They are listed from strongest to weakest:
1. A higher **e1RM** from the best set (Epley or Brzycki, valid up to 10–12 reps; RPE-adjusted through an RIR table when RPE is logged).
2. **More reps at the same load**, or **more load at the same reps**.
3. **More working sets** at the same load × reps.
4. **Higher volume load** (Σ load × reps over working sets) without the e1RM dropping.
5. **The same work at a lower RPE.**
6. For the other modes: a longer duration, more reps, less assistance, or more distance × load.

- **Verdict:** `progressed` | `matched` | `regressed` | `insufficient_data` | `deload`. A deload is never counted as a regression. **Deloads are marked manually.** The engine *suggests* marking one when a session shows a broad, deliberate drop across lifts (with versioned thresholds), and the user confirms.
- **Owner's requirement: show every way overload happened.** The UI lists **every** signal that fired, for each exercise and for the session. For example: "volume +8%", "+1 rep on set 2", "same load at RPE 7 vs 8". The verdict label is a summary on top of that list.
- **Output:** the verdict, **all** the rules that fired, the before and after numbers, and the rule version. For example: *"Progressed: +1 rep at 100 kg on set 2 (8→9); e1RM 124.0→126.7 kg."*
- **Session roll-up:** for example, "4 of 6 exercises progressed, 1 matched, 1 regressed (Incline DB press: −2 reps)."

### 3.4 Progress rate: "Am I getting stronger faster?"
- **Strength trend per exercise:** the best e1RM per session over time, fitted with a **Theil–Sen slope** over a rolling window (default: 6 weeks, or at least 4 sessions). Reported as %/week and kg/week.
- **Acceleration:** this window's slope compared with the previous window's, labelled `accelerating` | `steady` | `slowing` | `plateau` | `declining`. The thresholds are versioned config.
- **Plateau detection:** no e1RM or rep PR in N sessions (default 4) **and** a slope inside a ±band around zero. The resulting insights are deterministic, for example "change the rep range" or "deload".
- **Relative strength:** e1RM ÷ body weight.
- **Aggregates:**
  - a strength index (the weighted mean of %/week across the user's main lifts);
  - weekly volume per muscle group;
  - training frequency per lift.
- All of these must handle sparse, irregular data. Show confidence (the number of sessions) and never draw a trend below the minimum sample.

### 3.5 PRs and records: every effort type
PRs are **derived data**. They are computed locally, materialized, keyed by `algorithm_version` and **fully recomputable**. **They are never synced and never exported.** Editing or deleting a session updates or revokes the affected PRs. Each PR stores the set or session that achieved it, the date, the value and the previous record.

| Mode | PR types |
|---|---|
| `weight_reps` | **Actual rep maxes** 1RM–20RM (heaviest load for ≥ r reps) · **e1RM** · max reps at a load · best set volume · best session volume for the exercise · heaviest load ever |
| `bodyweight_reps` | Max reps in a set · max total reps in a session |
| `weighted_bodyweight` | Rep maxes by added load · e1RM including body weight |
| `assisted_bodyweight` | Least assistance for ≥ r reps |
| `duration` | Longest hold (at a load, if loaded) |
| `distance_load` | Longest distance at a load · heaviest load over a distance |
| **Cardio / activities** | Fastest time for standard distances (1 mi, 5 km, 10 km, half, full) **at the whole-activity level only** (split-based PRs need streams, which are E14 work) · longest distance · longest duration · biggest week or month |
| **Session / overall** | Most total volume in a session · longest training streak (weeks) · most sessions in a month |

- **PR board:** the current records per exercise, the PR timeline, and recent PRs on the dashboard.
- **PR notifications** in the logging flow are immediate and **local**. The device is authoritative, so no server confirms them.

### 3.6 Manual activity form and measurements
- **Activity fields:** type (run, ride, swim, walk, hike, row, MMA/boxing, climbing, yoga, other), title, start date/time and time zone, elapsed and moving duration, distance, elevation gain, average and max HR, calories, perceived effort (1–10), notes. **Which fields show depends on the type.**
- **Measurements:** body weight (quick entry from the dashboard and workout screen, with a trend chart), VO2 max (entered manually from a watch reading), resting HR and others, all stored on the Measurement entity. Bodyweight exercises use the most recent body weight on or before the session date.
- The VO2 max chart and cardio PRs work with **no file import at all**. Imports fill the same entities later.


---

## 4. Product decisions still in force

These decisions from the handoff are about the product, not the stack, so they still stand. IDs are kept so older references still resolve.

| ID | Decision |
|---|---|
| D1 | No Garmin developer dependency: no Connect API, no Garmin FIT SDK, no unofficial libraries. |
| D2 | FIT is a possible future import format only. |
| D5 | Lifting analytics is the core. Endurance is supported at activity-summary level. |
| D7 | No accounts in the local app: one local profile per device. Accounts exist only for sync. No social features. |
| D8 | Percentiles come from published reference data only, each labelled with its reference population, and only once redistribution terms are clear. |
| D9 | All verdicts, targets, plateau calls and PRs are deterministic rules. An LLM may later *phrase* insights, never decide them. |
| D10 | Loads are stored as entered value + entered unit, plus normalized kg. Display lbs by default, with a kg toggle and a per-exercise override. |
| D12 | Garmin Connect CSV exports (activities and the VO2 max report) are a supported import format. |
| D13 | Manual entry is the primary input for all activity types. File imports are accelerators. |
| D14 | The progression engine and the PR engine ship in M1. |
