# GymOs

Log your lifts and GymOs tells you, every session, whether you're getting stronger, how fast, and what to beat next. Your data, in an open format you own.

**Status:** pre-alpha. Nothing is deployed yet.

## Docs

- [`docs/HANDOFF.md`](docs/HANDOFF.md): product spec, decisions and work plan
- [`docs/adr/`](docs/adr/): architecture decision records
- [`docs/research/`](docs/research/): spikes and method research

## Development

Requires Node 26 (see `.nvmrc`) and pnpm 12.

```bash
pnpm install
cp apps/web/.env.example apps/web/.env.local   # then fill in DATABASE_URL
pnpm dev
```

| Command | What it does |
|---|---|
| `pnpm dev` | Runs the web app at http://localhost:3000 |
| `pnpm build`, `pnpm lint`, `pnpm typecheck`, `pnpm test` | Runs the task in every package through Turborepo |
| `pnpm db:generate` | Generates a SQL migration from the Drizzle schema |
| `pnpm db:migrate` | Applies pending migrations to `DATABASE_URL` |
| `pnpm db:studio` | Opens Drizzle Studio |

`GET /api/health` returns `{ "status": "ok", "db": "ok" }` when the app can reach the database, and a 503 when it can't.

### Environment variables

Every variable is listed with comments in [`apps/web/.env.example`](apps/web/.env.example) and validated by [`apps/web/src/env.ts`](apps/web/src/env.ts). Real values live only in `apps/web/.env.local` (gitignored) and in the host's settings. To build without secrets, as CI does, set `SKIP_ENV_VALIDATION=1`.

## License

[MIT](LICENSE)
