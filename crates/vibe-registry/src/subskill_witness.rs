//! Source-admitted configured lazy-pull bytes, not membership inferred from cache.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS");
use anyhow::{Result, bail, ensure};
use specmark::spec;
use std::{collections::BTreeSet, path::Path};
use vibe_core::{
    ContentHash,
    manifest::{
        DeliveryMode, LockedPackage, LockedSubskill, Manifest, SubskillFileWitness,
        SubskillManifest, SubskillSourceWitness, declarant_path,
    },
};
use vibe_safefs::{Project, StableFileSnapshot};

const DECLARATION_LIMIT: usize = 64 * 1024;
const MEMBER_LIMIT: usize = 1024 * 1024;

/// Private fields prevent a caller from blessing arbitrary current cache bytes.
#[derive(Debug)]
pub struct LocalSubskillAdmission {
    manifest: Manifest,
    observations: Vec<(LockedSubskill, StableFileSnapshot, StableFileSnapshot)>,
    source: Project,
}

fn snapshot(project: &Project, path: &str, cap: usize) -> Result<StableFileSnapshot> {
    declarant_path(Path::new(path))
        .map_err(|_| anyhow::anyhow!("invalid configured subskill path"))?;
    project
        .read_file_snapshot_bounded(path, cap)?
        .ok_or_else(|| anyhow::anyhow!("configured subskill member is missing"))
}
fn file_witness(path: String, observed: &StableFileSnapshot) -> SubskillFileWitness {
    SubskillFileWitness {
        path,
        byte_len: observed.size,
        sha256: observed.sha256.clone(),
    }
}
fn validate_declaration(row: &LockedSubskill, declaration: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(declaration)?;
    let declared: SubskillManifest = toml::from_str(text)?;
    ensure!(
        declared.validation_findings().is_empty(),
        "invalid configured subskill declaration"
    );
    ensure!(
        declared.subskill.path == row.path && declared.subskill.delivery == DeliveryMode::LazyPull,
        "configured subskill identity or delivery disagrees"
    );
    ensure!(
        declared
            .subskill
            .description
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty()),
        "lazy-pull description is empty"
    );
    ensure!(
        declared.content.files_written == row.cache_files && row.cache_files.len() == 1,
        "configured subskill requires one exact declared member"
    );
    ensure!(
        declared.activation == Default::default()
            && declared.recommends.is_empty()
            && declared.conflicts.is_empty(),
        "automatic or related subskill activation is not connected"
    );
    ensure!(
        declared
            .subskill
            .describes
            .as_ref()
            .map(ToString::to_string)
            == row.describes,
        "configured subskill describes disagrees"
    );
    Ok(())
}

impl LocalSubskillAdmission {
    /// Called only by the retained local-source owner, before cache acquisition.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS")]
    pub(crate) fn capture(
        root: &Path,
        manifest: &Manifest,
        paths: &BTreeSet<String>,
    ) -> Result<Self> {
        let source = Project::open(root)?;
        let actual = source
            .read_source_bytes(Manifest::FILENAME)?
            .ok_or_else(|| anyhow::anyhow!("source manifest is missing"))?;
        ensure!(
            Manifest::parse_str(std::str::from_utf8(&actual)?)? == *manifest,
            "selected source metadata changed"
        );
        let pin = ContentHash::parse(&crate::compute_content_hash(root)?)?;
        let mut observations = Vec::new();
        let mut roots = BTreeSet::new();
        for path in paths {
            declarant_path(Path::new(path))
                .map_err(|_| anyhow::anyhow!("invalid selected subskill path"))?;
            let Some(relative) = path.strip_prefix("subskills/") else {
                bail!("selected subskill requires a declaration under subskills/")
            };
            let Some((canonical, member)) = relative.rsplit_once('/') else {
                bail!("selected subskill requires an exact declared member")
            };
            ensure!(
                roots.insert(canonical.to_owned()),
                "ambiguous configured subskill members"
            );
            let declaration_path = format!("subskills/{canonical}/{}", SubskillManifest::FILENAME);
            let declaration = snapshot(&source, &declaration_path, DECLARATION_LIMIT)?;
            let selected = snapshot(&source, path, MEMBER_LIMIT)?;
            std::str::from_utf8(&selected.bytes)?;
            let declared: SubskillManifest =
                toml::from_str(std::str::from_utf8(&declaration.bytes)?)?;
            let row = LockedSubskill {
                path: canonical.into(),
                delivery: "lazy-pull".into(),
                describes: declared
                    .subskill
                    .describes
                    .as_ref()
                    .map(ToString::to_string),
                files_written: Vec::new(),
                cache_files: vec![member.into()],
                source_witness: Some(SubskillSourceWitness {
                    package_content_hash: pin.clone(),
                    declaration: file_witness(declaration_path, &declaration),
                    member: file_witness(path.clone(), &selected),
                }),
            };
            row.validate_source_witness(&pin)
                .map_err(anyhow::Error::msg)?;
            validate_declaration(&row, &declaration.bytes)?;
            observations.push((row, declaration, selected));
        }
        Ok(Self {
            manifest: manifest.clone(),
            observations,
            source,
        })
    }

    /// Retain only a source observation that still agrees with the fetched tree.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS")]
    pub fn finish(self, cached: &crate::CachedPackage) -> Result<Vec<LockedSubskill>> {
        ensure!(
            cached.manifest == self.manifest,
            "acquired manifest changed"
        );
        let source_manifest = self
            .source
            .read_source_bytes(Manifest::FILENAME)?
            .ok_or_else(|| anyhow::anyhow!("source manifest vanished"))?;
        ensure!(
            Manifest::parse_str(std::str::from_utf8(&source_manifest)?)? == self.manifest,
            "selected source metadata changed during acquisition"
        );
        ensure!(
            crate::compute_content_hash(self.source.root_path())? == cached.content_hash,
            "selected source content pin changed during acquisition"
        );
        let cache = Project::open(&cached.cache_dir)?;
        let mut rows = Vec::new();
        for (row, declaration, member) in self.observations {
            let witness = row
                .source_witness
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing source witness"))?;
            ensure!(
                witness.package_content_hash.as_str() == cached.content_hash,
                "acquired source identity changed"
            );
            for (file, observed, limit) in [
                (&witness.declaration, &declaration, DECLARATION_LIMIT),
                (&witness.member, &member, MEMBER_LIMIT),
            ] {
                let current = snapshot(&self.source, &file.path, limit)?;
                ensure!(
                    current == *observed,
                    "selected local source changed during acquisition"
                );
                let acquired = snapshot(&cache, &file.path, limit)?;
                ensure!(
                    acquired.bytes == observed.bytes,
                    "acquired member differs from admitted local source"
                );
            }
            rows.push(row);
        }
        Ok(rows)
    }
}

/// Validate the retained configured row and return the bytes of this held read.
/// There is no cache hydration, sidecar trust, pathname reopen or legacy fallback.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS")]
pub fn read_configured_subskill(
    store_root: &Path,
    package: &LockedPackage,
    row: &LockedSubskill,
) -> Result<Vec<u8>> {
    row.validate_source_witness(&package.content_hash)
        .map_err(anyhow::Error::msg)?;
    let witness = row
        .source_witness
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing source witness"))?;
    let root = crate::store::entry_dir(
        store_root,
        &package.group,
        package.name.as_str(),
        &package.version,
    );
    let recipe = if package.content_hash.as_str().starts_with("sha256-tree/1:") {
        crate::RecipeId::Tree1
    } else {
        crate::RecipeId::Legacy0
    };
    let project = Project::open(&root)?;
    let declaration = snapshot(&project, &witness.declaration.path, DECLARATION_LIMIT)?;
    let member = snapshot(&project, &witness.member.path, MEMBER_LIMIT)?;
    ensure!(
        crate::compute_content_hash_with(recipe, &root)? == package.content_hash.as_str(),
        "configured subskill package cache pin disagrees"
    );
    ensure!(
        Project::open(&root)?.root_identity()? == project.root_identity()?,
        "configured package cache root changed during observation"
    );
    for (file, observed) in [
        (&witness.declaration, &declaration),
        (&witness.member, &member),
    ] {
        ensure!(
            file.byte_len == observed.size && file.sha256 == observed.sha256,
            "configured subskill source bytes disagree with retained witness"
        );
    }
    validate_declaration(row, &declaration.bytes)?;
    std::str::from_utf8(&member.bytes)?;
    Ok(member.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> (tempfile::TempDir, Manifest, BTreeSet<String>) {
        let root = tempfile::tempdir().unwrap();
        let manifest = Manifest::parse_str(
            "[package]\ngroup='org.test'\nname='witness'\nversion='1.0.0'\nkind='flow'",
        )
        .unwrap();
        std::fs::write(
            root.path().join("vibe.toml"),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        let sub = root.path().join("subskills/audit");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("vibe-subskill.toml"),"[subskill]\npath='audit'\ndelivery='lazy-pull'\ndescription='inert'\n[content]\nfiles_written=['SKILL.md']").unwrap();
        std::fs::write(sub.join("SKILL.md"), "ORIGINAL\n").unwrap();
        (
            root,
            manifest,
            BTreeSet::from(["subskills/audit/SKILL.md".into()]),
        )
    }
    #[test]
    fn independently_captured_source_member_cannot_change_during_acquisition() {
        let (root, manifest, paths) = source();
        let admission = LocalSubskillAdmission::capture(root.path(), &manifest, &paths).unwrap();
        std::fs::write(root.path().join("subskills/audit/SKILL.md"), "CHANGED\n").unwrap();
        let registry_root = tempfile::tempdir().unwrap();
        let destination = registry_root.path().join("org.test/witness/v1.0.0");
        crate::shippable::copy_dir_recursive(root.path(), &destination).unwrap();
        let registry = crate::LocalRegistry::new(registry_root.path()).unwrap();
        let resolved = registry
            .resolve(&"org.test/witness@=1.0.0".parse().unwrap())
            .unwrap();
        let store = tempfile::tempdir().unwrap();
        let cached = registry.fetch(&resolved, store.path()).unwrap();
        assert!(admission.finish(&cached).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn source_member_symlink_and_shared_inode_are_not_admitted() {
        let (root, manifest, paths) = source();
        let path = root.path().join("subskills/audit/SKILL.md");
        let held = root.path().join("held-original");
        std::fs::rename(&path, &held).unwrap();
        std::os::unix::fs::symlink(&held, &path).unwrap();
        assert!(LocalSubskillAdmission::capture(root.path(), &manifest, &paths).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::hard_link(&held, &path).unwrap();
        assert!(LocalSubskillAdmission::capture(root.path(), &manifest, &paths).is_err());
    }
}
