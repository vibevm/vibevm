specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-PACKAGE");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use specmark::spec;
use vibe_core::manifest::SkillDecl;
use vibe_wire::generated::lifecycle_state::StateArtifact;

use super::{DeclaredSkill, DeclaredSkillProvider, collect_declared_skills, skill_agents};
use crate::agents::{Agent, Scope};
use crate::pkgskill::{PackageSkillReport, receipt, snapshot_source};

pub const PROJECT_SKILL_PREFIX: &str = "@vibe/package/skill/";
pub const PROJECT_SKILL_RECONCILE_KEY: &str = "@vibe/package/skill/reconcile";
pub const PROJECT_SKILL_RECOVER_KEY: &str = "@vibe/package/skill/recover";

/// One already-authenticated provider plus the skill declarations retained by
/// the lifecycle world loader. Automatic lowering accepts only this input and
/// never re-discovers a workspace or lockfile universe.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-PACKAGE")]
pub struct ProjectSkillProviderInput {
    pub provider: DeclaredSkillProvider,
    pub declarations: Vec<SkillDecl>,
}

/// One project-local target selected from a declaration's agent allow-list.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-PACKAGE")]
pub struct ProjectSkillTarget {
    pub agent: Agent,
    pub path: PathBuf,
}

/// One declared skill lowered for the automatic package-phase binding.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-PACKAGE")]
pub struct ProjectSkillBinding {
    pub skill: DeclaredSkill,
    pub targets: Vec<ProjectSkillTarget>,
    pub source_snapshot: String,
    /// Exact selected source bytes. `None` is the honest missing-source
    /// state; an empty selected snapshot is rejected during planning.
    pub selected_files: Option<BTreeMap<String, Vec<u8>>>,
}

impl ProjectSkillBinding {
    pub fn identity(&self) -> String {
        format!(
            "{PROJECT_SKILL_PREFIX}{}/{}",
            self.skill.provider.identity(),
            self.skill.decl.name
        )
    }

    pub fn artifact_id(&self, agent: Agent) -> String {
        format!(
            "{}#skill:{}:{}",
            self.skill.provider.identity(),
            self.skill.decl.name,
            agent.as_str()
        )
    }
}

/// Historical standalone inventory lowered through the same safe planner.
pub fn collect_project_skill_bindings(project_root: &Path) -> Result<Vec<ProjectSkillBinding>> {
    lower_skills(project_root, collect_declared_skills(project_root)?)
}

/// Lower exactly the authenticated selected host plus reachable lock-ordered
/// providers retained by the lifecycle world loader.
pub fn lower_project_skill_bindings(
    project_root: &Path,
    providers: Vec<ProjectSkillProviderInput>,
) -> Result<Vec<ProjectSkillBinding>> {
    let mut skills = Vec::new();
    for input in providers {
        let base = input.provider.root().to_path_buf();
        let origin = input.provider.identity();
        for decl in input.declarations {
            skills.push(DeclaredSkill {
                source: base.join(&decl.path),
                decl,
                origin: origin.clone(),
                provider: input.provider.clone(),
            });
        }
    }
    lower_skills(project_root, skills)
}

fn lower_skills(
    project_root: &Path,
    skills: Vec<DeclaredSkill>,
) -> Result<Vec<ProjectSkillBinding>> {
    let mut bindings = Vec::with_capacity(skills.len());
    let mut identities = BTreeSet::new();
    let mut physical_targets: BTreeMap<receipt::FoldKey, String> = BTreeMap::new();
    for skill in skills {
        let binding = lower_one_binding(project_root, skill)?;
        let identity = binding.identity();
        if !identities.insert(identity.clone()) {
            bail!("duplicate package skill binding identity `{identity}`");
        }
        for target in &binding.targets {
            let key = receipt::fold_key(&vibe_core::machine_json_path(&target.path));
            if let Some(first) = physical_targets.insert(key, identity.clone()) {
                bail!(
                    "package skill bindings `{first}` and `{identity}` collide at physical target `{}`",
                    target.path.display()
                );
            }
        }
        bindings.push(binding);
    }
    Ok(bindings)
}

fn lower_one_binding(project_root: &Path, skill: DeclaredSkill) -> Result<ProjectSkillBinding> {
    let provider_root = skill.provider.root();
    receipt::ensure_no_follow_walk(provider_root, &skill.source, true).with_context(|| {
        format!(
            "unsafe source for package skill `{}` from `{}`",
            skill.decl.name,
            skill.provider.identity()
        )
    })?;
    let selected_files = match fs::symlink_metadata(&skill.source) {
        Ok(_) => {
            let files = snapshot_source(&skill.source, &skill.decl.include)?;
            // Planning judges the **complete** selected set through the
            // shared portability law before anything is staged or written:
            // an unsafe spelling, or two spellings that are one file on a
            // case-insensitive host, refuse with no target mutation and no
            // `applying` receipt.
            if let Err(fault) = receipt::judge_selection(files.keys().map(String::as_str)) {
                bail!(
                    "package skill `{}` selects unsafe file path set: {fault}",
                    skill.decl.name
                );
            }
            if files.is_empty() {
                bail!(
                    "package skill `{}` from `{}` selects zero files; fix its path/include instead of advertising a nonexistent artifact",
                    skill.decl.name,
                    skill.provider.identity()
                );
            }
            Some(files)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "inspecting package skill source `{}`",
                    skill.source.display()
                )
            });
        }
    };
    let selected = skill_agents(&skill.decl, Agent::ALL)?;
    if !skill.decl.agents.is_empty() {
        for agent in &selected {
            if !super::agent_supports_project_skills(*agent, project_root) {
                bail!(
                    "[[skill]] `{}` names agent `{}` which has no project-scope skill loader; remove it from `agents` to project into the remaining agents",
                    skill.decl.name,
                    agent.as_str()
                );
            }
        }
        if selected.is_empty() {
            bail!(
                "[[skill]] `{}` has an explicit `agents` list that selects no skill-supporting agent; fix the list instead of advertising a zero-target projection",
                skill.decl.name
            );
        }
    }
    let targets = selected
        .into_iter()
        .filter_map(|agent| {
            agent
                .skills_root(Scope::Project, Some(project_root))
                .transpose()
                .map(|result| {
                    result.map(|root| ProjectSkillTarget {
                        agent,
                        path: root.join(&skill.decl.name),
                    })
                })
        })
        .collect::<Result<Vec<_>>>()?;
    for target in &targets {
        receipt::ensure_no_follow_walk(project_root, &target.path, true).with_context(|| {
            format!(
                "unsafe `{}` target for package skill `{}`",
                target.agent.as_str(),
                skill.decl.name
            )
        })?;
        if receipt::paths_overlap(&skill.source, &target.path) {
            bail!(
                "package skill `{}` source `{}` overlaps target `{}`",
                skill.decl.name,
                skill.source.display(),
                target.path.display()
            );
        }
    }
    let source_snapshot = selected_snapshot_digest(selected_files.as_ref());
    Ok(ProjectSkillBinding {
        skill,
        targets,
        source_snapshot,
        selected_files,
    })
}

pub fn reconcile_project_skill_binding(
    project_root: &Path,
    binding: &ProjectSkillBinding,
) -> Result<Vec<PackageSkillReport>> {
    receipt::reconcile_binding(project_root, binding)
}

pub fn reconcile_vanished_project_skill_bindings(
    project_root: &Path,
    desired: &BTreeSet<String>,
) -> Result<Vec<PackageSkillReport>> {
    receipt::reconcile_vanished(project_root, desired)
}

pub fn probe_project_skill_binding(
    project_root: &Path,
    binding: &ProjectSkillBinding,
    artifacts: &[StateArtifact],
) -> Result<bool> {
    receipt::probe_binding(project_root, binding, artifacts)
}

pub fn probe_vanished_project_skill_bindings(
    project_root: &Path,
    desired: &BTreeSet<String>,
    artifacts: &[StateArtifact],
) -> Result<bool> {
    receipt::probe_vanished(project_root, desired, artifacts)
}

pub fn project_skill_receipt_exists(project_root: &Path) -> Result<bool> {
    receipt::receipt_exists_project_root(project_root)
}

/// Finish any durable applying transaction before ordinary bindings run.
pub fn recover_project_skill_bindings(project_root: &Path) -> Result<Vec<PackageSkillReport>> {
    receipt::recover_pending(project_root)
}

/// Fresh probe for the engine-owned recovery row: no pending intent.
pub fn probe_recovered_project_skill_bindings(
    project_root: &Path,
    artifacts: &[StateArtifact],
) -> Result<bool> {
    receipt::probe_recovered(project_root, artifacts)
}

fn selected_snapshot_digest(snapshot: Option<&BTreeMap<String, Vec<u8>>>) -> String {
    let Some(snapshot) = snapshot else {
        return "missing".to_string();
    };
    let mut hash = Sha256::new();
    hash.update(b"vibe-package-skill-snapshot\0epoch=1\0");
    for (relative, bytes) in snapshot {
        hash.update((relative.len() as u64).to_be_bytes());
        hash.update(relative.as_bytes());
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    format!("sha256:{:x}", hash.finalize())
}

#[cfg(test)]
#[path = "binding/tests.rs"]
mod tests;
