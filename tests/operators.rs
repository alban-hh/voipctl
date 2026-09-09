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
