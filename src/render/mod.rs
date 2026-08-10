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

