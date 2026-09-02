use clap::{Args, Parser, Subcommand};
use ipnet::Ipv4Net;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "voipctl",
    version,
    about = "Manage an outbound Asterisk server with validated, recoverable configuration changes"
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        default_value = "/",
        help = "Filesystem root; any other root is an offline workspace"
    )]
    pub root: PathBuf,
    #[arg(long, global = true, help = "Emit machine-readable JSON")]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Initialize a new configuration without changing Asterisk")]
    Init {
        #[arg(long)]
        domain: String,
    },
    #[command(about = "Show configuration without credentials")]
    Show,
    #[command(about = "Validate configuration and activation prerequisites in the data model")]
    Check,
    #[command(about = "Preview changed file names without exposing credentials")]
    Plan,
    #[command(about = "Apply configuration with backups, verification, and automatic recovery")]
    Apply {
        #[arg(
            long,
            help = "Allow replacement of untracked or manually changed generated files"
        )]
        adopt_existing: bool,
        #[arg(
            long,
            help = "Restart Asterisk for transport changes; refused while channels are active"
        )]
        restart: bool,
    },
    #[command(about = "List configuration rollback checkpoints")]
    History,
    #[command(
        about = "Restore generated files from a checkpoint; desired configuration stays staged"
    )]
    Rollback {
        id: String,
        #[arg(long)]
        restart: bool,
    },
    #[command(about = "Recover a transaction interrupted by a crash or failed activation")]
    Recover {
        #[arg(long)]
        restart: bool,
    },
    #[command(about = "Check the installed Asterisk runtime and required modules")]
    Doctor,
    #[command(about = "Show configured resources and live service status")]
