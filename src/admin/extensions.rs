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
                } => {
                    ensure!(
                        !state.secrets.extensions.contains_key(&number),
                        "extension already exists"
                    );
                    ensure!(!ids.no_cid2, "--no-cid2 is only valid for ext set");
                    let primary = ids.primary().context("provide --cid or --pool")?;
                    customer_mut(state, &customer)?.extensions.insert(
                        number.clone(),
                        Extension {
                            caller_id: primary,
                            alternate_caller_id: ids.alternate(),
                        },
                    );
                    state.secrets.extensions.insert(number, new_password());
                }
                ExtensionCommand::Set {
                    customer,
                    number,
                    ids,
                } => {
                    ensure!(
                        ids.primary().is_some() || ids.alternate().is_some() || ids.no_cid2,
                        "provide a caller ID setting"
                    );
                    let extension = customer_mut(state, &customer)?
                        .extensions
                        .get_mut(&number)
                        .context("unknown extension")?;
                    if let Some(value) = ids.primary() {
                        extension.caller_id = value;
                    }
                    if let Some(value) = ids.alternate() {
                        extension.alternate_caller_id = Some(value);
                    }
                    if ids.no_cid2 {
                        extension.alternate_caller_id = None;
                    }
                }
                ExtensionCommand::Passwd {
                    customer,
                    number,
                    password_stdin,
                } => {
                    ensure!(
                        customer_mut(state, &customer)?
                            .extensions
                            .contains_key(&number),
                        "unknown extension"
                    );
                    let password = if password_stdin {
                        read_stdin()?.trim_end_matches(['\r', '\n']).to_owned()
                    } else {
                        new_password()
                    };
                    validate_secret(&password, 16)?;
                    state.secrets.extensions.insert(number, password);
                }
                ExtensionCommand::Remove {
                    customer, number, ..
                } => {
                    customer_mut(state, &customer)?
                        .extensions
                        .remove(&number)
                        .context("unknown extension")?;
                    state.secrets.extensions.remove(&number);
                }
                _ => unreachable!(),
            }
            Ok(())
        }),
    }
}
