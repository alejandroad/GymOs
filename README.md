# GymOs

[![CI](https://github.com/alejandroad/GymOs/actions/workflows/ci.yml/badge.svg?branch=dev)](https://github.com/alejandroad/GymOs/actions/workflows/ci.yml?query=branch%3Adev)

Offline-first lifting analytics. Your data lives on your devices, syncs between them if you want, and nobody paywalls your own history.

**Status:** pre-alpha. Nothing is deployed yet.

## Docs

- [`docs/HANDOFF.md`](docs/HANDOFF.md): product spec, decisions and work plan
- [`docs/platform-support.md`](docs/platform-support.md): supported platforms (Windows and macOS first) and acceptance criteria
- [`docs/pwa-service-worker.md`](docs/pwa-service-worker.md): offline caching, update strategy and the headers a host must send
- [`docs/adr/`](docs/adr/): architecture decision records
- [`docs/research/`](docs/research/): spikes and method research

## Development

Requires Node 26 (see `.nvmrc`) and pnpm 12.

Works on Windows, macOS and Linux; CI checks all three on every pull request.

```bash
pnpm install
pnpm build
pnpm preview   # serves the static export at http://localhost:3000
```

Use `build` + `preview` for anything involving the service worker or the local database. `pnpm dev` is for UI work only.

## License

[MIT](LICENSE)
