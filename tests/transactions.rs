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
    );
    assert_eq!(faults.activations, 2);
    assert!(
        !layout
            .path("var/lib/voipctl/pending.json")
            .unwrap()
            .exists()
    );
}

#[test]
fn failed_recovery_keeps_a_journal_and_blocks_new_changes() {
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout::new(dir.path().to_owned()).unwrap();
    let _lock = layout.lock().unwrap();
    let mut faults = Faults {
        remaining: 2,
        activations: 0,
    };
    assert!(transaction::apply(&layout, &[artifact("new")], false, &mut faults).is_err());
    assert!(
        layout
            .path("var/lib/voipctl/pending.json")
            .unwrap()
            .exists()
    );
    assert!(transaction::apply(&layout, &[artifact("other")], false, &mut Offline).is_err());
    transaction::recover(&layout, &mut Offline).unwrap();
    assert!(
        !layout
            .path("var/lib/voipctl/pending.json")
            .unwrap()
            .exists()
    );
}

#[test]
fn external_edits_require_explicit_adoption() {
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout::new(dir.path().to_owned()).unwrap();
    let _lock = layout.lock().unwrap();
    transaction::apply(&layout, &[artifact("generated")], false, &mut Offline).unwrap();
    fs::write(
        layout.path("etc/asterisk/pjsip.conf").unwrap(),
        "manual edit",
    )
    .unwrap();
    assert!(transaction::apply(&layout, &[artifact("next")], false, &mut Offline).is_err());
    transaction::apply(&layout, &[artifact("next")], true, &mut Offline).unwrap();
}

#[test]
fn unchanged_files_still_get_service_verification_on_retry() {
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout::new(dir.path().to_owned()).unwrap();
    let _lock = layout.lock().unwrap();
    transaction::apply(&layout, &[artifact("same")], false, &mut Offline).unwrap();
    let mut faults = Faults {
        remaining: 0,
        activations: 0,
    };
    transaction::apply(&layout, &[artifact("same")], false, &mut faults).unwrap();
    assert_eq!(faults.activations, 1);
}

#[test]
fn invalid_state_cannot_replace_a_valid_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout::new(dir.path().to_owned()).unwrap();
    let _lock = layout.lock().unwrap();
    layout.save(&State::default()).unwrap();
    let original = fs::read(layout.state_path().unwrap()).unwrap();
    let mut invalid = State::default();
    invalid.config.server.sip_port = 0;
    assert!(layout.save(&invalid).is_err());
    assert_eq!(fs::read(layout.state_path().unwrap()).unwrap(), original);
    assert_eq!(
        fs::metadata(layout.state_path().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn concurrent_writers_cannot_acquire_the_same_lock() {
    let dir = tempfile::tempdir().unwrap();
    let layout = Layout::new(dir.path().to_owned()).unwrap();
    let lock = layout.lock().unwrap();
    assert!(layout.lock().is_err());
    drop(lock);
    assert!(layout.lock().is_ok());
}
