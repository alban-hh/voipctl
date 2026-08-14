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
        let root = if root.exists() {
            root.canonicalize()?
        } else {
            root
        };
        Ok(Self { root })
    }

    pub fn live(&self) -> bool {
        self.root == Path::new("/")
    }

    pub fn path(&self, relative: &str) -> Result<PathBuf> {
        let path = Path::new(relative);
        ensure!(
            !path.is_absolute() && path.components().all(|c| matches!(c, Component::Normal(_))),
            "invalid managed path"
        );
        let full = self.root.join(path);
        reject_symlinks(&full)?;
        Ok(full)
    }

    pub fn state_path(&self) -> Result<PathBuf> {
        self.path("etc/voipctl/state.toml")
    }

    pub fn require_write_access(&self) -> Result<()> {
        ensure!(
            !self.live() || Uid::effective().is_root(),
            "live changes require root; use --root for an offline workspace"
        );
        Ok(())
    }

    pub fn lock(&self) -> Result<File> {
        self.require_write_access()?;
        let dir = self.path("var/lib/voipctl")?;
        private_directory(&dir)?;
        let path = self.path("var/lib/voipctl/lock")?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
