use crate::{
    render::Artifact,
    storage::{Layout, atomic_write, private_directory},
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub path: String,
    pub content: Option<String>,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub files: Vec<FileVersion>,
    pub manifest: Option<String>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Manifest {
    pub files: BTreeMap<String, String>,
}

#[derive(Serialize)]
pub struct Change {
    pub path: String,
    pub action: &'static str,
}

pub trait Activator {
    fn preflight(&mut self, artifacts: &[Artifact]) -> Result<()>;
    fn activate(&mut self) -> Result<()>;
    fn verify(&mut self) -> Result<()>;
}

pub struct Offline;

impl Activator for Offline {
    fn preflight(&mut self, _: &[Artifact]) -> Result<()> {
        Ok(())
    }
    fn activate(&mut self) -> Result<()> {
        Ok(())
    }
    fn verify(&mut self) -> Result<()> {
        Ok(())
    }
}

pub fn fingerprint(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

