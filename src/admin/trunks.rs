use super::{display, edit, read_stdin};
use crate::{
    cli::TrunkCommand,
    model::{Credentials, Trunk},
    storage::Layout,
};
use anyhow::{Context, Result, ensure};
use std::io::{self, IsTerminal};

pub fn trunk(layout: &Layout, command: TrunkCommand, json: bool) -> Result<()> {
    match command {
        TrunkCommand::List => display(&layout.load()?.config.trunks, json),
        TrunkCommand::Show { name } => display(
            layout
                .load()?
                .config
                .trunks
                .get(&name)
                .context("unknown trunk")?,
            json,
        ),
        other => edit(layout, json, |state| {
            match other {
                TrunkCommand::Add {
                    name,
                    host,
                    port,
                    max_calls,
                    signaling,
                    media,
                } => {
                    ensure!(
                        !state.config.trunks.contains_key(&name),
                        "trunk already exists"
                    );
                    state.config.trunks.insert(
                        name,
                        Trunk {
                            host,
                            port,
                            max_calls,
                            signaling,
                            media,
                        },
                    );
                }
                TrunkCommand::Set {
                    name,
                    host,
                    port,
                    max_calls,
                    signaling,
                    media,
                } => {
                    let trunk = state
                        .config
                        .trunks
                        .get_mut(&name)
                        .context("unknown trunk")?;
                    if let Some(value) = host {
                        trunk.host = value;
                    }
                    if let Some(value) = port {
                        trunk.port = value;
                    }
                    if let Some(value) = max_calls {
                        trunk.max_calls = value;
                    }
                    if let Some(value) = signaling {
                        trunk.signaling = value;
                    }
                    if let Some(value) = media {
                        trunk.media = value;
                    }
                }
                TrunkCommand::Credentials { name, stdin } => {
                    ensure!(state.config.trunks.contains_key(&name), "unknown trunk");
                    let credentials = if stdin {
                        serde_json::from_str::<Credentials>(&read_stdin()?).map_err(|_| {
                            anyhow::anyhow!(
                                "expected JSON with username and password; values omitted"
                            )
                        })?
                    } else {
                        ensure!(
                            io::stdin().is_terminal(),
                            "use --stdin for noninteractive credentials"
                        );
