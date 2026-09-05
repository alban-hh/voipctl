use crate::model::*;
use anyhow::{Context, Result, bail, ensure};
use ipnet::Ipv4Net;
use std::{collections::BTreeMap, fs, path::Path};

type Sections = BTreeMap<String, BTreeMap<String, String>>;

pub fn legacy(directory: &Path, domain: String) -> Result<State> {
    ensure!(directory.is_dir(), "legacy source must be a directory");
    let mut state = State::default();
    state.config.server.domain = domain;
    let mut trunk = Trunk {
        host: "sip.telnyx.com".into(),
        port: 5060,
        max_calls: 100,
        signaling: vec![],
        media: vec![],
    };
    for raw in fs::read_to_string(directory.join("telnyx.conf"))
        .context("missing legacy telnyx.conf")?
        .lines()
    {
        let words: Vec<&str> = body(raw).split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        ensure!(words.len() == 2, "invalid legacy trunk configuration");
        match words[0] {
            "host" => {
                let (host, port) = words[1].split_once(':').unwrap_or((words[1], "5060"));
                trunk.host = host.into();
                trunk.port = port.parse()?;
            }
            "signaling" => trunk.signaling.push(network(words[1])?),
            "media" => trunk.media.push(network(words[1])?),
            _ => bail!("unknown legacy trunk setting"),
        }
    }
    state.config.trunks.insert("telnyx".into(), trunk);
    let credentials = directory.join("secrets/telnyx.conf");
    if credentials.exists() {
        let sections = ini(&credentials)?;
        let values = sections
            .get("telnyx")
            .context("missing legacy credential section")?;
        state.secrets.trunks.insert(
