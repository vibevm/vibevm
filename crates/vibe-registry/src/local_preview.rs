//! Read-only exact local source observation through a caller-held project root.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#DESCRIBE-EVALUATE-PLAN");
use anyhow::{Result, anyhow, bail};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use vibe_core::{PackageRef, VersionSpec, manifest::Manifest};
use vibe_safefs::{FileIdentity, Pinned, Project, StableFileSnapshot};

/// One checked source epoch. No cache, fetch, template expansion or effects.
#[derive(Debug)]
pub struct LocalPreviewObservation {
    pub manifest: Manifest,
    pub content_hash: String,
}

const MAX_ENTRIES: usize = 1024;
const MAX_FILE: usize = 1024 * 1024;
const MAX_TOTAL: usize = 8 * 1024 * 1024;
const MAX_DEPTH: usize = 16;

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    ancestors: Vec<FileIdentity>,
    directories: BTreeMap<String, FileIdentity>,
    files: BTreeMap<String, StableFileSnapshot>,
    total: usize,
}

fn walk(
    project: &Project,
    directory: &Pinned,
    prefix: &str,
    depth: usize,
    out: &mut Snapshot,
) -> Result<()> {
    if depth > MAX_DEPTH || out.directories.len() + out.files.len() >= MAX_ENTRIES {
        bail!("local preview source exceeds bounded tree limits");
    }
    out.directories.insert(prefix.into(), directory.identity()?);
    let mut names = project.child_names_bounded(directory, MAX_ENTRIES)?;
    names.sort();
    for name in names {
        if crate::hash_recipe::LEGACY0_EXCLUDES.contains(&name.as_str()) {
            continue;
        }
        // Restrict to portable UTF-8 names so the frozen legacy recipe's
        // normalisation cannot alias two distinct observed members.
        if name.contains('\\') || name.chars().any(char::is_control) {
            bail!("local preview requires portable source member names");
        }
        if out.directories.len() + out.files.len() >= MAX_ENTRIES {
            bail!("local preview source exceeds bounded entry limit");
        }
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        match directory.open_child(&name) {
            Ok(child) => walk(project, &child, &relative, depth + 1, out)?,
            Err(_) => {
                // Directory open failure is not absence: only a checked
                // regular single-link file read can establish the other case.
                let snapshot = project
                    .read_file_snapshot_bounded_in(directory, &name, MAX_FILE)?
                    .ok_or_else(|| anyhow!("local preview source member disappeared"))?;
                out.total = out
                    .total
                    .checked_add(snapshot.bytes.len())
                    .ok_or_else(|| anyhow!("source size overflow"))?;
                if out.total > MAX_TOTAL {
                    bail!("local preview source exceeds bounded byte limit");
                }
                out.files.insert(relative, snapshot);
            }
        }
    }
    Ok(())
}

fn capture(project: &Project, components: &[&str]) -> Result<Snapshot> {
    let mut directory = project.root_dir()?;
    let mut snapshot = Snapshot {
        ancestors: Vec::new(),
        directories: BTreeMap::new(),
        files: BTreeMap::new(),
        total: 0,
    };
    for component in components {
        directory = directory.open_child(component)?;
        snapshot.ancestors.push(directory.identity()?);
    }
    walk(project, &directory, "", 0, &mut snapshot)?;
    Ok(snapshot)
}

/// Frozen source bytes for a narrowly admitted exact old local lock row.
/// Private fields ensure callers cannot bless a live slot as its own source.
pub struct PreparedRetirementSource {
    row: vibe_core::manifest::LockedPackage,
    source: Project,
    snapshot: Snapshot,
    frozen: tempfile::TempDir,
}
impl PreparedRetirementSource {
    pub fn package(&self) -> &vibe_core::manifest::LockedPackage {
        &self.row
    }
    pub fn frozen_tree(&self) -> &std::path::Path {
        self.frozen.path()
    }
    pub fn recheck(&self) -> Result<()> {
        let current = capture(&self.source, &[])?;
        if current != self.snapshot {
            bail!("exact retirement source changed after admission");
        }
        if crate::compute_content_hash(self.frozen.path())? != self.row.content_hash.as_str() {
            bail!("frozen retirement source no longer matches the exact old pin");
        }
        Ok(())
    }
}

/// Source ownership comes from the actual retained local registry, not a
/// request-owned arbitrary tree. Freeze exactly the bytes checked against the
/// old lock; the workspace remains independent of registry hash machinery.
impl crate::LocalRegistry {
    pub fn prepare_exact_retirement_source(
        &self,
        old: &vibe_core::manifest::LockedPackage,
    ) -> Result<PreparedRetirementSource> {
        use vibe_core::manifest::{Materialization, SourceKind};
        if old.materialization != Materialization::Copy
            || old.overridden
            || old.via_override.is_some()
            || old.via_redirect.is_some()
            || old.template.is_some()
            || !old.embedded_sources.is_empty()
            || !matches!(
                old.source_kind,
                None | Some(SourceKind::Registry) | Some(SourceKind::Local)
            )
        {
            bail!("retirement supports only an exact ordinary local plain-copy source");
        }
        let expected = self
            .root()
            .join(old.group.as_str())
            .join(old.name.as_str())
            .join(format!("v{}", old.version));
        if crate::localdir_path(old.source_url.as_str())? != expected {
            bail!("retirement source binding differs from retained local owner");
        }
        let root = Project::open(self.root())?;
        let version = format!("v{}", old.version);
        let components = [old.group.as_str(), old.name.as_str(), version.as_str()];
        let snapshot = capture(&root, &components)?;
        let manifest = snapshot
            .files
            .get(Manifest::FILENAME)
            .ok_or_else(|| anyhow!("retirement source manifest missing"))?;
        let manifest = Manifest::parse_str(std::str::from_utf8(&manifest.bytes)?)?;
        let metadata = manifest.require_package()?;
        if metadata.group != old.group
            || metadata.name != old.name
            || metadata.version != old.version
            || metadata.kind != old.kind
            || metadata.materialization != Materialization::Copy
            || manifest.template.is_some()
            || metadata.has_explicit_source_policy()
        {
            bail!("retirement source coordinate or plain-copy policy differs from old lock");
        }
        let hash = hash_snapshot(&snapshot);
        if hash != old.content_hash.as_str() {
            bail!("retirement source content differs from old immutable pin");
        }
        let frozen = tempfile::Builder::new()
            .prefix("vibe-retirement-source-")
            .tempdir()?;
        let frozen_project = Project::open(frozen.path())?;
        for (path, file) in &snapshot.files {
            frozen_project.write_atomic_with_mode(path, &file.bytes, file.unix_mode)?;
        }
        if capture(&root, &components)? != snapshot {
            bail!("retirement source changed while freezing");
        }
        let source = Project::open(&expected)?;
        let snapshot = capture(&source, &[])?;
        if hash_snapshot(&snapshot) != old.content_hash.as_str() {
            bail!("retirement source changed after freeze");
        }
        let prepared = PreparedRetirementSource {
            row: old.clone(),
            source,
            snapshot,
            frozen,
        };
        prepared.recheck()?;
        Ok(prepared)
    }
}

fn hash_snapshot(snapshot: &Snapshot) -> String {
    let mut hasher = Sha256::new();
    let entries = snapshot
        .files
        .iter()
        .map(|(path, file)| (path.clone(), file))
        .collect();
    for (path, file) in
        crate::hash_recipe::order_entries(crate::hash_recipe::RecipeId::Legacy0, entries)
    {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(&file.bytes);
        hasher.update([0]);
    }
    format!("sha256:{:x}", hasher.finalize())
}

/// Observe the ordinary `<registry>/<group>/<name>/v<exact>/` layout only.
/// Every traversal starts from `project`, not an ambient absolute pathname.
/// Two complete bounded observations must agree, including child identities.
/// This is observed source evidence, not supplier authentication or a promise
/// of atomicity against an unrestricted hostile same-user process.
pub fn observe_local_preview(
    project: &Project,
    registry_components: &[&str],
    reference: &PackageRef,
) -> Result<LocalPreviewObservation> {
    let VersionSpec::Req(requirement) = &reference.version else {
        bail!("local preview requires an exact version");
    };
    let [comparator] = requirement.comparators.as_slice() else {
        bail!("local preview requires one exact comparator");
    };
    if comparator.op != semver::Op::Exact
        || comparator.minor.is_none()
        || comparator.patch.is_none()
    {
        bail!("local preview requires a complete exact version");
    }
    let mut version = semver::Version::new(
        comparator.major,
        comparator.minor.unwrap(),
        comparator.patch.unwrap(),
    );
    version.pre = comparator.pre.clone();
    let group = reference
        .group
        .as_ref()
        .ok_or_else(|| anyhow!("local preview requires a qualified package"))?;
    let version_dir = format!("v{version}");
    let mut components = registry_components.to_vec();
    components.extend([
        group.as_str(),
        reference.name.as_str(),
        version_dir.as_str(),
    ]);
    // A sibling template/version would require the ordinary template/version
    // discovery policy. This first slice admits exactly one concrete version,
    // rather than silently bypassing that policy or warming template caches.
    let package_components = &components[..components.len() - 1];
    let package_directory = project
        .dir_if_present(package_components)?
        .ok_or_else(|| anyhow!("local package is unavailable"))?;
    let mut names = project.child_names_bounded(&package_directory, MAX_ENTRIES)?;
    names.sort();
    if names != [version_dir.clone()] {
        bail!(
            "local preview requires a unique concrete version directory without sibling template or metadata routes"
        );
    }
    let before = capture(project, &components)?;
    #[cfg(test)]
    test_hook::between();
    let manifest_bytes = &before
        .files
        .get(Manifest::FILENAME)
        .ok_or_else(|| anyhow!("local preview manifest is unavailable"))?
        .bytes;
    let manifest = Manifest::parse_str(std::str::from_utf8(manifest_bytes)?)?;
    let metadata = manifest.require_package()?;
    if &metadata.group != group
        || metadata.name != reference.name
        || metadata.version != version
        || manifest.template.is_some()
    {
        bail!("local preview source identity differs from exact project requirement");
    }
    if reference.kind.is_some_and(|kind| kind != metadata.kind) {
        bail!("local preview package kind mismatch");
    }
    let mut hasher = Sha256::new();
    let entries = before
        .files
        .iter()
        .map(|(path, snapshot)| (path.clone(), snapshot))
        .collect();
    for (path, snapshot) in
        crate::hash_recipe::order_entries(crate::hash_recipe::RecipeId::Legacy0, entries)
    {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(&snapshot.bytes);
        hasher.update([0]);
    }
    let content_hash = format!("sha256:{:x}", hasher.finalize());
    let after = capture(project, &components)?;
    let package_after = project
        .dir_if_present(package_components)?
        .ok_or_else(|| anyhow!("local package disappeared"))?;
    let mut names_after = project.child_names_bounded(&package_after, MAX_ENTRIES)?;
    names_after.sort();
    if names_after != names || package_after.identity()? != package_directory.identity()? {
        bail!("local package inventory changed during preview");
    }
    if before != after {
        bail!("local preview source changed during observation");
    }
    Ok(LocalPreviewObservation {
        manifest,
        content_hash,
    })
}

#[cfg(test)]
mod test_hook {
    use std::cell::RefCell;
    thread_local! { static HOOK: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) }; }
    pub fn arm(hook: Box<dyn FnOnce()>) {
        HOOK.with(|slot| *slot.borrow_mut() = Some(hook));
    }
    pub fn between() {
        if let Some(hook) = HOOK.with(|slot| slot.borrow_mut().take()) {
            hook();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Project, PackageRef) {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("registry/org.example/demo/v1.0.0");
        std::fs::create_dir_all(source.join("boot")).unwrap();
        std::fs::write(
            source.join("vibe.toml"),
            "[package]\ngroup='org.example'\nname='demo'\nversion='1.0.0'\nkind='flow'\n",
        )
        .unwrap();
        std::fs::write(source.join("boot/rules.md"), "synthetic source").unwrap();
        let project = Project::open(temp.path()).unwrap();
        (
            temp,
            project,
            PackageRef::parse("org.example/demo@=1.0.0").unwrap(),
        )
    }
    #[test]
    fn held_source_hash_matches_existing_install_recipe() {
        let (temp, project, reference) = fixture();
        let result = observe_local_preview(&project, &["registry"], &reference).unwrap();
        assert_eq!(
            result.content_hash,
            crate::compute_content_hash(&temp.path().join("registry/org.example/demo/v1.0.0"))
                .unwrap()
        );
    }
    #[test]
    fn source_names_cannot_replace_ancestor_observations() {
        let (temp, project, reference) = fixture();
        let source = temp.path().join("registry/org.example/demo/v1.0.0");
        std::fs::create_dir_all(source.join("@ancestor/0")).unwrap();
        std::fs::write(source.join("@ancestor/0/data"), "ordinary synthetic bytes").unwrap();
        let captured = capture(&project, &["registry", "org.example", "demo", "v1.0.0"]).unwrap();
        assert_eq!(captured.ancestors.len(), 4);
        assert!(captured.directories.contains_key("@ancestor/0"));
        let result = observe_local_preview(&project, &["registry"], &reference).unwrap();
        assert_eq!(
            result.content_hash,
            crate::compute_content_hash(&source).unwrap()
        );
    }
    #[test]
    fn changed_child_directory_even_with_equal_bytes_is_refused() {
        let (temp, project, reference) = fixture();
        let boot = temp.path().join("registry/org.example/demo/v1.0.0/boot");
        test_hook::arm(Box::new(move || {
            let retained = boot
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("retained-boot");
            std::fs::rename(&boot, retained).unwrap();
            std::fs::create_dir(&boot).unwrap();
            std::fs::write(boot.join("rules.md"), "synthetic source").unwrap();
        }));
        assert!(observe_local_preview(&project, &["registry"], &reference).is_err());
    }
    #[test]
    fn source_hardlink_and_child_symlink_refuse() {
        let (temp, project, reference) = fixture();
        let source = temp.path().join("registry/org.example/demo/v1.0.0");
        std::fs::hard_link(source.join("boot/rules.md"), temp.path().join("shared")).unwrap();
        assert!(observe_local_preview(&project, &["registry"], &reference).is_err());
        #[cfg(unix)]
        {
            std::fs::rename(source.join("boot"), temp.path().join("outside")).unwrap();
            std::os::unix::fs::symlink(temp.path().join("outside"), source.join("boot")).unwrap();
            assert!(observe_local_preview(&project, &["registry"], &reference).is_err());
        }
    }
}

/// Bounded full-content proof from held no-follow bytes, never a store sidecar.
#[derive(Debug)]
pub struct VerifiedPayload {
    project: Project,
    snapshot: Snapshot,
    expected: String,
}
impl VerifiedPayload {
    /// Manifest bytes from the same full payload observation, not a second read.
    pub fn manifest_bytes(&self) -> Result<&[u8]> {
        self.snapshot
            .files
            .get(Manifest::FILENAME)
            .map(|file| file.bytes.as_slice())
            .ok_or_else(|| anyhow!("verified payload manifest missing"))
    }
    pub fn recheck(&self) -> Result<()> {
        let rebound = Project::open(self.project.root_path())?;
        if rebound.root_identity()? != self.project.root_identity()? {
            bail!("verified package payload pathname changed");
        }
        let current = capture(&self.project, &[])?;
        if current != self.snapshot || hash_snapshot(&current) != self.expected {
            bail!("verified package payload changed");
        }
        Ok(())
    }
}
/// Verify a first-slice legacy-hash payload without fetching or trusting metadata.
pub fn verify_pinned_payload(path: &std::path::Path, expected: &str) -> Result<VerifiedPayload> {
    if !expected.starts_with("sha256:") {
        bail!("submitted payload requires admitted legacy recipe");
    }
    let project = Project::open(path)?;
    let snapshot = capture(&project, &[])?;
    if hash_snapshot(&snapshot) != expected {
        bail!("submitted pinned payload content differs from immutable expectation");
    }
    let proof = VerifiedPayload {
        project,
        snapshot,
        expected: expected.into(),
    };
    proof.recheck()?;
    Ok(proof)
}

#[cfg(test)]
mod payload_tests {
    use super::*;
    #[test]
    fn full_payload_proof_does_not_trust_sidecar_or_unchanged_manifest() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("payload");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("vibe.toml"), "synthetic unchanged manifest").unwrap();
        std::fs::write(path.join("data"), "original").unwrap();
        let pin = crate::compute_content_hash(&path).unwrap();
        let proof = verify_pinned_payload(&path, &pin).unwrap();
        std::fs::write(root.path().join("payload.sha256"), &pin).unwrap();
        std::fs::write(path.join("data"), "changed").unwrap();
        assert!(proof.recheck().is_err());
        assert!(verify_pinned_payload(&path, &pin).is_err());
    }
    #[test]
    fn full_payload_proof_rejects_same_byte_pathname_replacement() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("payload");
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("data"), "original").unwrap();
        let pin = crate::compute_content_hash(&path).unwrap();
        let proof = verify_pinned_payload(&path, &pin).unwrap();
        std::fs::rename(&path, root.path().join("retained")).unwrap();
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("data"), "original").unwrap();
        assert!(proof.recheck().is_err());
    }
}
