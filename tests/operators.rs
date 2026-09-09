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
