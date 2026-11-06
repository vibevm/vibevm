//! CLI-owned package discovery and pre-effect source conversion admission.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-ADMISSION-BOUNDARY");
use super::{ConversionTarget, is_foreign_directory, should_skip_directory};
use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use vibe_spec::source_policy::{
    PackagePolicyCheckpoint, PackageSourceMode, observe_package_policy,
};

#[derive(Default)]
pub(super) struct ConversionPolicies(BTreeMap<PathBuf, PackagePolicyCheckpoint>);

impl ConversionPolicies {
    pub(super) fn admit(targets: &[ConversionTarget]) -> Result<Self> {
        let mut policies = Self::default();
        for target in targets {
            let (path, bound) = match target {
                ConversionTarget::Path(path) => (path, None),
                ConversionTarget::Selected { root, path } => (path, Some(root.as_path())),
                ConversionTarget::Refused { .. } => continue,
            };
            if !path.exists() {
                continue;
            }
            let path = std::path::absolute(path)?;
            let bound = bound.map(std::path::absolute).transpose()?;
            policies.admit_argument(&path, bound.as_deref())?;
        }
        policies.recheck()?;
        Ok(policies)
    }

    // Explicit package roots bound discovery. Standalone arguments stop at
    // their nearest present manifest or filesystem root, never past a bad one.
    fn admit_argument(&mut self, path: &Path, bound: Option<&Path>) -> Result<()> {
        let start = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        for directory in start.ancestors() {
            if let Some(bound) = bound {
                anyhow::ensure!(
                    directory.starts_with(bound),
                    "conversion target escapes selected package root"
                );
            }
            if !self.0.contains_key(directory) {
                self.0.insert(
                    directory.to_owned(),
                    observe_package_policy(directory)?.into_checkpoint(),
                );
            }
            let policy = &self.0[directory];
            match policy.mode() {
                PackageSourceMode::Explicit | PackageSourceMode::ExplicitEmpty => {
                    return Err(policy.unsupported("source conversion").into());
                }
                PackageSourceMode::Legacy => break,
                PackageSourceMode::MissingManifest => {}
            }
            if bound == Some(directory) {
                break;
            }
        }
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let child = entry.path();
                    if !is_foreign_directory(&child) && !should_skip_directory(&child) {
                        self.admit_argument(&child, bound.or(Some(path)))?;
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn recheck(&self) -> Result<()> {
        for policy in self.0.values() {
            policy.recheck_unchanged()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
