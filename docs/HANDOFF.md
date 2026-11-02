# GymOs: Handoff for the Next Agent

**Version:** 1.4 (**FINAL**) · **Date:** 2026-11-01 · **Supersedes:** v1.0–v1.3 · **To:** the architecture and implementation agent
**Owner:** Alejandro, solo developer. He is a professional TypeScript engineer and builds web trading charts (PnL) at his day job.
**Repo:** `GymOs` (local: `C:\Users\Admin\Desktop\GymOs\GymOs`). It is currently empty apart from a placeholder `README.md` and `.git`.

> **Changes in v1.4 (FINAL):**
> - **Hosting: Vercel Hobby (free).** This replaces Cloudflare Workers/OpenNext from v1.3. The owner's reasoning: if the app follows best practice, moving hosts later should be easy. That is now enforced as **rule 14 (portability)**.
> - **The E0 ticket breakdown is added** (section 7, E0). The first coding session is **Prompt 0** (`docs/prompts/PROMPT-0.md`), covering tickets E0-T1 to E0-T6.
> - Way of working: **small, modular commits** (Conventional Commits), one short-lived branch per ticket.
>
> **Changes in v1.3 (FINAL). The owner answered every open question, so there are no more 🟡 decisions.**
> - **Framework: Next.js 16 (App Router).** TanStack Start is rejected.
> - ~~Hosting: Cloudflare Workers via the OpenNext adapter~~ (replaced by Vercel in v1.4). Neon Postgres, Better Auth and R2 are confirmed.
> - **Offline:** no offline support or sync in v1, but **build so it can become local/offline later** (rule 13, section 6).
> - **Units:** display lbs by default, with a kg toggle.
> - **Progression:** users choose their progression scheme. The app shows **every way** overload happened, not just one verdict.
> - **Body weight is P0.** It's needed for relative strength and percentiles.
> - **Deload weeks:** marked manually, with the app suggesting them.
> - **Start fresh.** Nothing is salvaged from AnabolicWorkoutTracker.
> - **The repo is public.** The employer IP policy is cleared.
> - **Deadline:** the owner wants to be ready for Google applications **by March 2027**, working as many hours as possible. The timeline in section 7 is set to that.
>
> **Changes in v1.2, all decided by the owner:**
> 1. GymOs is **primarily a lifting analytics tool: a smart lifting diary**. Progressive overload is built into everything.
> 2. **"Last time" comparison** is a core feature: while logging, users see what they did last time for the same exercise or workout.
> 3. **PRs and records for every effort type** are core.
> 4. **Manual entry is the primary input for every activity type.** There is a strength-workout form *and* a general activity form (Strava-style manual activity). File imports (Strong, Hevy, Garmin Connect CSV) are accelerators, not prerequisites.
> 5. As a result, **building starts now.** No CSV or owner input blocks E1–E7. The progression engine moves *into* the first demo (M1). Importers move after it.
> 6. The epics are renumbered (mapping in section 6). v1.1's Garmin decisions (no Garmin developer dependency; Garmin Connect CSV as a file format) still stand.

> Read the whole file before you act. Section 3 is the product core. Sections 4–6 hold the decisions, facts and rules. Section 7 is the work plan. Section 8 is what to do first.

---

## 1. Mission

Build **GymOs**: a **smart lifting diary** that makes progressive overload visible and actionable, with a trading-platform-style analytics dashboard.

It is a **portfolio piece for FAANG-level applications**, so a deployed, working, well-engineered demo matters more than feature count.

**Pitch:** "Log your lifts and GymOs tells you, every session, whether you're getting stronger, how fast, and what to beat next. Your data, in an open format you own."

**The core loop is the product. Everything else supports it:**

```
 Start workout ──► see LAST TIME (same exercise / same workout) + target to beat
      │
      ▼
 Log sets (fast, phone-first)  ──►  live feedback: "matched", "beat it", "new 5RM 🏆"
      │
      ▼
 Finish ──► session verdict (what progressed, what stalled, why)
      │
      ▼
 Dashboard ──► progress RATE per lift (getting stronger faster or slower? plateau?), PR board, history
```

**Feature priority:**
1. Strength workout logging with **last-time comparison** (P0).
2. **Progression engine**: per-set, per-exercise and per-session overload verdicts, progress rate and acceleration, plateau detection (P0).
3. **PR and records engine for every effort type** (P0).
4. Analytics dashboard: trading-style charts of e1RM, volume, PRs and progress rate (P0).
5. **Body-weight tracking** (P0: drives relative strength, bodyweight-exercise loads and percentiles). Also the **manual activity form** for non-lifting sessions (runs, rides, MMA, climbing…) and other **manual measurements** such as VO2 max and resting HR (P1, in M1).
6. Data ownership: archive export and re-import (P1).
7. File imports: Strong/Hevy CSV, Garmin Connect CSV including VO2 max (P2, accelerators).
8. Reference percentiles (P3).

**Deadline:** ready for Google applications by **March 2027**, which is about 17 weeks from 2026-11-01. The owner works on this as many hours as possible. **A deployed demo of the core loop (M1) is the top priority.** The owner also needs time for interview prep in February, so plan for GymOs to be feature-complete by mid-February.

## 2. Prior context

- An earlier attempt existed in a separate repo, **AnabolicWorkoutTracker**. Its stack was Next.js App Router, Prisma, Auth.js v5, Neon, Recharts/visx and Vercel, and it used `cuid()` IDs, lbs as the canonical unit, and a `PersonalRecord` model with a RecordType enum covering every rep range. Supabase was ruled out. **Mine that repo for ideas, especially the PR model.** Don't port code without a reason.
- The research proposal v0.1 and its staff review are in the claude.ai GymOs project. This handoff supersedes both.

## 3. Product core: specification

This section is what makes GymOs GymOs. E5 and E6 implement it. **Every rule is deterministic, versioned and explainable.**

### 3.1 Exercise tracking modes
Each catalog exercise has a tracking mode. **The schema, the PR types and the overload rules all depend on it**, so it is an E1 decision.

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
- **Same exercise:** the most recent session containing exercise X, with its working sets shown inline next to each set row as the user logs.
- **Same workout type:** if the session was started from a **template or routine** (for example "Push A"), "last time" means the last session from that template. Otherwise the app matches by exercise set similarity (Jaccard ≥ threshold), and the user can pick a different comparison session.
- Each exercise shows:
  - **last time** (sets × reps @ load, RPE);
  - **best ever** (the relevant PRs);
  - **target to beat**: a deterministic suggestion from the progression scheme **the user chooses** for each exercise or template: double progression, linear, RPE-based, or none (show last time and best ever only). New exercises default to double progression. Rep ranges and load increments are user input. Suggested defaults: compounds 5–8 reps, accessories 8–12, isolation 10–15, with the smallest plate or dumbbell step as the increment.
- **Live set feedback while logging:** `beat` / `matched` / `below` compared with the corresponding set last time, plus an instant 🏆 when a set is a new PR.

### 3.3 Overload verdicts
These are computed **per exercise per session** against the comparable previous session, then rolled up to the whole session.

**Progress signals,** any one of which counts, in order of strength:
1. Higher **e1RM** from the best set (Epley or Brzycki, valid ≤ 10–12 reps; RPE-adjusted via an RIR table when RPE is logged).
2. **More reps at the same load**, or **more load at the same reps**.
3. **More working sets** at the same load × reps.
4. **Higher volume load** (Σ load × reps over working sets) without the e1RM dropping.
5. **Same work at a lower RPE** (the same load and reps felt easier).
6. For the other modes: longer duration (`duration`), more reps (`bodyweight_reps`), less assistance (`assisted`), more distance × load (`distance_load`).

- **Verdict:** `progressed` | `matched` | `regressed` | `insufficient_data` | `deload` (if the session or week is marked as a deload, it is not counted as a regression). **Deloads are marked manually.** The engine *suggests* marking one when a session shows a broad, deliberate drop in load or volume across lifts (thresholds versioned), and the user confirms.
- **Owner's requirement: show every way overload happened.** The verdict is not a single winning rule. The UI lists **every** signal that fired, per exercise and for the session, for example "volume +8%", "+1 rep on set 2" and "same load at RPE 7 vs 8". If progress happened anywhere, the user sees where and how. The verdict label is a summary on top of that list.
- **Output:** the verdict, plus **all** the rules that fired, the before and after numbers, and the rule version. For example: *"Progressed: +1 rep at 100 kg on set 2 (8→9); e1RM 124.0→126.7 kg."*
- **Session roll-up:** "4 of 6 exercises progressed, 1 matched, 1 regressed (Incline DB press: −2 reps)."

### 3.4 Progress rate: "Are you getting stronger faster?"
- **Strength trend per exercise:** the best e1RM per session over time, fitted with a **robust slope** (Theil–Sen) over a rolling window (default 6 weeks or at least 4 sessions). Reported as **%/week** and kg/week.
- **Acceleration:** the current window's slope compared with the previous window's. Label it `accelerating` | `steady` | `slowing` | `plateau` | `declining`, with thresholds stored as config and versioned.
- **Plateau detection:** no e1RM or rep PR in N sessions (default 4) **and** a slope near zero (inside a ±band). Surface it as an insight ("Bench has plateaued for 3 weeks"), with deterministic suggestions only (change the rep range, deload). No LLM.
- **Relative strength:** e1RM divided by body weight when body weight is logged.
- **Aggregates:**
  - a "strength index", the weighted mean of %/week across the user's main lifts;
  - weekly volume per muscle group (via catalog muscle mappings);
  - training frequency per lift.
- All series must work with sparse, irregular data. Show confidence (n sessions) and never draw a trend from fewer than the minimum.

### 3.5 PRs and records: every effort type
PRs are **derived data**: materialized, keyed by `algorithm_version`, and **fully recomputable**. Editing or deleting a session must update or revoke the affected PRs. Each PR stores the set or session that achieved it, the date, the value and the previous record (for the PR timeline).

| Mode | PR types |
|---|---|
| `weight_reps` | **Actual rep maxes** 1RM–20RM (heaviest load for ≥ r reps; a heavier load for more reps also beats lower rep counts) · **e1RM** · max reps at a given load · best set volume (load × reps) · best session volume for the exercise · heaviest load ever |
| `bodyweight_reps` | Max reps in a set · max total reps in a session |
| `weighted_bodyweight` | Rep maxes by added load · e1RM including body weight |
| `assisted_bodyweight` | Least assistance for ≥ r reps |
| `duration` | Longest hold, at a given load if loaded |
| `distance_load` | Longest distance at a load · heaviest load over a distance |
| **Cardio / activities** (manual form or imports) | Fastest time for standard distances (1 mi, 5 km, 10 km, half, full) **at the activity level only** (the whole activity's distance must match the standard distance within a tolerance; splits need streams, which are E12 work) · longest distance · longest duration · biggest week or month by distance or time |
| **Session or overall** | Most total volume in a session · longest training streak (weeks) · most sessions in a month |

- **PR board:** the current records per exercise, the PR timeline, and "recent PRs" on the dashboard.
- **PR notifications** in the logging flow are immediate (computed client-side against the cached records, then confirmed server-side on save).

### 3.6 Manual activity form (Strava-style)
- **Fields:** sport/type (run, ride, swim, walk, hike, row, MMA/boxing, climbing, yoga, other), title, start date/time and time zone, duration (elapsed and moving), distance, elevation gain, average and max HR, calories, perceived effort (1–10), notes. **Which fields show depends on the type.**
- **Manual measurements:** body weight (**P0**: quick entry from the dashboard and the workout screen, with a trend chart), VO2 max (a watch reading, entered manually), resting HR, and others, all on the Measurement entity. Bodyweight exercises use the most recent body weight on or before the session date.
- **So the VO2 max chart and cardio PRs work with no file import at all.** Imports later fill the same entities.

## 4. Decisions

**Status key:** ✅ decided by the owner (all decisions are decided as of v1.3) · ⛔ ruled out

| # | Decision | Status | Rationale (short) |
|---|---|---|---|
| D5 | **Lifting analytics is the core.** Progressive overload is built into every surface. Endurance is supported at activity-summary level. | ✅ (v1.2) | The owner's product definition. |
| D13 | **Manual entry is the primary input for all activity types** (strength form, activity form, measurements). File imports are accelerators. | ✅ (v1.2) | No dependency on files to start building. Matches Strava. |
| D14 | **The progression engine (verdicts, last time, progress rate) and the PR engine for every effort type ship in M1.** | ✅ (v1.2) | They are the product. A demo without them is just a logger. |
| D1 | **No Garmin developer dependency:** no Connect API, no Garmin FIT SDK, no unofficial libraries. | ✅ | Volatile platform. Not needed. |
| D12 | **Garmin Connect CSV exports are a supported import format** (activities and the VO2 max report). | ✅ | The user's own data, with zero API dependency. |
| D2 | FIT is a possible future import format only (`fit-file-parser`, MIT), never the data model. | ✅ | Vendor-neutral. |
| D9 | All verdicts, targets, plateau calls and PRs are **deterministic rules**. An LLM may later *phrase* insights, never decide them. | ✅ | Testable and explainable. |
| D3 | Three layers: raw artifacts (hashed), canonical entities (with provenance), derived data (verdicts, PRs, trends; versioned and recomputable). | ✅ | Edits and deletes stay consistent, and rule changes can be replayed. |
| D4 | v1 is **level 1 plus local compute**: open archive with full round-trip, published schema, in-browser analytics. The workout in progress is saved as a local draft for flaky gym Wi-Fi. **No sync engine in v1, but the code must be built so full local/offline use can be added later** (rule 13). | ✅ (v1.3) | The owner wants offline eventually. Preparing costs little now and saves a rewrite later. |
| D6 | Framework: **Next.js 16, App Router**. TanStack Start is rejected by the owner. Server Components for data-loading shells; **client components for the logging screen and charts**; Server Actions or route handlers for mutations. Domain packages stay framework-free. | ✅ (v1.3) | The owner's choice. Mature, a strong hiring signal, and the owner has used it. |
| D7 | **Multi-user data model from day one**, single-user product. No social features in v1. | ✅ | Tenancy is painful to add later, and the demo needs accounts. |
| D8 | Percentiles come from **published reference data only**, labelled with the reference population. Pooled cohorts are deferred (k ≥ 50). | ✅ | Defensible at portfolio scale. |
| D10 | Store loads as **entered value + entered unit, plus normalized kg**. **Display lbs by default** with a kg toggle (a user preference, plus a per-exercise override for kg-plate gyms). | ✅ (v1.3) | No rounding drift (225 lb stays 225 lb), and PR ties stay exact. |
| D11 | Stack: Next.js 16, React 19, TS strict, Tailwind 4, shadcn/ui, **Neon Postgres** (serverless HTTP driver) with Drizzle, **Better Auth**, Zod, Vitest plus fast-check, Playwright, **Cloudflare R2**, TradingView Lightweight Charts for hero charts, ECharts where needed, Papa Parse for CSV. | ✅ (v1.3) | Confirmed by the owner. Pin versions at scaffold time. |
| D15 | **Hosting: Vercel Hobby (free plan)**, with Git integration and preview deploys for each PR. Hobby is for **personal, non-commercial use**, which is fine for a portfolio; monetizing would require Vercel Pro or a move. **Portability is mandatory (rule 14)** so a move to Cloudflare, Netlify or a container stays a small job. Cloudflare R2 stays for object storage (S3 API). | ✅ (v1.4) | The owner's choice. Zero-config Next.js hosting, and the owner has used it before. |
| D16 | Fresh start. No code from AnabolicWorkoutTracker. | ✅ (v1.3) | The owner's call. |
| D17 | Public GitHub repo. The employer IP policy is cleared. | ✅ (v1.3) | The owner confirmed. |
| ⛔ | TanStack Start. Garmin API and SDK, unofficial Garmin libraries, the Strava API, federated protocols, a sync engine in v1, LLM-decided verdicts. | ⛔ | Owner rejection, fragile, risky or out of scope. |

## 5. Facts and constraints

### 5.1 Garmin Connect CSV formats (from the owner's real exports; needed in E9)

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
- One row per activity, summary only.

| Issue | Example | Rule |
|---|---|---|
| **No unit metadata. Values are in the user's Garmin display units.** | `1.90` with `19:10` and pace `10:04` means miles and min/mi. Elevation is in feet. | **The user declares the unit system at import.** It can't be inferred. Run plausibility warnings. |
| Mixed units even in imperial [M] | Stride `1.04` is metres. Vertical oscillation `9.6` is cm. Ground contact time is in ms. | Per-column unit maps. Confirm with a metric vs statute export pair. |
| Local time with no offset | `2026-11-01 14:16:14` | Use the profile time zone and record the assumption. |
| Quoted numbers with thousands separators | `"2,519"` | Strip the separators. |
| Null marker | `--` | Treat as null. |
| Leading apostrophe | `'-5` (Body Battery) | Strip one leading `'`. This is Garmin's spreadsheet-injection guard. |
| Booleans | `false`, `No` | Normalize both. |
| Durations | `00:08:58.1` | Parse `HH:MM:SS(.f)` into seconds. |
| Glitches | Max Power `2,519` W on an easy run | Flag via plausibility ranges. Keep the row. |
| Placeholders | TSS `0.0` everywhere | Treat as unknown. |
| **Strength reps are untrustworthy** | Steps is exactly 2 × Total Reps in all 7 strength rows | Display only. **Never used for verdicts or PRs.** |
| Possibly partial export [M, unverified] | Exactly 20 rows | Tell users to load the full list first. Merge repeat exports. |

- **Dedup key:** `(activity_type, local start datetime)` with a duration tolerance.
- **Enrichment:** a Garmin strength row and a GymOs-logged workout in the same window are the same session from two sources, so they are **linked**: HR, calories and duration come from Garmin; sets, reps and load from GymOs.
- **Drift-proofing:** map columns by header name, tolerate unknown or missing columns, use versioned importer profiles, keep the raw file, and test against golden fixtures.

### 5.2 Other facts

| Topic | Fact | Confidence |
|---|---|---|
| e1RM formulas | Epley `w(1+r/30)`. Brzycki `w·36/(37−r)`. Both lose accuracy beyond about 10–12 reps, so cap them or flag the estimate as low confidence. RPE-adjusted e1RM uses an RTS-style RPE/RIR percentage table. | High |
| Progress-rate statistics | Theil–Sen is robust to outlier sessions and suits small, irregular samples. Report n and the window. | High |
| Novice vs advanced rates | Expected progress rates fall sharply with training age. Don't label a slow rate "bad" without context. Comparing users to "expected rates" is out of scope until a defensible source exists. | High (qualitative) |
| Next.js on Vercel | Vercel is Next.js's native platform. All Next.js 16 features are supported with no adapter. Hobby plan limits (function duration, invocations, bandwidth, build minutes) change over time: **record the current numbers in the R0.1 spike.** [M]: Hobby is limited to non-commercial use. | High / Medium |
| Next.js 16 notes | Follow the current Next.js 16 docs for request interception (Next 16 introduced `proxy.ts` in place of `middleware.ts` [M, verify]). Auth must be enforced **server-side** in layouts, actions and route handlers, never only in proxy or middleware. | Medium |
| Portability note | Avoid Vercel-only services (Vercel KV, Edge Config, Vercel Blob, `@vercel/*` runtime SDKs) in app code. Use Neon (Postgres), R2 (S3 API) and standard Next.js APIs. Analytics or Speed Insights are allowed only as optional, removable add-ons. | High |
| Neon on Vercel | Use `@neondatabase/serverless` with Drizzle's `neon-http` driver (works in serverless and edge). The Neon–Vercel integration (a database branch per preview deploy) is optional and useful later. | Medium–High |
| Norm data (E10) | WMA 2023 age-grading factors (terms unclear, cite them). FRIEND VO2 max standards (lab CPET, so a watch reading isn't comparable without a label). OpenPowerlifting data (public domain, competitive-lifter population). | Medium–High |
| Exercise catalog seeds | free-exercise-db (public domain?) and wger (CC-BY-SA?). **Re-verify in R0.3.** | Medium |
| Lightweight Charts | Apache 2.0, and its NOTICE requires TradingView attribution (the default `attributionLogo` covers it). | Medium |
| FIT (future) | Multi-vendor format. Use `fit-file-parser` (MIT), not the proprietary Garmin SDK. FIT strength data is unreliable. | Medium–High |

**Hard constraints:**
- No Fidelity code, designs or proprietary knowledge. The owner checks the employer's IP policy before the repo goes public.
- Health data: account deletion, export and a privacy policy from the first deploy.
- Every chart needs an accessible data-table fallback and keyboard support.
- There must always be a public read-only demo with seeded data.
- Real exports are scrubbed before they are committed as fixtures.

## 6. Data-model rules E1 must specify

1. **Tracking modes** (3.1) determine the set fields, PR types and overload rules.
2. **Loads:** entered value + entered unit + normalized kg (D10). Display lbs by default, with a kg toggle. Distances in metres, durations in seconds.
3. **Time:** UTC plus the original local offset. Aggregate days and weeks by the user's local date and configured week start.
4. **Provenance:** every row carries `source_kind` (`manual_strength` | `manual_activity` | `manual_measurement` | `csv:strong` | `csv:hevy` | `csv:garmin_activities` | `csv:garmin_vo2max` …), `importer_version` where relevant, and `source_artifact_id` where relevant.
5. **Derived data** (verdicts, PRs, trends, targets) is materialized with `algorithm_version`, **recomputed on any create, edit or delete** of the underlying sessions (incrementally for the affected exercise), and never treated as source of truth.
6. **Templates and routines** are first-class. A session records `template_id` when it was started from one, which drives "last time" for the same workout type.
7. **Corrections:** user edits to imported data are an overlay, so a re-import doesn't wipe them.
8. **Dedup vs enrichment:** the same source re-imported is deduplicated; different sources describing the same session are linked (5.1).
9. **Raw artifacts:** object storage, keyed by SHA-256. The database holds metadata only.
10. **Measurements:** `metric`, `value`, `unit`, `source`, `method` and `granularity`.
11. **Exercise identity:** canonical catalog IDs, aliases, user-defined custom exercises (with a tracking mode), and per-user, per-source mappings.
12. **Importer contract:** versioned profiles with `detect`, `parse`, `map` and `validate`, running in a Web Worker and producing canonical rows plus warnings.
13. **Offline readiness (D4).** No sync in v1, but do these now. They're cheap, and they keep a later local-first move feasible:
    - **Client-generated IDs** (UUIDv7) for every user-created row. No database serials.
    - **Mutations as idempotent commands** (`LogSet`, `EditSet`, `DeleteSession`…) carrying a client ID. The server applies a command only once, which is also what a future offline queue needs.
    - **Soft deletes (tombstones)**, plus `updated_at` and a `version` on mutable rows.
    - **All client data access goes through a repository interface.** Today it is backed by the server; later it can be backed by a local store (SQLite-WASM or IndexedDB).
    - **Domain engines are pure and run client-side** (already required), so verdicts and PRs work without a server.
    - **A portable schema definition.** Zod is the source of truth and avoids Postgres-only features in the canonical model where possible.

    Honest caveat: this lowers the later cost but doesn't remove it. Conflict resolution, local migrations and multi-device sync are still real work when that phase comes.
14. **Portability (no host lock-in, D15).**
    - Only standard Next.js APIs. No Vercel-only services in app code.
    - All configuration goes through one **Zod-validated `env` module**.
    - The database is reached only through `packages/db` (Drizzle), and file storage only through a `Storage` interface (R2 implementation via the S3 API).
    - Background work goes through a `Jobs` interface (in-process first; a queue later).
    - No hosting-specific config beyond a thin `vercel.json` (only if needed).
    - Moving host must not touch domain or feature code.

---

## 7. Work plan: 13 epics (E0–E12)

**v1.1 → v1.2 mapping:** v1.1's E8 (overload engine) becomes **E6**, expanded with PRs and progress rate. v1.1's E6 and E10 (CSV importers) become **E9**. v1.1's E9 (archive) becomes **E8**. The rest shift accordingly.

**M1 (first public demo) = E0 → E7: the full core loop, deployed.** Nothing in M1 needs a file import or owner input.

### E0 · Spikes and method research *(3–5 days, done by the agent, blocks nothing beyond itself)*
| ID | Task | Output |
|---|---|---|
| R0.1 | **Stack spike (about 1 day): tickets E0-T1 to E0-T6, run via Prompt 0.** Next.js 16 + Better Auth + Drizzle (`neon-http`) + Neon, deployed on Vercel Hobby with sign-up, sign-in and sign-out, a protected page, a health route, and CI. Record the current Hobby limits, First Load JS per route, cold-start latency, and any problems. R2 upload is checked in E2. | `docs/research/stack-spike.md`, `docs/adr/0001-*.md` |
| R0.2 | **Progression-method research:** e1RM formula comparison and caps; RPE/RIR table; double, linear and RPE progression rules; Theil–Sen windowing; plateau thresholds; how Strong, Hevy and similar apps define PRs and "previous" values (a benchmark of their UX, *not* copying it). Produce a written rule spec with worked examples. | `docs/research/progression-engine.md` |
| R0.3 | Exercise catalog seed: verify the licenses (free-exercise-db, wger), check muscle-mapping quality, and assign a tracking mode to every exercise. | `docs/research/exercise-catalog.md` |
| R0.4 | Lightweight Charts spike: React wrapper, multi-series, synced crosshairs, step series, PR markers on the series, keyboard and a11y, attribution. | `docs/research/charts.md` |
| R0.5 | UX benchmark of fast set logging on a phone (number pads, steppers, swipe-to-complete, rest timer). Produce wireframes for the logging screen and the last-time panel. | `docs/research/logging-ux.md` |

#### E0 tickets (in execution order)

**Way of working:**
- One short-lived branch per ticket (`e0/t1-repo-bootstrap`…), each opened as a PR into `main` and rebase-merged so the small commits are kept.
- **Conventional Commits** (`feat:`, `chore:`, `docs:`, `ci:`, `build:`, `test:`).
- **Every commit builds and passes the checks.**
- Prefer a few focused commits per ticket over one large one.

| Ticket | Title | Scope | Acceptance |
|---|---|---|---|
| **E0-T1** | Repo bootstrap | pnpm workspace, Turborepo, root scripts, Node LTS pin, `.editorconfig`, `.gitattributes`, `.gitignore`, LICENSE, README stub, `docs/{adr,research,prompts}` with this handoff at `docs/HANDOFF.md`, ADR template | `pnpm install` works on Windows and Linux. The docs are committed. |
| **E0-T2** | Next.js 16 app | `apps/web` (App Router, TS strict, Tailwind 4, ESLint), shared `tsconfig.base.json`, placeholder landing page | `pnpm dev`, `build`, `lint` and `typecheck` all pass from the root |
| **E0-T3** | Env and database | Zod `env` module, `packages/db` (Drizzle + `@neondatabase/serverless` neon-http), drizzle-kit config and migration scripts, UUIDv7 ID helper, `GET /api/health` (database `select 1`) | Health returns `ok` against the owner's Neon database. Migrations run. |
| **E0-T4** | Auth | Better Auth (email and password; verification off for the spike) with the Drizzle adapter, auth schema migration, `/sign-up`, `/sign-in`, sign-out, protected `/app` page showing the user, enforced server-side | Sign up, sign in, reload (the session persists), sign out, and `/app` redirects when signed out |
| **E0-T5** | CI | GitHub Actions: frozen install, lint, typecheck, build (env validation skipped in CI), pnpm cache | Green on a PR |
| **E0-T6** | Deploy and spike write-up | Vercel project (Git integration, env vars), production smoke test, measurements, `docs/research/stack-spike.md`, `docs/adr/0001-framework-hosting-portability.md` | The production URL passes the T4 checks. The doc and ADR are merged. |
| E0-T7 | Progression rule spec (R0.2) | `docs/research/progression-engine.md` with worked examples: the test oracle for E6 | Owner review |
| E0-T8 | Exercise catalog (R0.3) | Licenses, seed choice, tracking-mode mapping | Doc merged |
| E0-T9 | Charts spike (R0.4) | Lightweight Charts in Next.js (client component), synced crosshairs, step series, PR markers, a11y | Spike page and doc |
| E0-T10 | Logging UX (R0.5) | Benchmark plus wireframes for the logging screen and last-time panel | Doc and wireframes |

**Prompt 0 covers T1–T6. Prompt 1 covers T7 (the rule spec, the most important research). Prompt 2 covers T8–T10.**

**Owner tasks (non-blocking, run in parallel; the employer IP policy is already cleared):**
- export Garmin Connect CSVs (full activity history, VO2 max report at all ranges, one metric and one statute export of the same activities) and any Strong or Hevy history. **These are only needed for E9.**

**DoD:** the spike has passed or the fallback has been chosen, and the progression rule spec and catalog choice are written.

### E1 · Architecture and ADRs *(about 1 week)*
- **All decisions are confirmed (v1.3).** Record each as an ADR (MADR, `docs/adr/`). No owner confirmation round is needed. Ask only about genuinely new questions.
- ADRs to write:
  - framework (Next.js 16: the server/client component split, mutation pattern);
  - hosting (Vercel Hobby; portability rule 14; migration path to Cloudflare, Netlify or a container);
  - offline readiness (rule 13);
  - database, ORM and storage;
  - auth and tenancy;
  - canonical schema v1 (tracking modes, sets, templates, activities, measurements);
  - load and unit storage (D10);
  - time;
  - provenance;
  - **derived-data strategy** (materialize, version and recompute; incremental recompute per exercise);
  - progression engine architecture (pure package, client and server shared);
  - PR engine;
  - importer contract;
  - archive format;
  - charts and a11y;
  - percentile methodology;
  - no Garmin developer dependency;
  - deferred items.
- Write schema v1 as Zod, an ERD and a spec.
- Lay out the monorepo: `apps/web`, `packages/schema`, `packages/progression` (verdicts, targets, trends, plateaus), `packages/records` (PRs), `packages/catalog`, `packages/importers`, `packages/archive`, `packages/metrics`. Domain packages are framework-free, so the **same engine runs in the browser** (live feedback) **and on the server** (authoritative save).
- Set performance budgets. Logging-screen interactions must take under 100 ms.

**DoD:** the owner has approved the ADRs, and the schema spec is merged.
**Depends on:** E0.

### E2 · Foundation and platform *(about 1 week)*
- Monorepo (pnpm and Turborepo), TS strict, lint and format. CI on GitHub Actions: typecheck, unit and property tests, Playwright e2e (including mobile viewport), bundle-size budget check. Vercel preview deploys for each PR. Scripts must be cross-platform (the owner develops on Windows): no bash-only `package.json` scripts, and `.gitattributes` enforces LF.
- Better Auth (email and passkey, optionally Google). `user_id` on every table. Account deletion works end to end.
- Neon with Drizzle migrations. Object storage. Secrets handling. Error tracking.
- Privacy policy page, plus the export and delete endpoints stubbed.

**DoD:** sign up, log in and delete an account work in production. CI is green.
**Depends on:** E1.

### E3 · Canonical schema and data platform *(about 1 week)*
- `packages/schema`: sessions, exercises-in-session, sets (by tracking mode), templates, activities, measurements, provenance.
- Load and unit utilities (entered + normalized) with property tests (round-trips are exact).
- Derived-data framework: an `algorithm_version` registry and recompute-on-change hooks.
- **Seed-data generator:** deterministic, covering about 12 months of realistic lifting. It includes novice-to-intermediate progression, deloads, plateaus, missed weeks, templates (Push/Pull/Legs), and some runs and body-weight entries, so every engine and chart has interesting data.

**DoD:** the seed fills a demo account, and the schema tests pass.
**Depends on:** E1, E2.

### E4 · Exercise catalog *(about 1 week)*
- Seed the catalog from R0.3 with IDs, a tracking mode, primary and secondary muscles, equipment, movement pattern, aliases and a default load increment.
- Search UI (fast, fuzzy, recent and favourite exercises first). Custom exercises with a tracking mode.
- Alias and normalization matcher: exact → normalized → trigram → user confirmation → a remembered mapping. This is used now for search and later by the importers.

**DoD:** you can find and add an exercise in ≤ 3 taps or keystrokes for common lifts. Matcher golden tests pass.
**Depends on:** E3, R0.3.

### E5 · Smart diary: logging and manual entry *(about 2 weeks)*
- **Strength logging (3.1, 3.2):** start empty or from a template. Phone-first set rows. Set types. RPE or RIR. Rest timer. Supersets. Notes. A local draft of the workout in progress survives a reload or going offline. Edit and delete past sessions.
- **Last-time panel** inline for every exercise: last time, best ever and target to beat. Live set feedback (`beat` / `matched` / `below`) and instant PR badges, using `packages/progression` and `packages/records` client-side.
- **Templates and routines:** create one from a past session, reorder, set a rep range and progression scheme per exercise.
- **Manual activity form (3.6)** and **manual measurements** (body weight, VO2 max, resting HR).
- History: calendar and list views. Session detail page.

**DoD:** a full workout can be logged on a phone with less than about 2 minutes of interaction overhead, and last time and the targets are correct against golden scenarios. Manual activities and measurements save and show up. Playwright mobile tests pass.
**Depends on:** E3, E4. It develops in tandem with E6, because the engines are needed for live feedback. Build E6's pure packages first.

### E6 · Progression and records engines *(about 2 weeks, starting alongside E5)*
- `packages/progression`:
  - comparable-context resolution (3.2);
  - targets (double, linear and RPE schemes);
  - per-set, per-exercise and per-session verdicts with explanations (3.3);
  - progress rate, acceleration and plateau detection (3.4);
  - strength index;
  - muscle-group volume.
- `packages/records`: every PR type in 3.5. Incremental update on save. Full recompute. **Revocation on edit or delete.** PR timeline with previous values.
- Server integration: recompute derived data for the affected exercises on any change, keyed by `algorithm_version`. A backfill job recomputes everything when the version changes.
- **Testing is the portfolio showcase here:**
  - golden scenario suites (dozens of hand-written sessions with the expected verdicts and PRs);
  - property tests (for example, deleting a PR session never leaves an orphaned PR, and full recompute equals incremental update);
  - a published rule spec.
- UI: post-session verdict summary, and verdict history per exercise.

**DoD:** 100% branch coverage on the rule modules, all golden and property suites pass, every verdict and PR in the UI shows its explanation, and editing or deleting sessions keeps the PRs correct.
**Depends on:** E3, R0.2.

### E7 · Analytics dashboard, then **M1: demo deploy** *(about 1.5 weeks)*
- Chart adapter layer, with Lightweight Charts for hero series.
- **Lift view:** e1RM over time with PR markers, a trend line with %/week, an acceleration badge, rep-max curves, volume, and verdict history.
- **Overview:**
  - strength index;
  - a lift "watchlist" with sparklines and progress-rate tiles (the trading-style UX);
  - recent PRs;
  - plateau alerts;
  - weekly volume by muscle group;
  - a frequency heatmap (ECharts);
  - body weight and VO2 max measurement charts;
  - activity summaries.
- **PR board:** records per exercise and the timeline.
- Synced crosshairs, range selector, dashboard state in the URL.
- A11y: table fallbacks, keyboard crosshair, contrast in light and dark themes.
- **Public read-only demo account** on the seed data. README with screenshots, an architecture diagram and a link to the rule spec.

**DoD (M1):** from a public URL, a stranger sees the full loop in under 60 seconds: last time → log → verdict → PR → progress rate. Lighthouse and the performance budgets pass.
**Depends on:** E5, E6.

### E8 · Data ownership: archive export and import *(about 1 week, after M1)*
- GymOs archive v1 (zip): `manifest.json` (schema version, counts, checksums), `sessions.ndjson`, `activities.ndjson`, `measurements.ndjson`, `templates.ndjson`, `custom_exercises.ndjson`, `raw/`, optional CSV (escaped against formula injection).
- Derived data is **not** exported. It is recomputed on import, which proves that it can be recomputed.
- Published JSON Schema plus a spec. CI round-trip test: export → wipe → import → deep-equal, with PRs and verdicts identical after the recompute.
- Optional: DuckDB-WASM local analytics over the archive.

**DoD:** the round-trip test passes in CI, and the spec is published.
**Depends on:** E3, E6.

### E9 · File imports: Strong, Hevy and Garmin Connect CSV *(about 1.5 weeks, after M1)*
- Importer profiles: `strong-csv@1`, `hevy-csv@1`, `garmin-activities-csv@1`, `garmin-vo2max-csv@1` (every rule in 5.1).
- Import flow: preview → declare the unit system → confirm exercise mappings → dedup and enrichment check → commit → **recompute derived data** (imported history immediately produces PRs and trends).
- Golden tests from the owner's scrubbed exports (collected in E0), including the metric vs statute pair.

**DoD:** the owner's real exports import correctly, re-importing creates no duplicates, Garmin strength rows link to logged sessions, and imported Strong or Hevy history produces correct PRs.
**Depends on:** E3, E4, E6, and the owner's exports.

### E10 · Measurements and reference percentiles *(about 1 week)*
- VO2 max chart: manual entries, Garmin monthly values and VDOT (Daniels–Gilbert, only from activities marked as a race or time trial), each labelled by method and granularity.
- Reference scores, each labelled with its population:
  - strength vs OpenPowerlifting using DOTS ("vs competitive lifters");
  - running age grade % (WMA 2023);
  - VO2 max vs FRIEND ("estimated vs lab norms").
- A methodology page.

**DoD:** every score links to its methodology, and the licensing is resolved.
**Depends on:** E5 (measurements), E9 (optional enrichment).

### E11 · Hardening and portfolio polish *(ongoing, plus about 1 week focused)*
- Performance budgets, axe a11y in CI, security review (auth, upload validation, CSV injection, rate limits), export and delete audit, observability.
- Portfolio packaging:
  - architecture diagram;
  - ADR index;
  - published schema and progression rule spec;
  - demo GIF or video;
  - write-ups: *deterministic progression engine*, *derived data that stays correct under edits*, *importer framework and unit handling*.

**DoD:** a portfolio-ready README, with all audits passing.
**Depends on:** M1 for the first pass. Repeat after E10.

### E12 · Later and optional
- **Device files:** FIT via `fit-file-parser`, GPX and TCX, with laps and streams, enabling split-based cardio PRs (best 5 km within a longer run).
- Apple Health and Health Connect.
- **Social:** sharing of sessions and PRs, a feed.
- An LLM that *phrases* deterministic insights.
- Opt-in pooled cohorts with k ≥ 50.
- Expected-progress-rate comparisons, if a defensible source is found.

### Dependency summary
```
E0 ─► E1 ─► E2 ─► E3 ─┬─► E4 ─┐
                      │       ├─► E5 ◄─┐
                      └─► E6 ─┴────────┘ (built in tandem: pure engines first)
                                 │
                       E5 + E6 ──► E7 ══► M1 (core loop, deployed)
                                            │
                     E8 (archive) · E9 (imports) · E10 (measurements/percentiles)
E11: continuous; focused passes after M1 and after E10
```

**Timeline to the March 2027 deadline** (about 17 weeks from 2026-11-02, with the owner at maximum hours; week 1 starts 2026-11-02):

| Weeks | Dates (approx.) | Work |
|---|---|---|
| 1 | Nov 2–8 | E0 (spike, rule spec, catalog, chart and UX research) |
| 2 | Nov 9–15 | E1 ADRs and schema · start E2 |
| 3–4 | Nov 16–29 | E2 foundation · E3 schema and seed · E4 catalog |
| 5–8 | Nov 30 – Dec 27 | E6 engines (pure packages first) in tandem with E5 smart diary |
| 9–10 | Dec 28 – Jan 10 | E7 dashboard → **M1 deployed (target: ~Jan 10)** |
| 11–12 | Jan 11–24 | E8 archive · E10 body-weight percentiles (strength vs OpenPowerlifting via DOTS) |
| 13–14 | Jan 25 – Feb 7 | E9 imports (Strong, Hevy, Garmin CSV) · E11 hardening |
| 15 | Feb 8–14 | E11 portfolio packaging (write-ups, demo video, ADR index) → **feature freeze ~Feb 14** |
| 16–17 | Feb 15–28 | Buffer, bug fixes, owner's interview prep |

**Cut order if behind:** E9 Garmin CSV → E9 Strong/Hevy → E10 running/VO2 max percentiles → E8 DuckDB-WASM extra. **Never cut:** the core loop (E5–E7), the engine test suites, the deployed demo, or the README and ADRs.

---

## 8. First step

**Building starts now. Nothing is waiting on files or owner input.**

1. **Agent, day 1: run Prompt 0** (`docs/prompts/PROMPT-0.md`), covering tickets E0-T1 to E0-T6: repo bootstrap, the Next.js 16 app, the database, auth, CI, and a Vercel deploy with the spike write-up. Small, modular commits.
2. **Agent, days 2–5: R0.2 progression rule spec** (the heart of the product), plus R0.3, R0.4 and R0.5. The rule spec, with worked examples, becomes E6's test oracle.
3. **Then E1:** write the ADRs and schema v1. All decisions are already confirmed.
4. **Owner, in parallel, non-blocking:** collect the Garmin, Strong and Hevy exports for E9 when convenient.

**Exit gate into E1:** the spike has passed or the fallback has been chosen, and the progression rule spec, catalog choice and logging wireframes are drafted.

---

## 9. Instructions for the next agent

- **Read section 3 as the product.** If a design choice doesn't serve the core loop (last time → log → verdict → PR → progress rate), it waits.
- All decisions are confirmed as of v1.3. Don't reopen them without new evidence. If evidence changes one (for example, the spike shows the free plan can't work), report it with a recommendation.
- Hold to the timeline in section 7. If an epic slips by more than a week, apply the cut order and tell the owner.
- Keep `packages/progression` and `packages/records` pure, framework-free and exhaustively tested. They are the main portfolio showcase.
- Research goes in `docs/research/` (each doc ends with "Implications for architecture"). Decisions go in `docs/adr/`.
- Timebox spikes. If one overruns by more than 50%, stop and report back.
- Never reintroduce a Garmin API, SDK or unofficial-library dependency.
- Never commit unscrubbed personal exports.
- Recheck date-sensitive facts when you act on them. The facts here were gathered on 2026-11-01.
- The owner wants direct, structured assessments. Flag your confidence and disagree openly when warranted.

## 10. Owner answers (resolved 2026-11-01)

| # | Question | Answer |
|---|---|---|
| 1 | Three-layer data model (D3) | Agree |
| 2 | Decentralization (D4) | Agree for v1. **Must eventually support local/offline use, so build to prepare for it** (rule 13). |
| 3 | Framework (D6) | **Not TanStack. Next.js 16.** |
| 4 | Multi-user model, single-user product (D7) | Agree completely |
| 5 | Reference-only percentiles (D8) | Agree |
| 6 | Units (D10) | Agree. **Default lbs, kg toggle.** |
| 7 | Stack (D11) | React, Neon Postgres and Better Auth confirmed. No changes. |
| 8 | Progression scheme | **Users choose.** The app shows if and *how* overload happened: every signal (volume, reps, load, RPE…). |
| 9 | Default rep ranges | User input; keep the suggested defaults. |
| 10 | Deloads | Manual, with suggestions |
| 11 | Warm-up sets | Leave out of verdicts and PRs |
| 12 | RPE/RIR | Optional and visible |
| 13 | "Last time" resolution | Proceed as specified (template, then similarity, with user override) |
| 14 | M1 PR types | Proceed as specified |
| 15 | Body weight | **Must-have.** It gauges strength and enables percentile ranking. |
| 16 | M1 without imports or archive | Yes. File import is a later feature. |
| 17 | Old repo | **Start fresh** |
| 18 | Hosting | ~~Cloudflare~~ → **Vercel free (Hobby) plan** (v1.4). Built for portability, so switching later is easy. |
| 19 | Public repo / employer policy | **Public; cleared** |
| 20 | M1 target date | TBD by the owner; the plan targets ~Jan 10, 2027 |
| 21 | Time budget | As many hours as possible; **Google-ready by March 2027** |

**No open questions remain.** New questions should come only from spike or research results.
