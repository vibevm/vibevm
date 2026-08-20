//! `vibe cache check` (+ `--repair`) — the integrity sweep over the
//! machine store (PROP-010 §2.8 CMD-CHECK / CMD-CHECK-REPAIR).
//!
//! **This is the one place the store is fully re-hashed**
//! (`VERIFICATION-IS-A-COMMAND-NOT-A-TAX`, owner 2026-08-20):
//! re-hashing on every resolve would make a ten-gigabyte dependency
//! unusable, so the ordinary path never pays it — the sweep is a
//! command the operator runs. For every entry it recomputes the
//! content hash and compares it against the recorded
//! `v<version>.sha256` sidecar ([`vibe_registry::recorded_hash`]); a
//! mismatch is NAMED — identity, path, both hashes — never swallowed
//! (`A-MISMATCH-IS-NAMED-NEVER-SWALLOWED`).
//!
//! `--repair` climbs the ladder cheapest-first
//! (`REPAIR-CLIMBS-A-LADDER-CHEAPEST-FIRST` — the order is the whole
//! point): establish what the entry IS, try the local restore, and
//! only then re-fetch. With the sidecar carrying one hash line and no
//! commit, a git working copy has nothing recorded to reset to — the
//! ladder says so honestly (`unrepairable locally`) instead of
//! inventing a commit record, and steps to the re-fetch rung. The
//! re-fetch resolves the SAME exact version (`REPAIR-DOES-NOT-PULL`):
//! repair means «be what was recorded», and advancing is `vibe
//! update`'s job. An unrecorded entry gets a sidecar recorded from its
//! current bytes — the only honest action when no record exists.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::{Group, PackageRef};
use vibe_install::InstallSource;

use super::add::cache_resolver;
use crate::cli::CacheCheckArgs;
use crate::commands::install::InstallResolver;
use crate::output;

/// What the sweep concluded about one entry — and, under `--repair`,
/// what the ladder then did with it.
struct Verdict {
    group: Group,
    name: String,
    version: semver::Version,
    path: PathBuf,
    kind: Kind,
}

enum Kind {
    /// Recorded hash matches the recomputed one. Nothing to do,
    /// nothing to print beyond the count.
    Ok,
    /// The entry hashes to something other than what its sidecar
    /// recorded. `is_git_copy` carries the ladder's step-(a) finding.
    Mismatch {
        recorded: String,
        computed: String,
        is_git_copy: bool,
    },
    /// No sidecar — inserted before hash recording, or an interrupted
    /// record write. Not an error; under `--repair` the current bytes
    /// become the record (`RecordedNow`).
    Unrecorded,
    /// Repair outcome: the sidecar was written from the entry's
    /// current bytes.
    RecordedNow,
    /// Repair outcome: the bad entry died with its sidecar and the
    /// exact same version was fetched fresh. `upstream_changed` names
    /// the case where the re-fetched bytes differ from what the old
    /// record pinned — the registry itself moved at the same version.
    Refetched {
        was_git_copy: bool,
        upstream_changed: bool,
    },
    /// Repair outcome: the ladder's last rung failed (no registry
    /// serves the identity, network, …) — named, never swallowed.
    Failed { error: String },
}

impl Verdict {
    fn identity(&self) -> String {
        let name = &self.name;
        let version = &self.version;
        format!("{}/{name}@{version}", self.group.as_str())
    }

    /// Whether this entry is ok for the run's exit status — either it
    /// always was, or repair landed it back on its feet.
    fn is_ok(&self) -> bool {
        !matches!(
            self.kind,
            Kind::Mismatch { .. } | Kind::Unrecorded | Kind::Failed { .. }
        )
    }
}

/// The one comparison the sweep turns on: the recorded sidecar line
/// against the freshly computed content hash. A malformed record
/// simply never matches and is named as a mismatch.
fn recorded_matches(recorded: &str, computed: &str) -> bool {
    recorded == computed
}

pub(crate) fn run(ctx: &output::Context, args: CacheCheckArgs, root_offline: bool) -> Result<()> {
    let root = vibe_registry::store_root().context("resolving the machine store root")?;
    let mut verdicts = sweep(&root)?;

    if args.repair {
        repair(ctx, &root, &mut verdicts, &args.path, root_offline)?;
    }
    emit(ctx, &root, args.repair, &verdicts)?;

    let bad = verdicts.iter().filter(|v| !v.is_ok()).count();
    if bad > 0 {
        bail!(
            "{bad} of {} store entr{} failed the integrity check — see the report above",
            verdicts.len(),
            if bad == 1 { "y" } else { "ies" }
        );
    }
    Ok(())
}

/// The full walk: every entry, recomputed and compared against its
/// record. Entry enumeration is the store's own walk-as-index
/// (`list_all`); hashing is [`vibe_registry::compute_content_hash`]
/// over the entry (the recipe excludes `.git`, so a git working copy
/// hashes its tree).
fn sweep(root: &Path) -> Result<Vec<Verdict>> {
    let mut verdicts = Vec::new();
    for (group, name, version) in vibe_registry::list_all() {
        let path = vibe_registry::entry_dir(root, &group, &name, &version);
        let computed = vibe_registry::compute_content_hash(&path)
            .with_context(|| format!("re-hashing `{}`", path.display()))?;
        let recorded = vibe_registry::recorded_hash(&group, &name, &version)
            .with_context(|| format!("reading the recorded hash for {group}/{name}@{version}"))?;
        let kind = match &recorded {
            Some(recorded) if recorded_matches(recorded, &computed) => Kind::Ok,
            Some(recorded) => Kind::Mismatch {
                recorded: recorded.clone(),
                is_git_copy: path.join(".git").exists(),
                computed,
            },
            None => Kind::Unrecorded,
        };
        verdicts.push(Verdict {
            group,
            name,
            version,
            path,
            kind,
        });
    }
    Ok(verdicts)
}

/// The repair ladder, cheapest first. The registry resolver for the
/// re-fetch rung is built lazily — a store whose only problems are
/// unrecorded entries never touches a registry at all.
fn repair(
    ctx: &output::Context,
    root: &Path,
    verdicts: &mut [Verdict],
    path: &Path,
    root_offline: bool,
) -> Result<()> {
    // Step (г) pass — the mismatched entries, cheapest-first honest:
    // nothing cheaper than a re-fetch exists without a recorded
    // commit (git copies) or for extracted directories, so every
    // mismatch walks to the last rung, NAMED as such for git copies.
    let mut resolver: Option<InstallResolver> = None;
    for verdict in verdicts.iter_mut() {
        let Kind::Mismatch { .. } = verdict.kind else {
            continue;
        };
        // The ladder's last rung needs a registry; build it once, and
        // a construction failure (no registry configured at all)
        // fails every remaining re-fetch with it — named, not fatal
        // to the sweep's other classes.
        if resolver.is_none() {
            match cache_resolver(path, root_offline) {
                Ok((built, _in_project)) => resolver = Some(built),
                Err(e) => {
                    verdict.kind = Kind::Failed {
                        error: format!("{e:#}"),
                    };
                    continue;
                }
            }
        }
        let Some(resolver) = resolver.as_ref() else {
            // `resolver` was just set; the None arm exists only to
            // satisfy the borrow checker's view of the Option.
            continue;
        };
        let (recorded, is_git_copy) = match &verdict.kind {
            Kind::Mismatch {
                recorded,
                is_git_copy,
                ..
            } => (recorded.clone(), *is_git_copy),
            _ => continue,
        };
        let group = verdict.group.clone();
        let name = verdict.name.clone();
        let version = verdict.version.clone();
        let identity = verdict.identity();
        if !ctx.is_quiet() && !ctx.is_json() && is_git_copy {
            // Steps (а)/(б): a git working copy could be restored
            // locally — clean + hard reset — but only against a
            // RECORDED commit, and the sidecar records one hash line,
            // no commit. Honest classification, no invented record.
            ctx.skipped(
                &identity,
                "git working copy with no recorded commit — unrepairable locally, re-fetching",
            );
        }
        // Step (г): the bad entry dies WITH its sidecar, then the
        // exact same version is fetched fresh (write-once inserts a
        // new record with it).
        vibe_registry::remove_entry(&group, &name, &version)
            .with_context(|| format!("removing the damaged entry {identity}"))?;
        let spec = format!("{}/{name}@={version}", group.as_str());
        let pkgref =
            PackageRef::parse(&spec).with_context(|| format!("parsing the repair ref `{spec}`"))?;
        verdict.kind = match resolver.resolve_and_fetch(&pkgref, root, None) {
            Ok(cached) => Kind::Refetched {
                was_git_copy: is_git_copy,
                upstream_changed: cached.content_hash != recorded,
            },
            Err(e) => Kind::Failed {
                error: format!("{e}"),
            },
        };
    }
    // The unrecorded pass: record what IS — the only honest action
    // when no record exists. After the mismatch pass, so a re-fetched
    // entry's fresh sidecar (written by the insert) is never touched.
    for verdict in verdicts.iter_mut() {
        if !matches!(verdict.kind, Kind::Unrecorded) {
            continue;
        }
        let path = verdict.path.clone();
        let computed = vibe_registry::compute_content_hash(&path)
            .with_context(|| format!("re-hashing `{}`", path.display()))?;
        vibe_registry::record_hash(&verdict.group, &verdict.name, &verdict.version, &computed)
            .with_context(|| format!("recording the sidecar for {}", verdict.identity()))?;
        verdict.kind = Kind::RecordedNow;
    }
    Ok(())
}

/// The report. Human mode prints only what is not ok (a healthy
/// thousand-entry store must not scroll) plus the count line; JSON
/// carries every class as an array; quiet is one summary line.
fn emit(ctx: &output::Context, root: &Path, repair: bool, verdicts: &[Verdict]) -> Result<()> {
    let ok: Vec<String> = verdicts
        .iter()
        .filter(|v| matches!(v.kind, Kind::Ok))
        .map(Verdict::identity)
        .collect();
    let mismatched: Vec<&Verdict> = verdicts
        .iter()
        .filter(|v| matches!(v.kind, Kind::Mismatch { .. }))
        .collect();
    let unrecorded: Vec<String> = verdicts
        .iter()
        .filter(|v| matches!(v.kind, Kind::Unrecorded))
        .map(Verdict::identity)
        .collect();
    let recorded_now: Vec<String> = verdicts
        .iter()
        .filter(|v| matches!(v.kind, Kind::RecordedNow))
        .map(Verdict::identity)
        .collect();
    let refetched: Vec<&Verdict> = verdicts
        .iter()
        .filter(|v| matches!(v.kind, Kind::Refetched { .. }))
        .collect();
    let failed: Vec<&Verdict> = verdicts
        .iter()
        .filter(|v| matches!(v.kind, Kind::Failed { .. }))
        .collect();

    if ctx.is_json() {
        let mismatch_json: Vec<serde_json::Value> = mismatched
            .iter()
            .filter_map(|v| match &v.kind {
                Kind::Mismatch {
                    recorded,
                    computed,
                    is_git_copy,
                } => Some(serde_json::json!({
                    "identity": v.identity(),
                    "path": v.path.display().to_string(),
                    "recorded": recorded,
                    "computed": computed,
                    "is_git_copy": is_git_copy,
                })),
                _ => None,
            })
            .collect();
        let refetched_json: Vec<serde_json::Value> = refetched
            .iter()
            .filter_map(|v| match &v.kind {
                Kind::Refetched {
                    was_git_copy,
                    upstream_changed,
                } => Some(serde_json::json!({
                    "identity": v.identity(),
                    "upstream_changed": upstream_changed,
                    "was_git_copy": was_git_copy,
                })),
                _ => None,
            })
            .collect();
        let failed_json: Vec<serde_json::Value> = failed
            .iter()
            .filter_map(|v| match &v.kind {
                Kind::Failed { error } => Some(serde_json::json!({
                    "identity": v.identity(),
                    "error": error,
                })),
                _ => None,
            })
            .collect();
        ctx.emit_json(&serde_json::json!({
            "command": "cache:check",
            "mode": if repair { "repair" } else { "check" },
            "root": root.display().to_string(),
            "passed": mismatched.is_empty() && unrecorded.is_empty() && failed.is_empty(),
            "ok": ok,
            "mismatch": mismatch_json,
            "unrecorded": unrecorded,
            "recorded_now": recorded_now,
            "refetched": refetched_json,
            "failed": failed_json,
        }))?;
        return Ok(());
    }

    if ctx.is_quiet() {
        ctx.summary(&format!(
            "vibe cache check: {} ok, {} mismatched, {} unrecorded",
            ok.len(),
            mismatched.len(),
            unrecorded.len()
        ));
        return Ok(());
    }

    ctx.heading(&format!(
        "Integrity sweep of the machine store ({}){}",
        root.display(),
        if repair { " — repairing" } else { "" }
    ));
    for v in &mismatched {
        if let Kind::Mismatch {
            recorded, computed, ..
        } = &v.kind
        {
            // The StoreEntryMismatch grammar: the entry, its path,
            // what it hashes to, what the record pins.
            println!(
                "  {} store entry for `{}` at `{}` hashes to {computed}, but the recorded \
                 sidecar pins {recorded}",
                ctx.cross.apply_to("✗"),
                v.identity(),
                v.path.display()
            );
        }
    }
    for identity in &unrecorded {
        println!(
            "  {} {identity} — unrecorded (no sidecar: inserted before hash recording, \
             or an interrupted record)",
            ctx.warn.apply_to("•")
        );
    }
    for v in &refetched {
        let mut note = String::new();
        if let Kind::Refetched {
            was_git_copy,
            upstream_changed,
        } = &v.kind
        {
            if *was_git_copy {
                note.push_str(" (was a git working copy — unrepairable locally, re-fetched)");
            }
            if *upstream_changed {
                note.push_str(
                    " [re-fetched bytes differ from the old record — upstream moved at the \
                     same version]",
                );
            }
        }
        ctx.removed(&format!(
            "{} — re-fetched at the same version{note}",
            v.identity()
        ));
    }
    for identity in &recorded_now {
        ctx.created(&format!("{identity} — sidecar recorded from current bytes"));
    }
    for v in &failed {
        if let Kind::Failed { error } = &v.kind {
            println!(
                "  {} {} — repair failed: {error}",
                ctx.cross.apply_to("✗"),
                v.identity()
            );
        }
    }
    ctx.summary(&format!(
        "\n{} ok, {} mismatched, {} unrecorded{}.",
        ok.len(),
        mismatched.len(),
        unrecorded.len(),
        if repair {
            format!(
                "; repaired: {} re-fetched, {} recorded, {} failed",
                refetched.len(),
                recorded_now.len(),
                failed.len()
            )
        } else {
            String::new()
        }
    ));
    Ok(())
}
