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
            trunk: "carrier".into(),
            allowed_prefixes: vec!["355".into(), "1".into()],
            default_country: Some("355".into()),
            max_calls: 2,
            max_call_seconds: 3600,
            source_ips: vec![],
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

#[test]
fn dialing_preserves_both_caller_id_choices_and_national_format() {
    let state = state();
    let customer = &state.config.customers["main"];
    for input in [
        "100691234567",
        "10+355691234567",
        "10355691234567",
        "1000355691234567",
        "0691234567",
    ] {
        let call = dialing::resolve(&state, customer, "101", input).unwrap();
        assert_eq!(call.destination, "+355691234567");
        assert_eq!(call.caller_id, "+16135550100");
    }
    assert_eq!(
        dialing::resolve(&state, customer, "101", "110691234567")
            .unwrap()
            .caller_id,
        "pool:us"
    );
}

#[test]
fn malformed_and_disallowed_numbers_are_rejected() {
    let state = state();
    for input in [
        "",
        "10",
        "++355691234567",
        "35+5691234567",
        "10+355 691234567",
        "+19005550100",
        "+49301234567",
    ] {
        assert!(
            dialing::resolve(&state, &state.config.customers["main"], "101", input).is_err(),
            "{input}"
        );
    }
}

#[test]
fn duplicate_extensions_and_missing_pools_are_rejected() {
    let mut state = state();
    state.validate().unwrap();
    state
        .config
        .customers
        .insert("other".into(), state.config.customers["main"].clone());
    assert!(state.validate().is_err());
    state.config.customers.remove("other");
    state.config.pools.clear();
    assert!(state.validate().is_err());
}

#[test]
fn unconfigured_trunks_can_be_staged_but_not_activated() {
    let state = state();
    state.validate().unwrap();
    assert!(state.validate_activation().is_err());
}

#[test]
fn public_configuration_does_not_contain_credentials() {
    let state = state();
    let public = serde_json::to_string(&state.config).unwrap();
    assert!(!public.contains(&state.secrets.extensions["101"]));
    let encoded = toml::to_string_pretty(&state).unwrap();
    let decoded: State = toml::from_str(&encoded).unwrap();
    decoded.validate().unwrap();
}
