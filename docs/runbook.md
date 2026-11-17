# GymOs: Runbook

Commands for working on the Rust workspace day to day. These mirror what
`.github/workflows/ci.yml` runs — if a command passes here, the matching CI
job should pass too.

There is no binary yet: the workspace is domain + engine + store crates
(`docs/design.md` DD-03). "Running" the project today means building and
testing the workspace; a `cargo run` entry point arrives with the Tauri
shell (`apps/`) and `xtask`.

## Setup

```sh
# Installs the pinned toolchain from rust-toolchain.toml (rustfmt, clippy,
# wasm32-unknown-unknown) if it isn't already on this machine.
rustup toolchain install

# Test runner used in CI, not in stock cargo.
cargo install cargo-nextest --locked

# Supply-chain checks (advisories, licenses, bans) used in CI.
cargo install cargo-deny --locked
```

## Build

```sh
# Whole workspace.
cargo build --workspace

# One crate.
cargo build -p gymos-domain

# Fast type/borrow check without codegen — use this while iterating.
cargo check --workspace
```

## Test

```sh
# Everything (unit, integration, golden, property tests), as CI runs it.
cargo nextest run --workspace --all-features --locked

# Doctests — nextest does not run these, so CI checks them separately.
cargo test --workspace --all-features --locked --doc

# One crate.
cargo nextest run -p gymos-domain

# One test by name substring.
cargo nextest run -p gymos-domain progression::verdict

# Golden and property-based tests live in crates/gymos-domain/tests/
# (golden.rs, properties.rs) and run as part of the commands above.
```

## Lint and format

```sh
# Check formatting (CI mode: fails instead of rewriting).
cargo fmt --all --check

# Apply formatting.
cargo fmt --all

# Clippy with the same strictness as CI.
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

# Rustdoc, warnings as errors (catches broken doc links and missing_docs).
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked

# Open the generated docs in a browser.
cargo doc --workspace --no-deps --all-features --open
```

## Portability check (wasm32)

Domain and engine must stay free of OS, filesystem and clock assumptions
("phone last", `docs/design.md` §1). CI compiles them for `wasm32-unknown-unknown`
as proof:

```sh
cargo check -p gymos-domain -p gymos-engine --target wasm32-unknown-unknown --locked
```

## Supply chain

```sh
# Advisories, license policy and banned crates (deny.toml).
cargo deny check

# Just one category, e.g. while triaging a new advisory.
cargo deny check advisories
```

## Everything, before opening a PR

```sh
cargo fmt --all --check \
  && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings \
  && cargo nextest run --workspace --all-features --locked \
  && cargo test --workspace --all-features --locked --doc \
  && cargo deny check
```

## Dependencies

```sh
# Add a dependency to one crate (version lives in [workspace.dependencies]
# per the root Cargo.toml convention — add it there first, then reference
# it with `name.workspace = true` in the crate's Cargo.toml).
cargo add <crate> -p gymos-domain

# Update the lockfile within existing version constraints.
cargo update

# Check for outdated dependencies (requires cargo-outdated).
cargo outdated --workspace
```

## Commit messages

CI enforces Conventional Commits on PRs (`type(scope)?: subject`, lowercase
type, no space before the colon): `feat`, `fix`, `docs`, `style`, `refactor`,
`perf`, `test`, `build`, `ci`, `chore`, `revert`.
