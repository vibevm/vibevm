//! The mirror's decision per version: copy, leave, update, or alarm.
//!
//! Identity is the content hash. A version the target lacks is copied; one
//! it holds with the hash the source records is unchanged and never
//! fetched. A different hash is an alarm when the version is frozen (at the
//! source, in the target, or by its fetched manifest) — frozen bytes never
//! change — and otherwise a moved snapshot, updated only on request.
//! Fetched bytes are verified against the hash the source's index records.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::path::Path;

use anyhow::Result;
use vibe_index::index::Index;
use vibe_registry::RecipeId;
use vibe_wire::generated::registry_mirror_report::{MirrorAction, MirroredVersion};

use super::Fetched;
use super::source::SourceVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Decision {
    Copy,
    Update,
    Unchanged,
    SnapshotDiffers(String),
    Alarm(String),
    /// The source could not serve the version.
    Unavailable(String),
}

pub(super) struct Planned {
    pub version: SourceVersion,
    pub decision: Decision,
    pub frozen: bool,
    /// The version's fetched tree (copies and updates; when not a dry run).
    pub fetched: Option<Fetched>,
    /// The hash reported: the source's, or the fetched tree's.
    pub content_hash: Option<String>,
}

/// Decide every version against the target's projection. `fetch_copies`
/// is false on a dry run: then only what is needed to decide is fetched.
pub(super) fn plan_versions(
    versions: &[SourceVersion],
    target: &Index,
    update_snapshots: bool,
    fetch_copies: bool,
    mut fetch: impl FnMut(&SourceVersion) -> Result<Fetched>,
) -> Result<Vec<Planned>> {
    let mut sorted: Vec<&SourceVersion> = versions.iter().collect();
    sorted.sort_by(|a, b| {
        (a.group.as_str(), a.name.as_str(), &a.version).cmp(&(
            b.group.as_str(),
            b.name.as_str(),
            &b.version,
        ))
    });
    let mut out = Vec::with_capacity(sorted.len());
    for version in sorted {
        match plan_one(version, target, update_snapshots, fetch_copies, &mut fetch) {
            Ok(planned) => out.push(planned),
            Err(error) if is_fatal(&error) => return Err(error),
            Err(error) => out.push(Planned {
                version: version.clone(),
                decision: Decision::Unavailable(format!("{error:#}")),
                frozen: version.frozen,
                fetched: None,
                content_hash: version.content_hash.clone(),
            }),
        }
    }
    Ok(out)
}

fn plan_one(
    version: &SourceVersion,
    target: &Index,
    update_snapshots: bool,
    fetch_copies: bool,
    fetch: &mut impl FnMut(&SourceVersion) -> Result<Fetched>,
) -> Result<Planned> {
    let existing = target
        .get(&version.group, &version.name)
        .and_then(|pkg| pkg.versions.iter().find(|e| e.version == version.version));
    let mut fetched = None;
    let mut content_hash = version.content_hash.clone();
    let decision = match existing {
        None => Decision::Copy,
        Some(entry) => {
            let same = match version.content_hash.as_deref() {
                Some(hash) if same_recipe(hash, &entry.content_hash) => hash == entry.content_hash,
                _ => {
                    let got = fetch(version)?;
                    let hash = tree_hash(&got.tree, &entry.content_hash)?;
                    content_hash.get_or_insert_with(|| hash.clone());
                    let same = hash == entry.content_hash;
                    fetched = Some(got);
                    same
                }
            };
            let frozen = version.frozen
                || entry.frozen
                || fetched.as_ref().is_some_and(|f: &Fetched| f.frozen);
            if same {
                Decision::Unchanged
            } else if frozen {
                Decision::Alarm(format!(
                    "frozen, and its content differs: the source holds {}, the target {}",
                    content_hash.as_deref().unwrap_or("other bytes"),
                    entry.content_hash
                ))
            } else if update_snapshots {
                Decision::Update
            } else {
                Decision::SnapshotDiffers(format!(
                    "the source holds {}, the target {}",
                    content_hash.as_deref().unwrap_or("other bytes"),
                    entry.content_hash
                ))
            }
        }
    };
    let decision = if matches!(decision, Decision::Copy | Decision::Update) && fetch_copies {
        let got = match fetched.take() {
            Some(got) => got,
            None => fetch(version)?,
        };
        let verdict = verify(version, &got)?;
        fetched = Some(got);
        verdict.unwrap_or(decision)
    } else {
        decision
    };
    let frozen = version.frozen
        || existing.is_some_and(|e| e.frozen)
        || fetched.as_ref().is_some_and(|f| f.frozen);
    Ok(Planned {
        version: version.clone(),
        decision,
        frozen,
        fetched,
        content_hash,
    })
}

/// A refusal by source policy (a stop included) ends the run; anything
/// else is this version's problem.
fn is_fatal(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<vibe_registry::ContactRefusal>()
        .is_some()
}

/// `Some(Alarm)` when the fetched bytes disagree with the hash the source
/// records for them.
fn verify(version: &SourceVersion, fetched: &Fetched) -> Result<Option<Decision>> {
    let Some(expected) = version.content_hash.as_deref() else {
        return Ok(None);
    };
    let got = tree_hash(&fetched.tree, expected)?;
    if got == expected {
        return Ok(None);
    }
    Ok(Some(Decision::Alarm(format!(
        "the bytes read at `{}` hash to {got}, but the source's index records {expected}",
        fetched.provenance.source_ref
    ))))
}

fn recipe_of(hash: &str) -> RecipeId {
    if hash.starts_with(RecipeId::Tree1.label()) {
        RecipeId::Tree1
    } else {
        RecipeId::Legacy0
    }
}

fn same_recipe(a: &str, b: &str) -> bool {
    recipe_of(a) == recipe_of(b)
}

/// The hash of `tree` under the recipe `like` was computed with.
fn tree_hash(tree: &Path, like: &str) -> Result<String> {
    Ok(vibe_registry::compute_content_hash_with(
        recipe_of(like),
        tree,
    )?)
}

pub(super) fn report_row(planned: &Planned) -> MirroredVersion {
    let (action, note) = match &planned.decision {
        Decision::Copy => (MirrorAction::Copied, None),
        Decision::Update => (MirrorAction::Updated, None),
        Decision::Unchanged => (MirrorAction::Unchanged, None),
        Decision::SnapshotDiffers(note) => (MirrorAction::SnapshotDiffers, Some(note.clone())),
        Decision::Alarm(note) => (MirrorAction::Alarm, Some(note.clone())),
        Decision::Unavailable(note) => (MirrorAction::Unavailable, Some(note.clone())),
    };
    MirroredVersion {
        action,
        frozen: planned.frozen,
        package: format!("{}/{}", planned.version.group, planned.version.name),
        version: planned.version.version.to_string(),
        yanked: planned.version.yanked,
        content_hash: planned.content_hash.clone(),
        note,
        resolved_commit: planned
            .fetched
            .as_ref()
            .and_then(|f| f.provenance.resolved_commit.clone()),
        source_ref: planned
            .fetched
            .as_ref()
            .map(|f| f.provenance.source_ref.clone()),
        tree: planned
            .fetched
            .as_ref()
            .and_then(|f| f.git.as_ref())
            .map(|g| g.facts.tree.clone()),
        tree_verified: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_index::types::{NamingConvention, PackageKind, VersionEntry};

    fn group() -> vibe_core::Group {
        vibe_core::Group::parse("org.corp").unwrap()
    }

    fn tree(payload: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("payload"), payload).unwrap();
        dir
    }

    fn hash(dir: &tempfile::TempDir) -> String {
        vibe_registry::compute_content_hash_with(RecipeId::Tree1, dir.path()).unwrap()
    }

    fn target(entries: &[(&str, &str, &str, bool)]) -> Index {
        let at = "2026-11-08T00:00:00Z".parse().unwrap();
        let mut index = Index::new("corp", "file:///corp", NamingConvention::Fqdn, at);
        for (name, version, hash, frozen) in entries {
            let mut entry = VersionEntry::minimal(
                PackageKind::Flow,
                "org.corp".parse().unwrap(),
                *name,
                version.parse().unwrap(),
                at,
            );
            entry.content_hash = hash.to_string();
            entry.frozen = *frozen;
            index.upsert(entry);
        }
        index
    }

    fn fetched(dir: &tempfile::TempDir) -> Fetched {
        Fetched {
            tree: dir.path().to_path_buf(),
            provenance: vibe_publish::layout_publish::Provenance {
                source_url: "https://upstream.example/demo.git".into(),
                source_ref: "v1.0.0".into(),
                resolved_commit: None,
            },
            frozen: false,
            git: None,
        }
    }

    #[test]
    fn decisions_follow_hash_frozen_and_snapshot_rules() {
        let a = tree("a");
        let b = tree("b");
        let (ha, hb) = (hash(&a), hash(&b));
        let versions = [
            SourceVersion::listed(group(), "new", "1.0.0".parse().unwrap(), Some(&ha), false),
            SourceVersion::listed(group(), "same", "1.0.0".parse().unwrap(), Some(&ha), false),
            SourceVersion::listed(group(), "moved", "1.0.0".parse().unwrap(), Some(&hb), false),
            SourceVersion::listed(group(), "frozen", "1.0.0".parse().unwrap(), Some(&hb), true),
            SourceVersion::listed(group(), "liar", "1.0.0".parse().unwrap(), Some(&hb), false),
            SourceVersion::listed(group(), "gone", "1.0.0".parse().unwrap(), Some(&ha), false),
        ];
        let index = target(&[
            ("same", "1.0.0", &ha, false),
            ("moved", "1.0.0", &ha, false),
            ("frozen", "1.0.0", &ha, false),
        ]);
        let mut fetches = Vec::new();
        let planned = plan_versions(&versions, &index, false, true, |v| {
            fetches.push(v.name.clone());
            match v.name.as_str() {
                "gone" => anyhow::bail!("repository not found"),
                // The source's index says b, the bytes hash to a.
                "liar" => Ok(fetched(&a)),
                _ => Ok(fetched(&a)),
            }
        })
        .unwrap();
        let by_name = |name: &str| {
            planned
                .iter()
                .find(|p| p.version.name == name)
                .map(|p| p.decision.clone())
                .unwrap()
        };
        assert_eq!(by_name("new"), Decision::Copy);
        assert_eq!(by_name("same"), Decision::Unchanged);
        assert!(matches!(by_name("moved"), Decision::SnapshotDiffers(_)));
        assert!(matches!(by_name("frozen"), Decision::Alarm(_)));
        assert!(matches!(by_name("liar"), Decision::Alarm(_)));
        assert!(matches!(by_name("gone"), Decision::Unavailable(_)));
        // A version whose hash the source records is decided without a fetch.
        assert!(!fetches.contains(&"same".to_string()));
        assert!(!fetches.contains(&"moved".to_string()));

        let updated =
            plan_versions(&versions[2..3], &index, true, true, |_| Ok(fetched(&b))).unwrap();
        assert_eq!(updated[0].decision, Decision::Update);

        // A dry run fetches nothing it does not need to decide.
        let dry = plan_versions(&versions[..1], &index, false, false, |_| {
            anyhow::bail!("a dry run must not fetch a new version")
        })
        .unwrap();
        assert_eq!(dry[0].decision, Decision::Copy);
    }
}
