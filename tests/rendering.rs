use std::collections::BTreeMap;
use voipctl::{model::*, render, storage::Layout, transaction};

fn configured_state() -> State {
    let mut state = State::default();
    state.config.trunks.insert(
        "carrier".into(),
        Trunk {
            host: "sip.example.com".into(),
            port: 5060,
            max_calls: 10,
            signaling: vec!["192.0.2.1/32".parse().unwrap()],
            media: vec![],
        },
    );
    state.secrets.trunks.insert(
        "carrier".into(),
        Credentials {
            username: "testuser".into(),
            password: new_password(),
        },
    );
