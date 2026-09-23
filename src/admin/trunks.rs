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
