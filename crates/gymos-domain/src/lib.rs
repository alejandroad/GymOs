//! # gymos-domain
//!
//! The pure core of GymOs: value types (newtypes such as reps and loads),
//! the set and session model, and the progression and PR rules.
//!
//! ## Rules for this crate
//!
//! - **Pure.** No I/O, no clock, no randomness, no environment. Every rule is
//!   a function from data to data, so it is deterministic and testable
//!   without infrastructure (product D9, DD-03).
//! - **Parse, don't validate.** Every value is constructed through a fallible
//!   constructor, so invalid states are unrepresentable (DD-08).
//! - **No wire formats.** Types here derive no `serde` or `specta`; the IPC,
//!   database and archive boundaries own their own representations.
//! - **Dependencies:** `std` and small pure crates only. Never `rusqlite`,
//!   `tauri` or anything that touches the OS.
