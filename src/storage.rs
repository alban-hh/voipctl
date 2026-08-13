use crate::model::State;
use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use nix::unistd::{Gid, Uid, chown};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
};

#[derive(Clone)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn new(root: PathBuf) -> Result<Self> {
        ensure!(root.is_absolute(), "--root must be an absolute path");
        ensure!(
            !root.components().any(|c| matches!(c, Component::ParentDir)),
            "--root cannot contain .."
        );
