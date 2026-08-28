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
        return Ok(());
    }
    let layout = Layout::new(cli.root)?;
    let json = cli.json;
    match cli.command {
        Command::Init { domain } => {
            let _lock = layout.lock()?;
            ensure!(
                !layout.state_path()?.exists(),
                "configuration already exists"
            );
            let mut state = State::default();
            state.config.server.domain = domain;
            layout.save(&state)?;
            message(
                "Initialized. Add a trunk, its credentials, a customer, and extensions before applying.",
                json,
            )
        }
        Command::Migrate { from, domain } => {
            let _lock = layout.lock()?;
            ensure!(
                !layout.state_path()?.exists(),
                "migration requires an empty workspace"
            );
            let state = migrate::legacy(&from, domain)?;
            layout.save(&state)?;
            message(
                "Legacy configuration imported; source files unchanged. Review show and plan before apply --adopt-existing --restart.",
                json,
            )
        }
        Command::Show => display(&layout.load()?.config, json),
        Command::Check => {
            layout.load()?.validate_activation()?;
            message(
                "Configuration and credential references are valid. Run doctor on the target host to check its runtime.",
                json,
            )
        }
        Command::Plan => {
            let artifacts = render::generate(&layout.load()?)?;
            display(&transaction::plan(&layout, &artifacts)?, json)
        }
        Command::Apply {
            adopt_existing,
