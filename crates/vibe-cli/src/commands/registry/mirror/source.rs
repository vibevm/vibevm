//! The mirror's source side: list a registry's versions and fetch one,
//! always through the operation's controlled transport, source policy and
//! remap Locator.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::Group;
use vibe_core::manifest::{MirrorSection, RegistrySection, RegistryType};
use vibe_index::journal::Event;
use vibe_publish::layout_publish::Provenance;
use vibe_registry::{HttpAdmission, IndexClient};
use vibe_wire::generated::registry_mirror_report::Listing as ListingKind;

mod api_listing;
mod index_probe;

use index_probe::{probe_index, refusal_first};

use super::git_source::{self, GitStore};
use super::plan::{Decision, Planned};
use super::{Fetched, Selection};
use crate::commands::registry::layout_root::{self, LayoutRoot};

/// One version the source offers.
#[derive(Debug, Clone)]
pub(super) struct SourceVersion {
    pub group: Group,
    pub name: String,
    pub version: semver::Version,
    /// The hash the source records (its index); `None` without an index.
    pub content_hash: Option<String>,
    pub frozen: bool,
    pub yanked: bool,
    /// For a per-package source: the canonical repository url and the tag
    /// name as the source spells it.
    git: Option<(String, String)>,
    /// For a layout source: the version's directory in its tree, with the
    /// entry's own provenance.
    layout: Option<(PathBuf, Provenance)>,
}

/// What the source lists.
pub(super) struct SourceListing {
    pub kind: ListingKind,
    pub versions: Vec<SourceVersion>,
    /// Retired names: `(name, reason, superseded_by)`.
    pub tombstones: Vec<(String, String, Option<String>)>,
}

pub(super) enum SourceReader {
    /// A `monorepo` / `localdir` source: its tree and its index.
    Layout(LayoutRoot),
    /// Per-package repositories (`github`, `gitverse`, `git`), with the
    /// index when one answers.
    PerPackage {
        registry: Box<RegistrySection>,
        mirrors: Vec<MirrorSection>,
        store: GitStore,
        /// Hosts source policy excludes.
        denied: BTreeSet<String>,
        resolver: Box<vibe_registry::MultiRegistryResolver>,
        index: Option<Box<IndexClient>>,
        /// The operation's contact binding in a controlled posture, which
        /// the host-API listing requests under (`None` is the
        /// uncontrolled posture).
        admission: Option<Arc<HttpAdmission>>,
        offline: bool,
    },
}

impl SourceReader {
    pub(super) fn open(
        reg: &RegistrySection,
        mirrors: &[MirrorSection],
        operation: &vibe_package_source::SourceOperation,
        offline: bool,
        progress: &vibe_core::progress::Progress,
    ) -> Result<Self> {
        if matches!(
            reg.checked_type()?,
            RegistryType::Monorepo | RegistryType::Localdir
        ) {
            return Ok(SourceReader::Layout(layout_root::open(
                reg, mirrors, operation, offline, progress,
            )?));
        }
        let resolver = vibe_package_source::registry_resolver(
            operation,
            vec![reg.clone()],
            mirrors.to_vec(),
            offline,
            progress,
        )?;
        let index = if offline {
            None
        } else {
            probe_index(reg, operation)?
        };
        let denied = operation
            .source_policy()?
            .denied_host_keys()
            .into_iter()
            .map(|host| host.to_ascii_lowercase())
            .collect();
        Ok(SourceReader::PerPackage {
            registry: Box::new(reg.clone()),
            mirrors: mirrors.to_vec(),
            store: GitStore::new()?,
            denied,
            resolver: Box::new(resolver),
            index: index.map(Box::new),
            admission: operation.http_admission()?,
            offline,
        })
    }

    pub(super) fn list(&self, selection: &Selection) -> Result<SourceListing> {
        match self {
            SourceReader::Layout(opened) => {
                let (index, _) =
                    vibe_index::layout::load_or_project(opened.root(), opened.identity())?;
                let versions = index
                    .iter_versions()
                    .filter(|e| selection.admits(e.group.as_str(), &e.name))
                    .map(|e| SourceVersion {
                        group: e.group.clone(),
                        name: e.name.clone(),
                        version: e.version.clone(),
                        content_hash: Some(e.content_hash.clone()),
                        frozen: e.frozen,
                        yanked: e.yanked,
                        git: None,
                        layout: Some((
                            opened
                                .root()
                                .join(format!("{}/{}/v{}", e.group, e.name, e.version)),
                            Provenance {
                                source_url: e.source_url.clone(),
                                source_ref: e.source_ref.clone(),
                                resolved_commit: e.resolved_commit.clone(),
                            },
                        )),
                    })
                    .collect();
                let tombstones = index
                    .tombstones
                    .iter()
                    .filter(|(name, _)| tombstone_admitted(selection, name))
                    .map(|(name, t)| (name.clone(), t.reason.clone(), t.superseded_by.clone()))
                    .collect();
                Ok(SourceListing {
                    kind: ListingKind::Layout,
                    versions,
                    tombstones,
                })
            }
            SourceReader::PerPackage {
                registry,
                index: Some(client),
                ..
            } => {
                let entries = client.primary().map_err(refusal_first)?.ok_or_else(|| {
                    anyhow!(
                        "the index of registry `{}` answers its handshake but carries no \
                         `primary.jsonl` \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#layout; \
                         fix: regenerate the source index, or mirror with `--package`)",
                        registry.name
                    )
                })?;
                let living: BTreeSet<String> = entries.iter().map(|e| e.name.clone()).collect();
                let versions = entries
                    .into_iter()
                    .filter(|e| selection.admits(e.group.as_str(), &e.name))
                    .map(|e| SourceVersion {
                        git: Some((e.source_url.clone(), e.source_ref.clone())),
                        group: e.group,
                        name: e.name,
                        version: e.version,
                        content_hash: Some(e.content_hash),
                        frozen: e.frozen,
                        yanked: e.yanked,
                        layout: None,
                    })
                    .collect();
                let mut tombstones = Vec::new();
                if let Some(repomd) = client.repomd().map_err(refusal_first)? {
                    for key in repomd.files.keys() {
                        let Some(name) = key
                            .strip_prefix("by-name/")
                            .and_then(|k| k.strip_suffix(".json"))
                        else {
                            continue;
                        };
                        if living.contains(name) || !tombstone_admitted(selection, name) {
                            continue;
                        }
                        if let Some((reason, superseded_by)) =
                            client.tombstone(name).map_err(refusal_first)?
                        {
                            tombstones.push((name.to_string(), reason, superseded_by));
                        }
                    }
                }
                Ok(SourceListing {
                    kind: ListingKind::Index,
                    versions,
                    tombstones,
                })
            }
            SourceReader::PerPackage {
                registry,
                resolver,
                index: None,
                ..
            } => {
                if selection.packages().is_empty() {
                    return self.list_through_host_api(selection);
                }
                let mut versions = Vec::new();
                for (group, name) in selection.packages() {
                    let group = Group::parse(group)?;
                    let repository = repository_url(registry, &group, name)?;
                    let listed = resolver
                        .list_versions(&group, name)
                        .map_err(layout_root::refusal_first)?;
                    versions.extend(tagged_versions(&repository, &group, name, listed));
                }
                Ok(SourceListing {
                    kind: ListingKind::Tags,
                    versions,
                    tombstones: Vec::new(),
                })
            }
        }
    }

    /// Fetch one version's tree (into `scratch` when it must be copied
    /// out of the source first).
    pub(super) fn fetch(&self, version: &SourceVersion, scratch: &Path) -> Result<Fetched> {
        if let Some((tree, provenance)) = &version.layout {
            return Ok(Fetched {
                tree: tree.clone(),
                provenance: provenance.clone(),
                frozen: version.frozen,
                git: None,
            });
        }
        let SourceReader::PerPackage {
            registry,
            mirrors,
            store,
            denied,
            ..
        } = self
        else {
            bail!("internal: a layout source version without its directory");
        };
        let Some((repository, tag)) = &version.git else {
            bail!("internal: a per-package source version without its repository");
        };
        let repo = repo_key(repository);
        let url = admitted(registry, mirrors, denied, repository)?;
        let facts = store.fetch_tag(&url, &repo, tag)?;
        let dest = scratch.join(format!("{repo}-{}", version.version));
        store.checkout(&facts.tree, &dest)?;
        let manifest = vibe_core::manifest::Manifest::read(dest.join("vibe.toml"))
            .with_context(|| format!("reading the vibe.toml of {repository} at `{tag}`"))?;
        Ok(Fetched {
            frozen: manifest.package.as_ref().is_some_and(|p| p.frozen),
            provenance: Provenance {
                source_url: repository.clone(),
                source_ref: tag.clone(),
                resolved_commit: Some(facts.commit.clone()),
            },
            git: Some(GitOrigin {
                store: store.path().to_path_buf(),
                reference: GitStore::local_ref(&repo, tag),
                repository: repository.clone(),
                repo,
                facts,
            }),
            tree: dest,
        })
    }

    /// The per-package scratch store (for history bundles).
    pub(super) fn git_store(&self) -> Option<&GitStore> {
        match self {
            SourceReader::PerPackage { store, .. } => Some(store),
            SourceReader::Layout(_) => None,
        }
    }

    /// Bundle every tag and branch of `repository` into `out` (through
    /// the remap Locator and source policy, as a fetch is).
    pub(super) fn history_bundle(&self, repository: &str, out: &Path) -> Result<()> {
        let SourceReader::PerPackage {
            registry,
            mirrors,
            store,
            denied,
            ..
        } = self
        else {
            bail!("internal: a layout source has no repositories");
        };
        let url = admitted(registry, mirrors, denied, repository)?;
        store.bundle(&url, &repo_key(repository), out)
    }
}

/// The canonical repository of `(group, name)` in a per-package registry.
fn repository_url(registry: &RegistrySection, group: &Group, name: &str) -> Result<String> {
    Ok(format!(
        "{}/{}.git",
        registry.url.trim_end_matches('/'),
        registry.naming.repo_name(None, group, name)?
    ))
}

/// One `SourceVersion` per listed version of a per-package repository:
/// the version's bytes are the repository's `v<version>` tag, and the
/// source records no hash of them (there is no index to record one).
fn tagged_versions(
    repository: &str,
    group: &Group,
    name: &str,
    listed: Vec<semver::Version>,
) -> Vec<SourceVersion> {
    listed
        .into_iter()
        .map(|version| SourceVersion {
            git: Some((repository.to_string(), format!("v{version}"))),
            group: group.clone(),
            name: name.to_string(),
            version,
            content_hash: None,
            frozen: false,
            yanked: false,
            layout: None,
        })
        .collect()
}

/// The url a repository is actually read from: through the remap Locator,
/// then past the source-policy gate. A host the policy excludes stops the
/// run rather than being contacted.
fn admitted(
    registry: &RegistrySection,
    mirrors: &[MirrorSection],
    denied: &BTreeSet<String>,
    repository: &str,
) -> Result<String> {
    let url = remapped(registry, mirrors, repository)?;
    if let Some(host) = git_source::url_host(&url)
        && denied.contains(&host)
    {
        return Err(PolicyRefused {
            url: vibe_publish::git_publish::redact_credentials(&url),
        }
        .into());
    }
    Ok(url)
}

/// The repository's name: the last segment of its url, without `.git`.
pub(super) fn repo_key(repository: &str) -> String {
    repository
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit(['/', ':'])
        .next()
        .unwrap_or_default()
        .to_string()
}

impl SourceVersion {
    /// The canonical repository of a per-package source version.
    pub(super) fn repository(&self) -> Option<&str> {
        self.git.as_ref().map(|(url, _)| url.as_str())
    }
}

/// The repository url through the remap Locator (`[[remap]]` rows and the
/// registry's `[[mirror]]` tables), as the resolver's clone chain does: a
/// `[[mirror]]` row maps the registry's root, so the root is remapped and
/// the repository's path under it is kept. A repository outside the root
/// is remapped as itself.
fn remapped(registry: &RegistrySection, mirrors: &[MirrorSection], url: &str) -> Result<String> {
    use vibe_core::remap::{Artifact, ArtifactKind, Layer, rows_from_registry_tables};
    let locator = vibe_core::remap::active().with_registry_tables(
        rows_from_registry_tables(std::slice::from_ref(registry), mirrors, &[], Layer::Local),
        vec![(registry.name.clone(), registry.url.clone())],
    );
    let choose =
        |url: &str| locator.choose_url(&Artifact::new(ArtifactKind::RegistryClone).with_url(url));
    let whole = choose(url)?;
    let root = registry.url.trim_end_matches('/');
    match url
        .strip_prefix(root)
        .and_then(|rest| rest.strip_prefix('/'))
    {
        Some(repository) if !repository.is_empty() => {
            let chosen = choose(&registry.url)?;
            let chosen = chosen.trim_end_matches('/');
            // A row naming this repository itself wins; a row that only
            // knows the root (the whole url collapsed onto it, or nothing
            // matched) keeps the repository's path.
            if whole == url || whole.trim_end_matches('/') == chosen {
                Ok(format!("{chosen}/{repository}"))
            } else {
                Ok(whole)
            }
        }
        _ => Ok(whole),
    }
}

/// The source's host is excluded by source policy: the run stops.
#[derive(Debug, thiserror::Error)]
#[error(
    "source policy excludes the host of `{url}`; nothing was fetched from it \
     (violates spec://org.vibevm.core/vibevm/common/PROP-073#HTTP-ADMISSION-LAW; \
     fix: mirror while the source's hosting is not in `exclude_hosting`)"
)]
pub(super) struct PolicyRefused {
    url: String,
}

/// Where a fetched version's Git objects are.
pub(crate) struct GitOrigin {
    pub store: PathBuf,
    pub reference: String,
    pub repository: String,
    pub repo: String,
    pub facts: git_source::TagFacts,
}

fn tombstone_admitted(selection: &Selection, name: &str) -> bool {
    selection.packages().is_empty() || selection.packages().iter().any(|(_, n)| n == name)
}

/// The source's yanks (of versions the target will hold) and its
/// tombstones, as journal facts for the target. The batch drops the ones
/// the target already reflects.
pub(super) fn carried_facts(
    listing: &SourceListing,
    planned: &[Planned],
    source_name: &str,
) -> Vec<Event> {
    let mut facts = Vec::new();
    for p in planned {
        if !p.version.yanked || matches!(p.decision, Decision::Alarm(_)) {
            continue;
        }
        facts.push(Event::Yanked {
            group: p.version.group.clone(),
            name: p.version.name.clone(),
            version: p.version.version.clone(),
            reason: format!("yanked in the source registry `{source_name}`"),
        });
    }
    for (name, reason, superseded_by) in &listing.tombstones {
        facts.push(Event::Buried {
            name: name.clone(),
            reason: reason.clone(),
            superseded_by: superseded_by.clone(),
        });
    }
    facts
}

#[cfg(test)]
impl SourceVersion {
    /// A per-package version as an index lists it.
    pub(super) fn listed(
        group: Group,
        name: &str,
        version: semver::Version,
        hash: Option<&str>,
        frozen: bool,
    ) -> Self {
        SourceVersion {
            group,
            name: name.into(),
            version,
            content_hash: hash.map(str::to_owned),
            frozen,
            yanked: false,
            git: None,
            layout: None,
        }
    }
}

#[cfg(test)]
mod remap_tests {
    use super::*;

    fn registry(url: &str) -> RegistrySection {
        toml::from_str(&format!("name = 'src'\nurl = '{url}'\n")).unwrap()
    }

    #[test]
    #[specmark::spec(
        verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror"
    )]
    fn a_mirror_row_keeps_the_repository_under_the_mirrored_root() {
        let source = registry("https://github.com/vibespecs");
        let mirrors: Vec<MirrorSection> = vec![
            toml::from_str("of = 'src'\nurl = 'https://git.corp.example/vibespecs'\n").unwrap(),
        ];
        let repository = "https://github.com/vibespecs/org.vibevm.world.wal.git";
        let chosen = remapped(&source, &mirrors, repository).unwrap();
        assert_eq!(
            chosen.trim_end_matches('/'),
            "https://git.corp.example/vibespecs/org.vibevm.world.wal.git"
        );
        // Without a mirror the repository is its own url.
        assert_eq!(remapped(&source, &[], repository).unwrap(), repository);
    }
}
