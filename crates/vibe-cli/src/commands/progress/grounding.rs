//! The campaign-grounding cell — the observed tree, the campaign zone and
//! the two caches behind them, resolved once per invocation (PROP-043 §7.1).
//!
//! `ground` is the head of every subcommand and `refresh_state` the tail of
//! the ones that write, so the properties they carry — every subcommand is
//! incremental over the content-hash cache (DRIFT-010), a run that changes
//! nothing writes nothing (DRIFT-017) — are one cell's rather than eight
//! functions'. The campaign-path helpers live here for the same reason:
//! finding a zone and naming it is grounding's business and nothing else's.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#tool");

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use progress_core::doc::ParsedDoc;
use progress_core::{cache, journal, rollup, scope, sidecar, state};
use vibe_registry::ShellGit;

use crate::cli::ProgressCommonArgs;

/// The observed tree + campaign zone, resolved once per invocation.
pub(crate) struct Ground {
    pub(super) root: PathBuf,
    pub(crate) docs: Vec<ParsedDoc>,
    pub(super) campaign: Option<PathBuf>,
    /// The campaign cache, read once at the head of the run: it says what
    /// each observed file hashed to when it was judged, and carries the
    /// verdicts. `refresh_state` writes it back. One read and one write
    /// per invocation — a second `load` here would be a second megabyte of
    /// JSON for the same bytes, and worse, a second opinion about them.
    pub(super) cache: cache::Cache,
    /// The warning `load_tolerant` produced when the cache on disk could
    /// not be read and this run is proceeding on an empty one. Every
    /// subcommand prints it; `baseline` additionally **refuses** to run,
    /// because a baseline projected from a cache that failed to load is
    /// not an empty baseline — it is a truncated one, and it reads as
    /// knowledge (DRIFT-023 §4.3).
    pub(super) cache_warning: Option<String>,
    /// The parse-payload sidecar, outside the repository (DRIFT-016).
    /// Read alongside the cache and written alongside it; every way of not
    /// having it is an empty store, so a run whose bucket was never
    /// created is a cold run and says nothing about it.
    pub(super) payloads: sidecar::Payloads,
    /// Files the config `exclude` globs removed (`scope::ExcludeReport`).
    pub(super) excluded: usize,
    /// Observed files that entered as XML sources (PROP-045
    /// ##PROJECTION-READ): each was parsed through its canonical MD
    /// projection, so every diagnostic it produces cites projection-
    /// relative lines — the check verb marks them with the shared
    /// projection notice rather than letting the numbers pass as
    /// source-relative.
    pub(crate) xml_sources: BTreeSet<String>,
}

/// Resolve the tree, then produce one `ParsedDoc` per observed file —
/// from the sidecar where the cache's content hash says nothing changed,
/// from the parser where it does not (PROP-043 §7.1, DRIFT-010 §4,
/// DRIFT-016 §4).
///
/// The file is read either way: the hash is over its bytes, so there is no
/// version of this that trusts a timestamp. What a hit buys is the parse,
/// not the read. `--no-cache` skips the lookup entirely and parses
/// everything, which is what a run that must not inherit a verdict does.
///
/// Every subcommand grounds through here, so "all subcommands are
/// incremental over the content-hash cache" is one function's property
/// rather than eight — realises `TOOL-INCREMENTAL`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-047#tool")]
pub(crate) fn ground(common: &ProgressCommonArgs) -> Result<Ground> {
    let root = common
        .path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", common.path.display()))?;
    let root = super::super::init::strip_unc_public(root);
    let cfg = scope::load_config(&root)?;
    let campaign = resolve_campaign(&root, common.campaign.as_deref())?;

    // An unreadable cache is a warning and a cold run, never a failure:
    // the cache is derived acceleration and may be deleted at any time
    // (PROP-043 §7.5).
    let (cache, cache_warning) = match &campaign {
        Some(c) => {
            let (loaded, recovered) =
                cache::Cache::load_tolerant(&c.join("run").join("cache.json"));
            if let Some(warning) = &recovered {
                eprintln!("vibe progress: warning: {warning}");
            }
            (loaded, recovered)
        }
        None => (cache::Cache::default(), None),
    };
    // The payload store is pure acceleration and lives outside the tree,
    // so it has no warning to print and no failure to report: an absent
    // bucket, an unreadable file and a machine that has never run this
    // campaign are one case, and that case is "parse" (DRIFT-016 §4.3).
    let payloads = sidecar::Payloads::load(payload_dir(&root, &cfg, campaign.as_deref()));

    let (files, excludes) = scope::observed_files_reported(&root, &cfg)?;
    for p in &excludes.stale {
        eprintln!("vibe progress: warning: exclude pattern `{p}` matched no observed file");
    }
    // One logical document, one form (PROP-045 ##TARGET-MIXED): `X.md` and
    // `X.xml` beside each other are a split brain — parsing both would
    // count one document's units twice under two paths, so the pair is a
    // loud stop before any parse, naming both files.
    if let Some(collision) = vibe_specdoc::pair_collisions_in(&files).first() {
        bail!("{}", collision.message());
    }
    let mut docs = Vec::new();
    let mut xml_sources = BTreeSet::new();
    for rel in files {
        let full = root.join(&rel);
        // The projection dispatch (PROP-045 ##PROJECTION-READ): `.md` (and
        // anything else) verbatim, `.xml` through `from_xml →
        // to_markdown`. The hash is over the text the parser consumes —
        // the projection — which S1's emitter makes deterministic, so the
        // cache/verdict mechanics are unchanged: an edit moves the
        // projection exactly when it moves meaning.
        let (text, kind) = vibe_specdoc::load_spec_text(&full)
            .map_err(|e| anyhow::Error::msg(e.to_string()))
            .with_context(|| format!("reading {}", full.display()))?;
        if kind == vibe_specdoc::SourceKind::XmlProjected {
            xml_sources.insert(scope::rel_str(&rel));
        }
        let path = scope::rel_str(&rel);
        let hash = progress_core::parse::content_hash(&text);
        let cached = (!common.no_cache)
            .then(|| cache.cached_doc(&path, &hash, &payloads))
            .flatten();
        docs.push(match cached {
            Some(hit) => hit.clone(),
            None => progress_core::parse::parse_document(&path, &text),
        });
    }
    Ok(Ground {
        root,
        docs,
        campaign,
        cache,
        cache_warning,
        payloads,
        excluded: excludes.dropped,
        xml_sources,
    })
}

/// Where this run's payload sidecar lives, or `None` for a run without one
/// — no campaign zone to key it by, or no per-user home to hang it off.
///
/// Two pieces of knowledge the core is not allowed to have meet here, and
/// only here. Which checkout this is comes from git, asked exactly once
/// per run and answered as data — the same seam DRIFT-009 uses for the
/// crate→commit map (PROP-043 §2). And the per-user home comes from the
/// settings chokepoint, so `VIBE_SETTINGS` relocates this store with
/// everything else and the sidecar needs no environment variable of its
/// own — one variable to remember rather than two (F-055).
fn payload_dir(root: &Path, cfg: &scope::ScopeConfig, campaign: Option<&Path>) -> Option<PathBuf> {
    let campaign = campaign?;
    let home = vibe_core::settings::settings_dir();
    let branch = current_branch(root);
    sidecar::resolve_dir(
        root,
        cfg.progress.cache_dir.as_deref(),
        home.as_deref(),
        branch.as_deref(),
        &campaign_id(campaign),
    )
}

/// The branch checked out at `root`, or `None` when there is none to name.
///
/// A detached HEAD, a tree that is not a checkout, no git binary at all:
/// every one of them lands in the `detached` bucket rather than failing a
/// run. The payload is optional by construction, and so is knowing which
/// branch produced it.
fn current_branch(root: &Path) -> Option<String> {
    ShellGit::new().branch(root).ok().flatten()
}

/// `--campaign` wins; otherwise the single `campaigns/<id>/` when exactly
/// one exists; otherwise none (ad-hoc mode — reports work, state does not).
pub(super) fn resolve_campaign(root: &Path, flag: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(f) = flag {
        let spelling = f.to_string_lossy();
        if !spelling.contains('/') && !spelling.contains('\\') {
            let zone = root.join("campaigns");
            let selected = zone.join(f);
            if selected.is_dir() {
                return Ok(Some(selected));
            }
            let mut existing: Vec<String> = std::fs::read_dir(&zone)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            existing.sort();
            let existing = if existing.is_empty() {
                "<none>".to_string()
            } else {
                existing.join(", ")
            };
            bail!(
                "campaign `{}` does not exist under `{}`; existing campaigns: {existing} \
                 (violates spec://org.vibevm.core/vibevm/modules/vibe-facts/PROP-043#BOUNDARY-CLI; \
                 fix: pass an existing campaign id or a path containing a separator)",
                f.display(),
                zone.display()
            );
        }
        return Ok(Some(if f.is_absolute() {
            f.to_path_buf()
        } else {
            root.join(f)
        }));
    }
    let zone = root.join("campaigns");
    let entries: Vec<PathBuf> = std::fs::read_dir(&zone)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    Ok(match entries.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    })
}

pub(super) fn campaign_id(campaign: &Path) -> String {
    campaign
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "campaign".into())
}

/// What a refresh did to the disk (DRIFT-017 §4.3): one entry per
/// artifact the run could have written, `true` where it actually did.
///
/// A run over an unchanged tree writes none of them, and that is exactly
/// why the tally is reported rather than assumed — a skip nobody can see
/// is an optimisation nobody can debug.
#[derive(Debug, Default)]
pub(crate) struct Refresh {
    pub(super) campaign: Option<PathBuf>,
    pub(super) writes: BTreeMap<String, bool>,
}

impl Refresh {
    /// How many artifacts this run wrote, and how many it left alone.
    pub(super) fn tally(&self) -> (usize, usize) {
        let wrote = self.writes.values().filter(|w| **w).count();
        (wrote, self.writes.len() - wrote)
    }
}

/// Refresh cache + sidecar + state under the campaign zone from parsed
/// docs.
///
/// Takes the cache `ground` already read — the upsert is over the same
/// records the reuse decision was made against, and the campaign fields
/// those records carry ride through untouched (`upsert` preserves them).
///
/// Every write here first asks whether it would change anything, so a run
/// over an unchanged tree touches no file at all (DRIFT-017). What it
/// costs is one read per artifact; what it saves is the fsync'd rewrite of
/// several megabytes that DRIFT-010 §9 measured as the real cost of a run.
///
/// The two writes are deliberately unequal. `cache.json` is tracked, holds
/// the verdicts, and a failure to write it fails the run. The sidecar is
/// derived, lives outside the tree, and a failure to write it is a slower
/// next run — so it goes second and says nothing either way.
pub(crate) fn refresh_state(g: &mut Ground) -> Result<Refresh> {
    let Some(campaign) = &g.campaign else {
        return Ok(Refresh::default());
    };
    let run_dir = campaign.join("run");
    let cache_path = run_dir.join("cache.json");
    let c = &mut g.cache;
    for doc in &g.docs {
        let r = rollup::rollup_doc(doc);
        c.upsert(doc, &r);
    }
    // Prune records whose file left the observed scope, so the cache — and
    // thus `corpus.json` / `campaign.json` — describes exactly the parsed
    // corpus, not the union across every past scan (DRIFT-001). A pruned
    // record that still carried campaign verdicts is surfaced loudly, never
    // silently discarded (DRIFT-001 §5).
    let observed: BTreeSet<String> = g.docs.iter().map(|d| d.path.clone()).collect();
    for lost in c.retain_paths(&observed) {
        eprintln!(
            "vibe progress: warning: pruned out-of-scope record `{lost}` that carried campaign verdicts"
        );
    }
    // Stamping first is safe and deliberate: the identity test ignores the
    // `updated_at` a document carries, so the candidate may hold this
    // run's clock while the file keeps the one it was last changed at.
    c.touch();
    let mut writes = BTreeMap::new();
    writes.insert("cache.json".to_string(), c.store(&cache_path)?);
    writes.insert(
        sidecar::PAYLOAD_FILE.to_string(),
        g.payloads.store(g.docs.iter()),
    );
    // Phase is derived from the campaign's own journal (last `phase` event
    // wins; absent ⇒ "A") — never compiled in, never parsed from Markdown.
    let phase = journal::derive_phase(&journal::read_journal(&run_dir.join("journal.jsonl"))?);
    writes.extend(state::write_state(
        &run_dir.join("state"),
        &campaign_id(campaign),
        &phase,
        c,
    )?);
    Ok(Refresh {
        campaign: Some(campaign.clone()),
        writes,
    })
}
