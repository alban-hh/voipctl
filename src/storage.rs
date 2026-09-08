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
            .mode(0o600)
            .open(path)?;
        file.try_lock_exclusive()
            .context("another voipctl command is changing this workspace")?;
        Ok(file)
    }

    pub fn load(&self) -> Result<State> {
        let path = self.state_path()?;
        let metadata = fs::metadata(&path).context("configuration not found; run voipctl init")?;
        ensure!(
            metadata.len() <= 4 * 1024 * 1024,
            "configuration exceeds 4 MiB"
        );
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "state.toml contains credentials and must have mode 600"
        );
        let text = fs::read_to_string(path)?;
        let state: State = toml::from_str(&text).map_err(|_| {
            anyhow::anyhow!(
                "invalid state.toml syntax or unknown fields; values omitted to protect credentials"
            )
        })?;
        state.validate()?;
        Ok(state)
    }

    pub fn save(&self, state: &State) -> Result<()> {
        self.require_write_access()?;
        state.validate()?;
        let path = self.state_path()?;
        private_directory(path.parent().context("missing state directory")?)?;
        atomic_write(
            &path,
            toml::to_string_pretty(state)?.as_bytes(),
            0o600,
            None,
        )?;
        Ok(())
    }
}

pub fn reject_symlinks(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => ensure!(
                !meta.file_type().is_symlink(),
                "managed path contains a symlink: {}",
                ancestor.display()
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

pub fn private_directory(path: &Path) -> Result<()> {
    reject_symlinks(path)?;
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub fn atomic_write(
    path: &Path,
    content: &[u8],
    mode: u32,
    owner: Option<(u32, u32)>,
) -> Result<()> {
    reject_symlinks(path)?;
    let parent = path.parent().context("managed file has no parent")?;
    fs::create_dir_all(parent)?;
    let mut pending = tempfile::NamedTempFile::new_in(parent)?;
    pending
        .as_file()
        .set_permissions(fs::Permissions::from_mode(mode))?;
    pending.write_all(content)?;
    if let Some((uid, gid)) = owner {
        chown(
            pending.path(),
            Some(Uid::from_raw(uid)),
            Some(Gid::from_raw(gid)),
        )?;
    }
    pending.as_file().sync_all()?;
    pending.persist(path).map_err(|e| e.error)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn durable_remove(path: &Path) -> Result<()> {
    reject_symlinks(path)?;
    fs::remove_file(path)?;
    File::open(path.parent().context("managed file has no parent")?)?.sync_all()?;
    Ok(())
}
