use super::{customer_mut, display, edit, prefixes};
use crate::{cli::CustomerCommand, model::Customer, storage::Layout};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;

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
                        customer.trunk = value;
                    }
                    if let Some(value) = allow {
                        customer.allowed_prefixes = prefixes(value);
                    }
                    if let Some(value) = default_country {
                        customer.default_country = Some(value);
                    }
                    if clear_default_country {
                        customer.default_country = None;
                    }
                    if let Some(value) = max_calls {
                        customer.max_calls = value;
                    }
                    if let Some(value) = max_seconds {
                        customer.max_call_seconds = value;
                    }
                    if let Some(value) = source_ips {
                        customer.source_ips = value;
                    }
                    if clear_source_ips {
                        customer.source_ips.clear();
                    }
                }
                CustomerCommand::Remove { name, .. } => {
                    let customer = state
                        .config
                        .customers
                        .remove(&name)
                        .context("unknown customer")?;
                    for extension in customer.extensions.keys() {
                        state.secrets.extensions.remove(extension);
                    }
                }
                _ => unreachable!(),
            }
            Ok(())
        }),
    }
}
