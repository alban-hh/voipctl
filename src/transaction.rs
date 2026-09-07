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

pub fn plan(layout: &Layout, artifacts: &[Artifact]) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    for artifact in artifacts {
        let path = layout.path(&artifact.path)?;
        if !path.exists() {
            changes.push(Change {
                path: artifact.path.clone(),
                action: "create",
            });
        } else if fs::read_to_string(&path)? != artifact.content {
            changes.push(Change {
                path: artifact.path.clone(),
                action: "update",
            });
        } else if fs::metadata(path)?.permissions().mode() & 0o777 != artifact.mode {
            changes.push(Change {
                path: artifact.path.clone(),
                action: "permissions",
            });
        }
    }
    Ok(changes)
}

pub fn apply(
    layout: &Layout,
    artifacts: &[Artifact],
    adopt: bool,
    activator: &mut dyn Activator,
) -> Result<String> {
    layout.require_write_access()?;
    ensure!(
        !layout.path("var/lib/voipctl/pending.json")?.exists(),
        "an interrupted transaction exists; run recover first"
    );
    activator.preflight(artifacts)?;
    let manifest_path = layout.path("var/lib/voipctl/manifest.json")?;
    let previous_manifest = if manifest_path.exists() {
        Some(fs::read_to_string(&manifest_path)?)
    } else {
        None
    };
    let manifest: Manifest = previous_manifest
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?
        .unwrap_or_default();
    let mut files = Vec::new();
    for artifact in artifacts {
        let path = layout.path(&artifact.path)?;
        let metadata = fs::metadata(&path).ok();
        let content = if path.exists() {
            Some(fs::read_to_string(&path)?)
        } else {
            None
        };
        if let Some(content) = &content {
            let tracked = manifest.files.get(&artifact.path);
            ensure!(
                adopt || tracked.is_some_and(|expected| *expected == fingerprint(content)),
                "{} is untracked or changed outside voipctl; inspect plan and explicitly use --adopt-existing",
                artifact.path
            );
        }
        files.push(FileVersion {
            path: artifact.path.clone(),
            content,
            mode: metadata
                .as_ref()
                .map_or(artifact.mode, |m| m.permissions().mode() & 0o777),
            uid: metadata.as_ref().map_or(0, MetadataExt::uid),
            gid: metadata.as_ref().map_or(0, MetadataExt::gid),
        });
    }
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string();
    let snapshot = Snapshot {
        id: id.clone(),
        files,
        manifest: previous_manifest,
    };
    let history = layout.path("var/lib/voipctl/history")?;
    private_directory(&history)?;
    let encoded = serde_json::to_vec(&snapshot)?;
    atomic_write(&history.join(format!("{id}.json")), &encoded, 0o600, None)?;
    let pending = layout.path("var/lib/voipctl/pending.json")?;
    atomic_write(&pending, &encoded, 0o600, None)?;
    let operation = (|| -> Result<()> {
        for artifact in artifacts {
            let owner = if layout.live() {
                let group = nix::unistd::Group::from_name(&artifact.group)?
                    .context("required service group not found")?;
                Some((0, group.gid.as_raw()))
            } else {
                None
            };
            atomic_write(
                &layout.path(&artifact.path)?,
                artifact.content.as_bytes(),
                artifact.mode,
                owner,
            )?;
        }
        activator.activate()?;
        activator.verify()?;
        let manifest = Manifest {
            files: artifacts
                .iter()
                .map(|a| (a.path.clone(), fingerprint(&a.content)))
                .collect(),
        };
        atomic_write(&manifest_path, &serde_json::to_vec(&manifest)?, 0o600, None)?;
        Ok(())
    })();
    if let Err(error) = operation {
        let recovery = restore(layout, &snapshot)
            .and_then(|_| activator.activate())
            .and_then(|_| activator.verify());
        match recovery {
            Ok(()) => {
                fs::remove_file(pending)?;
                bail!("apply failed: {error:#}; previous files and services restored");
            }
            Err(recovery_error) => bail!(
                "apply failed: {error:#}; recovery also failed: {recovery_error:#}; pending journal retained; run recover"
            ),
        }
    }
    fs::remove_file(pending)?;
    Ok(id)
}

pub fn restore(layout: &Layout, snapshot: &Snapshot) -> Result<()> {
    for file in &snapshot.files {
        ensure!(
            file.path.starts_with("etc/asterisk/")
                || file.path == "etc/fail2ban/jail.d/voipctl.conf",
            "unexpected snapshot path"
        );
        let path = layout.path(&file.path)?;
        if let Some(content) = &file.content {
            atomic_write(
                &path,
                content.as_bytes(),
                file.mode,
                layout.live().then_some((file.uid, file.gid)),
            )?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
    }
    let path = layout.path("var/lib/voipctl/manifest.json")?;
    if let Some(manifest) = &snapshot.manifest {
        atomic_write(&path, manifest.as_bytes(), 0o600, None)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn recover(layout: &Layout, activator: &mut dyn Activator) -> Result<()> {
    let path = layout.path("var/lib/voipctl/pending.json")?;
    let snapshot: Snapshot =
        serde_json::from_slice(&fs::read(&path).context("no interrupted transaction")?)?;
    restore(layout, &snapshot)?;
    activator.activate()?;
    activator.verify()?;
    fs::remove_file(path)?;
    Ok(())
}

pub fn rollback(layout: &Layout, id: &str, activator: &mut dyn Activator) -> Result<String> {
    ensure!(
        !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()),
        "invalid checkpoint identifier"
    );
    ensure!(
        !layout.path("var/lib/voipctl/pending.json")?.exists(),
        "recover the interrupted transaction first"
    );
    let checkpoint = layout.path(&format!("var/lib/voipctl/history/{id}.json"))?;
    let target: Snapshot =
        serde_json::from_slice(&fs::read(checkpoint).context("checkpoint not found")?)?;
    let mut files = Vec::new();
    for old in &target.files {
        let path = layout.path(&old.path)?;
        let metadata = fs::metadata(&path).ok();
        files.push(FileVersion {
            path: old.path.clone(),
            content: if path.exists() {
                Some(fs::read_to_string(&path)?)
            } else {
                None
            },
            mode: metadata
                .as_ref()
                .map_or(old.mode, |m| m.permissions().mode() & 0o777),
            uid: metadata.as_ref().map_or(0, MetadataExt::uid),
            gid: metadata.as_ref().map_or(0, MetadataExt::gid),
        });
    }
    let manifest_path = layout.path("var/lib/voipctl/manifest.json")?;
    let current = Snapshot {
        id: SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string(),
        files,
        manifest: if manifest_path.exists() {
            Some(fs::read_to_string(manifest_path)?)
        } else {
            None
        },
    };
    let encoded = serde_json::to_vec(&current)?;
    atomic_write(
        &layout.path(&format!("var/lib/voipctl/history/{}.json", current.id))?,
        &encoded,
        0o600,
        None,
    )?;
    let pending = layout.path("var/lib/voipctl/pending.json")?;
    atomic_write(&pending, &encoded, 0o600, None)?;
    if let Err(error) = restore(layout, &target)
        .and_then(|_| activator.activate())
        .and_then(|_| activator.verify())
    {
        if let Err(recovery) = recover(layout, activator) {
            bail!("rollback failed: {error:#}; recovery failed: {recovery:#}; journal retained");
        }
        bail!("rollback failed: {error:#}; previous files restored");
    }
    fs::remove_file(pending)?;
    Ok(current.id)
}
