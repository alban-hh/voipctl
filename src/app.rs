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
            restart,
        } => {
            let _lock = layout.lock()?;
            let state = layout.load()?;
            let artifacts = render::generate(&state)?;
            let id = if layout.live() {
                transaction::apply(
                    &layout,
                    &artifacts,
                    adopt_existing,
                    &mut LiveServices::new(&layout, &state, restart)?,
                )?
            } else {
                transaction::apply(&layout, &artifacts, adopt_existing, &mut Offline)?
            };
            display(
                &json!({"result":"applied", "checkpoint":id, "mode":if layout.live() { "live" } else { "offline" }}),
                json,
            )
        }
        Command::Recover { restart } => {
            let _lock = layout.lock()?;
            if layout.live() {
                let state = layout.load()?;
                let mut services = LiveServices::new(&layout, &state, restart)?;
                transaction::recover(&layout, &mut services)?;
            } else {
                transaction::recover(&layout, &mut Offline)?;
            }
            message(
                "Previous generated files restored. Desired configuration remains staged.",
                json,
            )
        }
        Command::History => {
            let directory = layout.path("var/lib/voipctl/history")?;
            let mut entries = Vec::new();
            if directory.exists() {
                for entry in fs::read_dir(directory)? {
                    let path = entry?.path();
                    if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                        entries.push(id.to_owned());
                    }
                }
            }
            entries.sort();
            display(&entries, json)
        }
        Command::Rollback { id, restart } => {
            let _lock = layout.lock()?;
            let redo = if layout.live() {
                ensure!(
                    restart,
                    "live rollback requires --restart in a maintenance window"
                );
                let state = layout.load()?;
                let mut services = LiveServices::new(&layout, &state, true)?;
                services.preflight(&render::generate(&state)?)?;
                transaction::rollback(&layout, &id, &mut services)?
            } else {
                transaction::rollback(&layout, &id, &mut Offline)?
            };
            display(
                &json!({"result":"rolled_back", "undo_checkpoint":redo, "desired_configuration":"unchanged; review plan before next apply"}),
                json,
            )
        }
        Command::Doctor => {
            let state = layout.load()?;
            let artifacts = render::generate(&state)?;
            if layout.live() {
                LiveServices::new(&layout, &state, false)?.preflight(&artifacts)?;
            }
            display(
                &json!({"configuration":"valid", "runtime":if layout.live() { "checked" } else { "not checked in offline mode" }, "generated_files":artifacts.len()}),
                json,
            )
        }
        Command::Status => status(&layout, json),
        Command::DialCheck {
            customer,
            extension,
            number,
        } => {
            let state = layout.load()?;
            let customer = state
                .config
                .customers
                .get(&customer)
                .context("unknown customer")?;
            display(
                &dialing::resolve(&state, customer, &extension, &number)?,
                json,
            )
        }
        Command::Customer(command) => admin::customer(&layout, command, json),
        Command::Extension(command) => admin::extension(&layout, command, json),
        Command::Trunk(command) => admin::trunk(&layout, command, json),
        Command::Pool(command) => admin::pool(&layout, command, json),
        Command::Block(command) => admin::block(&layout, command, json),
        Command::Server(options) => admin::server(&layout, options, json),
        Command::Firewall { admin } => {
            let state = layout.load()?;
            let clients: Vec<_> = state
                .config
                .customers
                .values()
                .flat_map(|c| c.source_ips.iter().map(ToString::to_string))
                .collect();
            let signaling: Vec<_> = state
                .config
                .trunks
                .values()
                .flat_map(|t| t.signaling.iter().map(ToString::to_string))
                .collect();
            let media: Vec<_> = state
                .config
                .trunks
                .values()
                .flat_map(|t| t.media.iter().map(ToString::to_string))
                .collect();
            display(
                &json!({"policy":"cloud firewall: default-deny inbound; explicitly allow these sources; no changes made",
                "ssh":{"protocol":"tcp","port":22,"sources":admin},
                "sip":{"protocol":"udp","port":state.config.server.sip_port,"customer_sources":clients,"carrier_sources":signaling},
                "rtp":{"protocol":"udp","start":state.config.server.rtp_start,"end":state.config.server.rtp_end,"customer_sources":clients,"carrier_sources":media},
                "note":"Customers without optional source locks need their source addresses supplied directly in the cloud firewall. Review current carrier ranges before deployment."}),
                json,
            )
        }
        Command::Cdr {
            customer,
            today,
            summary,
            limit,
        } => cdr(&layout, customer, today, summary, limit, json),
        Command::Completions { .. } => unreachable!(),
    }
}

fn status(layout: &Layout, json_output: bool) -> Result<()> {
    let state = layout.load()?;
    let mut result = json!({"customers":state.config.customers.len(), "extensions":state.config.customers.values().map(|c| c.extensions.len()).sum::<usize>(),
        "trunks":state.config.trunks.len(), "pools":state.config.pools.len(), "mode":if layout.live(){"live"}else{"offline"},
        "recovery_pending":layout.path("var/lib/voipctl/pending.json")?.exists()});
    if layout.live() {
        result["asterisk"] = json!(
            services::command("/usr/bin/systemctl", &["is-active", "asterisk"])
                .map(|s| s.trim().to_owned())
                .unwrap_or_else(|_| "unavailable".into())
        );
        result["calls"] = json!(
            services::asterisk("core show channels count")
                .map(|s| s.trim().to_owned())
                .unwrap_or_else(|_| "unavailable".into())
        );
    }
    display(&result, json_output)
}

fn cdr(
    layout: &Layout,
    customer: Option<String>,
    today: bool,
    summary: bool,
    limit: u32,
    json_output: bool,
) -> Result<()> {
    ensure!(
        layout.live(),
        "CDR queries require the live database on --root /"
    );
    let state = layout.load()?;
    let mut conditions = Vec::new();
    if let Some(name) = customer {
        ensure!(valid_name(&name), "invalid customer name");
        conditions.push(format!("accountcode='{name}'"));
    }
    if today {
        conditions.push("calldate >= CURDATE()".into());
    }
    let filter = if conditions.is_empty() {
        String::new()
