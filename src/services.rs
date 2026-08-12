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

