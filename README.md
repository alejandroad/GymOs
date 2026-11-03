# GymOs

[![CI](https://github.com/alejandroad/GymOs/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/alejandroad/GymOs/actions/workflows/ci.yml?query=branch%3Adev)

Offline-first lifting analytics. Your data lives on your devices, syncs between them if you want, and nobody paywalls your own history.

**Status:** pre-alpha. GymOs runs on your own computer; nothing is hosted.

## Run it

You need [Node 26](https://nodejs.org/) (see `.nvmrc`) and [pnpm 12](https://pnpm.io/installation). Nothing else: no compiler, admin rights or installer.

```bash
git clone https://github.com/alejandroad/GymOs.git
cd GymOs
pnpm install
pnpm start
```

`pnpm start` builds the app, which takes about a minute the first time and is cached after that. It then starts a small local server and prints something like:

```
GymOs is running at http://127.0.0.1:4321
Database: C:\Users\you\AppData\Roaming\GymOs\gymos.sqlite3
```

Open that URL in any browser. Press Ctrl+C to stop; your data is saved.

### Where your data lives

Everything is in one SQLite file in your user's app-data folder, **outside the repo**, so deleting or recloning the GymOs folder never deletes it:

| OS | Folder |
|---|---|
| Windows | `%APPDATA%\GymOs` |
| macOS | `~/Library/Application Support/GymOs` |
| Linux | `$XDG_DATA_HOME/gymos` (usually `~/.local/share/gymos`) |

To back it up, stop GymOs and copy `gymos.sqlite3`.

### Settings

| Variable | Default | Meaning |
|---|---|---|
| `GYMOS_PORT` | `4321` | Port on `127.0.0.1` |
| `GYMOS_DATA_DIR` | the folder above | Where the database lives |

The server only listens on `127.0.0.1`, so other machines can't reach it, and it refuses requests from other websites. Only one GymOs server can use a database at a time.

## Development

```bash
pnpm install
pnpm dev        # UI with hot reload at http://127.0.0.1:4322
pnpm lint
pnpm typecheck
pnpm test
pnpm build
```

`pnpm dev` runs `next dev` behind the local server (port 4322), so the page and its API share one origin as in production. It uses a separate `GymOs-dev` (or `gymos-dev`) data folder, so it never touches your real data, and it can run alongside `pnpm start`.

Layout:

- `apps/web`: the UI, a Next.js static export.
- `apps/server`: the local Node server.
- `packages/store`: the database (schema, migrations, queries).
- `packages/schema`: shared IDs and types.

Works on Windows, macOS and Linux; CI checks all three on every pull request. No telemetry: the root scripts turn off Turbo's and Next.js's.

## Docs

- [`docs/HANDOFF.md`](docs/HANDOFF.md): product spec, decisions and work plan. ADR 0001 supersedes its hosting and browser-storage parts.
- [`docs/platform-support.md`](docs/platform-support.md): supported platforms (Windows first, then macOS) and acceptance criteria.
- [`docs/adr/`](docs/adr/): architecture decision records.
- [`docs/research/`](docs/research/): spikes and method research.

## License

[MIT](LICENSE)
