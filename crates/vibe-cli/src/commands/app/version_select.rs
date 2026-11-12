//! `[versions] select` applied to one application resolution (PROP-075
//! `#VERSIONS-SELECT`, PROP-066 `#drops`).
//!
//! The policy decides nothing here — [`vibe_core::version_select`] does —
//! and the drop is probed nowhere else: this module is only the join. It
//! answers two questions for one spelling: which policy is in force and
//! which layer said so, and, when that policy is not `newest`, what the
//! requirement should be **narrowed to** before the registries resolve it.
//!
//! **Narrowing, not a second selector.** The policy never reaches into a
//! registry's pick: it rewrites `org.vibevm.node/node@22` into
//! `org.vibevm.node/node@=22.23.4` and lets the ordinary resolution run,
//! exactly as a channel is lowered to an exact pin before anything
//! selects (`#CHANNEL-SYNTAX`). One chokepoint
//! ([`super::global::resolve_package_with_operation`]) therefore covers
//! `install -g`, `update -g`, `app plan`, a version file's partial value,
//! `describe`, `migrate`, the portable surfaces and every `requires`
//! entry of PROP-065 `#app-dependencies` — and nvm, which calls those
//! commands — without a single registry knowing the policy exists.
//! `app use` and `app env --use` need nothing: they choose among the
//! versions this home has **installed**, where every candidate is
//! available by the definition below, so no policy can move their answer.
//!
//! **What `newest` costs: nothing.** The default returns before a domain
//! is read, a drop is listed or the home's index is opened. The two other
//! policies pay for one domain read and one `in drop` probe per
//! resolution, which is the same skeleton-stage budget
//! `vibe app versions` holds itself to (`#TEMPLATE-TWO-STAGE`): no digest
//! document, no render, no byte of a payload read.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use vibe_app::{Domains, SettingsRoot};
use vibe_core::user_config::UserConfig;
use vibe_core::version_select::{Candidate, Selection, VersionSelect, admitted, select as decide};
use vibe_core::{Group, PackageRef};

use super::versions::RegistryDomains;

// The environment variable that overrides every file (`PROP-075`
// `#VERSIONS-SELECT`) is read at the composition root, with the
// invocation's other `VIBE_*` posture variables
// (`crate::output::env_versions_select`); this module only parses what it
// hands over, so nothing below dispatch touches the ambient environment.

/// Which layer decided the policy — what `vibe show config` and
/// `vibe self apply --show` print beside the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// `VIBE_VERSIONS_SELECT` in the live environment, or defaulted from
    /// `config.toml` `[env]`.
    Environment,
    /// `[versions] select` in the user's own `config.toml`.
    UserConfig,
    /// `[versions] select` in the applied site profile.
    SiteProfile,
    /// Nothing declared it: `newest`.
    BuiltIn,
}

impl Origin {
    /// The provenance word, the same one every other layered value prints.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Origin::Environment => "env",
            Origin::UserConfig => "config.toml",
            Origin::SiteProfile => "site profile",
            Origin::BuiltIn => "default",
        }
    }
}

/// The policy in force and the layer that decided it.
///
/// The ladder is the one every other user-config value walks
/// (`PROP-019#site-profile` `##SITE-PROFILE-LAYER`): the environment
/// first, then the user's own `config.toml`, then the applied site
/// profile, then the built-in `newest`. A site profile therefore sets a
/// closed network's posture once for every developer who applies it, and
/// a developer still overrides it on their own machine.
///
/// A misspelled policy **refuses** rather than falling through to
/// `newest`: the value decides which version gets installed, and a typo
/// that silently restored the default would be the one failure mode a
/// closed network cannot afford to debug.
pub(crate) fn effective() -> Result<(VersionSelect, Origin)> {
    // The user's own file and the site profile are read as two layers
    // rather than through the merged `UserConfig::load`, because the
    // answer has to name which of them decided.
    let own = UserConfig::load_own()?;
    let site = vibe_core::site_profile::installed()?;
    effective_from(&own, site.as_ref().map(|site| &site.profile))
}

/// [`effective`] over layers the caller has already read — what
/// `vibe self apply --show` and `--dry-run` need, because the profile a
/// dry run reports on is not installed in this process yet.
pub(crate) fn effective_from(
    own: &UserConfig,
    site: Option<&vibe_core::site_profile::SiteProfile>,
) -> Result<(VersionSelect, Origin)> {
    if let Some(raw) = crate::output::env_versions_select() {
        let policy = raw.parse::<VersionSelect>().map_err(|error| {
            anyhow::anyhow!(
                "`{}={raw}` is not a version-selection policy: {error}",
                crate::output::VERSIONS_SELECT_VAR
            )
        })?;
        return Ok((policy, Origin::Environment));
    }
    if let Some(policy) = own.versions.select {
        return Ok((policy, Origin::UserConfig));
    }
    if let Some(policy) = site.and_then(|site| site.versions.select) {
        return Ok((policy, Origin::SiteProfile));
    }
    Ok((VersionSelect::Newest, Origin::BuiltIn))
}

/// What the policy decided for one spelling.
pub(crate) enum Narrowed {
    /// Resolve `spelling` unchanged.
    AsWritten,
    /// Resolve this spelling instead; `note` is the one line printed so
    /// the operator is never silently moved off the newest version.
    Pinned { spelling: String, note: String },
}

/// `spelling` with its requirement narrowed to what `[versions] select`
/// admits, or a refusal under `drop-only`.
///
/// Returns [`Narrowed::AsWritten`] — before anything is read — for the
/// cases no policy touches: `newest`, an exact pin (`=X.Y.Z`), an
/// unqualified or unparsable spelling (the resolver's own refusal is the
/// better one), and `prefer-drop` with no drop configured.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT")]
pub(crate) fn narrow(
    settings: &SettingsRoot,
    registry: Option<&Path>,
    embedded: Option<&Path>,
    spelling: &str,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<Narrowed> {
    let (policy, _) = effective()?;
    if policy.is_newest() {
        return Ok(Narrowed::AsWritten);
    }
    narrow_under(
        policy, settings, registry, embedded, spelling, offline, operation,
    )
}

/// [`narrow`] with the policy already decided — the body, separated so
/// the `newest` fast path above reads as the one line it is.
fn narrow_under(
    policy: VersionSelect,
    settings: &SettingsRoot,
    registry: Option<&Path>,
    embedded: Option<&Path>,
    spelling: &str,
    offline: bool,
    operation: &vibe_package_source::SourceOperation,
) -> Result<Narrowed> {
    // An unparsable or unqualified spelling is the resolver's refusal to
    // make, by name, and it is a better message than anything this module
    // could say about a policy.
    let Ok(requested) = PackageRef::parse(spelling) else {
        return Ok(Narrowed::AsWritten);
    };
    let Some(group) = requested.group.clone() else {
        return Ok(Narrowed::AsWritten);
    };
    // **An exact pin is never redirected.** A person who named one
    // version meant that version; under `drop-only` it resolves as before
    // and refuses at the fetch, where the digest is known and the refusal
    // can name the file.
    if requested.version.is_exact_pin() {
        return Ok(Narrowed::AsWritten);
    }
    let name = requested.name.as_str();
    let coordinate = format!("{group}/{name}");

    let drops: Vec<PathBuf> = vibe_core::remap::active()
        .package_drops()
        .iter()
        .map(|path| path.to_path_buf())
        .collect();
    if drops.is_empty() {
        // `prefer-drop` has no drop to prefer, so it is `newest` — a
        // machine whose drop has not been declared yet still installs.
        // `drop-only` cannot honour "only what the drop holds" when there
        // is no drop at all, and says so rather than resolving the newest
        // version the policy exists to refuse.
        return match policy {
            VersionSelect::DropOnly => Err(no_drop(&coordinate, spelling)),
            _ => Ok(Narrowed::AsWritten),
        };
    }

    let domains = RegistryDomains::new(
        settings.clone(),
        registry.map(Path::to_path_buf),
        embedded.map(Path::to_path_buf),
        offline,
        operation.clone(),
        drops.clone(),
    );
    // A domain this chain cannot read is not a policy refusal: the
    // resolution that follows reports it properly, with the registry's own
    // words.
    let Ok(domain) = domains.domain(group.as_str(), name) else {
        return Ok(Narrowed::AsWritten);
    };
    let all: Vec<semver::Version> = domain
        .versions
        .iter()
        .map(|found| found.version.clone())
        .collect();
    // A channel names a set only the template knows; the domain read
    // already computed the table, so the members come from there and
    // `admitted` needs no second source (`#CHANNEL-SYNTAX`).
    let members: Option<Vec<semver::Version>> = requested.version.channel().map(|channel| {
        reserved_members(channel.as_str(), &all).unwrap_or_else(|| {
            domain
                .channels
                .get(channel.as_str())
                .cloned()
                .map_or_else(Vec::new, |set| set.into_iter().collect())
        })
    });
    let installed = installed_versions(settings, &group, name);
    // The two grounds are kept apart only so the note can say which one
    // applied: a version that is here because it is **installed** must not
    // be announced as "in drop", which would be a sentence about a file
    // that is not there. The policy itself sees one bit.
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut in_drop: BTreeSet<semver::Version> = BTreeSet::new();
    for version in admitted(&requested.version, &all, members.as_deref()) {
        let marked = domains.in_drop(group.as_str(), name, &version).is_some();
        if marked {
            in_drop.insert(version.clone());
        }
        candidates.push(Candidate {
            available: marked || installed.contains(&version),
            version,
        });
    }

    match decide(policy, &candidates) {
        Selection::Unchanged => Ok(Narrowed::AsWritten),
        Selection::Redirect {
            version,
            instead_of,
        } => Ok(Narrowed::Pinned {
            spelling: format!("{coordinate}@={version}"),
            note: redirect_note(spelling, &version, &instead_of, in_drop.contains(&version)),
        }),
        Selection::Refuse { newest } => Err(nothing_in_drop(
            &coordinate,
            spelling,
            newest.as_ref(),
            &drops,
            &domains,
            group.as_str(),
            name,
        )),
    }
}

/// The one line a redirected resolution prints: both versions, and which
/// ground made the picked one available.
///
/// A version that is here because this home **installed** it is announced
/// as `installed`, never as `in drop` — that would be a sentence about a
/// file which is not there, and the whole point of the note is that the
/// operator can act on it.
fn redirect_note(
    spelling: &str,
    version: &semver::Version,
    instead_of: &semver::Version,
    in_drop: bool,
) -> String {
    let ground = if in_drop { "in drop" } else { "installed" };
    format!("{spelling} → {version} ({ground}; {instead_of} is newer but not in the drop)")
}

/// The members of the two channel names every package has whether it
/// declares them or not (`#CHANNEL-SYNTAX`), out of the domain itself;
/// `None` for a declared channel, whose members only the template knows.
fn reserved_members(channel: &str, all: &[semver::Version]) -> Option<Vec<semver::Version>> {
    match channel {
        "latest" => Some(all.to_vec()),
        "stable" => Some(
            all.iter()
                .filter(|version| version.pre.is_empty())
                .cloned()
                .collect(),
        ),
        _ => None,
    }
}

/// Every version of `<group>/<name>` this home holds, superseded ones
/// excluded — a version already installed downloads nothing, so it counts
/// as available beside the drop's own contents.
fn installed_versions(settings: &SettingsRoot, group: &Group, name: &str) -> Vec<semver::Version> {
    let Ok(index) = vibe_app::Index::load(settings) else {
        return Vec::new();
    };
    index
        .installations
        .values()
        .filter(|record| {
            record.package.group == group.as_str()
                && record.package.name == name
                && record.lifecycle.superseded_by.is_none()
        })
        .filter_map(|record| semver::Version::parse(&record.package.version).ok())
        .collect()
}

/// `drop-only` with no `[artifacts] packages` drop declared at all.
fn no_drop(coordinate: &str, spelling: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "`[versions] select = \"drop-only\"` admits only versions an artifact drop already \
         holds, and this home declares no drop, so `{spelling}` has no candidate at all \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT; \
          fix: declare the directory in `~/.vibe/config.toml` as `[artifacts]` `packages = \
         \"<dir>\"` — `vibe artifacts where {coordinate}` prints the file names and paths it \
         is filled with — or set `[versions] select = \"prefer-drop\"` to fall back to the \
         newest version instead)"
    )
}

/// `drop-only`, a drop is declared, and nothing the requirement admits is
/// in it. The refusal names the version `newest` would have landed on and
/// the exact path its payload belongs at, because that is the whole of
/// what the operator has to do next.
fn nothing_in_drop(
    coordinate: &str,
    spelling: &str,
    newest: Option<&semver::Version>,
    drops: &[PathBuf],
    domains: &RegistryDomains,
    group: &str,
    name: &str,
) -> anyhow::Error {
    let where_to_put = newest.and_then(|newest| domains.drop_target(group, name, newest));
    anyhow::anyhow!(
        "{}",
        nothing_in_drop_message(coordinate, spelling, newest, drops, where_to_put)
    )
}

/// The text of [`nothing_in_drop`], with the one fact that needs a
/// registry — where the payload belongs — passed in, so the message is
/// pinned by a test and built in exactly one place.
fn nothing_in_drop_message(
    coordinate: &str,
    spelling: &str,
    newest: Option<&semver::Version>,
    drops: &[PathBuf],
    where_to_put: Option<String>,
) -> String {
    let Some(newest) = newest else {
        // The requirement matched no version at all, which is the
        // registry's own refusal; resolving says it better than a policy
        // can, so this names the requirement and nothing else.
        return format!(
            "`{spelling}` matches no version of `{coordinate}` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT; \
              fix: run `vibe app versions {coordinate}` for the versions there are)"
        );
    };
    // With no file name a skeleton can settle, the drop's own directory
    // is still the answer to "where do I look" — never nothing.
    let where_to_put = where_to_put.unwrap_or_else(|| {
        drops
            .first()
            .map(|drop| vibe_core::remap::drop::shown_path(drop))
            .unwrap_or_default()
    });
    format!(
        "`[versions] select = \"drop-only\"` admits only versions an artifact drop already \
         holds, and the drop holds none that satisfy `{spelling}`; the newest version it \
         matches is {newest} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-SELECT; \
          fix: put its payload at `{where_to_put}` — `vibe artifacts where \
         {coordinate}@={newest}` prints every file and digest — or set `[versions] select = \
         \"prefer-drop\"` to install the newest version the drop already holds)"
    )
}

#[cfg(test)]
#[path = "version_select/tests.rs"]
mod tests;
