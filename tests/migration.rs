use std::fs;
use voipctl::{migrate, model::new_password, render};

fn legacy_fixture() -> (tempfile::TempDir, String) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir(root.join("customers")).unwrap();
    fs::create_dir(root.join("cidpools")).unwrap();
    fs::create_dir(root.join("secrets")).unwrap();
    let password = new_password();
    fs::write(
        root.join("telnyx.conf"),
        "host sip.example.com:5060\nsignaling 192.0.2.1\nmedia 198.51.100.0/24\n",
    )
    .unwrap();
    fs::write(
        root.join("secrets/telnyx.conf"),
        format!("[telnyx]\nusername=testuser\npassword={password}\n"),
    )
    .unwrap();
    fs::write(
        root.join("cidpools/us.txt"),
        "# test numbers\n+12025550100\n+12025550101\n",
    )
    .unwrap();
    fs::write(root.join("customers/main.conf"), format!("[customer]\nmax_calls=2\nmax_call_seconds=3600\nallowed_prefixes=1 355 52 508\ndefault_country=355\n\n[ext 101]\npassword={password}\ncid=+16135550100\ncid2=pool:us\n")).unwrap();
    fs::write(
        root.join("whitelist.conf"),
        "203.0.113.1 admin\n203.0.113.2 customer:main\n",
    )
    .unwrap();
    fs::write(root.join("blocked_prefixes.conf"), "1900\n870\n").unwrap();
    (directory, password)
}

#[test]
fn migration_preserves_credentials_and_calling_policies() {
    let (directory, password) = legacy_fixture();
    let original = fs::read(directory.path().join("customers/main.conf")).unwrap();
    let state = migrate::legacy(directory.path(), "pbx.example.com".into()).unwrap();
    assert_eq!(state.secrets.extensions["101"], password);
    assert_eq!(state.secrets.trunks["telnyx"].password, password);
    assert_eq!(state.config.customers["main"].max_calls, 2);
    assert_eq!(
        state.config.customers["main"].default_country.as_deref(),
        Some("355")
    );
    assert_eq!(
        state.config.customers["main"].source_ips[0].to_string(),
        "203.0.113.2/32"
    );
    assert_eq!(
        state.config.customers["main"].extensions["101"]
            .alternate_caller_id
            .as_deref(),
        Some("pool:us")
    );
    assert_eq!(state.config.pools["us"].numbers.len(), 2);
    assert_eq!(
        fs::read(directory.path().join("customers/main.conf")).unwrap(),
        original
    );
    state.validate_activation().unwrap();
    render::generate(&state).unwrap();
}

