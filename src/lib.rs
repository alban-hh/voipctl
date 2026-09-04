pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
pub mod admin;
pub mod app;
pub mod cli;
pub mod dialing;
pub mod migrate;
pub mod model;
pub mod render;
pub mod services;
pub mod storage;
pub mod transaction;
