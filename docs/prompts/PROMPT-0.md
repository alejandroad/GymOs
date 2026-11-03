# GymOs: Prompt 0 v2 (Epic E0, tickets T1–T6: local-first stack spike)

> **v2 (2026-11-02) replaces v1.** v1's T3 (Neon) and T4 (Better Auth) are cancelled: accounts and the server move to the sync epic (E9). If v1's T1 or T2 already ran, keep that work and continue from the new T3. Paste everything below the line into Claude Code, run from the root of the `GymOs` repo.

---

You are a senior full-stack engineer starting **GymOs**: a local-first, open-source lifting analytics platform. Data lives on the user's device, the app works fully offline with no account, and optional multi-device sync comes later. This is a public portfolio project for FAANG-level applications, so code quality, commit hygiene and documentation are part of the deliverable.

## 0. Read first

1. Read `docs/HANDOFF.md` (v1.5) completely before writing code. Pay particular attention to:
   - §1.2, the principles, which are constraints;
   - decisions D18 (local-first), D23 (local store), D6 (Next.js as a static PWA), D15 (hosting) and D19 (license);
   - data-model rules 13 (local-first core) and 15 (portability);
   - §5.2, the local-first facts;
   - the E0 ticket table in §7.

   If `docs/HANDOFF.md` is missing or isn't v1.5, stop and ask me for it.
2. Run `git status`, `git remote -v`, `git log --oneline -10` and `git branch -a`. If work from Prompt 0 v1 exists, tell me what's there. Keep T1 and T2 if they're sound. Remove any Neon, Drizzle-on-Postgres, Better Auth or `/api/*` code from v1, each removal in its own commit with a clear message.
3. Check the toolchain: `node -v`, `pnpm -v`, `git --version`. I develop on Windows. Everything must work on Windows and on Linux CI.
4. Batch these questions to me up front:
   - License: MIT (recommended) or Apache-2.0?
   - Is a GitHub remote configured, and is `gh` authenticated?
   - Can I link the Vercel project when you reach T6?

## 1. Goal of this session

Prove the riskiest part of the architecture: a durable SQL database in the browser that works offline, including on an iPhone. At the end there should be:

- a pnpm + Turborepo monorepo with Next.js 16 as a statically exported, client-rendered PWA (`output: 'export'`, no server runtime);
- `packages/schema` with a UUIDv7 `newId()`;
- `packages/store`, which provides:
  - SQLite-WASM in a dedicated Web Worker, persisted to OPFS;
  - Drizzle as the query layer;
  - a migration runner;
  - a typed worker bridge;
  - a single writer across tabs;
- a service worker so the app loads and the database works in airplane mode;
- green GitHub Actions CI;
- a static production deploy on Vercel Hobby;
- `docs/research/stack-spike.md` with measurements and an iOS go/no-go verdict;
- `docs/adr/0001-local-first-architecture-hosting-portability.md`.

This is foundation work only. No product schema, no product UI, no auth, no server.

## 2. Way of working (mandatory)

- One branch per ticket: `e0/t1-repo-bootstrap`, `e0/t2-next-static-app`, `e0/t3-local-db`, `e0/t4-pwa-offline`, `e0/t5-ci`, `e0/t6-deploy-spike`. Open each as a PR into `main` and rebase-merge it so individual commits are kept. If no remote or `gh` is available, work locally on the branches, merge with `--ff-only`, and tell me.
- Small, modular Conventional Commits (`chore:`, `build:`, `feat:`, `fix:`, `docs:`, `ci:`, `test:`, `refactor:`). One logical change per commit. Every commit must install, lint, typecheck, test and build. The suggested commits below are guidance; split further if a commit does two things.
- Pin exact versions (no `^`) and commit the lockfile. Before installing, check the current stable versions of:
  - Next.js 16.x, React 19.x, Tailwind 4.x, TypeScript;
  - `@sqlite.org/sqlite-wasm`, Drizzle ORM and drizzle-kit;
  - Serwist (or the PWA approach the Next 16 docs recommend);
  - Zod, Vitest, Turborepo, pnpm, and the current Node Active LTS.

  Record every version in the spike report.
- Follow the current official docs for Next.js 16, SQLite WASM, Drizzle and Serwist. Don't rely on memory. If the docs contradict this prompt, follow the docs and note it in the report.
- Local-first rules (handoff rule 13):
  - IDs are UUIDv7, generated in the client. No database serials.
  - The worker owns the database. The UI never touches SQLite directly.
  - No network calls for data.
- Portability (rule 15):
  - `apps/web` must stay a pure static build: no route handlers, no Server Actions, no `@vercel/*` SDKs.
  - Host config files (for example `vercel.json` for headers) are allowed, but every one must be documented with its equivalent for other hosts.
- Principles: no telemetry, analytics or third-party scripts. Every dependency must be license-compatible with MIT. Record each dependency's license in the report.
- Cross-platform: no bash-only `package.json` scripts. LF line endings enforced via `.gitattributes`.
- Timebox: T3 gets about 1.5 days. If T3 or T4 overruns by more than 50%, stop and report with evidence and options.
- Stop and ask me for: the license, GitHub or Vercel access, iPhone testing (you can't run an iPhone; give me a checklist), or anything irreversible. Batch your questions.

## 3. Tickets

### E0-T1 · Repo bootstrap

Scope:

- pnpm workspace (`apps/*`, `packages/*`).
- `turbo.json` with `dev`, `build`, `lint`, `typecheck` and `test`.
- Root `package.json` with `packageManager`, `engines` and delegating scripts.
- `.nvmrc`, `.editorconfig`, `.gitattributes` (`* text=auto eol=lf`), `.gitignore` (Node, Next, Turbo, `out/`, env files, OS files).
- `LICENSE` (my answer).
- `README.md` stub: the pitch ("Offline-first lifting analytics. Your data lives on your devices, syncs between them if you want, and nobody paywalls your own history."), status "pre-alpha", and a link to `docs/`.
- `docs/HANDOFF.md`, `docs/adr/0000-template.md` (MADR), `docs/research/`, and `docs/prompts/PROMPT-0.md` (this prompt).

Suggested commits:

1. `chore: initialize pnpm workspace and turborepo`
2. `chore: add editor, git attributes and ignore rules`
3. `docs: add handoff, adr template and docs structure`
4. `chore: add license and readme stub`

Acceptance: `pnpm install` succeeds on Windows; `pnpm turbo run build` runs.

### E0-T2 · Next.js 16 static app

Scope:

- `apps/web` via the official `create-next-app` for Next.js 16: App Router, TypeScript, Tailwind 4, ESLint, `src/`, alias `@/*`.
- Set `output: 'export'`. Add a root `tsconfig.base.json` (`strict`, `noUncheckedIndexedAccess`, and `exactOptionalPropertyTypes` if the toolchain tolerates it) that every package extends.
- A placeholder landing page ("GymOs" plus the tagline) marked as a client-rendered app shell. No design work.
- A cross-platform `preview` script that serves `out/` with a pinned static server (for example `serve`). The service worker and OPFS behave differently under `next dev`, so all spike checks run against `build` + `preview`.

Suggested commits:

1. `feat(web): scaffold next.js 16 app`
2. `build(web): enable static export`
3. `build: add shared strict tsconfig base`
4. `chore(web): wire lint, typecheck and preview into turbo`
5. `feat(web): add placeholder landing page`

Acceptance: `pnpm dev`, `build`, `lint` and `typecheck` pass. `build` emits `apps/web/out/`, and `preview` serves it.

### E0-T3 · Local database spike (the main risk)

Scope:

- `packages/schema`
  - `newId()` returns a UUIDv7 (a small, well-tested implementation or a pinned MIT library).
  - Vitest tests for format, version and variant bits, and monotonic ordering within the same millisecond.
- `packages/store` (framework-free; the React wiring lives in `apps/web`)
  1. **Worker.** A dedicated module worker loads `@sqlite.org/sqlite-wasm` and opens the database on the `opfs-sahpool` VFS, which needs no COOP/COEP headers. If the docs now recommend otherwise, follow them and explain why. Confirm the `.wasm` asset is emitted and fetched correctly by the Next static build (bundler: Turbopack or webpack; record which one and any config needed).
  2. **Query layer.** Drizzle on SQLite, using the proxy driver, or whichever driver the Drizzle docs recommend for SQLite WASM. It runs inside the worker (preferred) or proxies from the UI. Document the choice.
  3. **Migrations.** drizzle-kit generates SQL. A small migration runner in the worker applies migrations at startup:
     - in order, each in a transaction;
     - recorded in a `__migrations` table;
     - forward-only;
     - with a unit test that runs the migrations twice and checks they're idempotent.
  4. **Demo table.** `spike_note(id TEXT PRIMARY KEY /* UUIDv7 */, body TEXT NOT NULL, created_at INTEGER NOT NULL)`.
  5. **Typed bridge.** A request/response RPC between UI and worker with typed messages, IDs and error propagation. Hand-rolled, or Comlink if you justify the dependency.
  6. **Single writer.** Elect a leader across tabs with the Web Locks API. Only the leader opens the database (`opfs-sahpool` allows one opener). Other tabs proxy requests to the leader over `BroadcastChannel`. If proxying overruns the timebox, ship a clear "GymOs is open in another tab" state instead and document it as an E2 follow-up.
  7. **Durability.** Call `navigator.storage.persist()`. Expose `persisted()` and `estimate()`.
- `apps/web` demo page (`/spike`):
  - add a note; list notes newest first;
  - show the row count;
  - show worker init time and the last query's latency;
  - show the persistence state and storage estimate;
  - show which tab is the leader.

Suggested commits:

1. `feat(schema): add uuidv7 id helper with tests`
2. `feat(store): load sqlite wasm in a dedicated worker on opfs`
3. `feat(store): add typed worker rpc bridge`
4. `feat(store): add drizzle query layer and demo table`
5. `feat(store): add forward-only migration runner with tests`
6. `feat(store): elect a single writer across tabs with web locks`
7. `feat(store): request persistent storage and expose estimates`
8. `feat(web): add local database spike page`

Acceptance (desktop, against `build` + `preview`):

- Rows survive a reload and a full browser restart in Chrome, Firefox and Safari.
- With two tabs open, both can add notes and nothing gets corrupted.
- The tests pass.

The iPhone checks happen in T6 because OPFS needs a secure context (HTTPS), and a plain LAN dev server doesn't provide one.

### E0-T4 · PWA offline shell

Scope:

- A web app manifest (`app/manifest.ts` if static export supports it, otherwise a static file) with placeholder icons, including Apple touch icons and iOS meta tags.
- A service worker that precaches the app shell, JS chunks, the worker script and the SQLite `.wasm`. Use Serwist if it supports Next 16 with `output: 'export'`. Otherwise run Serwist's build tooling (`injectManifest`) as a postbuild step over `out/`, or write a minimal hand-written service worker. Document which you chose and why.
- Service-worker update strategy: a new version activates on the next load. No surprise reloads mid-session.
- The service worker file must not be cached long-term. Document the cache headers it needs.

Suggested commits:

1. `feat(web): add web app manifest and icons`
2. `feat(web): precache app shell, worker and wasm with a service worker`
3. `docs: document service worker caching and update strategy`

Acceptance (desktop, against `preview`):

- With the network off in DevTools (after one online visit), the app loads, and `/spike` reads and writes.
- Chrome offers an install, and the installed app works offline.

### E0-T5 · Continuous integration

Scope:

- `.github/workflows/ci.yml` running on pull requests and on pushes to `main`.
- Steps: checkout → pnpm (version from `packageManager`) → Node from `.nvmrc` → `pnpm install --frozen-lockfile` → `lint` → `typecheck` → `test` → `build`.
- Cache the pnpm store and Turbo.
- Add a CI badge to the README.

Suggested commits:

1. `ci: add github actions workflow for lint, typecheck, test and build`
2. `docs: add ci badge to readme`

Acceptance: CI is green on the T5 PR.

### E0-T6 · Static deploy, iOS verdict and write-up

1. Ask me to link Vercel. Give me exact steps: import the repo, root directory `apps/web`, the build and output settings for a static export, and any headers (`vercel.json` only if needed; document the equivalents for Netlify, Cloudflare Pages and nginx). No environment variables should be needed. If one is, stop and tell me why.
2. Production smoke test (desktop):
   - the T3 persistence checks;
   - the T4 offline check;
   - installability.
3. Give me an iPhone checklist to run, and record my results:
   - Safari tab: add notes → close the tab → reopen → rows still there?
   - Safari, force-quit: force-quit Safari → reopen → rows still there?
   - Safari, airplane mode: with airplane mode on, reload → the app loads, reads and writes?
   - Home Screen PWA: add to Home Screen → open → add notes → force-quit → reopen → rows still there?
   - PWA, airplane mode: open with airplane mode on → the app works?
   - Storage: the `persisted()` result and the storage estimate in both modes.
   - Separate stores: are the Safari and Home Screen stores separate? (Expected: yes. Note it.)
   - Record the iOS version and the device model.
4. Measure and record:
   - First Load JS per route;
   - the SQLite `.wasm` size (raw and compressed);
   - worker init time (cold, and warm with a cached service worker);
   - insert and select latency (median of 50 operations);
   - build time;
   - the persistence result per browser;
   - the current Vercel Hobby limits relevant to a static site and to Functions (for the future sync server), including the non-commercial clause, with the source link and the date checked.
5. Write `docs/research/stack-spike.md`:
   - what was built;
   - exact versions and licenses;
   - measurements;
   - the browser matrix, including the iOS results;
   - problems and workarounds (Windows quirks, docs vs reality, bundler issues);
   - risks, especially iOS storage eviction and how M1 mitigates it (install prompt, `persist()`, archive backups);
   - "Implications for architecture";
   - a recommendation.

   The recommendation must give an explicit verdict:
   - **GO:** SQLite-WASM + OPFS works on iOS (at least as an installed PWA).
   - **FALLBACK:** it doesn't. Build a minimal IndexedDB (Dexie) version of the demo table behind the same store interface, run the same checklist, and recommend.
   - **REVISIT D6:** Next.js static export, the service worker or the WASM worker hit blocking problems. Include the evidence and the cost of switching to a Vite SPA.
6. Write `docs/adr/0001-local-first-architecture-hosting-portability.md` (MADR), covering:
   - Context: the local-first pitch, the principles, one user and a few devices.
   - Decision:
     - a client-rendered Next.js 16 static PWA;
     - SQLite-WASM on `opfs-sahpool` in a worker;
     - Drizzle;
     - the forward-only migrations policy;
     - client-generated UUIDv7 IDs;
     - a single writer across tabs;
     - static hosting on Vercel Hobby, portable to any static host;
     - the future `apps/sync` (Hono, HTTP push/pull) as a separate deployable.
   - Alternatives considered:
     - hosted-first with Neon and auth (the v1.4 plan);
     - IndexedDB/Dexie;
     - wa-sqlite;
     - PGlite;
     - a third-party sync engine (Zero, Electric, PowerSync);
     - a Vite SPA instead of Next.js;
     - native shells (Tauri, Capacitor).
   - Consequences: the iOS eviction risk; the archive becomes the M1 backup; no server in M1.

Suggested commits:

1. `chore(web): add host configuration for static deploy` (only if a file is actually needed)
2. `docs(research): add stack spike report`
3. `docs(adr): add 0001 local-first architecture, hosting and portability`

Acceptance: the production URL passes the desktop checks, my iPhone results are recorded, the verdict is stated, and the report and ADR are merged.

## 4. Target layout after this session

```
GymOs/
├─ apps/
│  └─ web/                    # Next.js 16, output: 'export', client-rendered PWA
│     ├─ public/              # icons, (generated) service worker
│     └─ src/
│        ├─ app/              # /, /spike, manifest
│        └─ lib/store/        # React hooks over packages/store
├─ packages/
│  ├─ schema/                 # newId() (UUIDv7) + tests
│  └─ store/                  # sqlite-wasm worker, OPFS, drizzle, migrations, rpc, leader election
├─ docs/
│  ├─ HANDOFF.md
│  ├─ adr/0000-template.md, 0001-local-first-architecture-hosting-portability.md
│  ├─ research/stack-spike.md
│  └─ prompts/PROMPT-0.md
├─ .github/workflows/ci.yml
├─ turbo.json  pnpm-workspace.yaml  tsconfig.base.json  package.json
└─ .editorconfig .gitattributes .gitignore .nvmrc LICENSE README.md
```

Small deviations are fine if the official tooling prefers them. Note them in the report.

## 5. Out of scope (don't do these)

- Auth, accounts, Neon or Postgres, `apps/sync`, any API routes or server code, and any sync logic.
- Product schema (sessions, sets, exercises), the progression and PR engines, charts, shadcn/ui, design work.
- Playwright (E2), telemetry or analytics of any kind, Vercel-only services, anything Garmin, TanStack.

## 6. Definition of done for Prompt 0 v2

- [ ] T1–T6 merged to `main` as small Conventional Commits. CI is green.
- [ ] The production static deploy works offline, and the data persists on desktop browsers.
- [ ] The iPhone checklist results are recorded, with an explicit GO / FALLBACK / REVISIT D6 verdict.
- [ ] `docs/research/stack-spike.md` and `docs/adr/0001-*.md` are merged.
- [ ] All dependency licenses are recorded. No telemetry. No secrets.

## 7. Final report to me

End the session with:

1. a short summary;
2. the production URL;
3. `git log --oneline`;
4. measurements and the browser matrix;
5. the iOS verdict;
6. deviations from this prompt and why;
7. open risks;
8. a recommendation for Prompt 1 (E0-T7: progression rule spec), including any questions for me that it raises.
