//! Vigo backend library: `main.rs` is a thin binary over this, and
//! integration tests in `tests/` build the router from here.

pub mod app;
pub mod auth;
pub mod cache;
pub mod config;
pub mod dto;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod state;
pub mod ws;
