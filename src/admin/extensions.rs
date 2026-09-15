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
