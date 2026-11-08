//! The mirror's source side: list a registry's versions and fetch one,
//! always through the operation's controlled transport, source policy and
//! remap Locator.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use vibe_core::Group;
use vibe_core::manifest::{MirrorSection, RegistrySection, RegistryType};
use vibe_index::journal::Event;
use vibe_publish::layout_publish::Provenance;
use vibe_registry::{IndexAuth, IndexClient, IndexUrlResolution, IndexUrlSource, ProbeOutcome};
use vibe_wire::generated::registry_mirror_report::Listing as ListingKind;

use super::git_source::{self, GitStore};
use super::plan::{Decision, Planned};
use super::{Fetched, Filters};
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
        })
    }

    pub(super) fn list(&self, filters: &Filters) -> Result<SourceListing> {
        match self {
            SourceReader::Layout(opened) => {
                let (index, _) =
                    vibe_index::layout::load_or_project(opened.root(), opened.identity())?;
                let versions = index
                    .iter_versions()
                    .filter(|e| filters.admits(e.group.as_str(), &e.name))
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
                    .filter(|(name, _)| tombstone_admitted(filters, name))
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
                    .filter(|e| filters.admits(e.group.as_str(), &e.name))
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
                        if living.contains(name) || !tombstone_admitted(filters, name) {
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
                if filters.packages().is_empty() {
                    bail!(
                        "registry `{}` has no index that answers, so its packages cannot be \
                         listed \
                         (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror; \
                         fix: name the packages to copy with `--package <group>/<name>` — their \
                         versions are then read from their tags — or publish an index for the \
                         registry)",
                        registry.name
                    );
                }
                let mut versions = Vec::new();
                for (group, name) in filters.packages() {
                    let group = Group::parse(group)?;
                    let repository = format!(
                        "{}/{}.git",
                        registry.url.trim_end_matches('/'),
                        registry.naming.repo_name(None, &group, name)?
                    );
                    let listed = resolver
                        .list_versions(&group, name)
                        .map_err(layout_root::refusal_first)?;
                    for version in listed {
                        let tag = format!("v{version}");
                        versions.push(SourceVersion {
                            group: group.clone(),
                            name: name.clone(),
                            version,
                            content_hash: None,
                            frozen: false,
                            yanked: false,
                            git: Some((repository.clone(), tag)),
                            layout: None,
                        });
                    }
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
        let url = remapped(registry, mirrors, repository)?;
        if let Some(host) = git_source::url_host(&url)
            && denied.contains(&host)
        {
            return Err(PolicyRefused {
                url: vibe_publish::git_publish::redact_credentials(&url),
            }
            .into());
        }
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
        let url = remapped(registry, mirrors, repository)?;
        if let Some(host) = git_source::url_host(&url)
            && denied.contains(&host)
        {
            return Err(PolicyRefused {
                url: vibe_publish::git_publish::redact_credentials(&url),
            }
            .into());
        }
        store.bundle(&url, &repo_key(repository), out)
    }
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
/// registry's `[[mirror]]` tables), as the resolver's clone chain does.
fn remapped(registry: &RegistrySection, mirrors: &[MirrorSection], url: &str) -> Result<String> {
    use vibe_core::remap::{Artifact, ArtifactKind, Layer, rows_from_registry_tables};
    let locator = vibe_core::remap::active().with_registry_tables(
        rows_from_registry_tables(std::slice::from_ref(registry), mirrors, &[], Layer::Local),
        vec![(registry.name.clone(), registry.url.clone())],
    );
    Ok(locator.choose_url(&Artifact::new(ArtifactKind::RegistryClone).with_url(url))?)
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

fn tombstone_admitted(filters: &Filters, name: &str) -> bool {
    filters.packages().is_empty() || filters.packages().iter().any(|(_, n)| n == name)
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

/// The source's index, when one answers (PROP-005 `#form-factor` ladder).
fn probe_index(
    reg: &RegistrySection,
    operation: &vibe_package_source::SourceOperation,
) -> Result<Option<IndexClient>> {
    let IndexUrlResolution::Url { base, source } = vibe_registry::resolve_index_url(reg) else {
        return Ok(None);
    };
    if source == IndexUrlSource::Default
        && vibe_registry::resolve_registry_auth(
            vibe_registry::RegistryAuthInput::SelectedRegistry(reg),
        )?
        .is_ssh_source()
    {
        return Ok(None);
    }
    let probe = match operation.http_admission()? {
        Some(binding) => {
            binding.check_address(&base)?;
            IndexClient::probe_with_admission(&base, IndexAuth::for_registry(reg, &base)?, binding)
                .map_err(refusal_first)?
        }
        None => IndexClient::probe(&base, IndexAuth::for_registry(reg, &base)?),
    };
    match probe {
        ProbeOutcome::Found(client) => Ok(Some(*client)),
        ProbeOutcome::Absent if source == IndexUrlSource::Default => Ok(None),
        ProbeOutcome::Absent => bail!(
            "the index configured for registry `{}` at `{base}` does not answer \
             (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; \
             fix: correct `index_url`, or set it to `none` and mirror with `--package`)",
            reg.name
        ),
        ProbeOutcome::Refused { reason } => bail!(
            "the index of registry `{}` refused the probe: {reason}",
            reg.name
        ),
    }
}

fn refusal_first(error: vibe_registry::IndexError) -> anyhow::Error {
    match error {
        vibe_registry::IndexError::ContactRefused(refusal) => refusal.into(),
        other => other.into(),
    }
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
