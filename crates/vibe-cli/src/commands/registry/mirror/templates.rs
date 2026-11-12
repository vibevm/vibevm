//! Writing a template's external inputs into the target's `_index/`
//! (PROP-075 `#MIRROR-TEMPLATE-SNAPSHOT`, `#MIRROR-SNAPSHOT-LAYOUT`).
//!
//! The layout, the record type and the filtered envelope live in
//! `vibe_registry::templates::snapshot`, written and read from one place
//! so the mirror and the client cannot disagree about them. What lives
//! here is the part that is a *mirror run*: which versions a
//! `[[template]]` row selects, the full render of each one, the alarm when
//! an already-snapshotted version's digests moved, and the three files
//! handed to `BatchConfig.files` — so the snapshot is fixed by the same
//! git commit as the versions beside it.
//!
//! The row's **human form** is not here: it is drawn by `super::payloads`,
//! which is where a row's counts and bytes are known. What this module
//! answers with instead is [`Snapshots`] — the batch files, one
//! [`SnapshotOutcome`] per row carrying the renders a payload row needs,
//! and the snapshot versions a `--prune` would drop.
//!
//! **Nothing is written outside the batch.** Every file is staged in the
//! run's scratch directory and named by its registry-relative path; the
//! batch writes only those whose bytes differ, so a re-run that
//! snapshotted nothing new commits nothing.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-SNAPSHOT-LAYOUT");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::Group;
use vibe_core::manifest::VersionSource;
use vibe_registry::templates::snapshot::{
    self, Snapshot, SnapshotRecord, SnapshotVersion, filtered_version_list,
};
use vibe_registry::templates::{ActiveTemplate, FullRender, TemplateLayer};

use super::config::VersionList;
use super::selection::{Selection, TemplateSelection};

/// Files written under `_index/` with the batch, as `BatchConfig.files`
/// names them: the registry-relative path and the staged source.
pub(super) type BatchFiles = Vec<(String, PathBuf)>;

/// Everything one `[[template]]` row decided, carried out of this module.
///
/// Commit (b) reported a row where it was produced and carried nothing,
/// because the only reader then was the human form. Payload projection and
/// `AV-FMT-009`'s `templates[]` are both readers of the same facts — and
/// the projection needs the renders this stage already performed — so the
/// outcome is the carrier it was always going to be, and the row's human
/// form is drawn where its payload facts are known.
#[derive(Debug, Clone)]
pub(super) struct SnapshotOutcome {
    pub package: String,
    pub revision: String,
    /// Selected versions, oldest first.
    pub versions: Vec<semver::Version>,
    /// Each channel the row named, and what it resolved to — the newest
    /// member of its set, which is what a channel lowers to.
    pub channels: Vec<(String, Option<semver::Version>)>,
    /// Versions already snapshotted and left untouched by this run.
    pub unchanged: usize,
    /// `(version, option id, missing file)` per row dropped by
    /// `template_optional`.
    pub dropped: Vec<(String, String, String)>,
    /// The installer options the row asks to keep, which select nothing
    /// in the snapshot itself — the snapshot carries a version's *inputs*
    /// (its version record and its digest document), which are per
    /// version and not per option. They are what payload projection acts
    /// on.
    pub installers: Vec<String>,
    /// Versions this run added to the snapshot.
    pub added: usize,
    /// The full renders this run performed, kept **only** when the row
    /// asks for payloads: the projection reads the inputs out of them, and
    /// a row that wants no payload must not hold a manifest per version
    /// for nothing.
    pub renders: Vec<FullRender>,
    /// How much of the version document the snapshot's `versions.json`
    /// carries, so the projection can say when a client behind the mirror
    /// sees fewer versions than the record names
    /// (PROP-075 `#MIRROR-CONFIG-SCHEMA`).
    pub version_list: VersionList,
}

/// One package's snapshot versions the run's selection no longer names.
///
/// Computed by the stage that knows both sides — what the target records
/// and what this selection selects — and acted on only by `--prune`
/// (`#MIRROR-NO-SILENT-PRUNE`).
#[derive(Debug, Clone)]
pub(super) struct StaleSnapshot {
    pub package: String,
    pub group: Group,
    pub name: String,
    /// Version spellings, as the record writes them.
    pub versions: BTreeSet<String>,
}

/// What the snapshot stage produced.
#[derive(Debug, Default)]
pub(super) struct Snapshots {
    /// Files for `BatchConfig.files`.
    pub files: BatchFiles,
    pub rows: Vec<SnapshotOutcome>,
    /// The snapshot versions a `--prune` would drop, whether or not one
    /// was asked for: the plan is what `--prune` prints before it removes
    /// anything.
    pub stale: Vec<StaleSnapshot>,
}

/// Snapshot every `[[template]]` row of the selection.
///
/// A row that selects nothing is reported, not an error
/// (`#MIRROR-CONFIG-SCHEMA`). Without `prune` a snapshot only ever grows,
/// because narrowing a selection never deletes
/// (`#MIRROR-NO-SILENT-PRUNE`); with it the record keeps **only** the
/// versions this selection names, so the record, its `versions.json` and
/// the digest files it drops are decided in the one place that writes
/// them.
pub(super) fn snapshots(
    selection: &Selection,
    source_root: &Path,
    target_root: &Path,
    scratch: &Path,
    prune: bool,
) -> Result<Snapshots> {
    if selection.templates().is_empty() {
        return Ok(Snapshots::default());
    }
    let layer = vibe_registry::templates::active().ok_or_else(|| {
        anyhow::anyhow!(
            "a `[[template]]` row needs this invocation's template layer, and none is installed \
             (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-CACHE; \
             fix: run with a readable settings root — `$VIBE_SETTINGS` or the user home — so a \
             template's inputs have somewhere to be cached)"
        )
    })?;
    let mut done = Snapshots::default();
    for row in selection.templates() {
        snapshot_row(
            &layer,
            row,
            source_root,
            target_root,
            scratch,
            prune,
            &mut done,
        )?;
    }
    Ok(done)
}

fn snapshot_row(
    layer: &TemplateLayer,
    row: &TemplateSelection,
    source_root: &Path,
    target_root: &Path,
    scratch: &Path,
    prune: bool,
    done: &mut Snapshots,
) -> Result<()> {
    let package = row.package();
    let group = Group::parse(&row.group)
        .with_context(|| format!("the `[[template]]` row for `{package}`"))?;
    let found = layer
        .for_directory(source_root, &group, &row.name)?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "the `[[template]]` row names `{package}`, and the source registry publishes no \
                 template revision of it \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DISCOVERY; \
                 fix: a template package declares `[package] template = true` — check the \
                 coordinate, or publish the template into the source first)"
            )
        })?;
    let template = &found.template;
    let selected = select_versions(row, &found.domain.all(), found.domain.channels());
    // The channels are reported over the versions a client behind this
    // mirror will actually see, so the row's `[lts → 22.23.5]` and the
    // client's own recomputed `lts` cannot disagree. Channels are not
    // stored in a snapshot: a client recomputes them from the version
    // document by the same `[[template.channel]]` rules, so trimming that
    // document trims the channel table with it — and the report has to
    // say the trimmed answer, not the source's
    // (PROP-075 `#MIRROR-CONFIG-SCHEMA`, `#CHANNELS`).
    let channels = resolved_channels(
        row,
        &listed_channels(row.version_list, found.domain.channels(), &selected),
    );
    let existing = Snapshot::read(target_root, &group, &row.name)?;
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let vibe = format!("vibe {}", env!("CARGO_PKG_VERSION"));

    // Every selected version is rendered first and nothing is staged
    // until all of them agree with what the target already records: an
    // alarm must stop the run **before anything is written**, as
    // PROP-002 `#MIRROR-RULES` stops on a changed hash of a frozen
    // version.
    let mut rows = Vec::new();
    let mut unchanged = 0usize;
    let mut dropped = Vec::new();
    let mut renders = Vec::new();
    for version in &selected {
        let Some(record) = found.domain.record(version) else {
            // A version the domain lists without a record is a concrete
            // publication, which `#TEMPLATE-CONCRETE-WINS` says is not
            // rendered at all; it is copied as an ordinary version.
            continue;
        };
        let render = layer.full(template, version, record)?;
        for drop in &render.dropped {
            dropped.push((version.to_string(), drop.id.clone(), drop.missing.clone()));
        }
        if row.payloads {
            // The projection reads this version's inputs out of the
            // render; a row that asks for no payload keeps none.
            renders.push(render.clone());
        }
        if let Some(previous) = existing.as_ref().and_then(|s| s.record.version(version)) {
            if previous.digest_sha256 != render.digest_source_sha256 {
                bail!(
                    "vibe registry mirror: the digest document of `{package}@{version}` has \
                     changed since this registry snapshotted it \
                     (recorded {}, upstream now {}); nothing was written \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD; \
                     fix: a released version's checksums are immutable — investigate upstream, \
                     because either the release was replaced or the snapshot is not of what it \
                     says it is)",
                    previous.digest_sha256.as_deref().unwrap_or("(none)"),
                    render.digest_source_sha256.as_deref().unwrap_or("(none)")
                );
            }
            // A re-run adds versions and leaves existing ones untouched.
            unchanged += 1;
            continue;
        }
        rows.push(SnapshotVersion {
            version: version.to_string(),
            digest_sha256: render.digest_source_sha256.clone(),
            rendered_sha256: render.sha256.clone(),
        });
    }

    done.rows.push(SnapshotOutcome {
        package: package.clone(),
        revision: template.revision.to_string(),
        versions: selected.clone(),
        channels,
        unchanged,
        dropped,
        installers: row.installers.clone(),
        added: rows.len(),
        renders,
        version_list: row.version_list,
    });

    let keeping: BTreeSet<String> = selected.iter().map(ToString::to_string).collect();
    let stale: BTreeSet<String> = existing
        .as_ref()
        .map(|snapshot| {
            snapshot
                .record
                .versions
                .iter()
                .map(|row| row.version.clone())
                .filter(|version| !keeping.contains(version))
                .collect()
        })
        .unwrap_or_default();
    if !stale.is_empty() {
        done.stale.push(StaleSnapshot {
            package: package.clone(),
            group: group.clone(),
            name: row.name.clone(),
            versions: stale,
        });
    }
    if selected.is_empty() {
        // Reported, not an error — and with nothing selected there is no
        // snapshot to write.
        return Ok(());
    }

    let mut record = match existing {
        Some(found) => {
            let mut record = found.record;
            record.sync.vibe = vibe;
            record.sync.last = now;
            if prune {
                record.versions.retain(|row| keeping.contains(&row.version));
            }
            record
        }
        None => SnapshotRecord::new(
            package.clone(),
            template.revision.to_string(),
            template.section.version_list_url().map(str::to_owned),
            template.section.digest_url_template().map(str::to_owned),
            vibe,
            now,
        ),
    };
    // The revision is kept current: a template can be fixed without
    // touching a produced version, and the snapshot must say which
    // revision wrote it (`#TEMPLATE-LOCK-RECORD`'s "the hash is what
    // binds, not the revision").
    record.source.revision = template.revision.to_string();
    record.merge(rows);
    done.files.extend(stage(
        layer,
        template,
        &group,
        &row.name,
        &record,
        &selected,
        row.version_list,
        scratch,
    )?);
    Ok(())
}

/// Write the three files into `scratch` and name them by their
/// registry-relative paths.
fn stage(
    layer: &TemplateLayer,
    template: &ActiveTemplate,
    group: &Group,
    name: &str,
    record: &SnapshotRecord,
    selected: &[semver::Version],
    version_list: VersionList,
    scratch: &Path,
) -> Result<BatchFiles> {
    let dir = scratch.join("templates").join(group.as_str()).join(name);
    std::fs::create_dir_all(dir.join("digests"))
        .with_context(|| format!("creating `{}`", dir.display()))?;
    let write = |path: PathBuf, body: &str| -> Result<PathBuf> {
        std::fs::write(&path, body).with_context(|| format!("writing `{}`", path.display()))?;
        Ok(path)
    };
    let mut files = vec![(
        snapshot::relative_record(group, name),
        write(dir.join("snapshot.toml"), &record.render()?)?,
    )];

    // `versions.json` — the document a client's whole version domain and
    // channel table are computed from, so **this** is what decides which
    // versions a closed network can see and resolve at all
    // (PROP-075 `#MIRROR-CONFIG-SCHEMA`).
    //
    // `version_list = "all"` (the default) restricts it to **every**
    // version the snapshot now carries, not only the ones this run added:
    // a client parses one document, so a re-run that narrowed it would
    // take away versions the record still names. `"selected"` restricts it
    // to the versions **this row** selects, which is how an operator trims
    // the list a client resolves within without waiting for a `--prune` —
    // the document is rewritten in place on every run, never deleted, so
    // the knob takes effect in both directions by itself.
    let recorded: Vec<semver::Version> = record
        .versions
        .iter()
        .filter_map(|row| semver::Version::parse(&row.version).ok())
        .collect();
    let listed: Vec<semver::Version> = match version_list {
        VersionList::All => recorded.clone(),
        VersionList::Selected => selected.to_vec(),
    };
    if let (
        Some(upstream),
        VersionSource::Json {
            select,
            version_field,
            strip_prefix,
            ..
        },
    ) = (
        layer.version_document(template)?,
        &template.section.versions,
    ) {
        let filtered = filtered_version_list(
            &upstream,
            select,
            version_field,
            strip_prefix.as_deref(),
            &listed,
        )?;
        files.push((
            snapshot::relative_version_list(group, name),
            write(
                dir.join("versions.json"),
                &serde_json::to_string_pretty(&filtered)
                    .context("serialising the snapshot's version list")?,
            )?,
        ));
    }

    // One digest document per version this run **selected**, which
    // includes the ones already snapshotted: their bytes are immutable
    // and the batch writes only what differs, so re-staging them costs a
    // byte comparison and buys a repair — a snapshot whose record names a
    // version but whose digest file went missing is filled in by the next
    // run instead of staying half-written. A version the snapshot carries
    // and this run did not select is skipped: its file is in the target
    // and nothing here has read it.
    // The digest documents follow the **record**, never the trimmed list:
    // a version the record names keeps its digest file whichever list a
    // client is shown, so flipping the knob back to `all` needs no repair
    // run to make those versions usable again.
    let added: BTreeSet<String> = selected.iter().map(ToString::to_string).collect();
    for version in &recorded {
        if !added.contains(&version.to_string()) {
            continue;
        }
        // A template with no digest source has no document to stage:
        // the snapshot carries its version list and its renders, which
        // is everything a closed-network client needs for it
        // (PROP-075 `#TEMPLATE-NO-DIGEST`).
        let Some(document) = layer.digest_document(template, version)? else {
            continue;
        };
        files.push((
            snapshot::relative_digest(group, name, version),
            write(
                dir.join("digests").join(version.to_string()),
                &document.body,
            )?,
        ));
    }
    Ok(files)
}

/// Which versions a row selects: what `versions` matches, unioned with
/// the members of every `channels` entry, then `newest = N` per major.
///
/// Neither key given selects the whole domain. The union is deliberate:
/// `versions = [">=22"]` with `channels = ["lts"]` means "everything from
/// 22 up, and the LTS line whatever its number", which is what an
/// operator keeping a closed network supplied actually wants.
fn select_versions(
    row: &TemplateSelection,
    all: &[semver::Version],
    channels: &BTreeMap<String, BTreeSet<semver::Version>>,
) -> Vec<semver::Version> {
    let mut selected: BTreeSet<semver::Version> =
        if row.versions.is_empty() && row.channels.is_empty() {
            all.iter().cloned().collect()
        } else {
            let mut chosen: BTreeSet<semver::Version> = all
                .iter()
                .filter(|version| row.versions.iter().any(|req| req.matches(version)))
                .cloned()
                .collect();
            for name in &row.channels {
                if let Some(members) = channels.get(name) {
                    chosen.extend(members.iter().cloned());
                }
            }
            chosen
        };
    if let Some(newest) = row.newest {
        selected = newest_per_major(&selected, newest);
    }
    selected.into_iter().collect()
}

/// For every major present, keep the newest `n`.
fn newest_per_major(selected: &BTreeSet<semver::Version>, n: u32) -> BTreeSet<semver::Version> {
    let mut by_major: BTreeMap<u64, Vec<&semver::Version>> = BTreeMap::new();
    for version in selected {
        by_major.entry(version.major).or_default().push(version);
    }
    by_major
        .into_values()
        .flat_map(|mut versions| {
            // `BTreeSet` already iterates ascending, so the tail is the
            // newest; take it from the end.
            versions.sort();
            versions
                .into_iter()
                .rev()
                .take(n as usize)
                .cloned()
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The channel table a client behind this mirror will compute, which is
/// what the row has to report (PROP-075 `#MIRROR-CONFIG-SCHEMA`).
///
/// Channels are not stored in a snapshot: a client recomputes them from
/// the version document by the same `[[template.channel]]` rules, so
/// trimming that document trims the channel table with it. Under
/// `version_list = "selected"` the report must therefore name the
/// **trimmed** resolution and not the source's, or the `channel lts → …`
/// an operator reads and the `lts` a client resolves would disagree.
///
/// A named channel's own members are unioned into `selected` before this
/// runs, so the intersection usually changes nothing; the case it exists
/// for is `newest = N`, which is applied **after** that union and can
/// therefore drop a channel member back out
/// ([`BACKLOG.md#b-308`](../../../../../../BACKLOG.md#b-308) is the
/// notice this does not yet give).
fn listed_channels(
    version_list: VersionList,
    channels: &BTreeMap<String, BTreeSet<semver::Version>>,
    selected: &[semver::Version],
) -> BTreeMap<String, BTreeSet<semver::Version>> {
    match version_list {
        VersionList::All => channels.clone(),
        VersionList::Selected => {
            let listed: BTreeSet<semver::Version> = selected.iter().cloned().collect();
            channels
                .iter()
                .map(|(name, members)| {
                    (
                        name.clone(),
                        members.intersection(&listed).cloned().collect(),
                    )
                })
                .collect()
        }
    }
}

/// Each channel the row named and the version it lowers to — the newest
/// member of its set — with `None` for a channel the template does not
/// declare, which is reported rather than refused: a channel that names
/// nothing selects nothing, and the row says so.
fn resolved_channels(
    row: &TemplateSelection,
    channels: &BTreeMap<String, BTreeSet<semver::Version>>,
) -> Vec<(String, Option<semver::Version>)> {
    row.channels
        .iter()
        .map(|name| {
            (
                name.clone(),
                channels
                    .get(name)
                    .and_then(|members| members.iter().next_back())
                    .cloned(),
            )
        })
        .collect()
}

#[cfg(test)]
#[path = "templates/tests.rs"]
mod tests;
