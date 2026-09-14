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
