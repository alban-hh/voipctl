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
