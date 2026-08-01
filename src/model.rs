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
