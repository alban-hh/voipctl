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
    state.config.pools.insert(
        "us".into(),
        Pool {
            numbers: vec!["+12025550100".into(), "+12025550101".into()],
        },
    );
    state.config.customers.insert(
        "main".into(),
        Customer {
            trunk: "carrier".into(),
            allowed_prefixes: vec!["355".into()],
            default_country: Some("355".into()),
            max_calls: 2,
            max_call_seconds: 3600,
            source_ips: vec!["203.0.113.1/32".parse().unwrap()],
            extensions: BTreeMap::from([(
                "101".into(),
                Extension {
                    caller_id: "+16135550100".into(),
                    alternate_caller_id: Some("pool:us".into()),
                },
            )]),
        },
    );
    state
        .secrets
        .extensions
        .insert("101".into(), new_password());
    state
}

fn rendered(state: &State, name: &str) -> String {
    render::generate(state)
        .unwrap()
        .into_iter()
        .find(|f| f.path == format!("etc/asterisk/{name}"))
        .unwrap()
        .content
}

#[test]
fn allocation_checks_and_reservations_share_one_lock() {
    let text = rendered(&configured_state(), "extensions.conf");
    let lock = text.find("${LOCK(voipctl-allocation)}").unwrap();
    let check = text.find("GROUP_COUNT(main@voipctl-customer)").unwrap();
    let reserve = text.find("Set(GROUP(voipctl-customer)=main)").unwrap();
    let unlock = text.find("${UNLOCK(voipctl-allocation)}").unwrap();
    let dial = text.find("Dial(PJSIP/").unwrap();
    assert!(lock < check && check < reserve && reserve < unlock && unlock < dial);
}

#[test]
fn busy_pool_candidates_advance_until_every_number_has_been_considered() {
    let text = rendered(&configured_state(), "extensions.conf");
    assert!(text.contains("GotoIf($[${ATTEMPTS} >= 2]?done)"));
    assert!(text.contains("Set(INDEX=$[(${INDEX} + 1) % 2])"));
    assert!(text.contains("Set(CID=+12025550100)"));
    assert!(text.contains("Set(CID=+12025550101)"));
    assert!(text.contains("GotoIf($[${AVAILABLE} != 1]?full)"));
}

