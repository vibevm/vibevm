//! The deterministic engine: join the lockfile graph, the committed boot
//! artifacts, and the node manifests into the [`PackageTree`] model
//! (PROP-036 §2.3–§2.5, §3).
//!
//! The effective load type is read from the committed `STATIC.md` /
//! `INDEX.md` — what an agent actually reads at boot — never a fresh
//! recompute (PROP-036 §2.3 decision).

specmark::scope!("spec://vibevm/modules/vibe-cli/PROP-036#effective-load");

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use vibe_core::manifest::{LinkType, LockedPackage, Lockfile, Manifest};
use vibe_spec::Directives;

use super::artifacts::{self, IndexParse};
use super::diagnostics;
use super::model::{
    Boot, Carrier, Condition, ConditionKind, DeclaredLink, HOST_NAMESPACE, InPlaceSpec, IndexLane,
    Load, LoadOrigin, LoadType, Package, PackageTree, Project, SCHEMA_VERSION, Source, SourceKind,
    StaticLane,
};

/// The host-authored boot files that seed the in-place `@spec` scan
/// (PROP-036 §2.9). The full scan set (built in [`build_tree`]) adds every
/// `STATIC.md` contribution source and every `INDEX.md` entry path — every
/// boot source file that actually loads. The generated `STATIC.md` /
/// `INDEX.md` themselves are never scanned: they are artifacts, not authored
/// boot lanes.
const HOST_BOOT_FILES: &[&str] = &["00-core.md", "90-user.md"];

/// Build the full [`PackageTree`] model for the project rooted at `root`.
///
/// Read-only: touches only `vibe.lock`, `vibe.toml`, the committed boot
/// artifacts, and each package's materialised `vibedeps/` manifest. Never
/// mutates the tree (PROP-036 §2.1).
pub fn build_tree(root: &Path) -> Result<PackageTree> {
    let lockfile = load_lockfile(root)?;
    let manifest = Manifest::read(root.join(Manifest::FILENAME))
        .with_context(|| format!("reading {}", root.join(Manifest::FILENAME).display()))?;

    // Committed boot artifacts — the effective lanes (PROP-036 §2.3).
    let static_text = read_opt(&root.join("spec/boot/STATIC.md"));
    let index_text = read_opt(&root.join("spec/boot/INDEX.md"));
    let static_contribs = static_text
        .as_deref()
        .map(artifacts::decompile_static)
        .unwrap_or_default();
    let index = match index_text.as_deref() {
        Some(t) => artifacts::read_index(t)?,
        None => IndexParse {
            static_pointer: None,
            entries: Vec::new(),
        },
    };

    // Lookup tables.
    let by_id: BTreeMap<String, &LockedPackage> = lockfile
        .packages
        .iter()
        .map(|p| (qualified(p), p))
        .collect();
    // STATIC.md origin ("group/name" or host rel-path) → source path.
    let static_origins: BTreeMap<&str, &str> = static_contribs
        .iter()
        .map(|c| (c.origin.as_str(), c.source_path.as_str()))
        .collect();
    // INDEX.md slot (kind, name) → the entry, for the packages it names.
    let mut index_by_slot: BTreeMap<(String, String), &super::model::IndexEntry> = BTreeMap::new();
    for entry in &index.entries {
        if let Some((kind, name)) = artifacts::slot_package(&entry.path) {
            index_by_slot.insert((kind.to_string(), name), entry);
        }
    }

    // Static-transitive closures: BFS each root edge declared
    // `static-transitive` over the lockfile dependency graph (PROP-036 §2.4).
    let declarers: BTreeSet<String> = manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(group, name)| {
            let group = group?;
            match manifest.requires.declared_link(group, name) {
                Some(LinkType::StaticTransitive) => Some(format!("{group}/{name}")),
                _ => None,
            }
        })
        .collect();
    let st_closure = static_transitive_closure(&by_id, &declarers);

    // Contract (PROP-036 §2.4, scaffold-c): a well-formed lockfile is
    // dependency-closed, so every static-transitive closure member is itself a
    // resolved package — the classifier only ever asks `st_closure.contains`
    // for ids that exist in `by_id`. (A declarer MAY also appear in another
    // declarer's closure — that is legal; `classify_origin` gives it `Declared`,
    // the declarer branch winning over in-closure, so the closure is NOT
    // asserted disjoint from `declarers`.)
    debug_assert!(
        st_closure.iter().all(|id| by_id.contains_key(id)),
        "static-transitive closure over a dependency-closed lockfile must contain only resolved packages"
    );

    // Per-package suggested link, from the materialised slot manifest.
    let suggested: BTreeMap<String, Option<LinkType>> = lockfile
        .packages
        .iter()
        .map(|p| (qualified(p), slot_suggested_link(root, p)))
        .collect();

    let packages = lockfile
        .packages
        .iter()
        .map(|p| {
            build_package(
                &manifest,
                p,
                &static_origins,
                &index_by_slot,
                &declarers,
                &st_closure,
                suggested.get(&qualified(p)).copied().flatten(),
            )
        })
        .collect();

    let roots: Vec<String> = manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(group, name)| group.map(|g| format!("{g}/{name}")))
        .collect();

    // Non-fatal diagnostics (PROP-036 §2.10) — computed before `roots` and the
    // boot artifacts below are moved into the model.
    let diagnostics = diagnostics::check(&roots, &lockfile);

    // Every boot source file that actually loads (PROP-036 §2.9): the two
    // host-authored boot files, plus every `STATIC.md` contribution source and
    // every `INDEX.md` entry path. Deduped, project-relative.
    let mut boot_file_set: BTreeSet<String> = HOST_BOOT_FILES
        .iter()
        .map(|n| format!("spec/boot/{n}"))
        .collect();
    boot_file_set.extend(static_contribs.iter().map(|c| c.source_path.clone()));
    boot_file_set.extend(index.entries.iter().map(|e| e.path.clone()));
    let boot_files: Vec<String> = boot_file_set.into_iter().collect();

    let boot = Boot {
        static_md: static_text.as_deref().map(|t| StaticLane {
            present: true,
            path: "spec/boot/STATIC.md".to_string(),
            bytes: t.len() as u64,
            lines: t.lines().count() as u64,
            contributions: static_contribs,
        }),
        index_md: IndexLane {
            present: index_text.is_some(),
            path: "spec/boot/INDEX.md".to_string(),
            static_pointer: index.static_pointer,
            entries: index.entries,
        },
    };

    let project = Project {
        root: root.display().to_string(),
        name: manifest.project.as_ref().map(|p| p.name.clone()),
        is_workspace: manifest.is_workspace_root(),
        host_namespace: HOST_NAMESPACE.to_string(),
    };

    Ok(PackageTree {
        schema_version: SCHEMA_VERSION,
        generated_at: Some(chrono::Utc::now().to_rfc3339()),
        tool_version: Some(format!("vibe {}", env!("CARGO_PKG_VERSION"))),
        project,
        roots,
        packages,
        boot,
        in_place_specs: collect_in_place(root, &boot_files),
        // Root-drift lands here now; the stale-artifacts check (needs a fresh
        // EffectiveBoot recompute) is still deferred — PROP-036 §2.10.
        diagnostics,
    })
}

/// Assemble one [`Package`] — its effective lane, flags, source, and edges.
#[allow(clippy::too_many_arguments)]
fn build_package(
    manifest: &Manifest,
    p: &LockedPackage,
    static_origins: &BTreeMap<&str, &str>,
    index_by_slot: &BTreeMap<(String, String), &super::model::IndexEntry>,
    declarers: &BTreeSet<String>,
    st_closure: &BTreeSet<String>,
    suggested: Option<LinkType>,
) -> Package {
    let id = qualified(p);
    let slot_key = (p.kind.as_str().to_string(), p.name.to_string());

    // Effective lane, read from the committed artifacts (PROP-036 §2.3).
    let (load_type, in_static, in_index, boot_path, condition) =
        if let Some(src) = static_origins.get(id.as_str()) {
            (
                LoadType::Static,
                true,
                false,
                Some((*src).to_string()),
                Condition::absent(),
            )
        } else if let Some(entry) = index_by_slot.get(&slot_key) {
            (
                LoadType::Dynamic,
                false,
                true,
                Some(entry.path.clone()),
                condition_from_when(entry.when.as_deref()),
            )
        } else {
            (LoadType::None, false, false, None, Condition::absent())
        };

    // Contract (PROP-036 §2.3, scaffold-c witness at the use site): the
    // effective lane is mutually exclusive — a package lands in STATIC.md or
    // INDEX.md or neither, never both, and each flag matches the load type.
    debug_assert!(
        !(in_static && in_index),
        "{id}: a package cannot be in both STATIC.md and INDEX.md"
    );
    debug_assert_eq!(
        in_static,
        load_type == LoadType::Static,
        "{id}: in_static_md must hold iff the effective load is static"
    );
    debug_assert_eq!(
        in_index,
        load_type == LoadType::Dynamic,
        "{id}: in_index_md must hold iff the effective load is dynamic"
    );

    let declared = manifest.requires.declared_link(&p.group, &p.name);
    let (transitive, origin) = classify_origin(
        load_type,
        condition.present,
        declarers.contains(&id),
        st_closure.contains(&id),
        declared,
        suggested,
    );

    Package {
        id,
        group: p.group.to_string(),
        name: p.name.to_string(),
        kind: p.kind.as_str().to_string(),
        version: p.version.to_string(),
        content_hash: Some(p.content_hash.as_str().to_string()),
        source: Some(to_source(p)),
        load: Load {
            load_type,
            transitive,
            declared: to_declared_link(declared),
            origin,
            in_static_md: in_static,
            in_index_md: in_index,
            boot_path,
        },
        condition,
        dependencies: p.dependencies.iter().map(|d| d.qualified_name()).collect(),
    }
}

/// Decide `(transitive, origin)` from the effective type and the declared /
/// suggested / closure inputs (PROP-036 §2.3–§2.5).
fn classify_origin(
    load_type: LoadType,
    has_condition: bool,
    is_declarer: bool,
    in_closure: bool,
    declared: Option<LinkType>,
    suggested: Option<LinkType>,
) -> (bool, LoadOrigin) {
    match load_type {
        LoadType::None => (false, LoadOrigin::None),
        LoadType::Dynamic => {
            if has_condition {
                (false, LoadOrigin::WhenForced)
            } else if declared.is_some() {
                (false, LoadOrigin::Declared)
            } else {
                (false, LoadOrigin::Default)
            }
        }
        LoadType::Static => {
            if is_declarer {
                // The static-transitive declarer's static-ness is its own.
                (false, LoadOrigin::Declared)
            } else if in_closure && !is_static_link(suggested) {
                (true, LoadOrigin::StaticTransitive)
            } else if is_static_link(declared) {
                (false, LoadOrigin::Declared)
            } else if is_static_link(suggested) {
                (false, LoadOrigin::Suggested)
            } else {
                (false, LoadOrigin::Default)
            }
        }
    }
}

/// BFS the lockfile dependency graph from each declarer, collecting the
/// reachable closure. Cycle-guarded on the `group/name` key.
fn static_transitive_closure(
    by_id: &BTreeMap<String, &LockedPackage>,
    declarers: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut closure = BTreeSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    for decl in declarers {
        if let Some(pkg) = by_id.get(decl) {
            queue.extend(pkg.dependencies.iter().map(|d| d.qualified_name()));
        }
    }
    while let Some(id) = queue.pop_front() {
        if !closure.insert(id.clone()) {
            continue;
        }
        if let Some(pkg) = by_id.get(&id) {
            queue.extend(pkg.dependencies.iter().map(|d| d.qualified_name()));
        }
    }
    closure
}

/// Read the suggested `[boot_snippet].link` from a package's materialised
/// slot manifest — best-effort, `None` when the slot or field is absent.
fn slot_suggested_link(root: &Path, p: &LockedPackage) -> Option<LinkType> {
    let slot = root
        .join("vibedeps")
        .join(format!("{}-{}", p.kind.as_str(), p.name))
        .join(p.version.to_string())
        .join(Manifest::FILENAME);
    let manifest = Manifest::read(&slot).ok()?;
    manifest.boot_snippet.and_then(|b| b.link)
}

/// Parse an `INDEX.md` `when` string into the model condition.
fn condition_from_when(when: Option<&str>) -> Condition {
    match when {
        None => Condition::absent(),
        Some(raw) => {
            let (kind, value) = match raw.strip_prefix("os:") {
                Some(os) => (Some(ConditionKind::Os), Some(os.to_string())),
                None => (None, None),
            };
            Condition {
                present: true,
                raw: Some(raw.to_string()),
                kind,
                value,
            }
        }
    }
}

/// Collect in-place `@spec` / `#use` / `#embed` / `#source` markers from every
/// boot source file that loads, via the canonical fence-aware parser
/// (PROP-036 §2.9). `boot_files` are project-relative paths (the host-authored
/// boot files plus every `STATIC.md` / `INDEX.md` source path); each that
/// exists is read and parsed. A bare `spec://` is discretionary and not
/// collected. An empty result is correct when no boot file carries a marker.
fn collect_in_place(root: &Path, boot_files: &[String]) -> Vec<InPlaceSpec> {
    let mut out = Vec::new();
    for file in boot_files {
        let Ok(text) = fs::read_to_string(root.join(file)) else {
            continue;
        };
        let directives = Directives::parse(&text);
        for u in directives.in_place_uses {
            out.push(InPlaceSpec {
                carrier: Carrier::AtSpec,
                address: u.address.without_pin(),
                file: file.clone(),
                line: (u.line + 1) as u64,
                resolved: false,
                target_package: None,
            });
        }
        for d in directives.directives {
            out.push(InPlaceSpec {
                carrier: carrier_of(d.kind),
                address: d.address.without_pin(),
                file: file.clone(),
                line: (d.line + 1) as u64,
                resolved: false,
                target_package: None,
            });
        }
    }
    out
}

/// Map a directive kind to the model carrier.
fn carrier_of(kind: vibe_spec::DirectiveKind) -> Carrier {
    match kind {
        vibe_spec::DirectiveKind::Embed => Carrier::Embed,
        vibe_spec::DirectiveKind::Use => Carrier::Use,
        vibe_spec::DirectiveKind::Source => Carrier::Source,
    }
}

/// The lockfile provenance for a package.
fn to_source(p: &LockedPackage) -> Source {
    use vibe_core::manifest::SourceKind as LockKind;
    Source {
        kind: p.source_kind.map(|k| match k {
            LockKind::Registry => SourceKind::Registry,
            LockKind::Git => SourceKind::Git,
            LockKind::Override => SourceKind::Override,
            LockKind::Path => SourceKind::Path,
            LockKind::Embedded => SourceKind::Embedded,
            LockKind::Local => SourceKind::Local,
        }),
        url: Some(p.source_url.as_str().to_string()),
        git_ref: p.source_ref.clone(),
        commit: p.resolved_commit.clone(),
    }
}

/// Whether a declared / suggested link puts a package in the static lane —
/// `static` or `static-hard` (both compile in; they differ only in hoisting,
/// PROP-038 §2.3). `static-transitive` is handled separately (closure).
fn is_static_link(link: Option<LinkType>) -> bool {
    matches!(link, Some(LinkType::Static | LinkType::StaticHard))
}

/// Map a declared [`LinkType`] to the model wire enum.
fn to_declared_link(link: Option<LinkType>) -> Option<DeclaredLink> {
    link.map(|l| match l {
        LinkType::Static => DeclaredLink::Static,
        LinkType::Dynamic => DeclaredLink::Dynamic,
        LinkType::StaticTransitive => DeclaredLink::StaticTransitive,
        LinkType::StaticHard => DeclaredLink::StaticHard,
    })
}

/// The `group/name` identity key of a locked package.
fn qualified(p: &LockedPackage) -> String {
    format!("{}/{}", p.group, p.name)
}

/// Read a file to a string, or `None` if it does not exist / cannot be read.
fn read_opt(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// Load the lockfile, or an empty one when the project has none yet.
fn load_lockfile(root: &Path) -> Result<Lockfile> {
    let path = root.join(Lockfile::FILENAME);
    if !path.exists() {
        return Ok(Lockfile::empty("vibe (no-lockfile)", "0"));
    }
    Lockfile::read(&path).with_context(|| format!("reading {}", path.display()))
}

#[cfg(test)]
#[path = "build/tests.rs"]
mod tests;
