use crate::{
    admin::{self, display, message},
    cli::{Cli, Command},
    dialing, migrate,
    model::{State, valid_name},
    render,
    services::{self, LiveServices},
    storage::Layout,
    transaction::{self, Activator, Offline},
};
use anyhow::{Context, Result, ensure};
use clap::CommandFactory;
use serde_json::json;
use std::fs;

pub fn run(cli: Cli) -> Result<()> {
    if let Command::Completions { shell } = cli.command {
        clap_complete::generate(
            shell,
            &mut Cli::command(),
            "voipctl",
            &mut std::io::stdout(),
        );
