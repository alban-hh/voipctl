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
