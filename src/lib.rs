// rustc 1.94+ trips a query-depth overflow when computing async layouts in
// the matrix-sdk timeline future graph. matrix-rust-sdk PR #6489 raises the
// limit, but `recursion_limit` is per-crate and applies to the crate currently
// being compiled — so the consumer has to repeat it.
#![recursion_limit = "256"]

mod agent;
mod bot;
mod controller;
mod conversation;
mod entity;
mod strings;
mod utils;

pub use bot::{Bot, load_config};
pub use entity::cfg::Config;
