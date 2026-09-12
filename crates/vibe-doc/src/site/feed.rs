//! The change feed — what the sources publish right now
//! (PROP-057 `##SITE-SOURCE-REGISTRY`, `##SITE-HOST-CHECKOUT`).
//!
//! Two sources answer one question in two ways, and the answer has the
//! same shape either way: a set of «coordinate · version · content hash»
//! as it stands NOW. There is no «since last time» anywhere in here. What
//! moved is decided by comparing this set with what the builder last
//! rendered ([`super::state`]), which is the only comparison this project
//! allows itself — a version number may be published many times and only
//! the last publication exists (`##SITE-VERSION-SHOWS-CURRENT`, §14).
//!
//! ## The registry: the index IS the feed
//!
//! `repomd.json` first, then `primary.jsonl`: the manifest is written
//! last on every batch update, so a reader that takes it first and then
//! reads the catalog it describes sees a consistent set. Every record
//! already carries a `content_hash`, so nothing has to be fetched to know
//! whether it moved — which is what makes a poll of a whole registry
//! cheap enough to run on a timer.
//!
//! ## The host: a checkout, and a hash recomputed from it
//!
//! The host's root is a `[project]` and not a publishable package, so it
//! is not in any index and has no `content_hash` to read: the deploy
//! keeps a checkout current and the renderer reads it
//! (`##SITE-HOST-CHECKOUT`). `##SITE-RENDER-KEY` says the key is then «a
//! recomputation for the checkout», and this is where that recomputation
//! is: a hash over exactly the files the render reads, which is what
//! makes it answer the right question. The commit would not — an in-tree
//! package keeps its version number while its content moves, and a
//! checkout is allowed to be dirty (a deploy that guarantees otherwise is
//! a promise made on the server, not a fact this reader can check).
//!
//! What the host contributes is what `##SITE-HOST-CHECKOUT` names: the
//! project itself at level 0, and the documentation packages it carries
//! in-tree. Not every in-tree package — the others are not published, and
//! the registry is where published packages come from.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SITE-SOURCE-REGISTRY");

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use vibe_registry::index_client::{IndexClient, IndexUrlResolution, ProbeOutcome};
use vibe_wire::generated::shared::VersionEntry;

use super::config::{Host, Registry};
use crate::error::{DocError, Result};

/// Where a version is read from when the time comes to render it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Published in a registry: warmed into the builder's store by
    /// coordinate, like any other consumer would.
    Registry,
    /// The host project itself, read out of the checkout on disk.
    HostProject { root: PathBuf },
    /// A package the host's checkout carries in-tree, read where it lies.
    HostPackage { dir: PathBuf },
}

/// One thing the site can render: a coordinate, a version, and the hash
/// of the bytes behind them right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// Which source published it — a registry alias, or the host's
    /// repository. Provenance for the report; the identity is the
    /// coordinate and the version.
    pub source: String,
    pub group: String,
    pub name: String,
    pub version: String,
    /// The render key of `##SITE-RENDER-KEY`.
    pub content_hash: String,
    pub origin: Origin,
    /// The catalog record, when this came from an index. It carries the
    /// card and the relation edges the shelves are computed from, so
    /// reading the feed is also reading everything the site knows about
    /// a package without fetching it.
    pub entry: Option<Box<VersionEntry>>,
}

impl Pair {
    /// `<group>/<name>` — the coordinate without its version.
    pub fn coordinate(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }

    /// `<group>/<name>@<version>` — how a report names one.
    pub fn spelled(&self) -> String {
        format!("{}@{}", self.coordinate(), self.version)
    }
}

/// Everything one poll of every source found.
#[derive(Debug, Clone, Default)]
pub struct Feed {
    pub pairs: Vec<Pair>,
    /// One line per source about what answered — the registry's own name
    /// and counts, or the checkout that was read. Printed by the report,
    /// because «the feed was empty» and «the index did not answer» are
    /// the two states an operator has to tell apart.
    pub sources: Vec<String>,
}

/// Poll every source a configuration names.
///
/// A source that will not answer stops the run. That is the opposite of
/// how a failing PACKAGE is treated — one bad package becomes a «render
/// failed» page and the rest of the site is built (`##SITE-RENDER-IDEMPOTENT`)
/// — and the difference is what each failure means: a package that will
/// not render affects one address, while a registry that will not answer
/// makes every one of its packages look unpublished. Rendering that
/// answer would deploy the disappearance of a whole registry over a
/// working site.
pub fn poll(site: &super::config::Site) -> Result<Feed> {
    let mut feed = Feed::default();
    for registry in &site.registries {
        let (pairs, line) = registry_feed(registry)?;
        feed.pairs.extend(pairs);
        feed.sources.push(line);
    }
    if let Some(host) = &site.host {
        let (pairs, line) = host_feed(host)?;
        feed.pairs.extend(pairs);
        feed.sources.push(line);
    }
    Ok(feed)
}

/// Poll one registry's index.
///
/// The index location comes from the ladder the resolver already uses
/// (`vibe_registry::index_client::resolve_index_url`) rather than from a
/// rule invented here, so the site reads the index that `vibe install`
/// reads. That ladder's top rung is an operator's environment variable,
/// and inheriting it is deliberate: a machine that re-points its index
/// re-points the site built on it too.
pub fn registry_feed(registry: &Registry) -> Result<(Vec<Pair>, String)> {
    let section = registry.as_section();
    let base = match vibe_registry::index_client::resolve_index_url(&section) {
        IndexUrlResolution::Url { base, .. } => base,
        IndexUrlResolution::Disabled => {
            return Err(DocError::Site {
                message: format!(
                    "the registry `{}` switches its index off (`index_url = \"none\"`), \
                     and the index is the change feed — there is no other way to ask a \
                     registry what it publishes",
                    registry.name
                ),
            });
        }
    };
    let client = match IndexClient::probe(&base, vibe_registry::index_client::IndexAuth::None) {
        ProbeOutcome::Found(client) => client,
        ProbeOutcome::Absent => {
            return Err(DocError::Site {
                message: format!(
                    "no index answered at `{base}` for the registry `{}`",
                    registry.name
                ),
            });
        }
        ProbeOutcome::Refused { reason } => {
            return Err(DocError::Site {
                message: format!(
                    "the index at `{base}` refused the registry `{}`: {reason} — the \
                     site reads every source anonymously and carries no credential",
                    registry.name
                ),
            });
        }
    };

    // The manifest first, the catalog it describes second: the manifest
    // is written last on every batch update, so this order reads a
    // consistent pair.
    let repomd = client.repomd().map_err(|e| DocError::Site {
        message: format!("the registry `{}`: {e}", registry.name),
    })?;
    let entries = client
        .primary()
        .map_err(|e| DocError::Site {
            message: format!("the registry `{}`: {e}", registry.name),
        })?
        .ok_or_else(|| DocError::Site {
            message: format!(
                "the index at `{base}` carries no `primary.jsonl` — a registry with no \
                 catalog has nothing to publish to a site",
                base = client.file_base()
            ),
        })?;

    let line = match &repomd {
        Some(repomd) => format!(
            "registry {} at {} — {} package(s), {} version(s), generated {}",
            registry.name,
            client.file_base(),
            repomd.package_count,
            repomd.version_count,
            repomd.generated_at,
        ),
        None => format!(
            "registry {} at {} — no `repomd.json`; read {} version(s) from the catalog",
            registry.name,
            client.file_base(),
            entries.len(),
        ),
    };

    let pairs = entries
        .into_iter()
        .map(|entry| Pair {
            source: registry.name.clone(),
            group: entry.group.to_string(),
            name: entry.name.clone(),
            version: entry.version.to_string(),
            content_hash: entry.content_hash.clone(),
            origin: Origin::Registry,
            entry: Some(Box::new(entry)),
        })
        .collect();
    Ok((pairs, line))
}

/// Read the host's checkout: the project itself, and the documentation
/// packages it carries in-tree.
pub fn host_feed(host: &Host) -> Result<(Vec<Pair>, String)> {
    let root = &host.checkout;
    let manifest = root.join("vibe.toml");
    if !manifest.is_file() {
        return Err(DocError::Site {
            message: format!(
                "the host's checkout `{}` holds no `vibe.toml` — the deploy keeps it \
                 current with `git checkout {} && git pull`, and the renderer reads it \
                 where it lies",
                root.display(),
                host.git_ref,
            ),
        });
    }
    let project = read_toml(&manifest)?;
    let (group, name, version) = project_coordinate(&manifest, &project)?;

    // What level 0 of the project reads: its own card, the text it
    // publishes, and the specifications it carries. The hash covers
    // exactly that, so a commit that touches nothing the site renders
    // costs nothing.
    let mut files = vec![manifest.clone()];
    for named in ["README.md", "AGENTS.md", "CLAUDE.md"] {
        let path = root.join(named);
        if path.is_file() {
            files.push(path);
        }
    }
    collect(&root.join(crate::pages::SPEC_ROOT), &mut files);
    let mut pairs = vec![Pair {
        source: host.git.clone(),
        group,
        name,
        version,
        content_hash: hash_files(root, &mut files)?,
        origin: Origin::HostProject { root: root.clone() },
        entry: None,
    }];

    let in_tree = in_tree_doc_packages(root)?;
    let carried = in_tree.len();
    for dir in in_tree {
        let manifest = dir.join("vibe.toml");
        let parsed = read_toml(&manifest)?;
        let (group, name, version) = package_coordinate(&manifest, &parsed)?;
        let mut files = Vec::new();
        collect(&dir, &mut files);
        pairs.push(Pair {
            source: host.git.clone(),
            group,
            name,
            version,
            content_hash: hash_files(&dir, &mut files)?,
            origin: Origin::HostPackage { dir },
            entry: None,
        });
    }

    let line = format!(
        "host {} @{} from {} — the project and {carried} documentation package(s) in tree",
        host.git,
        host.git_ref,
        root.display(),
    );
    Ok((pairs, line))
}

/// The in-tree packages of kind `doc` a checkout carries, in a stable
/// order.
///
/// Only `doc`. `##SITE-HOST-CHECKOUT` names two things the host
/// contributes — the project at level 0 and «the core documentation as an
/// in-tree package» — and the rest of `vibevm/vibepacks` is a project's
/// own working copies of packages that reach a site by being published.
/// Reading the kind rather than a list is what keeps a second
/// documentation package from being invisible the day somebody opens one.
fn in_tree_doc_packages(root: &Path) -> Result<Vec<PathBuf>> {
    let packs = root.join(PACKS_ROOT);
    if !packs.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    // `<group>/<name>/v<version>/vibe.toml` — the in-tree registry's
    // layout, walked to exactly that depth so a `node_modules` inside a
    // package is never entered.
    for group in sorted_dirs(&packs)? {
        for name in sorted_dirs(&group)? {
            for slot in sorted_dirs(&name)? {
                let manifest = slot.join("vibe.toml");
                if !manifest.is_file() {
                    continue;
                }
                let parsed = read_toml(&manifest)?;
                let kind = parsed
                    .get("package")
                    .and_then(|t| t.get("kind"))
                    .and_then(toml::Value::as_str);
                if kind == Some(DOC_KIND) {
                    out.push(slot);
                }
            }
        }
    }
    Ok(out)
}

/// Where a project keeps the packages it authors in tree.
const PACKS_ROOT: &str = "vibevm/vibepacks";

/// The one kind the host channel renders from its checkout.
const DOC_KIND: &str = "doc";

/// The directories directly under `dir`, sorted by name.
fn sorted_dirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| DocError::io("reading", dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| DocError::io("reading", dir, e))?;
        if entry.path().is_dir() {
            out.push(entry.path());
        }
    }
    out.sort();
    Ok(out)
}

/// Every file under `dir`, appended to `found`. A directory that is not
/// there contributes nothing, which is what an optional spec root is.
fn collect(dir: &Path, found: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in walkdir::WalkDir::new(dir).sort_by_file_name() {
        let Ok(entry) = entry else { continue };
        if entry.file_type().is_file() {
            found.push(entry.into_path());
        }
    }
}

/// The hash of a set of files, as a render key.
///
/// Path and bytes both, each length-prefixed: a hash over contents alone
/// would miss a rename, and one over names alone would miss an edit. The
/// paths are relative to `root` and forward-slashed, so a checkout on
/// another operating system keys the same content the same way.
fn hash_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<String> {
    files.sort();
    files.dedup();
    let mut hasher = Sha256::new();
    for file in files.iter() {
        let rel = file
            .strip_prefix(root)
            .unwrap_or(file)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        let bytes = std::fs::read(file).map_err(|e| DocError::io("reading", file, e))?;
        hasher.update(rel.len().to_le_bytes());
        hasher.update(rel.as_bytes());
        hasher.update(bytes.len().to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

/// Read a manifest as TOML DATA.
///
/// The one question asked of it is «what coordinate and kind is this», and
/// a strict manifest parse would refuse a checkout over a field this
/// reader never looks at — the same reason `vibe doc`'s other readers of
/// a foreign manifest read it this way.
fn read_toml(path: &Path) -> Result<toml::Value> {
    let text = std::fs::read_to_string(path).map_err(|e| DocError::io("reading", path, e))?;
    toml::from_str(&text).map_err(|e| DocError::manifest(path, format!("is not readable: {e}")))
}

/// The `[project]` coordinate of a checkout.
fn project_coordinate(path: &Path, parsed: &toml::Value) -> Result<(String, String, String)> {
    coordinate(path, parsed, "project")
}

/// The `[package]` coordinate of an in-tree package.
fn package_coordinate(path: &Path, parsed: &toml::Value) -> Result<(String, String, String)> {
    coordinate(path, parsed, "package")
}

fn coordinate(path: &Path, parsed: &toml::Value, table: &str) -> Result<(String, String, String)> {
    let read = |key: &str| {
        parsed
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    match (read("group"), read("name"), read("version")) {
        (Some(group), Some(name), Some(version)) => Ok((group, name, version)),
        _ => Err(DocError::manifest(
            path,
            format!(
                "does not name a `[{table}]` coordinate — the site addresses every \
                 version by `<group>/<name>/<version>` and has no other name for it"
            ),
        )),
    }
}

#[cfg(test)]
mod tests;
