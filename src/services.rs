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
            state,
            restart,
            manage_fail2ban: state.config.server.manage_fail2ban || old.contains("enabled=true"),
        })
    }

    fn active(&self, service: &str) -> Result<()> {
        command("/usr/bin/systemctl", &["is-active", "--quiet", service])?;
        Ok(())
    }
}

impl Activator for LiveServices<'_> {
    fn preflight(&mut self, artifacts: &[Artifact]) -> Result<()> {
        ensure!(
            cfg!(target_os = "linux") && self.layout.live(),
            "service activation requires a Linux host with --root /"
        );
        self.active("asterisk")?;
        let version = asterisk("core show version")?;
        ensure!(
            version.contains("Asterisk 22."),
            "this release supports Asterisk 22; detected an unsupported version"
        );
        for module in [
            "chan_pjsip.so",
            "res_pjsip.so",
            "pbx_config.so",
            "app_dial.so",
            "app_stack.so",
            "func_lock.so",
            "func_groupcount.so",
            "func_strings.so",
            "func_cdr.so",
            "func_callerid.so",
            "func_timeout.so",
            "func_rand.so",
        ] {
            let output = asterisk(&format!("module show like {module}"))?;
            ensure!(
                output.contains(module) && output.contains("Running"),
                "required Asterisk module {module} is not running"
            );
        }
        let old =
            fs::read_to_string(self.layout.path("etc/asterisk/pjsip.conf")?).unwrap_or_default();
        let new = &artifacts
            .iter()
            .find(|a| a.path == "etc/asterisk/pjsip.conf")
            .context("missing PJSIP output")?
            .content;
        ensure!(
            self.restart || transport(&old) == transport(new),
            "transport configuration changed; inspect plan and use apply --restart during a maintenance window"
        );
        if self.restart {
            let channels = asterisk("core show channels count")?;
            ensure!(
                channels
                    .lines()
                    .any(|line| line.trim() == "0 active channels"),
                "restart refused while calls or channels are active"
            );
        }
        if self.manage_fail2ban {
            self.active("fail2ban")?;
        }
        Ok(())
    }

