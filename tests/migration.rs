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
