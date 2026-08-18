use anyhow::{Result, bail};
use std::{fs, os::unix::fs::PermissionsExt};
use voipctl::{
    model::State,
    render::Artifact,
    storage::Layout,
    transaction::{self, Activator, Offline},
};

struct Faults {
    remaining: usize,
    activations: usize,
}

impl Activator for Faults {
    fn preflight(&mut self, _: &[Artifact]) -> Result<()> {
        Ok(())
    }
    fn activate(&mut self) -> Result<()> {
        self.activations += 1;
        if self.remaining > 0 {
            self.remaining -= 1;
            bail!("simulated activation failure");
        }
        Ok(())
    }
    fn verify(&mut self) -> Result<()> {
        Ok(())
    }
}

fn artifact(content: &str) -> Artifact {
    Artifact::asterisk("pjsip.conf", content.into())
}

#[test]
fn failed_activation_restores_all_files_and_previous_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout::new(dir.path().to_owned()).unwrap();
    let _lock = layout.lock().unwrap();
    transaction::apply(&layout, &[artifact("before")], false, &mut Offline).unwrap();
    let manifest = fs::read(layout.path("var/lib/voipctl/manifest.json").unwrap()).unwrap();
    let mut faults = Faults {
        remaining: 1,
        activations: 0,
    };
    assert!(
        transaction::apply(
            &layout,
            &[
                artifact("after"),
                Artifact::asterisk("extensions.conf", "new".into())
            ],
            false,
            &mut faults
        )
        .is_err()
    );
    assert_eq!(
        fs::read_to_string(layout.path("etc/asterisk/pjsip.conf").unwrap()).unwrap(),
        "before"
    );
    assert!(
        !layout
            .path("etc/asterisk/extensions.conf")
            .unwrap()
            .exists()
    );
    assert_eq!(
        fs::read(layout.path("var/lib/voipctl/manifest.json").unwrap()).unwrap(),
        manifest
