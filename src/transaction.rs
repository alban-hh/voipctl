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

