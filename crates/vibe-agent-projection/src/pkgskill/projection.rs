//! Declared-skill inventory and projection orchestration.
//!
//! The CLI is one surface over this owner: discovery, selection, per-skill
//! agent filtering, scope expansion, and source/include lowering all happen
//! here. The lower writer remains in the parent `pkgskill` cell.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill");

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use specmark::spec;
use vibe_core::manifest::{
    EmbeddedSourceDecl, LockedEmbeddedSource, Lockfile, Manifest, SkillDecl,
};
use vibe_core::{ContentHash, Group, PackageKind, PackageName};
use vibe_workspace::Workspace;

use super::{
    PackageSkillError, PackageSkillReport, install_package_skill_selecting, uninstall_package_skill,
};
use crate::agents::{Agent, Scope};

#[path = "projection/documentation.rs"]
mod documentation;
use documentation::{documentation_skills, lower_manifest_skills};

#[path = "projection/binding.rs"]
mod binding;
pub use binding::{
    PROJECT_SKILL_PREFIX, PROJECT_SKILL_RECONCILE_KEY, PROJECT_SKILL_RECOVER_KEY,
    ProjectSkillBinding, ProjectSkillProviderInput, ProjectSkillTarget,
    collect_project_skill_bindings, lower_project_skill_bindings, probe_project_skill_binding,
    probe_recovered_project_skill_bindings, probe_vanished_project_skill_bindings,
    project_skill_receipt_exists, reconcile_project_skill_binding,
    reconcile_vanished_project_skill_bindings, recover_project_skill_bindings,
};

/// A package or project `[[skill]]` declaration lowered to its absolute
/// source path and a stable human-facing origin label.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub struct DeclaredSkill {
    pub decl: SkillDecl,
    /// Absolute path to the skill body (`base.join(decl.path)`).
    pub source: PathBuf,
    /// Containment root for `source`. Normally the package slot; after
    /// hydration it may be an authenticated embedded tree or an immutable
    /// composed-skill cache entry.
    pub source_root: PathBuf,
    /// `"project"` / a member rel-path, or `"<kind>:<name>"` for an
    /// installed package.
    pub origin: String,
    /// Typed package provenance retained independently from the historical
    /// human-facing `origin` label.
    pub provider: DeclaredSkillProvider,
}

/// Provider metadata needed by package-phase adapters without rediscovering
/// manifests or lockfiles outside this library owner.
#[derive(Debug, Clone)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-PACKAGE")]
pub enum DeclaredSkillProvider {
    Authored {
        group: Group,
        name: PackageName,
        version: String,
        kind: PackageKind,
        root: PathBuf,
        embedded_sources: Vec<EmbeddedSourceDecl>,
    },
    Installed {
        group: Group,
        name: PackageName,
        version: String,
        kind: PackageKind,
        root: PathBuf,
        content_hash: ContentHash,
        embedded_sources: Vec<LockedEmbeddedSource>,
        declared_sources: Vec<EmbeddedSourceDecl>,
    },
}

impl DeclaredSkillProvider {
    pub fn identity(&self) -> String {
        match self {
            Self::Authored { group, name, .. } | Self::Installed { group, name, .. } => {
                format!("{group}/{name}")
            }
        }
    }

    pub(crate) fn root(&self) -> &Path {
        match self {
            Self::Authored { root, .. } | Self::Installed { root, .. } => root,
        }
    }

    fn embedded_root(&self, requested: &str, offline: bool) -> Result<PathBuf> {
        match self {
            Self::Authored {
                embedded_sources, ..
            } => {
                let declaration = embedded_sources
                    .iter()
                    .find(|source| source.name == requested)
                    .with_context(|| format!("undeclared embedded source `{requested}`"))?;
                Ok(vibe_registry::cache_embedded_source_with(declaration, offline)?.tree)
            }
            Self::Installed {
                embedded_sources,
                declared_sources,
                ..
            } => {
                let declaration = declared_sources
                    .iter()
                    .find(|source| source.name == requested)
                    .with_context(|| format!("undeclared embedded source `{requested}`"))?;
                let locked = embedded_sources
                    .iter()
                    .find(|source| source.name == requested)
                    .with_context(|| {
                        format!(
                            "installed package manifest names embedded source `{requested}` but vibe.lock has no matching authenticated pin"
                        )
                    })?;
                if !locked.matches_declaration(declaration) {
                    bail!(
                        "installed package embedded source `{requested}` disagrees with vibe.lock; rerun `vibe install` to regenerate the locked source view"
                    );
                }
                Ok(vibe_registry::cache_locked_embedded_source_with(locked, offline)?.tree)
            }
        }
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

/// Standalone/package-binding selection shared by every surface.
#[derive(Debug, Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-018#vibe-skill")]
pub struct DeclaredSkillFilter<'a> {
    names: &'a [String],
    agent: Option<&'a str>,
    offline: bool,
}

impl<'a> DeclaredSkillFilter<'a> {
    pub fn new(names: &'a [String], agent: Option<&'a str>) -> Self {
        Self {
            names,
            agent,
            offline: false,
        }
    }

    pub fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    /// The package-phase binding's default: every declared skill and every
    /// agent allowed by that skill's `agents` list.
    pub fn all() -> DeclaredSkillFilter<'static> {
        DeclaredSkillFilter {
            names: &[],
            agent: None,
            offline: false,
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
        lower_manifest_skills(manifest, &base, &origin, None, None, &mut out)?;
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
            lower_manifest_skills(
                &manifest,
                &slot,
                &origin,
                Some(pkg.content_hash.clone()),
                Some(&pkg.embedded_sources),
                &mut out,
            )?;
        }
    }
    documentation_skills(&ws, &mut out)?;
    Ok(out)
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
    let selected: Vec<DeclaredSkill> = all
        .into_iter()
        .filter(|skill| {
            filter.names.is_empty() || filter.names.iter().any(|name| name == &skill.decl.name)
        })
        .collect();
    if selected.is_empty() {
        bail!("no matching skills (run `vibe skill list` to see what is declared)");
    }

    let scopes = scope.expand();
    let mut tasks = Vec::new();
    for mut skill in selected {
        hydrate_declared_skill(&mut skill, filter.offline)?;
        super::receipt::ensure_no_follow_walk(&skill.source_root, &skill.source, true)
            .with_context(|| format!("unsafe source for declared skill `{}`", skill.decl.name))?;
        if skill.source.exists() {
            let selected = super::snapshot_source(&skill.source, &skill.decl.include)?;
            super::validate_skill_frontmatter(&skill.decl.name, &skill.source, &selected)?;
        }
        for agent in skill_agents(&skill.decl, &requested_agents)? {
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

/// Resolve a declaration's explicit content roots without changing the public
/// package root. Direct external skills point at the verified upstream cache;
/// a local adapter with resources becomes one immutable composed source tree.
pub(super) fn hydrate_declared_skill(skill: &mut DeclaredSkill, offline: bool) -> Result<()> {
    if let Some(source) = skill.decl.source.as_deref() {
        let root = skill.provider.embedded_root(source, offline)?;
        skill.source = root.join(&skill.decl.path);
        skill.source_root = root;
    }
    if skill.decl.resources.is_empty() {
        return Ok(());
    }

    super::receipt::ensure_no_follow_walk(&skill.source_root, &skill.source, false)
        .with_context(|| format!("unsafe local adapter source for `{}`", skill.decl.name))?;
    let mut desired = super::snapshot_source(&skill.source, &skill.decl.include)?;
    for resource in &skill.decl.resources {
        let root = skill
            .provider
            .embedded_root(&resource.embedded_source, offline)?;
        let source = root.join(&resource.path);
        super::receipt::ensure_no_follow_walk(&root, &source, false).with_context(|| {
            format!(
                "unsafe embedded resource `{}` for skill `{}`",
                resource.path.display(),
                skill.decl.name
            )
        })?;
        let selected = super::snapshot_source(&source, &resource.include)?;
        for (relative, bytes) in selected {
            let target = format!(
                "{}/{}",
                resource.target.to_string_lossy().replace('\\', "/"),
                relative
            );
            if desired.insert(target.clone(), bytes).is_some() {
                bail!(
                    "[[skill.resource]] for `{}` collides with another selected file at `{target}`",
                    skill.decl.name
                );
            }
        }
    }
    super::receipt::judge_selection(desired.keys().map(String::as_str)).map_err(|fault| {
        anyhow::anyhow!(
            "composed skill `{}` selects a non-portable file set: {fault}",
            skill.decl.name
        )
    })?;
    let tree = composed_skill_tree(&skill.provider.identity(), &skill.decl.name, &desired)?;
    skill.source = tree.clone();
    skill.source_root = tree;
    skill.decl.include.clear();
    skill.decl.resources.clear();
    Ok(())
}

fn composed_skill_tree(
    provider: &str,
    skill: &str,
    desired: &BTreeMap<String, Vec<u8>>,
) -> Result<PathBuf> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(provider.as_bytes());
    hasher.update([0]);
    hasher.update(skill.as_bytes());
    hasher.update([0]);
    for (path, bytes) in desired {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(bytes);
        hasher.update([0]);
    }
    let key = hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut rendered, byte| {
            use std::fmt::Write;
            let _ = write!(&mut rendered, "{byte:02x}");
            rendered
        });
    let settings = vibe_core::settings::settings_dir()
        .context("cannot resolve Vibe settings directory for composed skill cache")?;
    let entry = settings
        .join("cache")
        .join("skill-compositions")
        .join("v1")
        .join(key);
    let tree = entry.join("tree");
    if tree.is_dir() {
        let present = super::snapshot_source(&tree, &[])?;
        if present != *desired {
            bail!(
                "composed skill cache entry `{}` is corrupt; remove this derived entry and retry",
                entry.display()
            );
        }
        return Ok(tree);
    }
    let parent = entry
        .parent()
        .context("composed skill cache entry has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating composed skill cache `{}`", parent.display()))?;
    let temporary = parent.join(format!(".stage-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_dir_all(&temporary)
            .with_context(|| format!("removing stale stage `{}`", temporary.display()))?;
    }
    super::write_snapshot(&temporary.join("tree"), desired)?;
    match fs::rename(&temporary, &entry) {
        Ok(()) => Ok(tree),
        Err(_) if tree.is_dir() => {
            let _ = fs::remove_dir_all(&temporary);
            Ok(tree)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&temporary);
            Err(error)
                .with_context(|| format!("publishing composed skill cache `{}`", entry.display()))
        }
    }
}

/// The declaration's effective agent set, intersected with `requested`.
///
/// An explicit `agents` list must name known, skill-supporting agents: an
/// unknown spelling or an agent with no project-scope skill loader is a plan
/// error with remediation, never a silent zero-target success.
fn skill_agents(decl: &SkillDecl, requested: &[Agent]) -> Result<Vec<Agent>> {
    if decl.agents.is_empty() {
        return Ok(requested.to_vec());
    }
    let mut declared = Vec::new();
    for name in &decl.agents {
        let parsed = Agent::parse_filter(name).map_err(|error| {
            anyhow::Error::msg(error).context(format!(
                "[[skill]] `{}` names unknown agent `{name}`; fix its `agents` list \
                 (known ids: claude, opencode, codex)",
                decl.name
            ))
        })?;
        if parsed.is_empty() {
            bail!(
                "[[skill]] `{}` names agent filter `{name}` that selects no agent",
                decl.name
            );
        }
        for agent in parsed {
            if !declared.contains(&agent) {
                declared.push(agent);
            }
        }
    }
    // Preserve the canonical Agent::ALL order regardless of authored order.
    let declared = Agent::ALL
        .iter()
        .copied()
        .filter(|agent| declared.contains(agent))
        .collect::<Vec<_>>();
    Ok(requested
        .iter()
        .copied()
        .filter(|agent| declared.contains(agent))
        .collect())
}

/// Whether one agent can receive a project-scope skill projection here.
pub(crate) fn agent_supports_project_skills(agent: Agent, project_root: &Path) -> bool {
    matches!(
        agent.skills_root(Scope::Project, Some(project_root)),
        Ok(Some(_))
    )
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
