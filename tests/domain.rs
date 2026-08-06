use std::collections::BTreeMap;
use voipctl::{dialing, model::*};

fn state() -> State {
    let mut state = State::default();
    state.config.trunks.insert(
        "carrier".into(),
        Trunk {
            host: "sip.example.com".into(),
            port: 5060,
            max_calls: 10,
            signaling: vec![],
            media: vec![],
        },
    );
    state.config.pools.insert(
        "us".into(),
        Pool {
            numbers: vec!["+12025550100".into()],
        },
    );
    state.config.customers.insert(
        "main".into(),
        Customer {
