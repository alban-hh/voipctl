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
        .ok_or_else(|| anyhow::anyhow!("unknown extension"))?;
    let (raw, caller_id) = if let Some(raw) = dialed.strip_prefix("11") {
        (
            raw,
            endpoint
                .alternate_caller_id
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("extension has no alternate caller ID"))?,
        )
    } else {
        (
            dialed.strip_prefix("10").unwrap_or(dialed),
            &endpoint.caller_id,
        )
    };
    let destination = normalize(raw, customer.default_country.as_deref())?;
    let digits = &destination[1..];
    if state
        .config
        .blocked_prefixes
        .iter()
        .any(|p| digits.starts_with(p))
    {
        bail!("destination is globally blocked");
    }
    ensure!(
        customer
