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

