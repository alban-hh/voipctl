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

#[derive(Subcommand)]
pub enum ExtensionCommand {
    List {
        customer: String,
    },
    Show {
        customer: String,
        number: String,
        #[arg(long, help = "Explicitly print the SIP password")]
        reveal: bool,
    },
    Add {
        customer: String,
        number: String,
        #[command(flatten)]
        ids: CallerIds,
    },
    Set {
        customer: String,
        number: String,
        #[command(flatten)]
        ids: CallerIds,
    },
    Passwd {
        customer: String,
        number: String,
        #[arg(
            long,
            help = "Read a password from standard input instead of generating one"
        )]
        password_stdin: bool,
    },
    Remove {
        customer: String,
        number: String,
        #[arg(long, required = true)]
        yes: bool,
    },
}

#[derive(Args)]
pub struct CallerIds {
    #[arg(long, conflicts_with = "pool")]
    pub cid: Option<String>,
    #[arg(long)]
    pub pool: Option<String>,
    #[arg(long, conflicts_with_all = ["pool2", "no_cid2"])]
    pub cid2: Option<String>,
    #[arg(long, conflicts_with = "no_cid2")]
    pub pool2: Option<String>,
    #[arg(long)]
    pub no_cid2: bool,
}

impl CallerIds {
    pub fn primary(&self) -> Option<String> {
        self.cid
            .clone()
            .or_else(|| self.pool.as_ref().map(|p| format!("pool:{p}")))
    }
    pub fn alternate(&self) -> Option<String> {
        self.cid2
            .clone()
            .or_else(|| self.pool2.as_ref().map(|p| format!("pool:{p}")))
    }
}

#[derive(Subcommand)]
pub enum TrunkCommand {
    List,
    Show {
        name: String,
    },
    Add {
        name: String,
        #[arg(long)]
        host: String,
        #[arg(long, default_value_t = 5060)]
        port: u16,
        #[arg(long, default_value_t = 100)]
        max_calls: u32,
        #[arg(long)]
        signaling: Vec<Ipv4Net>,
        #[arg(long)]
        media: Vec<Ipv4Net>,
    },
    Set {
        name: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        max_calls: Option<u32>,
        #[arg(long)]
        signaling: Option<Vec<Ipv4Net>>,
        #[arg(long)]
        media: Option<Vec<Ipv4Net>>,
    },
    Credentials {
        name: String,
        #[arg(
            long,
            help = "Read a JSON object with username and password from standard input"
        )]
        stdin: bool,
    },
    Remove {
        name: String,
        #[arg(long, required = true)]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum PoolCommand {
    List,
    Show {
        name: String,
    },
    Add {
        name: String,
        #[arg(required = true)]
        numbers: Vec<String>,
    },
    Import {
        name: String,
        file: PathBuf,
    },
    Export {
        name: String,
    },
    Remove {
        name: String,
        numbers: Vec<String>,
        #[arg(long)]
        yes: bool,
    },
    Limit {
        name: String,
        #[arg(help = "Concurrent calls per number, shared across all customers")]
        calls: u32,
    },
}

#[derive(Subcommand)]
pub enum BlockCommand {
    List,
    Add {
        #[arg(required = true)]
        prefixes: Vec<String>,
    },
    Remove {
        #[arg(required = true)]
        prefixes: Vec<String>,
    },
}

#[derive(Args)]
pub struct ServerOptions {
    #[arg(long)]
    pub domain: Option<String>,
    #[arg(long)]
    pub bind_address: Option<std::net::Ipv4Addr>,
    #[arg(long)]
    pub sip_port: Option<u16>,
    #[arg(long)]
    pub rtp_start: Option<u16>,
    #[arg(long)]
    pub rtp_end: Option<u16>,
    #[arg(long)]
    pub dial_timeout: Option<u32>,
    #[arg(long)]
    pub max_calls_per_number: Option<u32>,
    #[arg(long, action = clap::ArgAction::Set)]
    pub manage_fail2ban: Option<bool>,
    #[arg(long)]
    pub cdr_database: Option<String>,
}
