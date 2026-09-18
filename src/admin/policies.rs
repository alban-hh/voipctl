use super::{display, edit, prefixes};
use crate::{
    cli::{BlockCommand, ServerOptions},
    storage::Layout,
};
use anyhow::Result;

pub fn block(layout: &Layout, command: BlockCommand, json: bool) -> Result<()> {
    match command {
        BlockCommand::List => display(&layout.load()?.config.blocked_prefixes, json),
        command => edit(layout, json, |state| {
            match command {
                BlockCommand::Add { prefixes: values } => {
                    state.config.blocked_prefixes.extend(prefixes(values))
                }
                BlockCommand::Remove { prefixes: values } => {
                    let values = prefixes(values);
                    state
                        .config
                        .blocked_prefixes
                        .retain(|p| !values.contains(p));
