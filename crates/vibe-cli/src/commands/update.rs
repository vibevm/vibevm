//! `vibe update [<pkgref>...] [--all]` — re-fetch installed packages
//! against their original root constraint, diff project files, apply.
//!
//! Spec: `VIBEVM-SPEC.md` §16 (M1 acceptance), ROADMAP §M1.2.
//!
//! v0 contract (this commit):
//!
//! - Updates one or more named pkgrefs OR every root via `--all`.
//! - Per package: re-resolves through the project's
//!   `MultiRegistryResolver` (mirror dispatch + cross-source
//!   `content_hash` gate inherited transparently from the install
//!   path), fetches the new content into the per-project cache, then
//!   asks `vibe-install::plan_update` for a per-file diff (added /
//!   removed / modified / identical / user-edited).
//! - Refuses to apply when any project file is user-edited (bytes
//!   diverge from the install-time cache) — the operator runs
//!   `vibe uninstall && vibe install` to consciously discard the
//!   edits or back them up.
//! - Refuses to apply when the new manifest's `[requires]` shape
//!   differs from the locked transitive set — narrow v0 does not
//!   cascade graph changes.
//! - `vibe update` does not touch transitive packages directly. They
//!   get re-fetched on `--all` only insofar as they are roots
//!   themselves; non-root transitives stay where the install pinned
//!   them. Broader graph evolution lands later.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use dialoguer::Confirm;
use serde::Serialize;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{PackageKind, PackageRef};
use vibe_install::{
    InstallError, UpdateChange, UpdatePlan, apply_update, plan_update, register_updated,
};
use vibe_registry::{LocalRegistry, MultiRegistryResolver};

use crate::cli::UpdateArgs;
use crate::output;

pub fn run(ctx: &output::Context, args: UpdateArgs) -> Result<()> {
    let project_root = resolve_project_root(&args.path)?;
    let mut manifest = load_project_manifest(&project_root)?;
    let mut lockfile = load_or_empty_lockfile(&project_root)?;

    if !args.all && args.packages.is_empty() {
        bail!(
            "vibe update: pass at least one `<kind>:<name>` argument or `--all`"
        );
    }

    if lockfile.packages.is_empty() {
        bail!(
            "vibe update: lockfile in `{}` is empty — nothing to update",
            project_root.display()
        );
    }

    if manifest.registries.is_empty() {
        bail!(
            "no `[[registry]]` configured in `{}/vibe.toml`. `vibe update` re-fetches from the registry; \
             configure one with `vibe registry add <name> <url>` before retrying.",
            project_root.display()
        );
    }

    let resolver = MultiRegistryResolver::open(
        &manifest.registries,
        &manifest.mirrors,
        &manifest.overrides,
    )
    .context("opening multi-registry resolver")?
    .with_strict_auth(args.auth_required)
    .with_git_packages(manifest.requires.git_packages.clone());

    // 1. Decide which packages to update. `--all` walks every entry
    // in the lockfile (roots + any transitives). Named pkgrefs walk
    // exactly those. The user constraint we re-resolve against is
    // the original `root_dependencies` entry's `version`, falling
    // back to `Latest` when the package isn't recorded as a root
    // (transitives in v0 of update — re-resolved at the same exact
    // version they were locked at, which usually means a no-op).
    let targets: Vec<UpdateTarget> = if args.all {
        lockfile
            .packages
            .iter()
            .map(|entry| UpdateTarget::for_locked(entry, &lockfile.meta.root_dependencies))
            .collect()
    } else {
        let mut v: Vec<UpdateTarget> = Vec::with_capacity(args.packages.len());
        for raw in &args.packages {
            let pkgref =
                PackageRef::parse(raw).with_context(|| format!("parsing `{raw}`"))?;
            let entry = lockfile
                .find(pkgref.kind, &pkgref.name)
                .ok_or_else(|| {
                    anyhow!(
                        "package `{}:{}` is not installed in `{}`. \
                         Use `vibe install {}:{}` first, or `vibe list` to see what's installed.",
                        pkgref.kind,
                        pkgref.name,
                        project_root.display(),
                        pkgref.kind,
                        pkgref.name
                    )
                })?;
            v.push(UpdateTarget::for_locked(entry, &lockfile.meta.root_dependencies));
        }
        v
    };

    let cache_root = project_root.join(".vibe/cache");
    fs::create_dir_all(&cache_root)
        .with_context(|| format!("creating cache dir `{}`", cache_root.display()))?;

    // 2. For each target: re-resolve, fetch, build UpdatePlan, accumulate.
    let mut plans: Vec<UpdatePlan> = Vec::new();
    let mut up_to_date: Vec<UpdateTarget> = Vec::new();

    ctx.heading(&format!(
        "Re-resolving {} package{}…",
        targets.len(),
        if targets.len() == 1 { "" } else { "s" },
    ));

    for target in &targets {
        let pkgref = target.constraint_pkgref();
        let resolution = resolver
            .resolve(&pkgref)
            .with_context(|| format!("resolving `{}:{}`", target.kind, target.name))?;
        // Pass None for expected_hash here: a tag-rewrite (force-push
        // upstream) producing a different content_hash for the same
        // version is exactly what the diff is supposed to surface, not
        // hard-fail on. The cross-source gate at install time is the
        // mirror-supply-chain check; update is a deliberate refresh.
        let new_cached = resolver
            .fetch_with_expected_hash(&resolution, &cache_root, None)
            .with_context(|| format!("fetching `{}:{}`", target.kind, target.name))?;

        if new_cached.resolved.version == target.from_version
            && new_cached.content_hash == target.from_content_hash
        {
            up_to_date.push(target.clone());
            continue;
        }

        let old_cache_dir = cache_root
            .join(target.kind.as_str())
            .join(&target.name)
            .join(format!("v{}", target.from_version));
        let plan = plan_update(&project_root, &lockfile, new_cached, &old_cache_dir)
            .with_context(|| format!("planning update for `{}:{}`", target.kind, target.name))?;
        plans.push(plan);
    }

    // 3. Present what we'd do.
    present_plans(ctx, &project_root, &plans, &up_to_date);

    if plans.iter().all(|p| !p.has_changes()) {
        // Every plan is pure-Identical (or the version bumped but the
        // payload didn't change at all). Nothing to write to disk.
        // Still bump the lockfile's `generated_at` + version on each
        // entry so a force-push (same version, new content_hash) gets
        // recorded — that's a real metadata change even if no file
        // bytes shift.
        if !plans.is_empty() {
            for plan in &plans {
                register_updated(
                    &mut lockfile,
                    plan,
                    plan.changes
                        .iter()
                        .filter_map(|c| match c {
                            UpdateChange::Identical { target_rel } => Some(target_rel.clone()),
                            _ => None,
                        })
                        .collect(),
                    crate::commands::init::current_timestamp_utc(),
                )?;
            }
            lockfile.write(project_root.join(Lockfile::FILENAME))?;
        }
        emit_report(ctx, &plans, &up_to_date, &project_root, &[])?;
        return Ok(());
    }

    // 4. Confirm.
    let approved = if args.assume_yes || ctx.is_unattended() || ctx.is_json() {
        true
    } else if !console::user_attended() {
        bail!(
            "no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively"
        );
    } else {
        let total_changes: usize = plans
            .iter()
            .map(|p| {
                p.changes
                    .iter()
                    .filter(|c| !matches!(c, UpdateChange::Identical { .. }))
                    .count()
            })
            .sum();
        let prompt = format!(
            "Apply this update plan ({} change{} across {} package{})?",
            total_changes,
            if total_changes == 1 { "" } else { "s" },
            plans.len(),
            if plans.len() == 1 { "" } else { "s" },
        );
        Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .context("reading user confirmation")?
    };
    if !approved {
        return Err(InstallError::UserDeclined.into());
    }

    // 5. Apply each plan; update lockfile after each success.
    let mut applied: Vec<AppliedReport> = Vec::new();
    for plan in &plans {
        let label = plan.package_label();
        ctx.step(&format!(
            "Updating {label} from {} to {}",
            plan.from_version, plan.to_version
        ));
        let written = apply_update(plan)?;
        register_updated(
            &mut lockfile,
            plan,
            written.clone(),
            crate::commands::init::current_timestamp_utc(),
        )?;
        applied.push(AppliedReport {
            package: label,
            from_version: plan.from_version.to_string(),
            to_version: plan.to_version.to_string(),
            added: count_changes(plan, |c| matches!(c, UpdateChange::Added { .. })),
            removed: count_changes(plan, |c| matches!(c, UpdateChange::Removed { .. })),
            modified: count_changes(plan, |c| matches!(c, UpdateChange::Modified { .. })),
            identical: count_changes(plan, |c| matches!(c, UpdateChange::Identical { .. })),
        });
    }

    // `--exact`: tighten each updated root's manifest constraint to
    // the freshly-resolved exact version. Equivalent of cargo's
    // `cargo update --precise X.Y.Z` plus a manifest pin in one
    // step. Only applied to packages declared in `vibe.toml`
    // `[requires].packages` — non-root transitives are not in the
    // manifest. No-op when --exact wasn't passed.
    if args.exact && !plans.is_empty() {
        let mut manifest_changed = false;
        for plan in &plans {
            let kind = plan.kind;
            let name = plan.name.clone();
            let version = plan.to_version.clone();
            let pos = manifest
                .requires
                .packages
                .iter()
                .position(|r| r.kind == kind && r.name == name);
            if let Some(i) = pos {
                let req = semver::VersionReq::parse(&format!("={version}"))
                    .expect("`=<version>` always parses as VersionReq");
                manifest.requires.packages[i] = vibe_core::PackageRef {
                    kind,
                    name,
                    version: vibe_core::VersionSpec::Req(req),
                };
                manifest_changed = true;
            }
        }
        if manifest_changed {
            manifest.write(project_root.join(Manifest::FILENAME))?;
        }
    }

    lockfile.write(project_root.join(Lockfile::FILENAME))?;
    emit_report(ctx, &plans, &up_to_date, &project_root, &applied)?;
    Ok(())
}

fn count_changes<F: Fn(&UpdateChange) -> bool>(plan: &UpdatePlan, pred: F) -> usize {
    plan.changes.iter().filter(|c| pred(c)).count()
}

#[derive(Debug, Clone)]
struct UpdateTarget {
    kind: PackageKind,
    name: String,
    from_version: semver::Version,
    from_content_hash: String,
    /// Original root constraint typed by the user, if this package
    /// is recorded under `[meta].root_dependencies`. `None` when the
    /// package is a non-root transitive — re-resolved at the exact
    /// pinned version (treats transitive update as a no-op unless
    /// upstream force-pushed).
    root_constraint: Option<PackageRef>,
}

impl UpdateTarget {
    fn for_locked(
        entry: &vibe_core::manifest::LockedPackage,
        roots: &[PackageRef],
    ) -> Self {
        let root_constraint = roots
            .iter()
            .find(|r| r.kind == entry.kind && r.name == entry.name)
            .cloned();
        UpdateTarget {
            kind: entry.kind,
            name: entry.name.clone(),
            from_version: entry.version.clone(),
            from_content_hash: entry.content_hash.clone(),
            root_constraint,
        }
    }

    /// Pkgref to feed `MultiRegistryResolver::resolve` with. For roots,
    /// preserve the original constraint (`Latest` / `^0.1` / etc.).
    /// For non-roots, pin to the exact locked version — they only
    /// move on a force-push.
    fn constraint_pkgref(&self) -> PackageRef {
        if let Some(root) = &self.root_constraint {
            return root.clone();
        }
        let req = semver::VersionReq::parse(&format!("={}", self.from_version))
            .expect("exact version always parses as VersionReq");
        PackageRef {
            kind: self.kind,
            name: self.name.clone(),
            version: vibe_core::VersionSpec::Req(req),
        }
    }
}

#[derive(Debug, Serialize)]
struct AppliedReport {
    package: String,
    from_version: String,
    to_version: String,
    added: usize,
    removed: usize,
    modified: usize,
    identical: usize,
}

fn present_plans(
    ctx: &output::Context,
    project_root: &Path,
    plans: &[UpdatePlan],
    up_to_date: &[UpdateTarget],
) {
    if ctx.is_json() {
        #[derive(Serialize)]
        struct JsonPlanEntry<'a> {
            package: String,
            from_version: String,
            to_version: String,
            from_content_hash: &'a str,
            to_content_hash: &'a str,
            changes: Vec<JsonChange<'a>>,
        }
        #[derive(Serialize)]
        struct JsonChange<'a> {
            kind: &'static str, // "added" | "removed" | "modified" | "identical"
            target: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            source: Option<&'a Path>,
        }
        let payload: Vec<JsonPlanEntry<'_>> = plans
            .iter()
            .map(|p| JsonPlanEntry {
                package: p.package_label(),
                from_version: p.from_version.to_string(),
                to_version: p.to_version.to_string(),
                from_content_hash: &p.from_content_hash,
                to_content_hash: &p.to_content_hash,
                changes: p
                    .changes
                    .iter()
                    .map(|c| match c {
                        UpdateChange::Added { target_rel, source_abs, .. } => JsonChange {
                            kind: "added",
                            target: target_rel.to_string_lossy().replace('\\', "/"),
                            source: Some(source_abs.as_path()),
                        },
                        UpdateChange::Removed { target_rel, .. } => JsonChange {
                            kind: "removed",
                            target: target_rel.to_string_lossy().replace('\\', "/"),
                            source: None,
                        },
                        UpdateChange::Modified { target_rel, source_abs, .. } => JsonChange {
                            kind: "modified",
                            target: target_rel.to_string_lossy().replace('\\', "/"),
                            source: Some(source_abs.as_path()),
                        },
                        UpdateChange::Identical { target_rel } => JsonChange {
                            kind: "identical",
                            target: target_rel.to_string_lossy().replace('\\', "/"),
                            source: None,
                        },
                    })
                    .collect(),
            })
            .collect();
        let envelope = serde_json::json!({
            "command": "update:plan",
            "plans": payload,
            "up_to_date": up_to_date
                .iter()
                .map(|t| format!("{}:{}@{}", t.kind, t.name, t.from_version))
                .collect::<Vec<_>>(),
        });
        let _ = ctx.emit_json(&envelope);
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    for t in up_to_date {
        ctx.step(&format!("up-to-date  {}:{}@{}", t.kind, t.name, t.from_version));
    }
    for plan in plans {
        ctx.heading(&format!(
            "\nPlan for {} ({} → {})",
            plan.package_label(),
            plan.from_version,
            plan.to_version,
        ));
        for change in &plan.changes {
            let (sigil, rel_path) = match change {
                UpdateChange::Added { target_rel, .. } => ("[+]", target_rel),
                UpdateChange::Removed { target_rel, .. } => ("[-]", target_rel),
                UpdateChange::Modified { target_rel, .. } => ("[~]", target_rel),
                UpdateChange::Identical { target_rel } => ("[=]", target_rel),
            };
            let rel_s = rel_path.to_string_lossy().replace('\\', "/");
            // `rel_s` is already project-relative; project_root only
            // rendered for absolute-path debugging.
            let _ = project_root;
            println!("  {sigil}  {}", rel_s);
        }
    }
    println!();
}

fn emit_report(
    ctx: &output::Context,
    plans: &[UpdatePlan],
    up_to_date: &[UpdateTarget],
    project_root: &Path,
    applied: &[AppliedReport],
) -> Result<()> {
    if ctx.is_json() {
        let payload = serde_json::json!({
            "ok": true,
            "command": "update",
            "project": project_root.display().to_string(),
            "updated": applied,
            "up_to_date": up_to_date
                .iter()
                .map(|t| format!("{}:{}@{}", t.kind, t.name, t.from_version))
                .collect::<Vec<_>>(),
        });
        ctx.emit_json(&payload)?;
        return Ok(());
    }
    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe update: {} updated, {} already up-to-date",
            applied.len(),
            up_to_date.len()
        ));
        return Ok(());
    }
    if plans.is_empty() && !up_to_date.is_empty() {
        ctx.summary(&format!(
            "\nvibe update: {} package{} already up-to-date.",
            up_to_date.len(),
            if up_to_date.len() == 1 { "" } else { "s" },
        ));
        return Ok(());
    }
    for a in applied {
        ctx.created(&format!(
            "{} {}→{} (+{} -{} ~{}{})",
            a.package,
            a.from_version,
            a.to_version,
            a.added,
            a.removed,
            a.modified,
            if a.identical > 0 {
                format!(" ={}", a.identical)
            } else {
                String::new()
            },
        ));
    }
    if !applied.is_empty() {
        ctx.summary(&format!(
            "\nUpdated {} package{} ({} up-to-date).",
            applied.len(),
            if applied.len() == 1 { "" } else { "s" },
            up_to_date.len(),
        ));
    }
    Ok(())
}

fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = super::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

fn load_project_manifest(root: &Path) -> Result<Manifest> {
    let path = root.join(Manifest::FILENAME);
    Ok(Manifest::read(&path)?)
}

fn load_or_empty_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if path.exists() {
        Ok(Lockfile::read(&path)?)
    } else {
        Ok(Lockfile::empty(
            format!("vibe {}", env!("CARGO_PKG_VERSION")),
            crate::commands::init::current_timestamp_utc(),
        ))
    }
}

// `LocalRegistry` is intentionally *not* used here: vibe update only
// makes sense against a real registry that can present new versions.
// Bringing it in would let `--registry <path>` reach `vibe update`,
// but the local-directory model has no version-bump mechanism — every
// new version requires a manual fixture rewrite. Leaving the import
// stubbed out keeps the contract honest. If someone needs local-dir
// updates, that's a separate slice with explicit `--registry <path>`
// support.
#[allow(dead_code)]
fn _unused_imports_keepalive() {
    let _ = std::any::type_name::<LocalRegistry>();
}
