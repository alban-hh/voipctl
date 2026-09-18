mod customers;
mod extensions;
mod policies;
mod pools;
mod trunks;

pub use customers::customer;
pub use extensions::extension;
pub use policies::{block, server};
pub use pools::pool;
pub use trunks::trunk;

use crate::{
    model::{Customer, State},
    storage::Layout,
};
use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use std::io::{self, Read};

pub fn display<T: Serialize>(value: &T, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!(
            "{}",
            toml::to_string_pretty(value).unwrap_or(serde_json::to_string_pretty(value)?)
        );
    }
    Ok(())
}

pub fn message(text: &str, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::json!({"message":text}));
    } else {
        println!("{text}");
    }
    Ok(())
}

pub fn edit(
    layout: &Layout,
    json: bool,
    mutation: impl FnOnce(&mut State) -> Result<()>,
) -> Result<()> {
    let _lock = layout.lock()?;
    ensure!(
        !layout.path("var/lib/voipctl/pending.json")?.exists(),
        "recover the interrupted transaction before editing configuration"
    );
    let mut state = layout.load()?;
    mutation(&mut state)?;
    layout.save(&state)?;
    message(
        "Configuration saved. Run plan, then apply to activate it.",
        json,
    )
}

pub(super) fn customer_mut<'a>(state: &'a mut State, name: &str) -> Result<&'a mut Customer> {
    state
        .config
        .customers
        .get_mut(name)
        .with_context(|| format!("unknown customer {name}"))
}
pub(super) fn prefixes(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|v| v.trim_start_matches('+').to_owned())
        .collect()
}

pub(super) fn read_stdin() -> Result<String> {
    let mut text = String::new();
    io::stdin().take(65537).read_to_string(&mut text)?;
    if text.len() > 65536 {
        bail!("credential input exceeds 64 KiB");
    }
    Ok(text)
}
