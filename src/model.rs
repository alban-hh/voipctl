use anyhow::{Result, bail, ensure};
use ipnet::Ipv4Net;
use rand::{Rng, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub config: Config,
    #[serde(default)]
    pub secrets: Secrets,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub server: Server,
    #[serde(default)]
    pub trunks: BTreeMap<String, Trunk>,
    #[serde(default)]
    pub customers: BTreeMap<String, Customer>,
    #[serde(default)]
    pub pools: BTreeMap<String, Pool>,
    #[serde(default)]
    pub caller_id_limits: BTreeMap<String, u32>,
    #[serde(default)]
    pub blocked_prefixes: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 1,
            server: Server::default(),
            trunks: BTreeMap::new(),
            customers: BTreeMap::new(),
            pools: BTreeMap::new(),
            caller_id_limits: BTreeMap::new(),
            blocked_prefixes: ["1900", "1976", "870", "881", "882", "883", "979"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Server {
    pub domain: String,
    pub bind_address: std::net::Ipv4Addr,
    pub sip_port: u16,
    pub rtp_start: u16,
    pub rtp_end: u16,
    pub dial_timeout: u32,
    pub max_calls_per_number: u32,
    pub manage_fail2ban: bool,
    pub cdr_database: String,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            domain: "pbx.example.com".into(),
            bind_address: std::net::Ipv4Addr::UNSPECIFIED,
            sip_port: 5060,
            rtp_start: 10000,
            rtp_end: 20000,
            dial_timeout: 60,
            max_calls_per_number: 0,
            manage_fail2ban: false,
            cdr_database: "asteriskcdr".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trunk {
    pub host: String,
    #[serde(default = "default_sip_port")]
    pub port: u16,
    #[serde(default = "default_trunk_limit")]
    pub max_calls: u32,
    #[serde(default)]
    pub signaling: Vec<Ipv4Net>,
    #[serde(default)]
    pub media: Vec<Ipv4Net>,
}

fn default_sip_port() -> u16 {
    5060
}
fn default_trunk_limit() -> u32 {
    100
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Customer {
    pub trunk: String,
    pub allowed_prefixes: Vec<String>,
    #[serde(default)]
    pub default_country: Option<String>,
    pub max_calls: u32,
    pub max_call_seconds: u32,
    #[serde(default)]
    pub source_ips: Vec<Ipv4Net>,
    #[serde(default)]
    pub extensions: BTreeMap<String, Extension>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extension {
    pub caller_id: String,
    #[serde(default)]
    pub alternate_caller_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pool {
    pub numbers: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Secrets {
    pub trunks: BTreeMap<String, Credentials>,
    pub extensions: BTreeMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

pub fn valid_extension(value: &str) -> bool {
    (2..=8).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit())
}

pub fn valid_prefix(value: &str) -> bool {
    (1..=15).contains(&value.len())
        && value.as_bytes()[0] != b'0'
        && value.bytes().all(|b| b.is_ascii_digit())
}

pub fn valid_number(value: &str) -> bool {
    value
        .strip_prefix('+')
        .is_some_and(|digits| (7..=15).contains(&digits.len()) && valid_prefix(digits))
}

pub fn valid_host(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && !part.starts_with('-')
                && !part.ends_with('-')
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

pub fn validate_secret(value: &str, minimum: usize) -> Result<()> {
    ensure!(
        (minimum..=256).contains(&value.len()),
        "credential length must be {minimum}..256 characters"
    );
    ensure!(
        value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_- .@+!$%&*:=?/".contains(&b))
            && !value.contains(' '),
        "credential contains characters unsupported by the Asterisk config writer"
    );
    Ok(())
}

pub fn new_password() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect()
}

impl State {
    pub fn validate(&self) -> Result<()> {
        let config = &self.config;
        ensure!(
            config.schema_version == 1,
            "unsupported configuration schema {}",
            config.schema_version
        );
        ensure!(valid_host(&config.server.domain), "invalid SIP domain");
        ensure!(config.server.sip_port > 0, "SIP port cannot be zero");
        ensure!(
            config.server.rtp_start >= 1024 && config.server.rtp_start < config.server.rtp_end,
            "RTP range must be increasing and start at 1024 or higher"
        );
        ensure!(
            !(config.server.rtp_start..=config.server.rtp_end).contains(&config.server.sip_port),
            "SIP and RTP ports overlap"
        );
        ensure!(
            (1..=300).contains(&config.server.dial_timeout),
            "dial timeout must be 1..300 seconds"
        );
        ensure!(
            config.server.max_calls_per_number <= 1000,
            "per-number limit cannot exceed 1000"
        );
        ensure!(
            valid_name(&config.server.cdr_database),
            "invalid CDR database name"
        );
        for prefix in &config.blocked_prefixes {
            ensure!(valid_prefix(prefix), "invalid blocked prefix");
        }
        for (number, limit) in &config.caller_id_limits {
            ensure!(
                valid_number(number) && (1..=1000).contains(limit),
                "invalid caller ID limit"
            );
        }
        for (name, pool) in &config.pools {
            ensure!(valid_name(name), "invalid pool name {name}");
            ensure!(
                !pool.numbers.is_empty() && pool.numbers.len() <= 1000,
                "pool {name} must contain 1..1000 numbers"
            );
            let unique: BTreeSet<_> = pool.numbers.iter().collect();
            ensure!(
                unique.len() == pool.numbers.len(),
                "pool {name} contains duplicate numbers"
            );
            ensure!(
                pool.numbers.iter().all(|n| valid_number(n)),
                "pool {name} contains an invalid E.164 number"
            );
        }
        for (name, trunk) in &config.trunks {
            ensure!(valid_name(name), "invalid trunk name {name}");
            ensure!(
                valid_host(&trunk.host) && trunk.port > 0,
                "invalid address for trunk {name}"
            );
            ensure!(
                (1..=10000).contains(&trunk.max_calls),
                "trunk {name}: max calls must be 1..10000"
            );
            validate_networks(&trunk.signaling)?;
            validate_networks(&trunk.media)?;
            if let Some(credentials) = self.secrets.trunks.get(name) {
                validate_secret(&credentials.username, 1)?;
                validate_secret(&credentials.password, 1)?;
            }
        }
        let mut extensions = BTreeSet::new();
        for (name, customer) in &config.customers {
            ensure!(valid_name(name), "invalid customer name {name}");
            ensure!(
                config.trunks.contains_key(&customer.trunk),
                "customer {name} refers to a missing trunk"
            );
            ensure!(
                (1..=1000).contains(&customer.max_calls),
                "customer {name}: max calls must be 1..1000"
            );
            ensure!(
                (30..=86400).contains(&customer.max_call_seconds),
                "customer {name}: duration must be 30..86400 seconds"
            );
            ensure!(
                customer.allowed_prefixes.iter().all(|p| valid_prefix(p)),
                "customer {name} has an invalid allowed prefix"
            );
            if let Some(country) = &customer.default_country {
                ensure!(
                    country.len() <= 3 && valid_prefix(country),
                    "invalid default country for {name}"
                );
            }
            validate_networks(&customer.source_ips)?;
            for (number, extension) in &customer.extensions {
                ensure!(valid_extension(number), "invalid extension number {number}");
                ensure!(
                    extensions.insert(number),
                    "extension {number} belongs to multiple customers"
                );
                self.validate_caller_id(&extension.caller_id)?;
                if let Some(alternate) = &extension.alternate_caller_id {
