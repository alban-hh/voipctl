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
    pub restart: bool,
    pub manage_fail2ban: bool,
}

impl<'a> LiveServices<'a> {
    pub fn new(layout: &'a Layout, state: &State, restart: bool) -> Result<Self> {
        let old = fs::read_to_string(layout.path("etc/fail2ban/jail.d/voipctl.conf")?)
            .unwrap_or_default();
        Ok(Self {
            layout,
            restart,
            manage_fail2ban: state.config.server.manage_fail2ban || old.contains("enabled=true"),
        })
    }

    pub fn recovery(layout: &'a Layout, restart: bool) -> Result<Self> {
        ensure!(
            !layout.live() || cfg!(target_os = "linux"),
            "live recovery requires Linux"
        );
        let old = fs::read_to_string(layout.path("etc/fail2ban/jail.d/voipctl.conf")?)
            .unwrap_or_default();
        Ok(Self {
            layout,
            restart,
            manage_fail2ban: old.contains("enabled=true"),
        })
    }

    pub fn prepare_recovery(&self) -> Result<()> {
        ensure!(self.restart, "recovery requires a controlled restart");
        self.ensure_restart_safe()
    }

    fn ensure_restart_safe(&self) -> Result<()> {
        let status = command(
            "/usr/bin/systemctl",
            &["show", "--property=ActiveState", "--value", "asterisk"],
        )?;
        if matches!(status.trim(), "inactive" | "failed") {
            return Ok(());
        }
        ensure!(
            status.trim() == "active",
            "Asterisk is changing state; retry after it settles"
        );
        let channels = asterisk("core show channels count")?;
        ensure!(
            channels
                .lines()
                .any(|line| line.trim() == "0 active channels"),
            "restart refused while calls or channels are active"
        );
        Ok(())
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
        let mut restart_needed = transport(&old) != transport(new);
        for artifact in artifacts.iter().filter(|a| {
            matches!(
                a.path.as_str(),
                "etc/asterisk/rtp.conf" | "etc/asterisk/manager.conf" | "etc/asterisk/http.conf"
            )
        }) {
            let old = fs::read_to_string(self.layout.path(&artifact.path)?).unwrap_or_default();
            restart_needed |= old != artifact.content;
        }
        ensure!(
            self.restart || !restart_needed,
            "transport, RTP, or service configuration changed; inspect plan and use apply --restart during a maintenance window"
        );
        if self.restart {
            self.ensure_restart_safe()?;
        }
        if self.manage_fail2ban {
            self.active("fail2ban")?;
        }
        Ok(())
    }

    fn activate(&mut self) -> Result<()> {
        if self.restart {
            self.ensure_restart_safe()?;
            command("/usr/bin/systemctl", &["restart", "asterisk"])?;
        } else {
            for query in ["pjsip reload", "dialplan reload", "logger reload"] {
                asterisk(query)?;
            }
        }
        let configuration =
            fs::read_to_string(self.layout.path("etc/fail2ban/jail.d/voipctl.conf")?)
                .unwrap_or_default();
        self.manage_fail2ban |= configuration.contains("enabled=true");
        if self.manage_fail2ban {
            command("/usr/bin/fail2ban-client", &["-t"])?;
            command("/usr/bin/systemctl", &["restart", "fail2ban"])?;
        }
        Ok(())
    }

    fn verify(&mut self) -> Result<()> {
        self.active("asterisk")?;
        asterisk("pjsip show transport transport-udp")?;
        let pjsip = fs::read_to_string(self.layout.path("etc/asterisk/pjsip.conf")?)?;
        for number in endpoint_names(&pjsip) {
            let response = asterisk(&format!("pjsip show endpoint {number}"))?;
            ensure!(
                response.contains("Endpoint:"),
                "endpoint {number} did not load"
            );
        }
        let dialplan = fs::read_to_string(self.layout.path("etc/asterisk/extensions.conf")?)?;
        for context in dialplan.lines().filter_map(|line| {
            line.strip_prefix("[voipctl-ext-")
                .and_then(|s| s.strip_suffix(']'))
        }) {
            let response = asterisk(&format!("dialplan show voipctl-ext-{context}"))?;
            ensure!(
                response.contains("priority") || response.contains("priorities"),
                "dialplan for {context} did not load"
            );
        }
        if self.manage_fail2ban {
            self.active("fail2ban")?;
        }
        Ok(())
    }
}

fn endpoint_names(config: &str) -> Vec<String> {
    let mut section = "";
    let mut endpoints = Vec::new();
    for line in config.lines() {
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = name;
        }
        if line == "type=endpoint" {
            endpoints.push(section.to_owned());
        }
    }
    endpoints
}

fn transport(config: &str) -> Vec<&str> {
    let mut active = false;
    config
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('[') {
                active = line == "[transport-udp]";
            }
            (active && !line.is_empty() && !line.starts_with(';')).then_some(line)
        })
        .collect()
}
