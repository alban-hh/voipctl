use crate::model::{Customer, State, valid_number};
use anyhow::{Result, bail, ensure};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DialDecision {
    pub destination: String,
    pub caller_id: String,
    pub trunk: String,
}

pub fn resolve(
    state: &State,
    customer: &Customer,
    extension: &str,
    dialed: &str,
) -> Result<DialDecision> {
    ensure!(
        !dialed.is_empty() && dialed.len() <= 20,
        "invalid dialed number length"
    );
    ensure!(
        dialed.bytes().all(|b| b.is_ascii_digit() || b == b'+'),
        "dial using digits and an optional leading + only"
    );
    let endpoint = customer
        .extensions
        .get(extension)
