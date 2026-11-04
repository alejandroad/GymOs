# GymOs: Handoff for the Next Agent

> **Note (2026-11-03):** the product sections below (§1–3) still stand. The stack, hosting, and data-model rules 13 and 15 in §4–6, and the work plan in §7, are superseded by `docs/design.md` (the Rust + Tauri 2 + React pivot).

**Version:** 1.5 · **Date:** 2026-11-02 · **Supersedes:** v1.0–v1.4 · **To:** the architecture and implementation agent
**Owner:** Alejandro, solo developer. He is a professional TypeScript engineer and builds web trading charts at his day job.
**Repo:** `GymOs` (local: `C:\Users\Admin\Desktop\GymOs\GymOs`). The repo is **public and open source**. It is currently empty apart from a placeholder `README.md` and `.git`.

> **Changes in v1.5: the local-first pivot (owner decision, 2026-11-02)**
> - **Local-first is the product and the main pitch.** Data lives on the user's device. The app works fully offline with **no account**. Sync between a user's own devices is an optional layer that runs through a server the user can self-host. (D18 replaces D4.)
> - **Open source** under MIT, to be confirmed at E0-T1. GymOs is a **platform**: an open schema, an open archive format, published engine packages, a documented importer contract and a self-hostable sync server. (D19, D21)
> - **"BS-free analytics" is a product principle** with concrete rules (§1.2): no paywalled history, no ads, no telemetry by default, no required account, and export at any time. (D20)
> - **A new sync epic (E9)** gets its own milestone, **M2**. The **archive export/import moves into M1**, because in a local-only app the archive *is* the backup. Percentiles move later.
> - **Accounts, Neon and Better Auth move out of M1 and into the sync server.** M1 has no backend at all. The public demo seeds a database inside the visitor's browser.
> - **Prompt 0 (v1.4) is stale** for T3 and T4 (Neon and auth). The E0 tickets are redefined in §7. **Reissue Prompt 0 as v2 before the first coding session.** If T1 or T2 already ran, keep them.
> - The owner accepts that this adds time. The March 2027 date no longer limits scope (§7).
> - Social features stay out of scope. Posting and feeds are deferred; local share cards are a later option.
>
> Changes from v1.2 to v1.4 are folded into the body. The decision table (§4) records which version decided what.

> Read the whole file before you act. §1–3 are the product. §4–6 are the decisions, facts and rules. §7 is the work plan. §8 says what to do first.

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

## 2. Prior context
- An earlier attempt, **AnabolicWorkoutTracker**, exists in a separate repo. It used Next.js, Prisma, Auth.js v5, Neon and Recharts, and had a `PersonalRecord` model with rep-range record types. **Mine it for ideas only. No code is ported (D16).**
- The research proposal v0.1 and its staff review are in the claude.ai GymOs project. The review recommended level-1 data ownership with no sync engine, to protect scope. **The owner overruled that in v1.5** and accepted the time cost. This handoff supersedes both documents.

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

## 4. Decisions

**Status key:** ✅ decided by the owner · ⛔ ruled out. Each decision notes the version that made it.

| # | Decision | Status | Rationale (short) |
|---|---|---|---|
| **D18** | **Local-first is the core.** An on-device database is the source of truth. All reads come from it. All writes are commands applied locally first. The network is needed only for sync and account management. **Replaces D4.** | ✅ v1.5 | The owner's main pitch. It is what makes GymOs free, private and decentralized, and it is the strongest engineering showcase. |
| **D19** | **Open source, MIT** (the owner confirms the license at E0-T1). Bundled data assets keep their own licenses in separate files. | ✅ v1.5 | The owner wants people to be able to "do what they need" with it, which calls for a permissive license. |
| **D20** | **BS-free principles** (§1.2) are product constraints. | ✅ v1.5 | The owner's definition of the product. |
| **D21** | **GymOs as a platform:** published `@gymos/*` packages (schema, progression, records, importers, archive, sync-protocol), the archive format published as JSON Schema, a documented importer contract and a self-hostable sync server. A runtime for third-party plugin code is deferred because of security. | ✅ v1.5 | The name and the open-source goal. |
| **D22** | **Sync design:** HTTP push/pull over a change log, hybrid logical clocks (HLC), field-level last-writer-wins, tombstones where delete wins, and derived data never synced. The server stores entity rows as **opaque, schema-agnostic JSON**, so domain schema changes need no server migration and end-to-end encryption can be added later. Hand-built, not a third-party sync engine. Details are finalized in R0.6 and an ADR. | ✅ v1.5 | One user and a few devices keeps conflicts rare and the scope small. A sync layer you designed and can explain is the showcase. |
| **D23** | **Local store:** SQLite compiled to WASM, running in a Web Worker and persisted to OPFS, with Drizzle as the query layer on the client (SQLite) and the server (Postgres). **IndexedDB (Dexie) is the fallback** if the spike fails on iOS Safari. | ✅ v1.5, pending the spike | Real SQL, migrations and indexes in the browser, with the same query layer on both sides. |
| D5 | **Lifting analytics is the core.** Endurance is supported at activity-summary level. | ✅ v1.2 | The owner's product definition. |
| D13 | **Manual entry is the primary input for all activity types.** File imports are accelerators. | ✅ v1.2 | No dependency on files to start building. |
| D14 | The progression engine and the PR engine ship in M1. | ✅ v1.2 | They are the product. |
| D1 | **No Garmin developer dependency:** no Connect API, no Garmin FIT SDK, no unofficial libraries. | ✅ | The platform is volatile, and none of it is needed. |
| D12 | Garmin Connect CSV exports are a supported import format (activities and the VO2 max report). | ✅ | It is the user's own data, with no API dependency. |
| D2 | FIT is a possible future import format only (`fit-file-parser`, MIT). | ✅ | Vendor-neutral. |
| D9 | All verdicts, targets, plateau calls and PRs are deterministic rules. An LLM may later *phrase* insights, never decide them. | ✅ | Testable and explainable. |
| D3 | Three layers, all on the device: raw artifacts (OPFS, keyed by SHA-256), canonical entities (SQLite, with provenance), and derived data (SQLite; versioned, recomputable, **never synced or exported**). | ✅ v1.5 | Edits and deletes stay consistent. Rule changes replay locally. |
| D6 | **Next.js 16** stays the framework. In v1.5, `apps/web` is a **client-rendered PWA built with static export** (`output: 'export'`): no server data loading, Server Actions or route handlers in `apps/web`. **Agent note:** this uses little of Next.js, and a Vite SPA would fit equally well. The owner's choice stands. Revisit only if static export, the PWA service worker or the WASM worker hit blocking problems in the spike, and report the evidence. | ✅ v1.3, scope v1.5 | The owner's choice. Static export keeps the web app hostable anywhere, including fully locally. |
| D7 | **No accounts in the local app.** There is one local profile per device and browser. The **sync server is multi-tenant** by account. No social features. | ✅ v1.5 | Accounts exist only for sync. |
| D8 | Percentiles come from published reference data only, each labelled with its reference population. The data is fetched or bundled as static files, but only once redistribution terms are clear. Pooled cohorts are deferred. | ✅ | Defensible, and safe for an open-source repo. |
| D10 | Loads are stored as entered value + entered unit, plus normalized kg. **Display lbs by default**, with a kg toggle and a per-exercise override. | ✅ v1.3 | No rounding drift, and PR ties stay exact. |
| D11 | **Stack:** Next.js 16 (static PWA), React 19, TS strict, Tailwind 4, shadcn/ui, **SQLite-WASM + OPFS** in a worker, Drizzle, Zod, Vitest + fast-check, Playwright, a PWA service worker (Serwist or whatever the Next 16 docs recommend), TradingView Lightweight Charts, ECharts where needed, Papa Parse. **Sync server:** Hono on Node, Better Auth, Drizzle + Postgres (Neon for the public instance), Docker. | ✅ v1.5 | Neon and Better Auth are kept, but moved to the sync server. Pin versions at scaffold time. |
| D15 | **Hosting:** `apps/web` is a static deploy on **Vercel Hobby**; any static host, or `localhost`, also works. `apps/sync` runs as Vercel Functions + Neon for the public instance (HTTP push/pull fits serverless; no WebSockets) and as **Docker Compose (sync + Postgres)** for self-hosters. Portability (rule 15) applies to both. R2 is deferred because raw files stay local. | ✅ v1.5 | Free to run, with no host lock-in. |
| D16 | Fresh start; no code from AnabolicWorkoutTracker. | ✅ v1.3 | The owner's call. |
| D17 | Public GitHub repo; the employer IP policy is cleared. | ✅ v1.3 | Confirmed. |
| ⛔ | TanStack Start · Garmin API/SDK and unofficial Garmin libraries · the Strava API · social feeds and posting in v1 · P2P sync in v1 · a third-party sync engine as the core (Zero, Electric, PowerSync, InstantDB…) · Vercel-only services · telemetry by default · LLM-decided verdicts · hosted-first architecture (v1.4) | ⛔ | Owner rejections, fragile, against the principles, or out of scope. |

## 5. Facts and constraints

### 5.1 Garmin Connect CSV formats (from the owner's real exports; needed in E10)

**A. VO2 max report CSV**
```
VO₂ Max,
,VO₂ Max
Dec 2025,VO₂ Max,51.5
May 2026,VO₂ Max,51.6
...
Nov 2026,VO₂ Max,53.7
```
- The rows are ragged: a title line, a two-column header, then three-column data rows. Parse by row shape.
- Monthly values only, with gaps. Store as `vo2max / garmin_monthly / granularity=month` and plot as steps.
- Unicode `₂`, English month abbreviations, implied unit ml/kg/min.
- Open question: whether shorter report ranges export finer granularity.

**B. Activities list CSV**
- 36 columns: `Activity Type, Date, Favorite, Title, Distance, Calories, Time, Avg HR, Max HR, Aerobic TE, Avg Run Cadence, Max Run Cadence, Avg Pace, Best Pace, Total Ascent, Total Descent, Avg Stride Length, Avg Vertical Ratio, Avg Vertical Oscillation, Avg Ground Contact Time, Avg GAP, Normalized Power® (NP®), Training Stress Score®, Avg Power, Max Power, Steps, Total Reps, Total Sets, Body Battery Drain, Decompression, Best Lap Time, Number of Laps, Moving Time, Elapsed Time, Min Elevation, Max Elevation`
- One row per activity, summary only. **That is enough for weekly and monthly distance and time charts over the entire history**, which is the direct answer to Strava's 12-week limit.

| Issue | Example | Rule |
|---|---|---|
| **No unit metadata; values use the user's Garmin display units** | `1.90` with `19:10` and pace `10:04` means miles and min/mi; elevation is in feet | **The user declares the unit system at import.** Run plausibility warnings. |
| Mixed units even in imperial [M] | Stride `1.04` is m; vertical oscillation `9.6` is cm; ground contact time is ms | Per-column unit maps. Confirm with a metric vs statute export pair. |
| Local time with no offset | `2026-11-01 14:16:14` | Use the profile time zone and record the assumption. |
| Quoted numbers with thousands separators | `"2,519"` | Strip the separators. |
| Null marker | `--` | Treat as null. |
| Leading apostrophe | `'-5` | Strip one leading `'` (Garmin's spreadsheet-injection guard). |
| Booleans | `false`, `No` | Normalize both. |
| Durations | `00:08:58.1` | Parse `HH:MM:SS(.f)` into seconds. |
| Glitches | Max Power `2,519` W on an easy run | Flag via plausibility ranges. Keep the row. |
| Placeholders | TSS `0.0` everywhere | Treat as unknown. |
| **Strength reps are untrustworthy** | Steps is exactly 2 × Total Reps in every strength row | Display only. **Never used for verdicts or PRs.** |
| Possibly partial export [M, unverified] | Exactly 20 rows | Tell users to load the full list first. Merge repeat exports. |

- **Dedup key:** `(activity_type, local start datetime)` with a duration tolerance.
- **Enrichment:** a Garmin strength row and a GymOs-logged workout in the same time window are **linked**: HR, calories and duration come from Garmin; sets, reps and load from GymOs.
- **Drift-proofing:** map columns by header name, tolerate unknown or missing columns, use versioned importer profiles, keep the raw file, and test against golden fixtures.

### 5.2 Local-first and sync facts (verify date-sensitive items in the spike)

| Topic | Fact | Confidence |
|---|---|---|
| SQLite in the browser | The official SQLite WASM build persists to OPFS from a Worker. The plain `opfs` VFS needs COOP/COEP headers (SharedArrayBuffer). The **`opfs-sahpool`** VFS doesn't, and is the likely choice. Alternatives: wa-sqlite. | Medium |
| iOS/Safari storage eviction | WebKit may evict script-writable storage for sites the user hasn't interacted with in about 7 days, **unless the web app is installed to the Home Screen**. The behavior has changed across iOS versions. **This is the top data-loss risk for a local-first web app.** Mitigations: a PWA install prompt, `navigator.storage.persist()`, archive backups and sync. | Medium, verify |
| Storage quota | Varies by browser. Request persistence, and show usage and persistence state in settings. | High |
| Multiple tabs | Only one writer may touch the SQLite database. Use a single worker owner (Web Locks leader election or a SharedWorker), and have other tabs proxy to it. | Medium, spike |
| Hybrid logical clocks | Kulkarni et al. (2014): physical time plus a logical counter, which gives monotonic, causally consistent ordering despite clock skew between devices. | High |
| Field-level LWW | Simple and convergent. It drops one of two concurrent edits to the *same field*, which is acceptable for a single-user, few-device diary. Record dropped writes in a local conflict log. | High |
| Convergence testing | Deterministic simulation testing: seeded random clients, operations, reorderings, duplicates and partitions; assert that all replicas converge. Failures reproduce from the seed. | High (technique) |
| Vercel and WebSockets | Vercel Functions don't hold long-lived WebSocket connections, so use **HTTP push/pull**, triggered on focus, on reconnect and on a timer. | Medium |
| Hono | Runs on Node, Bun, Vercel, Cloudflare and Deno with standard Web APIs, which keeps the sync server portable. | High |

### 5.3 Other facts

| Topic | Fact | Confidence |
|---|---|---|
| e1RM formulas | Epley `w(1+r/30)`; Brzycki `w·36/(37−r)`. Both lose accuracy beyond 10–12 reps, so cap them or flag low confidence. RPE-adjusted e1RM uses an RTS-style RPE/RIR table. | High |
| Progress-rate statistics | Theil–Sen is robust to outlier sessions and suits small, irregular samples. Report n and the window. | High |
| Novice vs advanced rates | Expected progress rates fall sharply with training age. Don't label a slow rate "bad" without context. | High (qualitative) |
| Next.js 16 | Static export (`output: 'export'`) has no server runtime: no route handlers or Server Actions. Verify PWA service-worker integration and Web Worker + WASM bundling against the current Next 16 docs. | Medium, verify |
| Neon (sync server) | `@neondatabase/serverless` with Drizzle's `neon-http` driver works in serverless. Self-hosters use standard Postgres. | Medium–High |
| Data licenses in an MIT repo | free-exercise-db (reported as public domain). wger data (reported CC-BY-SA; share-alike binds the data files, not the code, so keep a separate license file). OpenPowerlifting (public domain, but large, so derive compact tables at build time). WMA 2023 age-grading factors (terms unclear; **don't commit them until redistribution is clear**). FRIEND VO2 max norms (from published papers; cite them and verify reuse). | Medium / Low (WMA) |
| Lightweight Charts | Apache 2.0. The NOTICE requires TradingView attribution, which the default `attributionLogo` covers. | Medium |
| FIT (future) | Use `fit-file-parser` (MIT), not the Garmin SDK. FIT strength data is unreliable. | Medium–High |

**Hard constraints:**
- No Fidelity code, designs or proprietary knowledge.
- **Health data stays on the device unless the user turns on sync.** Erasing all data on a device must work. Deleting a sync account deletes all of that account's server data. The privacy page states plainly what leaves the device and when.
- Every chart has an accessible data-table fallback and keyboard support.
- The public demo always works: **"Try demo"** loads seeded data into a separate, clearly labelled local database that can be wiped, with no account needed.
- Real exports are scrubbed before they are committed as fixtures.
- Every dependency and bundled asset is license-compatible with an MIT repo. Record the licenses.

## 6. Data-model rules E1 must specify

1. **Tracking modes** (3.1) determine the set fields, PR types and overload rules.
2. **Loads:** entered value + entered unit + normalized kg (D10). Distances in metres, durations in seconds.
3. **Time:** UTC plus the original local offset. Aggregate by the user's local date and configured week start.
4. **Provenance:** every row carries `source_kind` (`manual_strength` | `manual_activity` | `manual_measurement` | `csv:strong` | `csv:hevy` | `csv:garmin_activities` | `csv:garmin_vo2max` …), plus `importer_version` and `source_artifact_id` where relevant.
5. **Derived data** (verdicts, PRs, trends, targets) is materialized locally with `algorithm_version`, recomputed incrementally for the affected exercise on every create, edit or delete (including changes arriving through sync), and is **never synced, never exported and never treated as source of truth**.
6. **Templates and routines** are first-class. A session records `template_id` when it was started from one.
7. **Corrections:** user edits to imported data are an overlay, so a re-import doesn't wipe them.
8. **Dedup vs enrichment:** a re-import from the same source is deduplicated; different sources describing the same session are linked (5.1).
9. **Raw artifacts:** stored in OPFS, keyed by SHA-256, with metadata in SQLite. Raw files are **not synced in M2** (deferred to E14).
10. **Measurements:** `metric`, `value`, `unit`, `source`, `method` and `granularity`.
11. **Exercise identity:** the canonical catalog ships with the app as versioned static data. Custom exercises and per-source mappings are user rows, and they sync.
12. **Importer contract:** versioned profiles with `detect`, `parse`, `map` and `validate`. Importers run in a Web Worker and **emit commands**, never direct table writes, so imported data flows through the same write path and syncs.
13. **Local-first core (D18, replaces "offline readiness"):**
    - The SQLite database in the worker is the source of truth. The UI reads through a **repository interface with reactive queries**, so views update whenever the data changes, including changes from sync.
    - **Every write is a command** (`LogSet`, `EditSet`, `DeleteSession`…) with a UUIDv7 command ID. A command is applied in one local transaction that updates rows **and** appends field-level changes to an **outbox**. Commands are idempotent: applying one twice equals applying it once.
    - **UUIDv7 IDs, generated by the client,** for every row. No database serials.
    - Every syncable row has `id`, `hlc_updated`, a per-field clock map, a `deleted` tombstone and `schema_version`.
    - **Client migrations** are versioned and forward-only. They run in the worker at startup, before the UI, and are **tested against fixture databases from every earlier schema version**.
    - **Durability:** request `navigator.storage.persist()`, prompt PWA install on iOS, show storage state in settings, and offer archive backup (one dismissible notice, never a nag).
    - Exactly one writer across tabs (5.2).
14. **Sync protocol (M2; D22):**
    - **Push:** the client sends outbox batches (field-level changes with HLC timestamps). The server applies them idempotently, assigns a monotonically increasing per-account `server_seq`, and acknowledges.
    - **Pull:** the client sends its last cursor. The server returns rows changed since then. The client merges them with **field-level LWW by HLC**, and **delete wins**.
    - **The server store is schema-agnostic:** per account, it keeps `(entity_type, id, fields JSON, field clocks, deleted, server_seq)`. The server never interprets field contents, which keeps end-to-end encryption possible later.
    - **Versioning:** the protocol carries a protocol version and the client's schema version. The server tells outdated clients to update before they sync.
    - Derived data and raw files are never synced.
    - Dropped concurrent writes go to a **local conflict log** that the user can see in settings (principle 7).
    - **First sign-in on a device with existing local data merges it into the account; it doesn't overwrite.** Signing out keeps the local data.
    - The sync server URL is a setting: the public instance by default, or a custom URL for self-hosters.
15. **Portability (D15):**
    - `apps/web` is a pure static build with no server dependency.
    - `apps/sync` uses only Hono and standard Web APIs.
    - Each app has its own Zod-validated `env` module.
    - Database access goes only through packages.
    - File storage goes through a `Storage` interface (OPFS locally).
    - No Vercel-only services or `@vercel/*` runtime SDKs in app code.
    - Changing hosts must not touch domain or feature code.

---

## 7. Work plan: 15 epics (E0–E14)

**v1.4 → v1.5 mapping:**
- E0–E7 keep their roles. E2 becomes the *local* platform (no auth).
- v1.4's E8 archive stays **E8 and moves into M1**.
- **E9 is the new sync and accounts epic (M2).**
- v1.4's E9 imports become **E10**.
- **E11 is the new open-source release and platform epic.**
- v1.4's E10 percentiles become **E12**, v1.4's E11 hardening becomes **E13**, and v1.4's E12 later items become **E14**.

**M1 (first public demo) = E0 → E8: the full core loop, local-first, installable, offline, no account, with backup and restore, deployed as a static site.**
**M2 = E9: multi-device sync, an optional account, a self-hostable server.**

### E0 · Spikes and method research *(about 1 week)*
| ID | Task | Output |
|---|---|---|
| R0.1 | **Local stack spike: tickets E0-T1 to E0-T6, run via Prompt 0 v2.** Next.js 16 static PWA, SQLite-WASM + OPFS in a worker, offline shell, CI and a static deploy. **The iOS Safari result is the go/no-go.** | `docs/research/stack-spike.md`, `docs/adr/0001-*.md` |
| R0.2 | **Progression-method research:** e1RM formulas and caps; the RPE/RIR table; double, linear and RPE progression; Theil–Sen windowing; plateau thresholds; how Strong and Hevy define PRs and "previous" values (a UX benchmark, not a copy). A rule spec with worked examples. | `docs/research/progression-engine.md` |
| R0.3 | Exercise catalog seed: licenses **for redistribution in an MIT repo**, muscle-mapping quality, and a tracking mode per exercise. | `docs/research/exercise-catalog.md` |
| R0.4 | Lightweight Charts spike: React wrapper, multi-series, synced crosshairs, step series, PR markers, keyboard and a11y, attribution. | `docs/research/charts.md` |
| R0.5 | UX benchmark of fast phone set logging, **plus offline/sync status, install prompt and backup UX.** Wireframes. | `docs/research/logging-ux.md` |
| R0.6 | **Sync research:** Replicache/Zero-style push/pull, CRDTs vs field LWW (Automerge, cr-sqlite), Linear's sync model, HLC, delete semantics, migration across versions, simulation testing. A **protocol spec with worked conflict scenarios**. Draft in week 2; final before E9. | `docs/research/sync-protocol.md` |

#### E0 tickets (in execution order)

**Way of working:**
- One short-lived branch per ticket (`e0/t1-repo-bootstrap`…), each a PR into `main`, rebase-merged.
- **Conventional Commits.** **Every commit builds and passes the checks.**
- Small, focused commits.

| Ticket | Title | Scope | Acceptance |
|---|---|---|---|
| **E0-T1** | Repo bootstrap | pnpm workspace, Turborepo, root scripts, Node LTS pin, `.editorconfig`, `.gitattributes` (LF), `.gitignore`, **LICENSE (MIT, owner confirms)**, README stub stating the local-first pitch, `docs/{adr,research,prompts}` with this handoff at `docs/HANDOFF.md`, ADR template | `pnpm install` works on Windows and Linux; the docs are committed |
| **E0-T2** | Next.js 16 static app | `apps/web` (App Router, TS strict, Tailwind 4, ESLint, `src/`), `output: 'export'`, shared `tsconfig.base.json`, placeholder landing page | `pnpm dev`, `build`, `lint` and `typecheck` pass; `build` emits a static `out/` that any static server can serve |
| **E0-T3** | Local database spike | `packages/schema` (`newId()` UUIDv7 with Vitest tests); `packages/store`: SQLite-WASM in a dedicated worker, persisted via OPFS (`opfs-sahpool` unless the docs say otherwise), Drizzle on SQLite (proxy or the driver the docs recommend), a migration runner with one demo table, a typed request/response bridge between worker and UI, a single writer across tabs (Web Locks), `navigator.storage.persist()`. A demo page writes and reads rows. | Rows survive reload and browser restart in desktop Chrome, Firefox and Safari, **and on an iPhone in Safari and as an installed PWA**. If iOS fails: document it, try the IndexedDB fallback, report |
| **E0-T4** | PWA offline shell | Web app manifest, placeholder icons, a service worker precaching the app shell and WASM (Serwist or the approach the Next 16 docs recommend) | With the network off, the app loads and the demo page reads and writes; installable on iOS and Android |
| **E0-T5** | CI | GitHub Actions: frozen install, lint, typecheck, test, build; pnpm and Turbo cache; README badge | Green on a PR |
| **E0-T6** | Deploy and spike write-up | Static deploy on Vercel (Git integration), production smoke test (offline reload, data persistence), measurements, `docs/research/stack-spike.md`, `docs/adr/0001-local-first-architecture-hosting-portability.md` | The production URL passes the T3 and T4 checks; the doc and ADR are merged |
| E0-T7 | Progression rule spec (R0.2) | `docs/research/progression-engine.md`, the test oracle for E6 | Owner review |
| E0-T8 | Exercise catalog (R0.3) | Licenses, seed choice, tracking-mode mapping | Doc merged |
| E0-T9 | Charts spike (R0.4) | A client component in the static app | Spike page and doc |
| E0-T10 | Logging and local-first UX (R0.5) | Benchmark plus wireframes | Doc and wireframes |
| E0-T11 | Sync protocol draft (R0.6) | Draft spec | Doc merged (final before E9) |

**Measurements to record in T6:** First Load JS per route; SQLite WASM size (raw and compressed); worker init time (cold and warm); typical query and insert latency; storage persistence result per browser; iOS results (Safari browser vs installed PWA); build time; the current Vercel Hobby limits relevant to a static site and to Functions (for the future sync server), with the source and the date checked.

**Prompt 0 v2 covers T1–T6. Prompt 1 covers T7. Prompt 2 covers T8–T10. T11 runs alongside E1.**

**Owner tasks (non-blocking):** export the Garmin Connect CSVs (the full activity history, the VO2 max report at every range, and one metric and one statute export of the same activities) and any Strong or Hevy history. These are needed for E10.

**DoD:** the local-store spike passed (or the fallback was chosen with evidence), and the progression rule spec and catalog choice are written.

### E1 · Architecture and ADRs *(about 1 week)*
- All decisions are confirmed. Record each one as a MADR in `docs/adr/`. Ask the owner only about genuinely new questions.
- **ADRs to write:**
  - local-first architecture (D18);
  - local store, worker model and migrations (D23);
  - the command/write model and outbox;
  - sync protocol boundaries (D22; the full spec follows R0.6);
  - sync server and accounts;
  - hosting and portability (D15, rule 15);
  - open-source license and data-asset licensing (D19);
  - platform packages and publishing (D21);
  - privacy and no telemetry;
  - canonical schema v1;
  - loads and units;
  - time;
  - provenance;
  - the derived-data strategy;
  - progression engine architecture;
  - the PR engine;
  - the importer contract;
  - the archive format;
  - charts and a11y;
  - percentile methodology;
  - no Garmin developer dependency;
  - deferred items.
- **Schema v1:** Zod as the source of truth, Drizzle SQLite tables, an ERD and a spec.
- **Monorepo:**
  - `apps/web`, and `apps/sync` (created in E9);
  - `packages/schema`, `packages/store`, `packages/progression`, `packages/records`, `packages/catalog`, `packages/importers`, `packages/archive`, `packages/metrics`, `packages/sync-protocol`.
  - Domain packages are pure and framework-free.
- **Performance budgets:**
  - logging interactions take under 100 ms;
  - a cold start to a usable offline app takes under 2 s on a mid-range phone (target; adjust from spike data).

**DoD:** the owner has approved the ADRs, and the schema spec is merged.

### E2 · Foundation and local platform *(about 1 week)*
- CI: typecheck, unit and property tests, **Playwright e2e including a mobile viewport and an offline mode** (`context.setOffline`), and a bundle-size budget. Vercel preview deploys for each PR. Cross-platform scripts.
- `packages/store` hardened:
  - a migration framework with **fixture-database upgrade tests**;
  - reactive queries;
  - the repository interface;
  - a command bus and outbox table;
  - a single writer across tabs;
  - error recovery (a corrupt or locked database shows a safe message, never a blank screen).
- PWA install and persistence UX. A settings page: storage usage and persistence state, **"Erase all data on this device"**, and an export placeholder.
- A privacy page ("your data stays on this device unless you turn on sync").
- **No auth.**

**DoD:** the offline-capable shell is deployed, the migration upgrade tests pass, and erase works.

### E3 · Canonical schema and data platform *(about 1 week)*
- `packages/schema`: sessions, exercises-in-session, sets (by tracking mode), templates, activities, measurements, provenance and **command definitions**.
- Load and unit utilities with property tests (exact round-trips).
- The derived-data framework: an `algorithm_version` registry and incremental recompute hooks on commands.
- **Seed-data generator:** deterministic, about 12 months of realistic lifting (progression, deloads, plateaus, missed weeks, a Push/Pull/Legs template, some runs and body-weight entries). It **runs in the browser into a separate demo database**. On first run the user chooses "Try demo" or "Start fresh".

**DoD:** "Try demo" fills a local demo database, and the schema tests pass.

### E4 · Exercise catalog *(about 1 week)*
- The catalog ships as versioned static data with license files. Each entry has an ID, tracking mode, muscles, equipment, movement pattern, aliases and a default increment.
- Search (fast, fuzzy, recent and favourite exercises first). Custom exercises.
- The alias matcher: exact → normalized → trigram → user confirmation → a remembered mapping.

**DoD:** a common lift can be found and added in 3 taps or keystrokes or fewer, and the matcher golden tests pass.

### E5 · Smart diary: logging and manual entry *(about 2 weeks, in tandem with E6)*
- Strength logging (3.1, 3.2): start empty or from a template; phone-first set rows; set types; RPE/RIR; rest timer; supersets; notes; edit and delete past sessions. **Everything writes locally, so an in-progress workout is just rows** and survives reloads and network loss by construction.
- The last-time panel, live set feedback and instant PR badges.
- Templates and routines.
- The manual activity form and measurements (3.6).
- History: calendar and list views, and a session detail page.

**DoD:** a full workout can be logged on a phone **in airplane mode** with less than about 2 minutes of interaction overhead. Last time and the targets are correct against golden scenarios. Playwright mobile and offline tests pass.

### E6 · Progression and records engines *(about 2 weeks, starting alongside E5)*
- `packages/progression`:
  - comparable-context resolution;
  - targets;
  - verdicts with explanations;
  - progress rate, acceleration and plateaus;
  - the strength index;
  - muscle-group volume.
- `packages/records`: every PR type in 3.5, with incremental update, full recompute, revocation on edit or delete, and a timeline.
- The engines run **in the worker**, triggered by commands. A backfill recomputes everything when `algorithm_version` changes.
- **Testing is the showcase:**
  - golden scenario suites;
  - property tests (full recompute equals incremental update; deleting a PR session leaves no orphaned PR);
  - the published rule spec.
- UI: the post-session verdict summary and verdict history.

**DoD:** 100% branch coverage on the rule modules, all suites pass, every verdict and PR shows its explanation, and edits and deletes keep the PRs correct.

### E7 · Analytics dashboard *(about 1.5 weeks)*
- The chart adapter layer, with Lightweight Charts for hero series.
- **Lift view:** e1RM with PR markers, a trend with %/week, an acceleration badge, rep-max curves, volume and verdict history.
- **Overview:** the strength index, a lift watchlist with sparklines, recent PRs, plateau alerts, weekly volume by muscle group, a frequency heatmap, body weight and VO2 max charts, and activity summaries.
- **The range selector always offers "All time".** No range is gated (principle 2).
- The PR board. Synced crosshairs. Dashboard state in the URL. A11y: table fallbacks, a keyboard crosshair, contrast in both themes.

**DoD:** the dashboard renders smoothly over the full seed range on a mid-range phone, and the a11y checks pass.

### E8 · Archive: backup, restore and device transfer *(about 1 week; now part of M1)*
- GymOs archive v1 (zip): `manifest.json` (schema version, counts, checksums), `sessions.ndjson`, `activities.ndjson`, `measurements.ndjson`, `templates.ndjson`, `custom_exercises.ndjson`, `mappings.ndjson`, `raw/`, and an optional CSV (escaped against formula injection).
- Derived data is **not** exported; it is recomputed on import.
- A published JSON Schema plus a spec. Before sync exists, this is **the backup and the way to move to a new device**.
- CI round-trip test: export → erase → import → deep-equal, with identical PRs and verdicts after the recompute.

**DoD (M1):** a stranger opens the public URL, taps "Try demo" and sees the full loop in under 60 seconds: last time → log → verdict → PR → progress rate. With airplane mode on, it keeps working. Data survives a reload. Export → erase → import round-trips. README with screenshots and an architecture diagram. Lighthouse and the performance budgets pass.

### E9 · Sync and accounts *(about 4 weeks) → **M2***
- Finalize R0.6 and the sync ADR.
- **`packages/sync-protocol`:** HLC, field-clock merge, change encoding, protocol versioning. Pure and property-tested.
- **`apps/sync`:**
  - Hono, Better Auth (email and password; passkeys later), Drizzle + Postgres (Neon for the public instance);
  - push and pull endpoints with idempotent apply;
  - per-account authorization and rate limits, and payload size limits;
  - account deletion wipes all server data;
  - a Zod `env` module and a health route;
  - a **Docker image plus `docker-compose.yml` (sync + Postgres)**.
- **Client sync engine (in the worker):**
  - outbox flush;
  - pull on focus, on the `online` event and on a timer, with backoff;
  - sync status UI;
  - the conflict log;
  - sign-in only from settings;
  - the server URL setting;
  - merging existing local data on first sign-in;
  - sign-out keeps local data.
- **Deterministic simulation test harness (the headline):**
  - N simulated clients issue random commands;
  - the harness injects random reordering, duplication and partitions, plus server restarts;
  - assert convergence, no lost tombstones and no resurrected deletes;
  - seeded and reproducible.
- **A Playwright two-context e2e** (two "devices" edit offline, reconnect and converge).

**DoD (M2):** a phone and a laptop each edit offline, reconnect and converge. The simulation suite runs thousands of seeded scenarios in CI within a time budget. The public instance is deployed. Self-hosting from the README works on a clean machine.

### E10 · File imports *(about 1.5 weeks)*
- Order: **`garmin-activities-csv@1` first**, then `garmin-vo2max-csv@1`, `strong-csv@1` and `hevy-csv@1` (every rule in 5.1).
- Flow: preview → declare the unit system → confirm exercise mappings → dedup and enrichment check → commit (as commands, so the data syncs) → recompute.
- **The demo story:** "Strava shows you 12 weeks. GymOs shows everything." Weekly distance over the entire imported history.

**DoD:** the owner's real exports import correctly, re-importing creates no duplicates, Garmin strength rows link to logged sessions, and imported Strong/Hevy history produces correct PRs.

### E11 · Open-source release and platform *(about 1 week)*
- User README and developer README; `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`; issue and PR templates.
- Self-hosting guide; architecture docs; the archive JSON Schema; an importer guide ("write your own importer").
- Publish the `@gymos/*` packages to npm (semver, changesets).
- A data-asset license audit. Tag **v1.0**.

**DoD:** someone outside the project can self-host, contribute or write an importer from the docs alone.

### E12 · Measurements and reference percentiles *(about 1 week)*
- The VO2 max chart (manual, Garmin monthly, and VDOT from race-flagged activities), each value labelled by method and granularity.
- Reference scores, each labelled with its population:
  - strength vs OpenPowerlifting via DOTS;
  - running age grade (WMA 2023, only if redistribution is clear);
  - VO2 max vs FRIEND.
- A methodology page.

**DoD:** every score links to its methodology, and the licensing is resolved.

### E13 · Hardening and portfolio polish *(continuous, plus focused passes after M1 and M2)*
- Performance budgets; axe a11y in CI; a security review (sync authz, replay and idempotency, payload limits, CSV injection, CSP, upload validation); an erase and account-deletion audit.
- **Write-ups:**
  - *Local-first sync, proven by deterministic simulation*;
  - *A deterministic progression engine*;
  - *Derived data that stays correct under edits and sync*;
  - *Importer framework and unit handling*.
- A demo video, an ADR index and an architecture diagram.

### E14 · Later and optional
- End-to-end encryption for sync (the server already stores opaque payloads).
- Raw-file (blob) sync.
- P2P sync (WebRTC) or folder-based sync (iCloud, Dropbox, Syncthing) with no server.
- Native shells: Tauri for desktop, Capacitor for mobile. Native storage would eliminate the iOS eviction risk.
- Apple Health and Health Connect (needs a native shell).
- FIT, GPX and TCX with streams (split-based cardio PRs).
- Local share cards (image export); public share links through the sync server.
- A runtime for third-party importer plugins.
- An LLM that phrases insights.
- Opt-in pooled cohorts (k ≥ 50).
- Multiple local profiles.

### Dependency summary
```
E0 ─► E1 ─► E2 ─► E3 ─┬─► E4 ─┐
                      │       ├─► E5 ◄─┐
                      └─► E6 ─┴────────┘ (in tandem: pure engines first)
                                 │
                       E5 + E6 ──► E7 ──► E8 ══► M1 (local-first core loop, deployed)
                                                  │
                                         E9 sync ══► M2
                                                  │
                         E10 imports · E11 OSS v1.0 · E12 percentiles
E13: continuous, with focused passes after M1 and M2
```

### Timeline (week 1 starts 2026-11-02)

| Weeks | Dates (approx.) | Work |
|---|---|---|
| 1 | Nov 2–8 | E0: Prompt 0 v2 (local stack spike), rule spec, catalog, charts, UX |
| 2 | Nov 9–15 | E1 ADRs and schema · R0.6 sync draft |
| 3–4 | Nov 16–29 | E2 local platform · E3 schema and seed · E4 catalog |
| 5–8 | Nov 30 – Dec 27 | E6 engines in tandem with E5 smart diary |
| 9–10 | Dec 28 – Jan 10 | E7 dashboard |
| 11 | Jan 11–17 | E8 archive → **M1 deployed (target ~Jan 17)** |
| 12–15 | Jan 18 – Feb 14 | E9 sync and accounts → **M2 (target ~Feb 14)** |
| 16–17 | Feb 15–28 | E10 imports (Garmin activities first) · E13 pass |
| 18–20 | Mar 1–21 | E11 open-source v1.0 · E12 percentiles |

- **Owner decision (v1.5):** the added time is acceptable, and the date no longer cuts scope. Google applications go out in March with M1 and M2 deployed; interview prep runs in parallel from February. **Agent: if M2 slips past Feb 28, tell the owner, because it overlaps interview prep.**
- **Slip policy.** Slip dates rather than cutting the core. If behind, defer in this order:
  1. E12 percentiles;
  2. the Strong, Hevy and VO2 max importers;
  3. a docs site;
  4. npm publishing (the repo stays open).
- **Never cut:** the core loop (E5–E7), the local store and its migration tests, archive backup, sync with its convergence tests, the Garmin activities import, the deployed demo, and the README and ADRs.

---

## 8. First step

1. **Reissue Prompt 0 as v2**, covering E0-T1 to T6 as defined in §7. **Don't run the v1.4 Prompt 0's T3 (Neon) or T4 (auth).** If T1 or T2 from v1.4 already ran, keep them and continue from the new T3.
2. **Agent, day 1:** run Prompt 0 v2. **The highest-risk item is T3 on an iPhone.** If SQLite-WASM/OPFS fails there, report back with evidence before anything else is built on it.
3. **Agent, days 2–5:** R0.2 progression rule spec, then R0.3, R0.4 and R0.5.
4. **Then E1** (ADRs and schema), with the R0.6 sync draft alongside.
5. **Owner, in parallel:** collect the Garmin, Strong and Hevy exports for E10.

**Exit gate into E1:** the local-store spike has passed on iOS (or the fallback was chosen), and the rule spec, catalog choice and wireframes are drafted.

## 9. Instructions for the next agent

- **Read §1.2 as constraints and §3 as the product.** If a design choice doesn't serve the core loop, or it breaks a principle, it waits.
- **The device is authoritative.** Never add a feature that needs the network without an offline path, except sync and account management themselves.
- **Derived data is never synced or exported.** It is always recomputed.
- Don't reopen decisions without new evidence. If evidence changes one (for example, iOS storage makes OPFS unworkable), report it with a recommendation.
- Keep `packages/progression`, `packages/records` and `packages/sync-protocol` pure, framework-free and exhaustively tested. They are the portfolio showcase.
- No telemetry, no third-party scripts, no Vercel-only services. Every dependency must be license-compatible with MIT; record the licenses.
- Research goes in `docs/research/`, each doc ending with "Implications for architecture". Decisions go in `docs/adr/`.
- Timebox spikes. If one overruns by more than 50%, stop and report.
- Never reintroduce a Garmin API, SDK or unofficial-library dependency. Never commit unscrubbed personal exports.
- Recheck date-sensitive facts when you act on them; they were gathered on 2026-11-01 and 2026-11-02.
- The owner wants direct, structured assessments. Flag your confidence and disagree openly when warranted.

## 10. Owner answers

| # | Question | Answer |
|---|---|---|
| 1 | Three-layer data model | Agree (now all on the device, v1.5) |
| 2 | Decentralization | ~~Level 1 plus prep for offline~~ → **v1.5: completely decentralized, local-first. Offline is the main pitch.** |
| 3 | Framework | Next.js 16 (not TanStack) |
| 4 | Multi-user model | v1.5: no accounts locally; multi-tenant sync server |
| 5 | Reference-only percentiles | Agree |
| 6 | Units | Default lbs, kg toggle |
| 7 | Stack | React, Neon and Better Auth confirmed (Neon and Better Auth now live on the sync server) |
| 8 | Progression scheme | Users choose; show every signal of how overload happened |
| 9 | Default rep ranges | User input with the suggested defaults |
| 10 | Deloads | Manual, with suggestions |
| 11 | Warm-up sets | Excluded from verdicts and PRs |
| 12 | RPE/RIR | Optional and visible |
| 13 | "Last time" resolution | As specified |
| 14 | M1 PR types | As specified |
| 15 | Body weight | Must-have |
| 16 | Imports in M1 | No; file import comes after sync |
| 17 | Old repo | Start fresh |
| 18 | Hosting | Vercel Hobby (static web app; sync server on Functions + Neon, or self-hosted Docker) |
| 19 | Public repo | Public; cleared |
| 20 | Milestones | M1 ~Jan 17, M2 ~Feb 14 (plan targets) |
| 21 | Time budget | As many hours as possible. **v1.5: added time is acceptable; local-first is worth it.** |
| 22 | Social vs local-first | **Local-first.** Social is deferred (E14). |
| 23 | Open source | **Yes, so people can do what they need with it.** MIT recommended; the owner confirms at E0-T1. |
| 24 | Platform | **GymOs is a platform**: open schema, open format, published packages, importer contract, self-hostable sync. |
| 25 | Meaning of "decentralized" | Interpreted as user sovereignty: local data, optional sync through a server of your choice, open format. P2P deferred. *Owner: correct this if you meant true P2P.* |

**Open items for the owner:** the license (MIT vs Apache-2.0) at E0-T1, and #25 if the interpretation is wrong. Everything else is decided. New questions should come only from spike or research results.
