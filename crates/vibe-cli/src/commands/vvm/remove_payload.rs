//! Refuse opaque binary-instance removal when the owned payload changed.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#remove");
use super::super::{model::InstallRecord, store::VersionStore};
use anyhow::{Result, bail};
use std::collections::BTreeSet;

pub(in crate::commands::vvm) fn guard(store: &VersionStore, record: &InstallRecord) -> Result<()> {
    let home = store.instance_dir(&record.version_id(), record.instance);
    if !home.exists() {
        return Ok(());
    }
    // Legacy single-binary imports have no distribution manifest. Keep the
    // existing shape, but refuse any extra entry in that private instance.
    if record.distribution_manifest_sha256.is_none() {
        if !super::super::placer::installed_files_match(store, record) {
            bail!(
                "refusing to remove {}: imported files no longer match their receipt",
                home.display()
            );
        }
        let receipt = super::super::placer::read_manifest(&home)
            .ok_or_else(|| anyhow::anyhow!("missing placement receipt"))?;
        let expected: BTreeSet<_> = receipt
            .files
            .iter()
            .map(|file| file.rel.replace('\\', "/"))
            .chain(std::iter::once(".vvm-manifest.toml".to_owned()))
            .collect();
        let mut pending = vec![home.clone()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(directory)? {
                let entry = entry?;
                let path = entry.path();
                let rel = path
                    .strip_prefix(&home)?
                    .to_string_lossy()
                    .replace('\\', "/");
                if entry.file_type()?.is_dir() && rel == "bin" {
                    pending.push(path);
                } else if !expected.contains(&rel) {
                    bail!(
                        "refusing to remove {}: unrecorded file {}; preserve or move it first",
                        home.display(),
                        path.display()
                    );
                }
            }
        }
        return Ok(());
    }
    if !super::super::bundle::installed_bundle_intact(store, record) {
        bail!(
            "refusing to remove {}: the recorded binary bundle has changed; files were retained",
            home.display()
        );
    }
    let mut expected = BTreeSet::from([
        "DISTRIBUTION.json".to_owned(),
        "vibevm-source.zip".to_owned(),
        format!("bin/{}", super::super::store::BINARY_NAME),
        format!("bin/{}", super::super::store::INDEX_BINARY_NAME),
        format!("bin/{}", super::super::store::APP_PROXY_BINARY_NAME),
    ]);
    let receipt = super::super::placer::read_manifest(&home);
    for name in ["LICENSE.md", "README.md"] {
        let path = home.join(name);
        if !path.exists() {
            continue;
        }
        let recorded = receipt
            .as_ref()
            .and_then(|receipt| receipt.content_hash_for(name));
        use sha2::Digest;
        let bytes = std::fs::read(&path)?;
        let actual = format!("{:x}", sha2::Sha256::digest(&bytes));
        if recorded != Some(actual.as_str()) {
            bail!(
                "refusing to remove {}: {} has no matching placement receipt; files were retained",
                home.display(),
                name
            );
        }
        expected.insert(name.to_owned());
    }
    if receipt.is_some() {
        expected.insert(".vvm-manifest.toml".to_owned());
    }
    // Source verification above checks its complete file set against the
    // authenticated archive, not merely files named by directory convention.
    let mut pending = vec![home.clone()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let rel = path.strip_prefix(&home)?;
            if rel.starts_with("source") {
                continue;
            }
            if entry.file_type()?.is_dir() {
                if rel != std::path::Path::new("bin") {
                    bail!(
                        "refusing to remove {}: unrecorded directory {}",
                        home.display(),
                        path.display()
                    );
                }
                pending.push(path);
                continue;
            }
            let key = rel.to_string_lossy().replace('\\', "/");
            if !expected.remove(&key) {
                bail!(
                    "refusing to remove {}: unrecorded file {}; files were retained",
                    home.display(),
                    path.display()
                );
            }
        }
    }
    Ok(())
}
