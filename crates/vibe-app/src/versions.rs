//! One listing over an application package's whole version domain
//! (PROP-075 `#VERSIONS-LIST`, `#VERSIONS-LIST-OUTPUT`).
//!
//! The listing is the join of two sides that know nothing of each other.
//! The **domain** — every version a package has, where each came from and
//! which channels hold it — is whatever a registry answers, and it enters
//! through the [`Domains`] seam so this crate reaches no registry, no
//! index server and no socket. The **local state** is this home's index:
//! which versions are installed, which one the slot resolves to, which
//! are hidden or held, and whether the slot is off.
//!
//! Nothing here renders a template. A domain is the skeleton stage of
//! PROP-075 `#TEMPLATE-TWO-STAGE`: it costs the version list and no
//! digest document at all, which is what lets one listing cover Node's
//! 878 records.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT");

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;
use vibe_wire::generated::application_versions as wire;

use crate::error::AppError;
use crate::error_versions::VersionsError;
use crate::index::{Index, Installation};

/// The protocol tag of the `--json` answer (`AV-FMT-014`).
pub const VERSIONS_PROTOCOL: &str = "vibe-app-versions/1";

/// How many rows are shown without `--all` or `--limit`
/// (PROP-075 `#VERSIONS-LIST-OUTPUT`: Node's domain is 878 records).
pub const DEFAULT_LIMIT: usize = 20;

/// Where one version of a package comes from — the registry layer's own
/// verdict, reused rather than restated (PROP-075
/// `#TEMPLATE-CONCRETE-WINS`).
pub use vibe_registry::templates::Origin;

/// One version of a package's domain with its origin.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST")]
pub struct DomainVersion {
    pub version: semver::Version,
    pub origin: Origin,
}

/// A template's provenance, as the `--json` answer carries it
/// (PROP-075 `#VERSIONS-LIST-OUTPUT`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
pub struct TemplateFacts {
    /// `<group>/<name>` of the template package.
    pub coordinate: String,
    /// The template's own version — a revision, never a produced version.
    pub revision: String,
    /// The version-list url the template declares, before remap.
    pub version_list_url: Option<String>,
    /// When the cached version list was fetched.
    pub fetched_at: Option<String>,
    /// A mirror snapshot's `[sync] last`, in a closed network.
    pub snapshot_synced: Option<String>,
}

/// What a registry answered about one package's whole domain.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST")]
pub struct Domain {
    /// Every version, in any order; the listing sorts them.
    pub versions: Vec<DomainVersion>,
    /// The resolved channel table: channel name to its members.
    pub channels: BTreeMap<String, BTreeSet<semver::Version>>,
    /// The template provenance, for a package that carries one.
    pub template: Option<TemplateFacts>,
    /// What the answer reported and did not fail on.
    pub notices: Vec<String>,
}

impl Domain {
    /// A domain of concretely published versions alone — the answer for
    /// every package that carries no template.
    pub fn of_published(versions: impl IntoIterator<Item = semver::Version>) -> Self {
        Domain {
            versions: versions
                .into_iter()
                .map(|version| DomainVersion {
                    version,
                    origin: Origin::Published,
                })
                .collect(),
            ..Domain::default()
        }
    }
}

/// The seam by which a registry's version domain enters the listing.
///
/// An implementor that knows nothing about a package answers an empty
/// domain, which is the honest answer for a home listing what it holds
/// with no registry configured.
///
/// ```
/// use vibe_app::{Domain, Domains};
///
/// struct Two;
/// impl Domains for Two {
///     fn domain(&self, _group: &str, _name: &str) -> Result<Domain, String> {
///         Ok(Domain::of_published([
///             "1.0.0".parse().unwrap(),
///             "2.0.0".parse().unwrap(),
///         ]))
///     }
/// }
/// let domains: &dyn Domains = &Two;
/// assert_eq!(domains.domain("org.example", "tool").unwrap().versions.len(), 2);
/// ```
pub trait Domains {
    /// Every version of `<group>/<name>`, with each version's origin and
    /// the channel table the package's rules produced.
    fn domain(&self, group: &str, name: &str) -> Result<Domain, String>;
}

/// What `vibe app versions` was asked for.
#[derive(Debug, Clone, Default)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
pub struct VersionsRequest<'a> {
    /// The `PACKAGE|SLOT` argument as it was written.
    pub target: &'a str,
    /// `--channel NAME`: keep only the versions that channel holds.
    pub channel: Option<&'a str>,
    /// `--installed`: only the versions this home holds, and no registry.
    pub installed: bool,
    /// `--limit N`; `None` is [`DEFAULT_LIMIT`].
    pub limit: Option<usize>,
    /// `--all`: every row, however many.
    pub all: bool,
}

/// One package the listing answers under its own heading.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Heading {
    group: String,
    name: String,
    slot: Option<String>,
}

impl Heading {
    fn coordinate(&self) -> String {
        format!("{}/{}", self.group, self.name)
    }
}

/// The whole listing: one heading per package, with the slot's trailer.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT",
    r = 1
)]
pub fn versions_listing(
    index: &Index,
    request: &VersionsRequest<'_>,
    domains: &dyn Domains,
) -> Result<wire::ApplicationVersions, AppError> {
    let (slot, headings) = resolve_target(index, request.target)?;
    let mut packages = Vec::with_capacity(headings.len());
    for heading in &headings {
        packages.push(one_package(index, heading, request, domains)?);
    }
    let slot_state = slot.as_deref().and_then(|slot| index.slots.get(slot));
    Ok(wire::ApplicationVersions {
        protocol: VERSIONS_PROTOCOL.to_owned(),
        target: request.target.to_owned(),
        slot: slot.clone(),
        default: slot_state.and_then(|state| state.default.clone()),
        off: slot_state.is_some_and(|state| state.off),
        // The snapshot a closed network reads the domain from is one per
        // template; the trailer names it when every heading agrees,
        // which is the only case a single line can speak for.
        snapshot_synced: snapshot_synced(&packages),
        packages,
        notices: Vec::new(),
    })
}

/// The one `[sync] last` the trailer may name: the snapshot time every
/// heading that has one reports, or `None` when they disagree.
fn snapshot_synced(packages: &[wire::Package]) -> Option<String> {
    let mut synced: BTreeSet<&str> = BTreeSet::new();
    for package in packages {
        let Some(template) = &package.template else {
            continue;
        };
        synced.insert(template.snapshot_synced.as_deref()?);
    }
    (synced.len() == 1).then(|| synced.iter().next().copied().unwrap_or_default().to_owned())
}

/// What the `PACKAGE|SLOT` argument names: the slot, and one heading per
/// package to list.
///
/// A `<group>/<name>` coordinate is itself. A bare word is a slot of this
/// home when one is recorded — a slot with providers from several packages
/// lists each package under its own heading. Otherwise it is matched
/// against the installed packages' names, and a word matching several is
/// the refusal that names the spellings, never a guess.
fn resolve_target(
    index: &Index,
    target: &str,
) -> Result<(Option<String>, Vec<Heading>), VersionsError> {
    let target = target.trim();
    if let Some((group, name)) = target.split_once('/') {
        let name = name.split(['@', ':']).next().unwrap_or(name);
        let slot = slot_of(index, group, name);
        return Ok((
            slot.clone(),
            vec![Heading {
                group: group.to_owned(),
                name: name.to_owned(),
                slot,
            }],
        ));
    }
    let providers = providers_of_slot(index, target);
    if !providers.is_empty() || index.slots.contains_key(target) {
        return Ok((Some(target.to_owned()), providers));
    }
    let named: Vec<Heading> = index
        .installations
        .values()
        .filter(|record| record.package.name == target)
        .map(|record| Heading {
            group: record.package.group.clone(),
            name: record.package.name.clone(),
            slot: Some(record.slot.clone()),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if named.len() > 1 {
        return Err(VersionsError::AmbiguousTarget {
            target: target.to_owned(),
            spellings: join(named.iter().map(Heading::coordinate)),
        });
    }
    // Moved out rather than indexed, so the one-heading case needs no
    // `expect` and the empty case is the refusal itself.
    let Some(heading) = named.into_iter().next() else {
        return Err(VersionsError::UnknownTarget {
            target: target.to_owned(),
            slots: join(index.slots.keys().map(String::as_str)),
        });
    };
    let slot = heading.slot.clone();
    Ok((slot, vec![heading]))
}

/// Every distinct package providing `slot`, in coordinate order.
fn providers_of_slot(index: &Index, slot: &str) -> Vec<Heading> {
    index
        .installations
        .values()
        .filter(|record| record.slot == slot)
        .map(|record| Heading {
            group: record.package.group.clone(),
            name: record.package.name.clone(),
            slot: Some(record.slot.clone()),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The slot `<group>/<name>` provides, as this home's installations
/// record it.
fn slot_of(index: &Index, group: &str, name: &str) -> Option<String> {
    index
        .installations
        .values()
        .find(|record| record.package.group == group && record.package.name == name)
        .map(|record| record.slot.clone())
}

fn join<S: AsRef<str>>(items: impl IntoIterator<Item = S>) -> String {
    let joined: Vec<String> = items
        .into_iter()
        .map(|item| format!("`{}`", item.as_ref()))
        .collect();
    if joined.is_empty() {
        return "(none)".to_owned();
    }
    joined.join(", ")
}

/// One heading's rows: the domain joined with this home's installations,
/// narrowed, sorted newest first and cut to the limit.
fn one_package(
    index: &Index,
    heading: &Heading,
    request: &VersionsRequest<'_>,
    domains: &dyn Domains,
) -> Result<wire::Package, VersionsError> {
    let coordinate = heading.coordinate();
    let domain = domains
        .domain(&heading.group, &heading.name)
        .map_err(|reason| VersionsError::Domain {
            package: coordinate.clone(),
            reason,
        })?;
    let installed = installed_of(index, heading);

    // The union: every version the domain carries plus every version this
    // home installed, because an installation whose registry row is gone
    // is still installed and must still be listed.
    let mut origins: BTreeMap<semver::Version, Origin> = BTreeMap::new();
    for row in &domain.versions {
        origins.insert(row.version.clone(), row.origin);
    }
    for version in installed.keys() {
        origins.entry(version.clone()).or_insert(Origin::Published);
    }

    let channel = match request.channel {
        Some(name) if !domain.channels.contains_key(name) => {
            return Err(VersionsError::UnknownChannel {
                package: coordinate,
                channel: name.to_owned(),
                channels: join(domain.channels.keys().map(String::as_str)),
            });
        }
        other => other,
    };
    let selected: Vec<(semver::Version, Origin)> = origins
        .into_iter()
        .rev()
        .filter(|(version, _)| !request.installed || installed.contains_key(version))
        .filter(|(version, _)| match channel {
            Some(name) => domain
                .channels
                .get(name)
                .is_some_and(|members| members.contains(version)),
            None => true,
        })
        .collect();

    let total = selected.len();
    let shown = if request.all {
        total
    } else {
        request.limit.unwrap_or(DEFAULT_LIMIT).min(total)
    };
    let versions = selected
        .iter()
        .take(shown)
        .map(|(version, origin)| row(heading, version, *origin, &domain, &installed, index))
        .collect();
    Ok(wire::Package {
        package: coordinate,
        slot: heading.slot.clone(),
        total: total as u32,
        shown: shown as u32,
        more: (total - shown) as u32,
        versions,
        channels: domain
            .channels
            .iter()
            .map(|(name, members)| {
                (
                    name.clone(),
                    members
                        .iter()
                        .rev()
                        .map(semver::Version::to_string)
                        .collect(),
                )
            })
            .collect(),
        template: domain.template.as_ref().map(template_wire),
        notices: domain.notices.clone(),
    })
}

/// This home's installations of one package, by parsed version.
fn installed_of<'i>(
    index: &'i Index,
    heading: &Heading,
) -> BTreeMap<semver::Version, (&'i String, &'i Installation)> {
    index
        .installations
        .iter()
        .filter(|(_, record)| {
            record.package.group == heading.group && record.package.name == heading.name
        })
        .filter_map(|(key, record)| {
            semver::Version::parse(&record.package.version)
                .ok()
                .map(|version| (version, (key, record)))
        })
        .collect()
}

fn row(
    heading: &Heading,
    version: &semver::Version,
    origin: Origin,
    domain: &Domain,
    installed: &BTreeMap<semver::Version, (&String, &Installation)>,
    index: &Index,
) -> wire::Row {
    let found = installed.get(version);
    wire::Row {
        version: version.to_string(),
        origin: match origin {
            Origin::Template => wire::Origin::Template,
            Origin::Published => wire::Origin::Published,
            Origin::Overrides => wire::Origin::Overrides,
        },
        state: match found {
            None => wire::State::Available,
            Some((key, record)) => state_of(index, heading, key, record),
        },
        channels: domain
            .channels
            .iter()
            .filter(|(_, members)| members.contains(version))
            .map(|(name, _)| name.clone())
            .collect(),
        key: found.map(|(key, _)| (*key).clone()),
    }
}

/// One installed version's local state (PROP-075
/// `#VERSIONS-LIST-OUTPUT`). The slot's default outranks every other
/// mark, because it is the one a bare command reaches.
fn state_of(index: &Index, heading: &Heading, key: &str, record: &Installation) -> wire::State {
    let default = heading
        .slot
        .as_deref()
        .and_then(|slot| index.slots.get(slot))
        .and_then(|state| state.default.as_deref());
    if default == Some(key) {
        return wire::State::InstalledDefault;
    }
    if record.lifecycle.hidden {
        return wire::State::InstalledHidden;
    }
    if record.lifecycle.held {
        return wire::State::InstalledHeld;
    }
    wire::State::Installed
}

fn template_wire(facts: &TemplateFacts) -> wire::Template {
    wire::Template {
        coordinate: facts.coordinate.clone(),
        revision: facts.revision.clone(),
        version_list_url: facts.version_list_url.clone(),
        fetched_at: facts.fetched_at.clone(),
        snapshot_synced: facts.snapshot_synced.clone(),
    }
}

#[cfg(test)]
#[path = "versions/tests.rs"]
mod tests;
