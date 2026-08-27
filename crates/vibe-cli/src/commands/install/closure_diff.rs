//! PROP-050 ##VERIFY-LOCK-DIFF — the closure-diff cell. `vibe install` /
//! `vibe update` print the diff of the dependency closure across their
//! apply: members entering/leaving the lock, version moves, and the boot
//! lanes' byte cost before/after — so a mid-graph re-export widening
//! (##CLOSURE-DRIFT-CONTROL) is a reviewed event, not a silent seep.
//!
//! Pure calculator + renderer over two [`Lockfile`] snapshots and two
//! lane-size samples; nothing here mutates state. The CLI's install and
//! update paths each snapshot their pre-apply lock (and lane bytes) before
//! applying and call [`emit_closure_diff`] after a successful apply.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#verification");

use std::collections::BTreeMap;
use std::path::Path;

use vibe_core::manifest::{LockedPackage, Lockfile};

use crate::output;

/// The boot lanes whose byte cost the closure diff watches — the
/// lane-cost delta half of PROP-050 ##VERIFY-LOCK-DIFF. A lane that does
/// not exist at sample time is `None`, so an `.md` → `.xml` format flip
/// renders as both lanes moving.
///
/// The lane names route through the layout module (PROP-052 L2), so the
/// R4 flip moves every watched lane in one edit. Order is fixed:
/// manifest, Markdown static, XML static.
pub fn watched_lanes() -> [String; 3] {
    let lane = |path: std::path::PathBuf| vibe_core::machine_json_path(&path);
    [
        lane(vibe_core::layout::current_boot_index()),
        lane(vibe_core::layout::current_boot_static_md()),
        lane(vibe_core::layout::current_boot_static_xml()),
    ]
}

/// The closure diff between two lockfile snapshots plus two lane-size
/// samples. Members are compared by `(group, name)` identity; a version
/// move is a `changed` row, arrival is `added`, departure is `removed`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ClosureDiff {
    /// Entering members — `group/name@version (admitted_by)`, the
    /// visibility rule that admitted them in parentheses when the lock
    /// records one (lock v6+; absent on older locks).
    pub added: Vec<String>,
    /// Leaving members — `group/name@version`.
    pub removed: Vec<String>,
    /// Members whose version moved — `group/name old -> new`.
    pub changed: Vec<String>,
    /// Boot lanes whose size moved — `(lane rel path, before, after)`,
    /// `None` meaning the file was absent at that sample. Only lanes
    /// that actually changed are kept.
    pub lane_bytes: Vec<(String, Option<u64>, Option<u64>)>,
    /// The closure size after apply — the quiet line's `N packages`.
    pub packages_after: usize,
}

impl ClosureDiff {
    /// Whether anything moved. An empty diff renders as ONE quiet line —
    /// ##VERIFY-LOCK-DIFF is a review event, not per-install noise.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
            && self.lane_bytes.is_empty()
    }
}

/// Compute the closure diff between the pre-apply lock and the freshly
/// written one, with the lane-size samples taken before and after the
/// apply. Deterministic: member rows sort by their rendered text, lane
/// rows keep [`watched_lanes`] order.
pub fn closure_diff(
    old: &Lockfile,
    new: &Lockfile,
    lanes_before: &[(String, Option<u64>)],
    lanes_after: &[(String, Option<u64>)],
) -> ClosureDiff {
    let before: BTreeMap<(String, String), &LockedPackage> =
        old.packages.iter().map(|p| (key_of(p), p)).collect();
    let after: BTreeMap<(String, String), &LockedPackage> =
        new.packages.iter().map(|p| (key_of(p), p)).collect();

    let mut added: Vec<String> = Vec::new();
    let mut changed: Vec<String> = Vec::new();
    for p in &new.packages {
        match before.get(&key_of(p)) {
            None => added.push(format!(
                "{}/{}@{}{}",
                p.group,
                p.name,
                p.version,
                admitted_suffix(p.admitted_by.as_deref())
            )),
            Some(old_p) if old_p.version != p.version => changed.push(format!(
                "{}/{} {} -> {}",
                p.group, p.name, old_p.version, p.version
            )),
            Some(_) => {}
        }
    }
    let mut removed: Vec<String> = old
        .packages
        .iter()
        .filter(|p| !after.contains_key(&key_of(p)))
        .map(|p| format!("{}/{}@{}", p.group, p.name, p.version))
        .collect();
    added.sort();
    changed.sort();
    removed.sort();

    // Lane samples come from lane_sizes() over the same watched_lanes()
    // order, so a plain zip pairs before with after; only a size move
    // (including absent <-> present) keeps its row.
    let lane_bytes: Vec<(String, Option<u64>, Option<u64>)> = lanes_before
        .iter()
        .zip(lanes_after.iter())
        .filter(|((lane_before, size_before), (lane_after, size_after))| {
            lane_before == lane_after && size_before != size_after
        })
        .map(|((lane, size_before), (_, size_after))| (lane.clone(), *size_before, *size_after))
        .collect();

    ClosureDiff {
        added,
        removed,
        changed,
        lane_bytes,
        packages_after: new.packages.len(),
    }
}

/// Render the diff as human lines. An empty diff is the single quiet line
/// `closure unchanged (N packages)`; a real change renders one line per
/// move — `+ …` / `- …` / `~ …` members first, then `lane …: A -> B B`
/// rows for the boot lanes whose byte cost moved.
pub fn render(diff: &ClosureDiff) -> Vec<String> {
    if diff.is_empty() {
        return vec![format!(
            "closure unchanged ({} packages)",
            diff.packages_after
        )];
    }
    let mut lines: Vec<String> = Vec::new();
    lines.extend(diff.added.iter().map(|entry| format!("+ {entry}")));
    lines.extend(diff.removed.iter().map(|entry| format!("- {entry}")));
    lines.extend(diff.changed.iter().map(|entry| format!("~ {entry}")));
    lines.extend(diff.lane_bytes.iter().map(|(lane, before, after)| {
        format!(
            "lane {lane}: {} -> {} B",
            size_text(*before),
            size_text(*after)
        )
    }));
    lines
}

/// Sample the byte sizes of [`watched_lanes`] under `root`; a missing file
/// is `None`. Called before and after an apply to feed the lane-cost half
/// of the closure diff.
pub fn lane_sizes(root: &Path) -> Vec<(String, Option<u64>)> {
    watched_lanes()
        .iter()
        .map(|lane| {
            let size = std::fs::metadata(root.join(lane))
                .ok()
                .map(|metadata| metadata.len());
            (lane.clone(), size)
        })
        .collect()
}

/// Emit the closure diff on `ctx` after a successful apply — the shared
/// tail of `vibe install` and `vibe update` (PROP-050 ##VERIFY-LOCK-DIFF).
///
/// Human mode: a real diff renders under a `closure diff:` heading, one
/// `→` step per line, and `--quiet` stays silent (the diff is a review
/// surface, not a completion message). An empty diff is the single
/// `closure unchanged` line, so a routine re-install adds no noise. JSON
/// mode emits one `"<command>:closure-diff"` document ahead of the final
/// report, shaped like the neighbouring events (`install:plan` et al).
pub(crate) fn emit_closure_diff(
    ctx: &output::Context,
    command: &str,
    old: &Lockfile,
    new: &Lockfile,
    lanes_before: &[(String, Option<u64>)],
    lanes_after: &[(String, Option<u64>)],
) {
    let diff = closure_diff(old, new, lanes_before, lanes_after);
    if ctx.is_json() {
        let payload = if diff.is_empty() {
            serde_json::json!({
                "command": format!("{command}:closure-diff"),
                "unchanged": true,
                "packages": diff.packages_after,
            })
        } else {
            let lanes: Vec<serde_json::Value> = diff
                .lane_bytes
                .iter()
                .map(|(lane, before, after)| {
                    serde_json::json!({ "lane": lane, "before": before, "after": after })
                })
                .collect();
            serde_json::json!({
                "command": format!("{command}:closure-diff"),
                "added": diff.added,
                "removed": diff.removed,
                "changed": diff.changed,
                "lanes": lanes,
            })
        };
        // Supplementary, like the plan preview: held back until the
        // outcome is known so a parked run emits its handoff alone.
        let _ = ctx.defer_json_plan(&payload);
        return;
    }
    if ctx.is_quiet() {
        return;
    }
    let lines = render(&diff);
    if !diff.is_empty() {
        ctx.heading("closure diff:");
    }
    for line in &lines {
        ctx.step(line);
    }
}

/// The `(group, name)` identity key of a locked member — the comparison
/// key of the diff (identity, not version, decides added/removed).
fn key_of(p: &LockedPackage) -> (String, String) {
    (p.group.to_string(), p.name.to_string())
}

/// The ` (admitted_by)` provenance suffix for an added member — empty
/// when the lock records no rule (a pre-v6 lock or a hand-built fixture).
fn admitted_suffix(admitted_by: Option<&str>) -> String {
    admitted_by
        .map(|rule| format!(" ({rule})"))
        .unwrap_or_default()
}

/// `None` renders as `absent` — a lane that did not exist at that sample.
fn size_text(size: Option<u64>) -> String {
    match size {
        Some(bytes) => bytes.to_string(),
        None => "absent".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A locked member fixture. The surrounding lockfile carries
    /// CURRENT_SCHEMA_VERSION via Lockfile::empty — the lock_header()
    /// discipline of the visibility fixtures, minus the TOML round-trip
    /// these unit tests do not need.
    fn locked(group: &str, name: &str, version: &str, admitted_by: Option<&str>) -> LockedPackage {
        LockedPackage {
            kind: vibe_core::PackageKind::Flow,
            name: vibe_core::PackageName::from_validated(name.to_string()),
            group: vibe_core::Group::parse(group).expect("valid group"),
            version: version.parse().expect("valid version"),
            registry: None,
            source_url: vibe_core::SourceUrl::new("file:///fixture".to_string()),
            source_ref: None,
            resolved_commit: None,
            content_hash: vibe_core::ContentHash::from_validated("sha256:00".to_string()),
            boot_snippet: None,
            files_written: Vec::new(),
            dependencies: Vec::new(),
            admitted_by: admitted_by.map(str::to_string),
            via_override: None,
            overridden: false,
            source_kind: None,
            via_redirect: None,
            features: Vec::new(),
            subskills_active: Vec::new(),
            describes: None,
            language: None,
            materialization: Default::default(),
        }
    }

    fn lockfile(packages: Vec<LockedPackage>) -> Lockfile {
        let mut lock = Lockfile::empty("vibe-test", "2026-08-23T00:00:00Z");
        assert_eq!(
            lock.meta.schema_version,
            vibe_core::manifest::CURRENT_SCHEMA_VERSION
        );
        lock.packages = packages;
        lock
    }

    fn lanes(pairs: &[(&str, Option<u64>)]) -> Vec<(String, Option<u64>)> {
        pairs
            .iter()
            .map(|(lane, size)| ((*lane).to_string(), *size))
            .collect()
    }

    /// The watched lanes as owned `(lane, size)` samples — the test-side
    /// spelling of [`watched_lanes`], so every fixture below rides the R4
    /// layout flip without an edit.
    fn lane_samples(sizes: [Option<u64>; 3]) -> Vec<(String, Option<u64>)> {
        watched_lanes().into_iter().zip(sizes).collect()
    }

    #[test]
    fn added_member_carries_its_admitting_rule() {
        let old = lockfile(vec![locked("org.x", "wal", "1.0.0", None)]);
        let new = lockfile(vec![
            locked("org.x", "wal", "1.0.0", None),
            locked("org.x", "wal", "2.0.0", Some("friends-chain")),
        ]);
        // Same (group, name), different version — a changed row, not added.
        let diff = closure_diff(&old, &new, &lanes(&[]), &lanes(&[]));
        assert_eq!(diff.added, Vec::<String>::new());
        assert_eq!(diff.changed, ["org.x/wal 1.0.0 -> 2.0.0"]);

        // A genuinely new member names the rule that admitted it; a member
        // whose lock records no rule arrives without the parentheses.
        let new = lockfile(vec![
            locked("org.x", "wal", "1.0.0", None),
            locked("org.x", "api", "1.0.0", None),
            locked("org.x", "red", "1.0.0", Some("friends-chain")),
        ]);
        let diff = closure_diff(&old, &new, &lanes(&[]), &lanes(&[]));
        assert_eq!(
            diff.added,
            ["org.x/api@1.0.0", "org.x/red@1.0.0 (friends-chain)"]
        );
    }

    #[test]
    fn removed_member_renders_a_minus_row() {
        let old = lockfile(vec![locked("org.x", "wal", "1.0.0", None)]);
        let new = lockfile(Vec::new());
        let diff = closure_diff(&old, &new, &lanes(&[]), &lanes(&[]));
        assert_eq!(diff.removed, ["org.x/wal@1.0.0"]);
        assert!(render(&diff).contains(&"- org.x/wal@1.0.0".to_string()));
    }

    #[test]
    fn version_change_is_a_tilde_row() {
        let old = lockfile(vec![locked("org.x", "wal", "1.0.0", None)]);
        let new = lockfile(vec![locked("org.x", "wal", "1.1.0", None)]);
        let diff = closure_diff(&old, &new, &lanes(&[]), &lanes(&[]));
        let lines = render(&diff);
        assert!(
            lines.contains(&"~ org.x/wal 1.0.0 -> 1.1.0".to_string()),
            "{lines:?}"
        );
    }

    #[test]
    fn empty_diff_is_one_quiet_line() {
        let old = lockfile(vec![locked("org.x", "wal", "1.0.0", None)]);
        let same_lanes = lane_samples([Some(120), None, None]);
        let diff = closure_diff(&old, &old, &same_lanes, &same_lanes);
        assert!(diff.is_empty());
        assert_eq!(
            render(&diff),
            ["closure unchanged (1 packages)".to_string()]
        );
    }

    #[test]
    fn lane_rows_render_only_on_change() {
        let lock = lockfile(vec![locked("org.x", "wal", "1.0.0", None)]);
        let before = lane_samples([Some(120), None, Some(248429)]);
        let after = lane_samples([Some(120), Some(412), Some(249001)]);
        let diff = closure_diff(&lock, &lock, &before, &after);
        let lines = render(&diff);
        let lanes = watched_lanes();
        // The unchanged manifest row stays silent; moves render with `B`,
        // and an absent lane reads `absent`.
        assert_eq!(
            lines,
            [
                format!("lane {}: absent -> 412 B", lanes[1]),
                format!("lane {}: 248429 -> 249001 B", lanes[2]),
            ],
            "{lines:?}"
        );
        // Both samples absent — the lane never existed, nothing to report.
        let before = vec![(lanes[1].clone(), None)];
        let after = vec![(lanes[1].clone(), None)];
        assert!(
            closure_diff(&lock, &lock, &before, &after)
                .lane_bytes
                .is_empty()
        );
    }

    #[test]
    fn lane_sizes_samples_watched_lanes_from_disk() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // An empty project: every watched lane is absent.
        assert!(
            lane_sizes(tmp.path())
                .iter()
                .all(|(_, size)| size.is_none())
        );

        let index_lane = vibe_core::layout::current_boot_index();
        if let Some(parent) = index_lane.parent() {
            std::fs::create_dir_all(tmp.path().join(parent)).expect("mkdir");
        }
        std::fs::write(tmp.path().join(&index_lane), "x".repeat(7)).expect("write");
        let sizes = lane_sizes(tmp.path());
        assert_eq!(sizes.len(), watched_lanes().len());
        let index = sizes
            .iter()
            .find(|(lane, _)| *lane == watched_lanes()[0])
            .expect("the boot manifest is watched");
        assert_eq!(index.1, Some(7));
    }
}
