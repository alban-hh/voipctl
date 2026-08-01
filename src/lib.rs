pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
pub mod dialing;
pub mod model;
