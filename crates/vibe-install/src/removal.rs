//! Dependency-aware removal planning. Selected lock edges, not package-directory
//! presence or all possible providers, define the retained dependency closure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{Group, PackageRef};
use vibe_workspace::Workspace;

fn refuse(message: impl Into<String>) -> crate::Error {
    vibe_resolver::SolveError::UnsupportedRoute(message.into()).into()
}

fn coordinate(reference: &PackageRef) -> crate::error::Result<String> {
    reference
        .group
        .as_ref()
        .map(|group| format!("{group}/{}", reference.name))
        .ok_or_else(|| refuse(format!("ungrouped removal edge `{reference}`")))
}

/// A complete desired-world decision, produced before confirmation or deletion.
/// The fields carrying authored state stay private so callers cannot turn an
/// explanation into a different deletion plan.
#[derive(Debug)]
pub struct RemovalPlan {
    pub selected: String,
    pub explicit_root_removed: bool,
    pub affected_dependents: Vec<String>,
    pub retained: Vec<String>,
    removed: Vec<String>,
    pub(crate) manifest: Manifest,
    pub(crate) lockfile: Lockfile,
    pub(crate) project_root: PathBuf,
    pub(crate) workspace: Workspace,
    pub(crate) preconditions: crate::preconditions::FilePreconditions,
}

impl RemovalPlan {
    /// Exact deletion membership is immutable outside this owner.
    pub fn removed(&self) -> &[String] {
        &self.removed
    }

    pub fn explanation(&self) -> String {
        format!(
            "remove declaration {}: {}; affected dependents: [{}]; keep: [{}]; remove owned slots: [{}]",
            self.selected,
            self.explicit_root_removed,
            self.affected_dependents.join(", "),
            self.retained.join(", "),
            self.removed.join(", "),
        )
    }
}

/// Apply only the exact confirmed plan. Failed prefixes retain before-images;
/// boot publication continues to use the existing role-restricted journal.
pub fn apply_removal(
    mut plan: RemovalPlan,
    spec_format: vibe_core::manifest::SpecFormat,
) -> crate::error::Result<()> {
    plan.preconditions.validate()?;
    let mut slots = Vec::new();
    for package in &plan.lockfile.packages {
        // The kept world must remain complete before boot sees it.
        let slot = if package.materialization.is_in_place() {
            vibe_workspace::vibedeps::in_place_slot_rel_path(&package.group, &package.name)
        } else {
            vibe_workspace::vibedeps::slot_rel_path(&package.group, &package.name, &package.version)
        };
        if !plan.workspace.root.join(&slot).is_dir() {
            return Err(refuse(format!("kept slot `{slot}` is unavailable")));
        }
    }
    let original = Lockfile::read(plan.workspace.lockfile_path())?;
    for package in &original.packages {
        if !plan
            .removed
            .contains(&format!("{}/{}", package.group, package.name))
        {
            continue;
        }
        // Keep the established in-place guard in the frontend; its grant does
        // not authorize deletion of a second transitive working tree.
        if package.materialization.is_in_place() {
            return Err(refuse(
                "recoverable removal of in-place clones requires an explicit per-slot recovery plan",
            ));
        }
        let relative = PathBuf::from(vibe_workspace::vibedeps::slot_rel_path(
            &package.group,
            &package.name,
            &package.version,
        ));
        vibe_workspace::vibedeps::validate_owned_slot_removal(
            &plan.workspace.root.join(&relative),
        )?;
        slots.push(relative);
    }
    let desired_workspace =
        Workspace::discover_with_selected_manifest(&plan.project_root, &plan.manifest)?;
    vibe_workspace::install::preflight_removal(&desired_workspace, &plan.lockfile, spec_format)?;
    plan.preconditions.validate()?;
    let mut transaction =
        vibe_workspace::configuration_transaction::ConfigurationTransaction::begin(
            &plan.workspace,
            &slots,
            plan.explanation(),
        )?;
    transaction.enter("manifest", false)?;
    plan.manifest
        .write(plan.project_root.join(Manifest::FILENAME))?;
    transaction.checkpoint()?;
    transaction.enter("lock", false)?;
    plan.lockfile.meta.generated_at = vibe_core::timestamp::now_utc();
    plan.lockfile.write(plan.workspace.lockfile_path())?;
    transaction.checkpoint()?;
    transaction.enter("slot", false)?;
    for slot in &slots {
        let path = plan.workspace.root.join(slot);
        vibe_workspace::vibedeps::validate_owned_slot_removal(&path)?;
        std::fs::remove_dir_all(&path).map_err(|e| vibe_workspace::WorkspaceError::Io {
            path,
            reason: e.to_string(),
        })?;
    }
    transaction.checkpoint()?;
    transaction.enter("boot", false)?;
    let workspace = Workspace::discover_with_selected_manifest(&plan.project_root, &plan.manifest)?;
    vibe_workspace::install::regenerate_boot_with_spec_format(&workspace, spec_format)?;
    transaction.checkpoint()?;
    transaction.complete()?;
    Ok(())
}

/// Prepared configured removal uses newly admitted authored intent and the
/// incumbent whole-world planner. Its selected graph is never reconstructed
/// from resolved choices in the old lock.
#[derive(Debug)]
pub struct AdmittedRemovalPlan {
    plan: crate::PlannedInstall,
    explanation: String,
    removed: Vec<String>,
}
impl AdmittedRemovalPlan {
    pub fn explanation(&self) -> &str {
        &self.explanation
    }
    pub fn removed(&self) -> &[String] {
        &self.removed
    }
}

#[allow(clippy::too_many_arguments)]
pub fn plan_admitted_removal<S: crate::InstallSource + ?Sized>(
    source: &S,
    workspace: &Workspace,
    project_root: &Path,
    group: &Group,
    name: &str,
    request: crate::InstallRequest,
    spec_format: vibe_core::manifest::SpecFormat,
    observer: &dyn crate::PlanObserver,
) -> crate::error::Result<AdmittedRemovalPlan> {
    let admitted = source.normalized_request().ok_or_else(|| {
        refuse("NeedsSelection: removal requires current admitted authored intent")
    })?;
    if !request.roots.is_empty() {
        return Err(refuse("removal cannot add CLI install roots"));
    }
    let original = Lockfile::read(workspace.lockfile_path())?;
    let selected = format!("{group}/{name}");
    if original.find(group, name).is_none() {
        return Err(refuse(format!("`{selected}` is not installed")));
    }
    let mut manifest = Manifest::read(project_root.join(Manifest::FILENAME))?;
    let before = manifest.requires.packages.len() + manifest.requires.git_packages.len();
    manifest
        .requires
        .packages
        .retain(|p| !(p.group.as_ref() == Some(group) && p.name == name));
    manifest
        .requires
        .git_packages
        .retain(|p| !(&p.group == group && p.name == name));
    if before == manifest.requires.packages.len() + manifest.requires.git_packages.len() {
        return Err(refuse(
            "selected package is not an explicit declaration of this workspace node",
        ));
    }
    let mut desired_workspace =
        Workspace::discover_with_selected_manifest(project_root, &manifest)?;
    let crate::Plan::Ready(mut plan) = crate::plan::plan_prepared_mode(
        source,
        project_root,
        &mut manifest,
        &mut desired_workspace,
        request,
        spec_format,
        observer,
        true,
    )?
    else {
        return Err(refuse("removal requires a complete selected desired world"));
    };
    let canonical = |roots: &[PackageRef]| {
        let mut roots = roots.iter().map(ToString::to_string).collect::<Vec<_>>();
        roots.sort();
        roots
    };
    if plan.captured_catalog.is_none() || canonical(&admitted.roots) != canonical(&plan.roots) {
        return Err(refuse(
            "admitted removal root union disagrees with workspace declarations",
        ));
    }
    let desired = plan
        .resolution
        .iter()
        .map(|p| format!("{}/{}", p.group, p.name))
        .collect::<BTreeSet<_>>();
    let mut affected = BTreeSet::from([selected.clone()]);
    loop {
        let additions = original
            .packages
            .iter()
            .filter(|p| {
                !affected.contains(&format!("{}/{}", p.group, p.name))
                    && p.dependencies
                        .iter()
                        .any(|edge| coordinate(edge).is_ok_and(|id| affected.contains(&id)))
            })
            .map(|p| format!("{}/{}", p.group, p.name))
            .collect::<Vec<_>>();
        if additions.is_empty() {
            break;
        }
        affected.extend(additions);
    }
    affected.remove(&selected);
    let removed = original
        .packages
        .iter()
        .filter(|p| !desired.contains(&format!("{}/{}", p.group, p.name)))
        .map(|p| format!("{}/{}", p.group, p.name))
        .collect::<Vec<_>>();
    for package in &original.packages {
        if !removed.contains(&format!("{}/{}", package.group, package.name)) {
            continue;
        }
        if package.materialization.is_in_place() {
            return Err(refuse(
                "in-place removal requires explicit per-slot destructive approval",
            ));
        }
        vibe_workspace::vibedeps::validate_owned_slot_removal(
            &vibe_workspace::vibedeps::slot_abs_path(
                &workspace.root,
                &package.group,
                &package.name,
                &package.version,
            ),
        )?;
    }
    plan.manifest_pending = true;
    plan.authored_reconfiguration = true;
    let explanation = format!(
        "remove declaration {selected}; affected dependents: [{}]; keep: [{}]; remove owned slots: [{}]",
        affected.into_iter().collect::<Vec<_>>().join(", "),
        desired.into_iter().collect::<Vec<_>>().join(", "),
        removed.join(", ")
    );
    Ok(AdmittedRemovalPlan {
        plan: *plan,
        explanation,
        removed,
    })
}

/// Apply the confirmed prepared removal through the ordinary guarded install
/// writers, including selected-catalog validation, journalling and owned pruning.
pub fn apply_admitted_removal<S: crate::InstallSource + ?Sized>(
    source: &S,
    plan: AdmittedRemovalPlan,
    integrity: vibe_core::user_config::SlotIntegrity,
    spec_format: vibe_core::manifest::SpecFormat,
    hooks: &vibe_workspace::hooks::HookPolicy,
) -> crate::error::Result<crate::ApplyReport> {
    crate::apply_with_spec_format(source, plan.plan, integrity, spec_format, hooks)
}

/// Removing a root is monotone over the exact locked selected edges. No solver
/// may choose another provider or configuration as an incidental removal effect.
/// Unsupported root forms refuse, preserving the whole installed world.
pub fn plan_removal(
    workspace: &Workspace,
    project_root: &Path,
    group: &Group,
    name: &str,
) -> crate::error::Result<RemovalPlan> {
    let preconditions = crate::preconditions::FilePreconditions::capture(workspace)?;
    let mut manifest = Manifest::read(project_root.join(Manifest::FILENAME))?;
    let mut lockfile = Lockfile::read(workspace.lockfile_path())?;
    let selected = format!("{group}/{name}");
    if lockfile.find(group, name).is_none() {
        return Err(refuse(format!("`{selected}` is not installed")));
    }
    let original = lockfile.clone();
    let before = manifest.requires.packages.len() + manifest.requires.git_packages.len();
    manifest
        .requires
        .packages
        .retain(|p| !(p.group.as_ref() == Some(group) && p.name == name));
    manifest
        .requires
        .git_packages
        .retain(|p| !(&p.group == group && p.name == name));
    let explicit_root_removed =
        before != manifest.requires.packages.len() + manifest.requires.git_packages.len();

    let mut graph = BTreeMap::new();
    for package in &original.packages {
        let id = format!("{}/{}", package.group, package.name);
        if graph.insert(id.clone(), package).is_some() {
            return Err(refuse(format!("duplicate locked coordinate `{id}`")));
        }
    }
    // Validate the ENTIRE graph before using any part of it as a keep set.
    for (id, package) in &graph {
        for edge in &package.dependencies {
            let target = coordinate(edge)?;
            let row = graph.get(&target).ok_or_else(|| {
                refuse(format!(
                    "incomplete removal graph: `{id}` requires `{target}`"
                ))
            })?;
            if !edge.version.matches(&row.version) {
                return Err(refuse(format!("locked edge `{id}` -> `{edge}` disagrees")));
            }
        }
    }

    let mut roots = Vec::new();
    let mut explanations = BTreeSet::new();
    for (rel, current) in workspace.iter_nodes() {
        let current = if workspace.node_abs_path(rel) == project_root {
            &manifest
        } else {
            current
        };
        // Technical disjunction/capability roots need their actual selected
        // witness; never infer one from a possible-provider catalogue.
        if !current.requires.capabilities.is_empty() || !current.requires_any.is_empty() {
            return Err(refuse(
                "removal requires selected witnesses for capability/OR workspace roots",
            ));
        }
        for root in &current.requires.packages {
            let id = coordinate(root)?;
            let row = graph
                .get(&id)
                .ok_or_else(|| refuse(format!("workspace root `{rel}` requires missing `{id}`")))?;
            if !root.version.matches(&row.version) {
                return Err(refuse(format!("workspace root `{rel}` changed: `{root}`")));
            }
            if id == selected {
                explanations.insert(format!("workspace node {rel}"));
            }
            roots.push(root.clone());
        }
        for root in &current.requires.git_packages {
            let id = format!("{}/{}", root.group, root.name);
            let row = graph
                .get(&id)
                .ok_or_else(|| refuse(format!("missing git root `{id}`")))?;
            if id == selected {
                explanations.insert(format!("workspace node {rel}"));
            }
            roots.push(crate::exact_pinned_pkgref(&vibe_resolver::ResolvedNode {
                group: row.group.clone(),
                name: row.name.to_string(),
                version: row.version.clone(),
                dependencies: vec![],
                is_root: true,
            }));
        }
    }
    let mut reverse = BTreeSet::from([selected.clone()]);
    loop {
        let additions = graph
            .iter()
            .filter(|(id, p)| {
                !reverse.contains(*id)
                    && p.dependencies
                        .iter()
                        .any(|edge| coordinate(edge).is_ok_and(|target| reverse.contains(&target)))
            })
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if additions.is_empty() {
            break;
        }
        reverse.extend(additions);
    }
    reverse.remove(&selected);
    explanations.extend(reverse);
    let mut desired = BTreeSet::new();
    let mut pending = roots
        .iter()
        .map(coordinate)
        .collect::<crate::error::Result<Vec<_>>>()?;
    while let Some(id) = pending.pop() {
        if desired.insert(id.clone()) {
            pending.extend(
                graph[&id]
                    .dependencies
                    .iter()
                    .map(coordinate)
                    .collect::<crate::error::Result<Vec<_>>>()?,
            );
        }
    }
    if !explicit_root_removed && desired.contains(&selected) {
        return Err(refuse(format!(
            "`{selected}` is required, not an explicit root of this node; affected dependents: [{}]",
            explanations.into_iter().collect::<Vec<_>>().join(", ")
        )));
    }
    let removed = graph
        .keys()
        .filter(|id| !desired.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    // A configured request cannot be reconstructed from the derived graph.
    // Keep the exact D1 authority until a caller supplies a new admitted request.
    if original.meta.configuration.as_ref().is_some_and(|record| {
        vibe_wire::behaviour::lock_configuration::decode(record)
            .map_or(true, |(request, _)| request.configuration.is_some())
    }) {
        return Err(refuse(
            "configured removal requires a newly admitted authored request; retained choices cannot be inferred from the lock graph",
        ));
    }
    lockfile
        .packages
        .retain(|p| desired.contains(&format!("{}/{}", p.group, p.name)));
    lockfile.meta.root_dependencies = roots.clone();
    lockfile.meta.active_features = lockfile
        .packages
        .iter()
        .flat_map(|p| {
            p.features
                .iter()
                .map(|f| format!("{}/{}/{f}", p.group, p.name))
        })
        .collect();
    if let Some(record) = &original.meta.configuration {
        let (mut requested, mut resolved) =
            vibe_wire::behaviour::lock_configuration::decode(record).map_err(refuse)?;
        requested.roots = roots.iter().map(ToString::to_string).collect();
        // Retain surviving declaration origins verbatim; remove only the
        // selected node's removed coordinate, not another member's declaration.
        requested.origins.retain(|origin| {
            !(origin.consumer == project_root.display().to_string()
                && origin
                    .constraint
                    .parse::<PackageRef>()
                    .is_ok_and(|p| coordinate(&p).is_ok_and(|id| id == selected)))
        });
        resolved.packages.retain(|p| {
            p.package
                .parse::<PackageRef>()
                .is_ok_and(|p| coordinate(&p).is_ok_and(|id| desired.contains(&id)))
        });
        resolved.edges.retain(|e| {
            e.parent
                .parse::<PackageRef>()
                .is_ok_and(|p| coordinate(&p).is_ok_and(|id| desired.contains(&id)))
        });
        lockfile.meta.configuration = Some(
            vibe_wire::behaviour::lock_configuration::encode(requested, resolved)
                .map_err(refuse)?,
        );
    }
    Ok(RemovalPlan {
        selected,
        explicit_root_removed,
        affected_dependents: explanations.into_iter().collect(),
        retained: desired.into_iter().collect(),
        removed,
        manifest,
        lockfile,
        project_root: project_root.to_path_buf(),
        workspace: workspace.clone(),
        preconditions,
    })
}

#[cfg(test)]
mod m_05_e_3_cases {
    use super::*;

    fn package(name: &str, dependencies: &[&str]) -> vibe_core::manifest::LockedPackage {
        vibe_core::manifest::LockedPackage {
            kind: vibe_core::PackageKind::Flow,
            group: Group::parse("org.removal").unwrap(),
            name: vibe_core::PackageName::from_validated(name.into()),
            version: semver::Version::new(1, 0, 0),
            bridge: false,
            authors: vec![],
            registry: None,
            source_url: vibe_core::SourceUrl::new("file:///fixture"),
            source_ref: None,
            resolved_commit: None,
            content_hash: vibe_core::ContentHash::from_validated(format!(
                "sha256:{}",
                "1".repeat(64)
            )),
            embedded_sources: vec![],
            boot_snippet: None,
            files_written: vec![],
            dependencies: dependencies
                .iter()
                .map(|name| format!("org.removal/{name}@=1.0.0").parse().unwrap())
                .collect(),
            admitted_by: None,
            via_override: None,
            overridden: false,
            source_kind: None,
            via_redirect: None,
            features: vec![],
            subskills_active: vec![],
            describes: None,
            language: None,
            materialization: vibe_core::manifest::Materialization::Copy,
        }
    }

    #[test]
    fn removal_explains_reverse_closure_and_keeps_shared_transitives_before_writes() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("vibe.toml"), "[project]\nname='remove'\nversion='1.0.0'\n[requires.packages]\n'org.removal/a'='*'\n'org.removal/b'='*'\n").unwrap();
        let mut lock = Lockfile::empty("fixture", "2026-10-17T00:00:00Z");
        lock.packages = vec![
            package("a", &["shared"]),
            package("b", &["shared"]),
            package("shared", &[]),
        ];
        lock.write(temp.path().join("vibe.lock")).unwrap();
        let workspace = Workspace::discover(temp.path()).unwrap();
        let before = std::fs::read(workspace.lockfile_path()).unwrap();
        let group = Group::parse("org.removal").unwrap();
        let plan = plan_removal(&workspace, &workspace.root, &group, "a").unwrap();
        assert_eq!(plan.removed, ["org.removal/a"]);
        assert_eq!(plan.retained, ["org.removal/b", "org.removal/shared"]);
        assert!(
            plan.explanation()
                .contains("keep: [org.removal/b, org.removal/shared]")
        );
        assert_eq!(before, std::fs::read(workspace.lockfile_path()).unwrap());
        let refusal = plan_removal(&workspace, &workspace.root, &group, "shared")
            .unwrap_err()
            .to_string();
        assert!(refusal.contains("org.removal/a") && refusal.contains("org.removal/b"));
        assert_eq!(before, std::fs::read(workspace.lockfile_path()).unwrap());
    }

    #[test]
    fn incomplete_world_and_confirmation_drift_refuse_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("vibe.toml"),
            "[project]\nname='remove'\nversion='1.0.0'\n[requires.packages]\n'org.removal/a'='*'\n",
        )
        .unwrap();
        let mut lock = Lockfile::empty("fixture", "2026-10-17T00:00:00Z");
        lock.packages = vec![package("a", &["missing"])];
        lock.write(temp.path().join("vibe.lock")).unwrap();
        let workspace = Workspace::discover(temp.path()).unwrap();
        let group = Group::parse("org.removal").unwrap();
        assert!(
            plan_removal(&workspace, &workspace.root, &group, "a")
                .unwrap_err()
                .to_string()
                .contains("incomplete removal graph")
        );
        lock.packages[0].dependencies.clear();
        lock.write(workspace.lockfile_path()).unwrap();
        let plan = plan_removal(&workspace, &workspace.root, &group, "a").unwrap();
        std::fs::write(workspace.root.join("vibe.toml"), "changed").unwrap();
        let before = std::fs::read(workspace.lockfile_path()).unwrap();
        assert!(
            apply_removal(plan, vibe_core::manifest::SpecFormat::Mixed)
                .unwrap_err()
                .to_string()
                .contains("changed since planning")
        );
        assert_eq!(before, std::fs::read(workspace.lockfile_path()).unwrap());
    }
}
