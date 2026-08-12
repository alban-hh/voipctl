use crate::{model::State, render::Artifact, storage::Layout, transaction::Activator};
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn command(program: &str, arguments: &[&str]) -> Result<String> {
    let mut output = tempfile::tempfile()?;
    let mut child = Command::new(program)
        .args(arguments)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::from(output.try_clone()?))
        .stderr(Stdio::from(output.try_clone()?))
        .spawn()
        .with_context(|| format!("could not start {program}"))?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(30) {
            child.kill()?;
            child.wait()?;
            bail!("{program} timed out after 30 seconds");
        }
        thread::sleep(Duration::from_millis(50));
    };
    ensure!(
        status.success(),
        "{program} failed with {status}; inspect its service journal for details"
    );
    output.seek(SeekFrom::Start(0))?;
    let mut text = String::new();
    output.take(2 * 1024 * 1024).read_to_string(&mut text)?;
    Ok(text)
}

pub fn asterisk(query: &str) -> Result<String> {
    let output = command("/usr/sbin/asterisk", &["-rx", query])?;
    ensure!(
        ![
            "No such command",
            "Unable to connect",
            "Unable to find",
            "No objects found"
        ]
        .iter()
        .any(|message| output.contains(message)),
        "Asterisk could not complete the requested operation"
    );
    Ok(output)
}

pub struct LiveServices<'a> {
    pub layout: &'a Layout,
    pub state: &'a State,
    pub restart: bool,
    pub manage_fail2ban: bool,
}

impl<'a> LiveServices<'a> {
    pub fn new(layout: &'a Layout, state: &'a State, restart: bool) -> Result<Self> {
        let old = fs::read_to_string(layout.path("etc/fail2ban/jail.d/voipctl.conf")?)
            .unwrap_or_default();
        Ok(Self {
            layout,
