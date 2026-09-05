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
            "telnyx".into(),
            Credentials {
                username: required(values, "username")?.into(),
                password: required(values, "password")?.into(),
            },
        );
    }
    let pools = directory.join("cidpools");
    if pools.is_dir() {
        for entry in fs::read_dir(pools)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("txt") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("invalid legacy pool filename")?
                .to_owned();
            let mut numbers: Vec<_> = fs::read_to_string(path)?
                .lines()
                .map(body)
                .filter(|s| !s.is_empty())
                .map(|s| s.replace(' ', ""))
                .collect();
            numbers.sort();
            numbers.dedup();
            state.config.pools.insert(name, Pool { numbers });
        }
    }
    let customers = directory.join("customers");
    if customers.is_dir() {
        for entry in fs::read_dir(customers)? {
            let path = entry?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("conf") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("invalid legacy customer filename")?
                .to_owned();
            let sections = ini(&path)?;
            let values = sections
                .get("customer")
                .context("missing legacy customer section")?;
            let mut customer = Customer {
                trunk: "telnyx".into(),
                max_calls: values
                    .get("max_calls")
                    .map(String::as_str)
                    .unwrap_or("2")
                    .parse()?,
                max_call_seconds: values
                    .get("max_call_seconds")
                    .map(String::as_str)
                    .unwrap_or("3600")
                    .parse()?,
                allowed_prefixes: values
                    .get("allowed_prefixes")
                    .cloned()
                    .unwrap_or_default()
                    .replace(',', " ")
                    .split_whitespace()
                    .map(|s| s.trim_start_matches('+').to_owned())
                    .collect(),
                default_country: values
                    .get("default_country")
                    .filter(|v| !v.is_empty())
                    .cloned(),
                source_ips: vec![],
                extensions: BTreeMap::new(),
            };
            for (section, values) in &sections {
                if section == "customer" {
                    continue;
                }
                let number = section
                    .strip_prefix("ext ")
                    .context("unrecognized legacy section")?
                    .trim()
                    .to_owned();
                ensure!(
                    !state.secrets.extensions.contains_key(&number),
                    "duplicate legacy extension"
                );
                customer.extensions.insert(
                    number.clone(),
                    Extension {
                        caller_id: required(values, "cid")?.into(),
                        alternate_caller_id: values.get("cid2").filter(|v| !v.is_empty()).cloned(),
                    },
