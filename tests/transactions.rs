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
