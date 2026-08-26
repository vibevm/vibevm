//! Strict receipt reading, writing, validation, and freshness probing over
//! the project capability.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use vibe_core::manifest::SkillDecl;
use vibe_core::{Group, PackageName, machine_json_path};
use vibe_wire::generated::lifecycle_state::StateArtifact;
use vibe_wire::generated::package_skill_receipt::{
    PackageSkillApplying, PackageSkillBinding as ReceiptBinding, PackageSkillFile as ReceiptFile,
    PackageSkillReceipt, PackageSkillTarget as ReceiptTarget,
};

use super::containment::{FoldSet, ensure_lexically_contained, fold_key, valid_relative_file};
use super::nofollow::Project;
use crate::agents::{Agent, Scope};
use crate::pkgskill::{PROJECT_SKILL_PREFIX, PROJECT_SKILL_RECONCILE_KEY, ProjectSkillBinding};

const SCHEMA: u32 = 2;
const RECEIPT_FILE: &str = "package-skills.toml";

pub(crate) fn receipt_exists_project_root(project_root: &Path) -> Result<bool> {
    let project = Project::open(project_root)?;
    receipt_exists(&project)
}

fn receipt_exists(project: &Project) -> Result<bool> {
    let Ok(vibe) = project.dir(&[".vibe"], false) else {
        return Ok(false);
    };
    project
        .read_file(&vibe, RECEIPT_FILE)
        .map(|bytes| bytes.is_some())
}

pub(crate) fn probe_binding(
    project_root: &Path,
    binding: &ProjectSkillBinding,
    artifacts: &[StateArtifact],
) -> Result<bool> {
    let project = Project::open(project_root)?;
    let Some(mut receipt) = read_receipt(&project)? else {
        return Ok(false);
    };
    if receipt.applying.is_some() {
        return Ok(false);
    }
    canonicalize_receipt(&mut receipt);
    let actual = receipt
        .binding
        .iter()
        .find(|row| row.key == binding.identity());
    let Some(files) = binding.selected_files.as_ref() else {
        return Ok(actual.is_none() && artifacts.is_empty());
    };
    let expected = receipt_binding(binding, files);
    if actual != Some(&expected) || !artifacts_match(binding, artifacts) {
        return Ok(false);
    }
    for target in &expected.target {
        if !owned_target_matches(&project, project_root, target)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn probe_vanished(
    project_root: &Path,
    desired: &BTreeSet<String>,
    artifacts: &[StateArtifact],
) -> Result<bool> {
    let project = Project::open(project_root)?;
    let Some(receipt) = read_receipt(&project)? else {
        return Ok(false);
    };
    Ok(receipt.applying.is_none()
        && artifacts.is_empty()
        && receipt
            .binding
            .iter()
            .all(|binding| desired.contains(&binding.key)))
}

pub(crate) fn probe_recovered(project_root: &Path, artifacts: &[StateArtifact]) -> Result<bool> {
    let project = Project::open(project_root)?;
    let Some(receipt) = read_receipt(&project)? else {
        return Ok(artifacts.is_empty());
    };
    Ok(receipt.applying.is_none() && artifacts.is_empty())
}

pub(super) fn read_receipt(project: &Project) -> Result<Option<PackageSkillReceipt>> {
    let vibe = match project.dir(&[".vibe"], false) {
        Ok(vibe) => vibe,
        Err(_) => return Ok(None),
    };
    let Some(bytes) = project.read_file(&vibe, RECEIPT_FILE)? else {
        return Ok(None);
    };
    let text = String::from_utf8(bytes)
        .with_context(|| format!("decoding `{}`", vibe.join(RECEIPT_FILE).display()))?;
    let mut receipt: PackageSkillReceipt = toml::from_str(&text).with_context(|| {
        format!(
            "malformed package-skill receipt `{}`",
            vibe.join(RECEIPT_FILE).display()
        )
    })?;
    if receipt.schema != SCHEMA {
        bail!(
            "unsupported package-skill receipt schema {} in `{}`; this build supports schema {SCHEMA} (remove the stale cache and rerun)",
            receipt.schema,
            vibe.join(RECEIPT_FILE).display()
        );
    }
    validate_receipt(project.root_path(), &receipt)?;
    canonicalize_receipt(&mut receipt);
    Ok(Some(receipt))
}

pub(super) fn write_receipt(project: &Project, receipt: &PackageSkillReceipt) -> Result<()> {
    let vibe = project.dir(&[".vibe"], true)?;
    let mut canonical = receipt.clone();
    canonicalize_receipt(&mut canonical);
    let bytes = toml::to_string_pretty(&canonical)
        .context("encoding strict package-skill receipt")?
        .into_bytes();
    if matches!(project.read_file(&vibe, RECEIPT_FILE), Ok(Some(ref existing)) if *existing == bytes)
    {
        return Ok(());
    }
    project.write_atomic(&vibe, RECEIPT_FILE, &bytes)
}

pub(super) fn empty_receipt() -> PackageSkillReceipt {
    PackageSkillReceipt {
        applying: None,
        binding: Vec::new(),
        schema: SCHEMA,
    }
}

/// A fresh transaction nonce: never reused within a process, and practically
/// never across processes (pid + nanosecond clock + counter, hashed).
pub(super) fn fresh_nonce() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|span| span.as_nanos())
        .unwrap_or_default();
    let mut hash = Sha256::new();
    hash.update(b"vibe-package-skill-nonce\0");
    hash.update(std::process::id().to_le_bytes());
    hash.update(nanos.to_le_bytes());
    hash.update(SEQUENCE.fetch_add(1, Ordering::Relaxed).to_le_bytes());
    format!("{:x}", hash.finalize())
}

pub(super) fn canonicalize_receipt(receipt: &mut PackageSkillReceipt) {
    canonicalize_bindings(&mut receipt.binding);
    if let Some(applying) = &mut receipt.applying {
        canonicalize_bindings(&mut applying.binding);
    }
}

fn canonicalize_bindings(bindings: &mut [ReceiptBinding]) {
    for binding in bindings.iter_mut() {
        for target in &mut binding.target {
            target
                .file
                .sort_by(|left, right| left.path.cmp(&right.path));
        }
        binding.target.sort_by(|left, right| {
            left.agent
                .cmp(&right.agent)
                .then_with(|| left.path.cmp(&right.path))
        });
    }
    bindings.sort_by(|left, right| left.key.cmp(&right.key));
}

pub(super) fn receipt_binding(
    binding: &ProjectSkillBinding,
    files: &BTreeMap<String, Vec<u8>>,
) -> ReceiptBinding {
    let owned = files
        .iter()
        .map(|(path, bytes)| ReceiptFile {
            path: path.clone(),
            sha256: digest(bytes),
        })
        .collect::<Vec<_>>();
    ReceiptBinding {
        key: binding.identity(),
        provider: binding.skill.provider.identity(),
        skill: binding.skill.decl.name.clone(),
        source_snapshot: binding.source_snapshot.clone(),
        target: binding
            .targets
            .iter()
            .map(|target| ReceiptTarget {
                agent: target.agent.as_str().to_string(),
                path: machine_json_path(&target.path),
                file: owned.clone(),
            })
            .collect(),
    }
}

pub(super) fn owned_target_matches(
    project: &Project,
    project_root: &Path,
    target: &ReceiptTarget,
) -> Result<bool> {
    let absolute = Path::new(&target.path);
    if ensure_lexically_contained(project_root, absolute).is_err() {
        return Ok(false);
    }
    let Some(components) = relative_components(project_root, absolute) else {
        return Ok(false);
    };
    let directory = match project.dir(&components, false) {
        Ok(directory) => directory,
        Err(_) => return Ok(false),
    };
    for file in &target.file {
        match project.read_file(&directory, &file.path) {
            Ok(Some(bytes)) => {
                if digest(&bytes) != file.sha256 {
                    return Ok(false);
                }
            }
            Ok(None) => return Ok(false),
            Err(_) => return Ok(false),
        }
    }
    Ok(true)
}

pub(super) fn relative_components<'a>(root: &Path, path: &'a Path) -> Option<Vec<&'a str>> {
    ensure_lexically_contained(root, path).ok()?;
    let components = path
        .strip_prefix(root)
        .ok()?
        .components()
        .map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_str()?),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(components)
}

fn artifacts_match(binding: &ProjectSkillBinding, artifacts: &[StateArtifact]) -> bool {
    let mut actual = artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.id.clone(),
                artifact.path.clone(),
                artifact.kind.clone(),
            )
        })
        .collect::<Vec<_>>();
    let mut expected = binding
        .targets
        .iter()
        .map(|target| {
            (
                binding.artifact_id(target.agent),
                machine_json_path(&target.path),
                "agent-skill".to_string(),
            )
        })
        .collect::<Vec<_>>();
    actual.sort();
    expected.sort();
    actual == expected
}

fn validate_receipt(project_root: &Path, receipt: &PackageSkillReceipt) -> Result<()> {
    validate_bindings(project_root, &receipt.binding)?;
    if let Some(applying) = &receipt.applying {
        validate_nonce(&applying.nonce)?;
        validate_bindings(project_root, &applying.binding)?;
        validate_transition(&receipt.binding, applying)?;
    }
    Ok(())
}

fn validate_nonce(nonce: &str) -> Result<()> {
    if nonce.is_empty() || nonce.len() > 64 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("invalid package-skill applying nonce `{nonce}`");
    }
    Ok(())
}

fn validate_bindings(project_root: &Path, bindings: &[ReceiptBinding]) -> Result<()> {
    let mut keys = BTreeSet::new();
    let mut physical_targets = BTreeSet::new();
    for binding in bindings {
        if !keys.insert(binding.key.as_str()) {
            bail!("duplicate receipt binding key `{}`", binding.key);
        }
        let (group, name) = binding
            .provider
            .split_once('/')
            .with_context(|| format!("invalid receipt provider `{}`", binding.provider))?;
        Group::parse(group)
            .with_context(|| format!("invalid receipt provider `{}`", binding.provider))?;
        PackageName::parse(name)
            .with_context(|| format!("invalid receipt provider `{}`", binding.provider))?;
        if !SkillDecl::valid_name(&binding.skill) {
            bail!("invalid receipt skill name `{}`", binding.skill);
        }
        if binding.key
            != format!(
                "{PROJECT_SKILL_PREFIX}{}/{}",
                binding.provider, binding.skill
            )
        {
            bail!(
                "receipt binding key `{}` does not match provider/skill",
                binding.key
            );
        }
        validate_digest_or_missing(&binding.source_snapshot)?;
        let mut agents = BTreeSet::new();
        for target in &binding.target {
            if !agents.insert(target.agent.as_str()) {
                bail!(
                    "duplicate target agent `{}` in receipt binding `{}`",
                    target.agent,
                    binding.key
                );
            }
            let parsed = Agent::parse_filter(&target.agent)?;
            if parsed.len() != 1 || parsed[0].as_str() != target.agent {
                bail!(
                    "receipt target agent `{}` is not one exact skill agent",
                    target.agent
                );
            }
            let root = parsed[0]
                .skills_root(Scope::Project, Some(project_root))?
                .with_context(|| {
                    format!("receipt agent `{}` has no project skill root", target.agent)
                })?;
            // The engine writes the canonical spelling itself, so ownership
            // requires the exact canonical string on every host — a
            // case-fold-equivalent alias must not authorize the canonical
            // target. Fold-aware collision detection below stays for planned
            // physical-target safety.
            let expected = machine_json_path(&root.join(&binding.skill));
            if target.path != expected {
                bail!(
                    "receipt target `{}` is not the canonical `{}` project skill target",
                    target.path,
                    target.agent
                );
            }
            ensure_lexically_contained(project_root, Path::new(&target.path))?;
            if !physical_targets.insert(fold_key(target.path.clone())) {
                bail!(
                    "receipt contains duplicate physical target `{}`",
                    target.path
                );
            }
            let mut files = FoldSet::new();
            for file in &target.file {
                if !valid_relative_file(&file.path) || !files.insert(&file.path) {
                    bail!("invalid or duplicate owned file `{}` in receipt", file.path);
                }
                validate_digest(&file.sha256)?;
            }
        }
    }
    Ok(())
}

fn validate_transition(before: &[ReceiptBinding], applying: &PackageSkillApplying) -> Result<()> {
    if !applying.key.starts_with(PROJECT_SKILL_PREFIX) {
        bail!("invalid package-skill applying key `{}`", applying.key);
    }
    let before = before
        .iter()
        .map(|binding| (binding.key.as_str(), binding))
        .collect::<BTreeMap<_, _>>();
    let after = applying
        .binding
        .iter()
        .map(|binding| (binding.key.as_str(), binding))
        .collect::<BTreeMap<_, _>>();
    if applying.key == PROJECT_SKILL_RECONCILE_KEY {
        if after
            .iter()
            .any(|(key, binding)| before.get(key).copied() != Some(*binding))
        {
            bail!("vanished-binding applying intent may only remove receipt rows");
        }
    } else {
        let unchanged = before
            .keys()
            .chain(after.keys())
            .filter(|key| **key != applying.key)
            .all(|key| before.get(key) == after.get(key));
        if !unchanged {
            bail!(
                "applying intent `{}` modifies an unrelated receipt binding",
                applying.key
            );
        }
    }
    Ok(())
}

fn validate_digest_or_missing(value: &str) -> Result<()> {
    if value == "missing" {
        Ok(())
    } else {
        validate_digest(value)
    }
}

fn validate_digest(value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        bail!("invalid sha256 digest `{value}`");
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("invalid sha256 digest `{value}`");
    }
    Ok(())
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
