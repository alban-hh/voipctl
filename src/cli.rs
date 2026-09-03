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
    Status,
    #[command(about = "Import the original Python CLI's /etc/voip directory")]
    Migrate {
        #[arg(long)]
        from: PathBuf,
        #[arg(long)]
        domain: String,
    },
    #[command(about = "Explain routing and caller ID selection without placing a call")]
    DialCheck {
        customer: String,
        extension: String,
        number: String,
    },
    #[command(subcommand)]
    Customer(CustomerCommand),
    #[command(subcommand, name = "ext")]
    Extension(ExtensionCommand),
    #[command(subcommand)]
    Trunk(TrunkCommand),
    #[command(subcommand)]
    Pool(PoolCommand),
    #[command(subcommand)]
    Block(BlockCommand),
    #[command(about = "Change server settings; values remain staged until apply")]
    Server(ServerOptions),
    #[command(about = "Print cloud firewall requirements; never change host or cloud firewalls")]
    Firewall {
        #[arg(long)]
        admin: Vec<Ipv4Net>,
    },
    #[command(about = "Read the existing MariaDB CDR database")]
    Cdr {
        #[arg(long)]
        customer: Option<String>,
        #[arg(long)]
        today: bool,
        #[arg(long)]
        summary: bool,
        #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=10000))]
        limit: u32,
    },
    #[command(about = "Generate shell completion definitions")]
    Completions { shell: clap_complete::Shell },
}

#[derive(Subcommand)]
pub enum CustomerCommand {
    List,
    Show {
        name: String,
    },
    Add {
        name: String,
        #[arg(long)]
        trunk: String,
        #[arg(long, value_delimiter = ',', required = true)]
        allow: Vec<String>,
        #[arg(long)]
        default_country: Option<String>,
        #[arg(long, default_value_t = 2)]
        max_calls: u32,
        #[arg(long, default_value_t = 3600)]
        max_seconds: u32,
        #[arg(long = "source-ip")]
        source_ips: Vec<Ipv4Net>,
    },
    Set {
        name: String,
        #[arg(long)]
        trunk: Option<String>,
        #[arg(long, value_delimiter = ',')]
        allow: Option<Vec<String>>,
        #[arg(long)]
        default_country: Option<String>,
        #[arg(long, conflicts_with = "default_country")]
        clear_default_country: bool,
        #[arg(long)]
        max_calls: Option<u32>,
        #[arg(long)]
        max_seconds: Option<u32>,
        #[arg(long = "source-ip", conflicts_with = "clear_source_ips")]
        source_ips: Option<Vec<Ipv4Net>>,
        #[arg(long)]
        clear_source_ips: bool,
    },
    Remove {
        name: String,
        #[arg(long, required = true)]
        yes: bool,
    },
}

