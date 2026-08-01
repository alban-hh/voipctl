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

