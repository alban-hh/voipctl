pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
pub mod dialing;
pub mod model;
pub mod render;
pub mod services;
pub mod storage;
pub mod transaction;
