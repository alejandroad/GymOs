# GymOs

Offline-first lifting analytics. Your data lives on your devices, and
nobody paywalls your own history.

**Status:** pre-alpha, rebuilding on Rust + Tauri.

## Stack (planned)

- **Core:** Rust — domain, engine and SQLite store (`rusqlite`)
- **Shell:** Tauri 2 — desktop first (Windows and macOS), phone later
- **UI:** React + TypeScript + Vite, with types generated from Rust

See [`docs/design.md`](docs/design.md) for the full set of design
decisions.

The previous TypeScript/Node local-first spike is preserved at the
[`archive/ts-node-spike`](https://github.com/alejandroad/GymOs/tree/archive/ts-node-spike) tag.

## Docs

- [`docs/design.md`](docs/design.md): design decisions for the Rust + Tauri stack.
- [`docs/adr/`](docs/adr/): architecture decision records.

## License

[MIT](LICENSE)
