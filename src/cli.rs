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

