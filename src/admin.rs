use crate::{cli::*, model::*, storage::Layout};
use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, IsTerminal, Read},
};

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

fn customer_mut<'a>(state: &'a mut State, name: &str) -> Result<&'a mut Customer> {
    state
        .config
        .customers
        .get_mut(name)
        .with_context(|| format!("unknown customer {name}"))
}

pub fn customer(layout: &Layout, command: CustomerCommand, json: bool) -> Result<()> {
    match command {
        CustomerCommand::List => display(&layout.load()?.config.customers, json),
        CustomerCommand::Show { name } => display(
            layout
                .load()?
                .config
                .customers
                .get(&name)
                .context("unknown customer")?,
            json,
        ),
        other => edit(layout, json, |state| {
            match other {
                CustomerCommand::Add {
                    name,
                    trunk,
                    allow,
                    default_country,
                    max_calls,
                    max_seconds,
                    source_ips,
                } => {
                    ensure!(
                        !state.config.customers.contains_key(&name),
                        "customer already exists"
                    );
                    state.config.customers.insert(
                        name,
                        Customer {
                            trunk,
                            allowed_prefixes: prefixes(allow),
                            default_country,
                            max_calls,
                            max_call_seconds: max_seconds,
                            source_ips,
                            extensions: BTreeMap::new(),
                        },
                    );
                }
                CustomerCommand::Set {
                    name,
                    trunk,
                    allow,
                    default_country,
                    clear_default_country,
                    max_calls,
                    max_seconds,
                    source_ips,
                    clear_source_ips,
                } => {
                    let customer = customer_mut(state, &name)?;
                    if let Some(value) = trunk {
