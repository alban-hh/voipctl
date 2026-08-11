mod dialplan;
mod pjsip;

use crate::model::State;
use anyhow::Result;

#[derive(Clone)]
pub struct Artifact {
    pub path: String,
    pub content: String,
    pub mode: u32,
    pub group: String,
}

impl Artifact {
    pub fn asterisk(name: &str, content: String) -> Self {
        Self {
            path: format!("etc/asterisk/{name}"),
            content,
            mode: 0o640,
            group: "asterisk".into(),
        }
    }
}

pub fn generate(state: &State) -> Result<Vec<Artifact>> {
    state.validate_activation()?;
    let server = &state.config.server;
    let mut files = vec![
        Artifact::asterisk("pjsip.conf", pjsip::render(state)),
        Artifact::asterisk("extensions.conf", dialplan::render(state)),
        Artifact::asterisk(
            "rtp.conf",
            format!(
                "[general]\nrtpstart={}\nrtpend={}\nstrictrtp=yes\nicesupport=no\n",
                server.rtp_start, server.rtp_end
            ),
        ),
        Artifact::asterisk(
            "manager.conf",
            "[general]\nenabled=no\nwebenabled=no\n".into(),
        ),
        Artifact::asterisk("http.conf", "[general]\nenabled=no\nenabletls=no\n".into()),
        Artifact::asterisk(
            "logger.conf",
            "[general]\n[logfiles]\nmessages => notice,warning,error\nsecurity => security\n"
                .into(),
        ),
    ];
    if server.manage_fail2ban {
        let ignores: Vec<String> = std::iter::once("127.0.0.1/8".into())
            .chain(
                state
                    .config
                    .trunks
                    .values()
                    .flat_map(|t| t.signaling.iter().map(ToString::to_string)),
            )
            .collect();
        files.push(Artifact {
            path: "etc/fail2ban/jail.d/voipctl.conf".into(),
            content: format!("[voipctl-asterisk]\nenabled=true\nfilter=asterisk\nbackend=auto\nlogpath=/var/log/asterisk/security\naction=nftables[type=custom, blocktype=drop]\nignoreip={}\nmaxretry=8\nfindtime=10m\nbantime=1h\n", ignores.join(" ")),
            mode: 0o644, group: "root".into(),
        });
    } else {
        files.push(Artifact {
            path: "etc/fail2ban/jail.d/voipctl.conf".into(),
            content: "[voipctl-asterisk]\nenabled=false\n".into(),
            mode: 0o644,
            group: "root".into(),
        });
    }
    Ok(files)
}
