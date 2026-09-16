use super::{customer_mut, display, edit, read_stdin};
use crate::{
    cli::ExtensionCommand,
    model::{Extension, new_password, validate_secret},
    storage::Layout,
};
use anyhow::{Context, Result, ensure};

pub fn extension(layout: &Layout, command: ExtensionCommand, json: bool) -> Result<()> {
    match command {
        ExtensionCommand::List { customer } => display(
            &layout
                .load()?
                .config
                .customers
                .get(&customer)
                .context("unknown customer")?
                .extensions,
            json,
        ),
        ExtensionCommand::Show {
            customer,
            number,
            reveal,
        } => {
            let state = layout.load()?;
            let extension = state
                .config
                .customers
                .get(&customer)
                .context("unknown customer")?
                .extensions
                .get(&number)
                .context("unknown extension")?;
            display(
                &serde_json::json!({
                    "server":state.config.server.domain, "port":state.config.server.sip_port, "transport":"udp", "username":number,
                    "caller_id":extension.caller_id, "alternate_caller_id":extension.alternate_caller_id,
                    "password":if reveal { state.secrets.extensions[&number].as_str() } else { "[redacted; use --reveal]" },
                }),
                json,
            )
        }
        other => edit(layout, json, |state| {
            match other {
                ExtensionCommand::Add {
                    customer,
                    number,
                    ids,
