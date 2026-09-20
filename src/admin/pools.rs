use super::{display, edit};
use crate::{cli::PoolCommand, model::Pool, storage::Layout};
use anyhow::{Context, Result, ensure};
use std::fs;

pub fn pool(layout: &Layout, command: PoolCommand, json: bool) -> Result<()> {
    match command {
        PoolCommand::List => display(&layout.load()?.config.pools, json),
        PoolCommand::Show { name } => display(
            layout
                .load()?
                .config
                .pools
                .get(&name)
                .context("unknown pool")?,
            json,
        ),
        PoolCommand::Export { name } => {
            let state = layout.load()?;
            let numbers = &state
                .config
                .pools
