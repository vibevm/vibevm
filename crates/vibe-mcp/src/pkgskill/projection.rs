//! Declared-skill inventory and projection orchestration.
//!
//! The CLI is one surface over this owner: discovery, selection, per-skill
//! agent filtering, scope expansion, and source/include lowering all happen
//! here. The lower writer remains in the parent `pkgskill` cell.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::manifest::{Lockfile, Manifest, SkillDecl};
use vibe_workspace::Workspace;

use super::{
    PackageSkillError, PackageSkillReport, install_package_skill_selecting, uninstall_package_skill,
};
use crate::agents::{Agent, Scope};

/// A package or project `[[skill]]` declaration lowered to its absolute
/// source path and a stable human-facing origin label.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub struct DeclaredSkill {
    pub decl: SkillDecl,
    /// Absolute path to the skill body (`base.join(decl.path)`).
    pub source: PathBuf,
    /// `"project"` / a member rel-path, or `"<kind>:<name>"` for an
    /// installed package.
    pub origin: String,
}

/// Standalone/package-binding selection shared by every surface.
#[derive(Debug, Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub struct DeclaredSkillFilter<'a> {
    names: &'a [String],
    agent: Option<&'a str>,
}

impl<'a> DeclaredSkillFilter<'a> {
    pub fn new(names: &'a [String], agent: Option<&'a str>) -> Self {
        Self { names, agent }
    }

    /// The package-phase binding's default: every declared skill and every
    /// agent allowed by that skill's `agents` list.
    pub fn all() -> DeclaredSkillFilter<'static> {
        DeclaredSkillFilter {
            names: &[],
            agent: None,
        }
    }
}

/// Collect every declared skill reachable from `project_root`: the root and
/// workspace members, followed by each installed lockfile package.
pub fn collect_declared_skills(project_root: &Path) -> Result<Vec<DeclaredSkill>> {
    let ws = Workspace::discover(project_root)
        .with_context(|| format!("loading workspace at `{}`", project_root.display()))?;
    let mut out = Vec::new();

    for (rel, manifest) in ws.iter_nodes() {
        let base = ws.node_abs_path(rel);
        let origin = if rel == "." {
            "project".to_string()
        } else {
            rel.to_string()
        };
        lower_manifest_skills(manifest, &base, &origin, &mut out);
    }

    let lock_path = ws.lockfile_path();
    if lock_path.exists() {
        let lockfile = Lockfile::read(&lock_path)
            .with_context(|| format!("reading lockfile `{}`", lock_path.display()))?;
        for pkg in &lockfile.packages {
            let slot = ws.vibedeps_slot(&pkg.group, &pkg.name, &pkg.version);
            let manifest_path = slot.join(Manifest::FILENAME);
            if !manifest_path.exists() {
                continue;
            }
            // A malformed dependency manifest never blocks skill listing.
            let Ok(manifest) = Manifest::read(&manifest_path) else {
                continue;
            };
            let origin = format!("{}:{}", pkg.kind.as_str(), pkg.name);
            lower_manifest_skills(&manifest, &slot, &origin, &mut out);
        }
    }
    Ok(out)
}

fn lower_manifest_skills(
    manifest: &Manifest,
    base: &Path,
    origin: &str,
    out: &mut Vec<DeclaredSkill>,
) {
    for decl in &manifest.skills {
        out.push(DeclaredSkill {
            source: base.join(&decl.path),
            decl: decl.clone(),
            origin: origin.to_string(),
        });
    }
}

#[derive(Debug, Clone)]
struct ProjectionTask {
    agent: Agent,
    scope: Scope,
    name: String,
    source: PathBuf,
    include: Vec<String>,
}

/// A prepared inventory snapshot. Preparing performs every fallible parse and
/// filter before the CLI prints its plan heading; preview and apply then walk
/// the same ordered task set on opposite sides of confirmation.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub struct DeclaredSkillProjection {
    project_root: PathBuf,
    tasks: Vec<ProjectionTask>,
}

impl DeclaredSkillProjection {
    pub fn install(&self, dry_run: bool) -> Result<Vec<PackageSkillReport>> {
        self.install_with(dry_run, install_package_skill_selecting)
    }

    pub fn uninstall(&self, dry_run: bool) -> Result<Vec<PackageSkillReport>> {
        let mut reports = Vec::with_capacity(self.tasks.len());
        for task in &self.tasks {
            reports.push(uninstall_package_skill(
                task.agent,
                task.scope,
                Some(&self.project_root),
                &task.name,
                dry_run,
            )?);
        }
        Ok(reports)
    }

    fn install_with<F>(&self, dry_run: bool, mut project_one: F) -> Result<Vec<PackageSkillReport>>
    where
        F: FnMut(
            Agent,
            Scope,
            Option<&Path>,
            &str,
            &Path,
            &[String],
            bool,
        ) -> Result<PackageSkillReport, PackageSkillError>,
    {
        let mut reports = Vec::with_capacity(self.tasks.len());
        for task in &self.tasks {
            reports.push(project_one(
                task.agent,
                task.scope,
                Some(&self.project_root),
                &task.name,
                &task.source,
                &task.include,
                dry_run,
            )?);
        }
        Ok(reports)
    }
}

/// Discover, select, agent-filter, and scope-expand declared skills into one
/// reusable ordered projection plan.
pub fn prepare_declared_skill_projection(
    project_root: &Path,
    filter: &DeclaredSkillFilter<'_>,
    scope: Scope,
) -> Result<DeclaredSkillProjection> {
    // Keep standalone diagnostic precedence byte-compatible: CLI agent
    // syntax was validated before workspace discovery in the original
    // surface-owned orchestration.
    let requested_agents = match filter.agent {
        Some(value) => Agent::parse_filter(value)?,
        None => Agent::ALL.to_vec(),
    };
    let all = collect_declared_skills(project_root)?;
    let selected: Vec<&DeclaredSkill> = all
        .iter()
        .filter(|skill| {
            filter.names.is_empty() || filter.names.iter().any(|name| name == &skill.decl.name)
        })
        .collect();
    if selected.is_empty() {
        bail!("no matching skills (run `vibe skill list` to see what is declared)");
    }

    let scopes = scope.expand();
    let mut tasks = Vec::new();
    for skill in selected {
        for agent in skill_agents(&skill.decl, &requested_agents) {
            for concrete_scope in &scopes {
                tasks.push(ProjectionTask {
                    agent,
                    scope: *concrete_scope,
                    name: skill.decl.name.clone(),
                    source: skill.source.clone(),
                    include: skill.decl.include.clone(),
                });
            }
        }
    }
    Ok(DeclaredSkillProjection {
        project_root: project_root.to_path_buf(),
        tasks,
    })
}

fn skill_agents(decl: &SkillDecl, requested: &[Agent]) -> Vec<Agent> {
    if decl.agents.is_empty() {
        return requested.to_vec();
    }
    requested
        .iter()
        .copied()
        .filter(|agent| {
            decl.agents.iter().any(|name| {
                Agent::parse_filter(name)
                    .map(|values| values.contains(agent))
                    .unwrap_or(false)
            })
        })
        .collect()
}

/// Project every selected declaration into project-local agent roots only.
/// This is the reusable package-phase seam: `Scope::User` is never created or
/// passed to a resolver, including during dry-run.
pub fn project_declared_skills_project_scope(
    project_root: &Path,
    filter: &DeclaredSkillFilter<'_>,
    dry_run: bool,
) -> Result<Vec<PackageSkillReport>> {
    project_declared_skills_project_scope_with(
        project_root,
        filter,
        dry_run,
        install_package_skill_selecting,
    )
}

fn project_declared_skills_project_scope_with<F>(
    project_root: &Path,
    filter: &DeclaredSkillFilter<'_>,
    dry_run: bool,
    project_one: F,
) -> Result<Vec<PackageSkillReport>>
where
    F: FnMut(
        Agent,
        Scope,
        Option<&Path>,
        &str,
        &Path,
        &[String],
        bool,
    ) -> Result<PackageSkillReport, PackageSkillError>,
{
    prepare_declared_skill_projection(project_root, filter, Scope::Project)?
        .install_with(dry_run, project_one)
}

#[cfg(test)]
#[path = "projection/tests.rs"]
mod tests;
