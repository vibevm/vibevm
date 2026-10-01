//! Pinned host source declarations and exact selected boot bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use vibe_core::manifest::{
    Manifest, SelectedSource, SourceDescriptor, SourceInterpretation, SourceLoading,
    SourceSelector, select_source_rows,
};
use vibe_safefs::{Project, StableFileSnapshot};

const HOST_MANIFEST_CAP: usize = 1024 * 1024;
const HOST_BOOT_SOURCE_CAP: usize = 16 * 1024 * 1024;

/// Streaming selected file identities. Exact bytes are held for only one
/// validation at a time; there is no total boot quota.
pub(super) struct DeclaredBoot {
    pub(super) coordinate: String,
    pub(super) sources: Vec<SourceDescriptor>,
    selected: BTreeMap<String, String>,
    pinned: Project,
    manifest: StableFileSnapshot,
    source_cap: usize,
}

impl DeclaredBoot {
    pub(super) fn read_selected(&self, path: &str) -> Result<Vec<u8>, String> {
        let expected = self.selected.get(path).ok_or_else(|| {
            format!("selected host boot source `{path}` has no preflight identity")
        })?;
        let captured = self
            .pinned
            .read_file_snapshot_bounded(path, self.source_cap)
            .map_err(|error| format!("selected host boot source `{path}`: {error}"))?
            .ok_or_else(|| format!("selected host boot source `{path}` disappeared"))?;
        if &captured.sha256 != expected {
            return Err(format!(
                "selected host boot source `{path}` changed since preflight"
            ));
        }
        recheck_manifest(&self.pinned, &self.manifest)?;
        Ok(captured.bytes)
    }
}

pub(super) fn declared_boot(
    project_root: &Path,
    boot_rel: &Path,
) -> Result<Option<DeclaredBoot>, String> {
    declared_boot_with_cap(project_root, boot_rel, HOST_BOOT_SOURCE_CAP)
}

fn declared_boot_with_cap(
    project_root: &Path,
    boot_rel: &Path,
    source_cap: usize,
) -> Result<Option<DeclaredBoot>, String> {
    let pinned = Project::open(project_root).map_err(|error| format!("host root: {error}"))?;
    let manifest = pinned
        .read_file_snapshot_bounded(Manifest::FILENAME, HOST_MANIFEST_CAP)
        .map_err(|error| format!("host manifest: {error}"))?
        .ok_or_else(|| "host manifest is unavailable".to_owned())?;
    let text = std::str::from_utf8(&manifest.bytes)
        .map_err(|error| format!("host manifest is not UTF-8: {error}"))?;
    let parsed = Manifest::parse_str(text).map_err(|error| error.to_string())?;
    let Some(project) = parsed.project else {
        recheck_manifest(&pinned, &manifest)?;
        return Ok(None);
    };
    if project.sources.is_empty() {
        recheck_manifest(&pinned, &manifest)?;
        return Ok(None);
    }
    let group = project
        .group
        .ok_or_else(|| "declared host boot sources require a self-coordinate group".to_owned())?;
    vibe_core::PackageName::parse(&project.name)
        .map_err(|error| format!("declared host source name: {error}"))?;
    if project
        .sources
        .iter()
        .any(|row| matches!(row.selector(), SourceSelector::Directory(_)))
    {
        return Err("declared host boot directory sources need a complete captured census".into());
    }
    let coordinate = format!("{group}/{}", project.name);
    let boot_prefix = format!("{}/", boot_rel.to_string_lossy().replace('\\', "/"));
    let mut selected = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for row in &project.sources {
        let SourceSelector::File(path) = row.selector() else {
            unreachable!()
        };
        if !path.starts_with(&boot_prefix) || !seen.insert(path.as_str()) {
            continue;
        }
        let selection = select_source_rows(&coordinate, &project.sources, path)
            .map_err(|error| format!("host boot source `{path}`: {error:?}"))?;
        let descriptor = match selection {
            SelectedSource::Descriptor { descriptor, .. } => descriptor,
            SelectedSource::NeedsInput { .. } => {
                return Err(format!(
                    "host boot source `{path}` has ambiguous descriptors"
                ));
            }
            SelectedSource::Unclassified { .. } => continue,
        };
        if !matches!(
            descriptor.interpretation(),
            SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc
        ) || descriptor.loading() != SourceLoading::Scan
        {
            continue;
        }
        let state = pinned
            .stable_file_state_bounded(path, source_cap as u64)
            .map_err(|error| format!("selected host boot source `{path}`: {error}"))?
            .ok_or_else(|| format!("selected host boot source `{path}` is unavailable"))?;
        selected.insert(path.clone(), state.sha256);
    }
    recheck_manifest(&pinned, &manifest)?;
    Ok(Some(DeclaredBoot {
        coordinate,
        sources: project.sources,
        selected,
        pinned,
        manifest,
        source_cap,
    }))
}

fn recheck_manifest(
    pinned: &Project,
    manifest: &vibe_safefs::StableFileSnapshot,
) -> Result<(), String> {
    let after = pinned
        .read_file_snapshot_bounded(Manifest::FILENAME, HOST_MANIFEST_CAP)
        .map_err(|error| format!("host manifest recheck: {error}"))?
        .ok_or_else(|| "host manifest disappeared during boot check".to_owned())?;
    if manifest != &after {
        return Err("host manifest changed during boot check".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn selected_files_have_independent_per_file_caps_without_total_boot_quota() {
        let root = tempfile::tempdir().unwrap();
        let boot = vibe_core::layout::current_boot_dir();
        fs::create_dir_all(root.path().join(&boot)).unwrap();
        let mut rows = String::new();
        let mut paths = Vec::new();
        for name in ["a.xml", "b.xml"] {
            let path = boot.join(name).to_string_lossy().replace('\\', "/");
            rows.push_str(&format!(
                "[[project.sources]]\nfile='{path}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n"
            ));
            fs::write(root.path().join(&path), b"ok").unwrap();
            paths.push(path);
        }
        fs::write(
            root.path().join("vibe.toml"),
            format!("[project]\ngroup='org.example'\nname='host'\nversion='1.1.0'\n{rows}"),
        )
        .unwrap();
        let limited = declared_boot_with_cap(root.path(), &boot, 1).err().unwrap();
        assert!(limited.contains("selected host boot source"), "{limited}");
        let selected = declared_boot_with_cap(root.path(), &boot, 2)
            .unwrap()
            .unwrap();
        assert_eq!(selected.read_selected(&paths[0]).unwrap(), b"ok");
        assert_eq!(selected.read_selected(&paths[1]).unwrap(), b"ok");
    }
}
