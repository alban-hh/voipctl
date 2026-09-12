use clap::Parser;
use std::fs;
use voipctl::{app, cli::Cli, model::State, storage::Layout};

fn invoke(layout: &Layout, args: &[&str]) -> anyhow::Result<()> {
    let mut command = vec!["voipctl", "--root", layout.root.to_str().unwrap(), "--json"];
    command.extend(args);
    app::run(Cli::try_parse_from(command)?)
}

fn workspace() -> (tempfile::TempDir, Layout) {
    let directory = tempfile::tempdir().unwrap();
    let layout = Layout::new(directory.path().to_owned()).unwrap();
    invoke(&layout, &["init", "--domain", "pbx.example.com"]).unwrap();
    invoke(
        &layout,
        &["trunk", "add", "carrier", "--host", "sip.example.com"],
    )
    .unwrap();
    invoke(
        &layout,
        &[
            "customer",
            "add",
            "main",
            "--trunk",
            "carrier",
            "--allow",
            "355,1",
            "--default-country",
            "355",
        ],
    )
    .unwrap();
    (directory, layout)
}

#[test]
fn invalid_customer_creation_leaves_no_partial_configuration() {
    let (_directory, layout) = workspace();
    let before = fs::read(layout.state_path().unwrap()).unwrap();
    assert!(
        invoke(
            &layout,
            &[
                "customer",
                "add",
                "other",
                "--trunk",
                "carrier",
                "--allow",
                "355",
                "--max-calls",
                "0"
            ]
        )
        .is_err()
    );
    assert_eq!(fs::read(layout.state_path().unwrap()).unwrap(), before);
}

#[test]
fn duplicate_extension_numbers_are_rejected_across_customers() {
    let (_directory, layout) = workspace();
    invoke(
        &layout,
        &["ext", "add", "main", "101", "--cid", "+12025550100"],
    )
    .unwrap();
    invoke(
        &layout,
        &[
            "customer", "add", "other", "--trunk", "carrier", "--allow", "1",
        ],
    )
    .unwrap();
    let before = fs::read(layout.state_path().unwrap()).unwrap();
    assert!(
        invoke(
            &layout,
            &["ext", "add", "other", "101", "--cid", "+12025550101"]
        )
        .is_err()
    );
    assert_eq!(fs::read(layout.state_path().unwrap()).unwrap(), before);
}

#[test]
fn removing_an_extension_removes_its_credential() {
    let (_directory, layout) = workspace();
    invoke(
        &layout,
        &["ext", "add", "main", "101", "--cid", "+12025550100"],
    )
    .unwrap();
    invoke(&layout, &["ext", "remove", "main", "101", "--yes"]).unwrap();
    let state = layout.load().unwrap();
    assert!(state.secrets.extensions.is_empty());
    assert!(state.config.customers["main"].extensions.is_empty());
}

#[test]
fn removing_a_customer_removes_all_its_credentials() {
    let (_directory, layout) = workspace();
    for extension in ["101", "102"] {
        invoke(
            &layout,
            &["ext", "add", "main", extension, "--cid", "+12025550100"],
        )
        .unwrap();
    }
    invoke(&layout, &["customer", "remove", "main", "--yes"]).unwrap();
    assert!(layout.load().unwrap().secrets.extensions.is_empty());
}

#[test]
fn referenced_pools_cannot_be_deleted_or_emptied() {
    let (_directory, layout) = workspace();
    invoke(&layout, &["pool", "add", "us", "+12025550100"]).unwrap();
    invoke(&layout, &["ext", "add", "main", "101", "--pool", "us"]).unwrap();
    assert!(invoke(&layout, &["pool", "remove", "us", "--yes"]).is_err());
    assert!(invoke(&layout, &["pool", "remove", "us", "+12025550100"]).is_err());
    assert_eq!(layout.load().unwrap().config.pools["us"].numbers.len(), 1);
}

#[test]
fn adding_duplicate_pool_numbers_does_not_bias_selection() {
    let (_directory, layout) = workspace();
    invoke(
        &layout,
        &["pool", "add", "us", "+12025550100", "+12025550100"],
    )
    .unwrap();
    invoke(&layout, &["pool", "add", "us", "+12025550100"]).unwrap();
    assert_eq!(layout.load().unwrap().config.pools["us"].numbers.len(), 1);
}

#[test]
fn per_number_limits_are_shared_across_pools() {
    let (_directory, layout) = workspace();
    invoke(
        &layout,
        &["pool", "add", "us", "+12025550100", "+12025550101"],
    )
    .unwrap();
    invoke(&layout, &["pool", "limit", "us", "2"]).unwrap();
    let state = layout.load().unwrap();
    assert_eq!(state.config.caller_id_limits["+12025550100"], 2);
    assert_eq!(state.config.caller_id_limits["+12025550101"], 2);
}

#[test]
fn alternate_caller_id_can_be_changed_and_removed() {
    let (_directory, layout) = workspace();
    invoke(
        &layout,
        &[
            "ext",
            "add",
            "main",
            "101",
            "--cid",
            "+12025550100",
            "--cid2",
            "+12025550101",
        ],
    )
    .unwrap();
    invoke(&layout, &["ext", "set", "main", "101", "--no-cid2"]).unwrap();
    assert!(
        layout.load().unwrap().config.customers["main"].extensions["101"]
            .alternate_caller_id
            .is_none()
    );
}

#[test]
fn password_rotation_preserves_other_extension_settings() {
    let (_directory, layout) = workspace();
    invoke(
        &layout,
        &["ext", "add", "main", "101", "--cid", "+12025550100"],
    )
    .unwrap();
    let before = layout.load().unwrap();
    invoke(&layout, &["ext", "passwd", "main", "101"]).unwrap();
    let after = layout.load().unwrap();
    assert_ne!(
        before.secrets.extensions["101"],
        after.secrets.extensions["101"]
    );
    assert_eq!(
        after.config.customers["main"].extensions["101"].caller_id,
        "+12025550100"
    );
}

#[test]
fn in_use_trunks_cannot_be_removed() {
    let (_directory, layout) = workspace();
    assert!(invoke(&layout, &["trunk", "remove", "carrier", "--yes"]).is_err());
    assert!(layout.load().unwrap().config.trunks.contains_key("carrier"));
}

#[test]
fn edits_stop_while_recovery_is_pending() {
    let (_directory, layout) = workspace();
    let before = fs::read(layout.state_path().unwrap()).unwrap();
    fs::write(layout.path("var/lib/voipctl/pending.json").unwrap(), "{}").unwrap();
    assert!(invoke(&layout, &["server", "--domain", "other.example.com"]).is_err());
    assert_eq!(fs::read(layout.state_path().unwrap()).unwrap(), before);
}

#[test]
fn reinitialization_never_overwrites_configuration() {
    let (_directory, layout) = workspace();
    let before = fs::read(layout.state_path().unwrap()).unwrap();
    assert!(invoke(&layout, &["init", "--domain", "other.example.com"]).is_err());
    assert_eq!(fs::read(layout.state_path().unwrap()).unwrap(), before);
}

#[test]
fn malformed_configuration_errors_do_not_disclose_input() {
    let (_directory, layout) = workspace();
    fs::write(
        layout.state_path().unwrap(),
        "sensitive_value = [BROKEN_PRIVATE_VALUE",
