//! # gymos-engine
//!
//! The GymOs engine: the `GymOs` struct, the commands that change state, the
//! queries that read it, and the ports (traits) it needs from the outside
//! world (store, clock, ID generator).
//!
//! ## Rules for this crate
//!
//! - **Owns the ports.** The engine defines the traits; adapters such as
//!   `gymos-store` implement them (dependency inversion, DD-03).
//! - **Orchestrates, doesn't decide.** Business rules live in `gymos-domain`.
//!   The engine sequences load, decide and persist inside one transaction.
//! - **Synchronous.** No async runtime here; the shell decides how to keep
//!   the UI responsive.
//! - **Dependencies:** `gymos-domain` only. Never SQLite, never Tauri.
