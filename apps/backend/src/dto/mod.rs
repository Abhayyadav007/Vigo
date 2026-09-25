//! Request/response types shared with the TypeScript clients.
//!
//! Every type here derives `ts_rs::TS` with `#[ts(export)]`. Running
//! `cargo test export_bindings` writes them to `packages/types/src/bindings/`
//! (the directory is set via `TS_RS_EXPORT_DIR` in `.cargo/config.toml`).

pub mod admin;
pub mod auth;
pub mod catalog;
pub mod customer;
pub mod error;
pub mod geo;
pub mod health;
pub mod page;
pub mod store;
