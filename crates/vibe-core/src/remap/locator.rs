//! The one resolution point (`PROP-066#locator`).
//!
//! Every crate that fetches something asks the same question here —
//! "where may these bytes come from?" — and gets back an ordered candidate
//! list or a refusal. Nothing in this file opens a socket, reads a file or
//! knows what a caller will do with a candidate: it is a pure function
//! from the artifact's identity and the loaded rows to the places to try,
//! which is what makes a surprising redirection one `remap explain` away
//! instead of a debugging session across six crates.
//!
//! The declared origin is the last candidate, never the first, and it is
//! dropped only on a refusal. That ordering is the whole behavioural
//! promise: a remap can make a fetch succeed where it would have failed,
//! or fail where it would have succeeded, and — because the digest check
//! above it never moves — it can never change what gets installed.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#locator");

use specmark::spec;

use super::artifact::{Artifact, Candidate, Explanation, Resolution, RowMatch, RowRef};
use super::layers::{Layer, LoadedSource};
use super::row::{RemapKey, RemapRow, RemapScope, RemapTarget, UnmatchedPolicy};

/// The resolution point built once per invocation from the layers
/// (`PROP-066#locator`).
///
/// ```
/// use vibe_core::remap::{Artifact, ArtifactKind, Locator, Resolution, RowDecl};
/// use vibe_core::remap::{Layer, LoadedSource, Provenance, UnmatchedPolicy};
///
/// let decl: RowDecl =
///     toml::from_str("prefix = \"https://github.com/\"\nurl = \"https://nexus/gh/\"").unwrap();
/// let row = decl.into_row(Provenance::new(Layer::Local, "settings.toml", 0)).unwrap();
/// let locator = Locator::from_sources(
///     vec![LoadedSource {
///         layer: Layer::Local,
///         source: "settings.toml".into(),
///         rows: vec![row],
///         policy: UnmatchedPolicy::Allow,
///     }],
///     Vec::new(),
/// );
/// let artifact =
///     Artifact::new(ArtifactKind::RawFile).with_url("https://github.com/vibespecs/index");
/// let Resolution::Candidates(candidates) = locator.locate(&artifact) else {
///     panic!("no deny row here")
/// };
/// // The rewrite is first, the declared origin last.
/// assert_eq!(candidates[0].target.display(), "https://nexus/gh/vibespecs/index");
/// assert_eq!(candidates[1].target.display(), "https://github.com/vibespecs/index");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#locator")]
pub struct Locator {
    rows: Vec<RemapRow>,
    // Constructor-owned parallel order; public provenance stays source-local.
    source_ordinals: Vec<usize>,
    registries: Vec<(String, String)>,
    policy: UnmatchedPolicy,
    sources: Vec<(Layer, String, UnmatchedPolicy)>,
}

#[derive(Clone, Copy)]
enum RegistryBinding<'a> {
    Current,
    Held(Option<&'a str>),
}
impl Locator {
    /// Build the Locator from the loaded sources and the declared
    /// registries (`name`, `url`), which is what a `registry = …` row is
    /// matched through. The strictest `[policy]` of all sources applies
    /// (`PROP-066#policy`).
    pub fn from_sources(sources: Vec<LoadedSource>, registries: Vec<(String, String)>) -> Self {
        let mut rows = Vec::new();
        let mut source_ordinals = Vec::new();
        let mut policy = UnmatchedPolicy::Allow;
        let mut seen = Vec::new();
        for (ordinal, source) in sources.into_iter().enumerate() {
            policy = policy.strictest(source.policy);
            seen.push((source.layer, source.source, source.policy));
            for row in source.rows {
                rows.push(row);
                source_ordinals.push(ordinal);
            }
        }
        debug_assert_eq!(rows.len(), source_ordinals.len());
        Locator {
            rows,
            source_ordinals,
            registries,
            policy,
            sources: seen,
        }
    }

    /// Replace only catalog-generated local rows for a newly demanded catalog.
    /// Other sources retain their order, provenance, policy and consent.
    pub fn with_registry_tables(
        &self,
        rows: Vec<RemapRow>,
        registries: Vec<(String, String)>,
    ) -> Self {
        let label = "[[mirror]] / [[override]]";
        let mut inserted = false;
        let mut sources = Vec::new();
        for (ordinal, (layer, source, policy)) in self.sources.iter().enumerate() {
            if source == label && *layer == Layer::Local {
                sources.push(LoadedSource {
                    layer: *layer,
                    source: source.clone(),
                    policy: *policy,
                    rows: rows.clone(),
                });
                inserted = true;
            } else {
                if !inserted && matches!(layer, Layer::Local | Layer::Invocation) {
                    sources.push(LoadedSource {
                        layer: Layer::Local,
                        source: label.into(),
                        policy: UnmatchedPolicy::Allow,
                        rows: rows.clone(),
                    });
                    inserted = true;
                }
                sources.push(LoadedSource {
                    layer: *layer,
                    source: source.clone(),
                    policy: *policy,
                    rows: self
                        .rows
                        .iter()
                        .zip(&self.source_ordinals)
                        .filter(|(_, owner)| **owner == ordinal)
                        .map(|(row, _)| row.clone())
                        .collect(),
                });
            }
        }
        if !inserted {
            sources.push(LoadedSource {
                layer: Layer::Local,
                source: label.into(),
                policy: UnmatchedPolicy::Allow,
                rows,
            });
        }
        Self::from_sources(sources, registries)
    }

    /// Every row the Locator holds, in load order.
    pub fn rows(&self) -> &[RemapRow] {
        &self.rows
    }

    /// The row a [`RowRef`] points at.
    pub fn row(&self, reference: &RowRef) -> Option<&RemapRow> {
        self.rows.get(reference.position)
    }

    /// The sources that contributed, in load order, with the policy each
    /// declared — the header blocks of `remap list` (`PROP-066#display`).
    pub fn sources(&self) -> &[(Layer, String, UnmatchedPolicy)] {
        &self.sources
    }

    /// The strictest `unmatched` policy of the active layers.
    pub fn policy(&self) -> UnmatchedPolicy {
        self.policy
    }

    /// Whether any row is active. A Locator with no rows and the default
    /// policy answers every artifact with its own declared origin, which
    /// is the pre-PROP-066 behaviour exactly.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.policy == UnmatchedPolicy::Allow
    }

    /// The places to try for one artifact, remapped targets first and the
    /// declared origin last, or the refusal (`PROP-066#locator`).
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#locator")]
    pub fn locate(&self, artifact: &Artifact) -> Resolution {
        self.locate_matches(artifact, self.matching(artifact))
    }
    fn locate_matches(&self, artifact: &Artifact, matched: Vec<Found>) -> Resolution {
        if matched.is_empty() {
            if self.policy == UnmatchedPolicy::Deny {
                return Resolution::Denied {
                    row: None,
                    reason: Some(
                        "no remap row matched and the active policy is `unmatched = \"deny\"`"
                            .to_owned(),
                    ),
                };
            }
            return Resolution::Candidates(self.origin_only(artifact));
        }
        let mut candidates: Vec<Candidate> = Vec::with_capacity(matched.len() + 1);
        let mut refused = false;
        for found in &matched {
            if found.target.is_deny() {
                if candidates.is_empty() {
                    return Resolution::Denied {
                        row: Some(found.row.clone()),
                        reason: found.reason.clone(),
                    };
                }
                // A `deny` further down the chain is still a refusal of
                // everything below it: falling through to the origin past
                // it would be the one thing a deny exists to prevent.
                refused = true;
                break;
            }
            if candidates
                .iter()
                .any(|existing| existing.target == found.target)
            {
                continue;
            }
            candidates.push(Candidate {
                target: found.target.clone(),
                row: Some(found.row.clone()),
            });
        }
        if !refused {
            candidates.extend(self.origin_only(artifact));
        }
        Resolution::Candidates(candidates)
    }

    /// The whole chain for one artifact: every matching row in winning
    /// order, the chosen target and the digest to be checked
    /// (`PROP-066#precedence`).
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
    pub fn explain(&self, artifact: &Artifact) -> Explanation {
        let matches = self
            .matching(artifact)
            .into_iter()
            .map(|found| RowMatch {
                row: found.row,
                key: found.key,
                target: found.target,
                scope: found.scope,
                reason: found.reason,
            })
            .collect();
        let (chosen, denied) = match self.locate(artifact) {
            Resolution::Candidates(candidates) => {
                (candidates.first().map(|c| c.target.clone()), false)
            }
            Resolution::Denied { .. } => (None, true),
        };
        Explanation {
            kind: artifact.kind,
            origin: artifact.url.clone(),
            digest: artifact.digest.clone(),
            policy: self.policy,
            matches,
            chosen,
            denied,
        }
    }

    /// The one URL to use for an artifact whose source is a single
    /// configuration value — a release base, the documentation shell's
    /// release root, an index endpoint, a git clone URL.
    ///
    /// Where a byte-stream fetch can try each candidate in turn, these
    /// callers have one slot to fill, so the winning candidate fills it
    /// and the refusal is an error.
    ///
    /// ```
    /// use vibe_core::remap::{Artifact, ArtifactKind, Locator};
    ///
    /// // With no rows, the declared base comes back untouched.
    /// let base = "https://github.com/vibevm/vibevm/releases/download";
    /// let chosen = Locator::default()
    ///     .choose_url(&Artifact::new(ArtifactKind::Distribution).with_url(base))
    ///     .unwrap();
    /// assert_eq!(chosen, base);
    /// ```
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-066#locator")]
    pub fn choose_url(&self, artifact: &Artifact) -> Result<String, RemapRefusal> {
        self.choose_candidate(artifact)
            .map(|candidate| candidate.target.display())
    }
    /// Select once while retaining the winning row's existing provenance.
    pub fn choose_candidate(&self, artifact: &Artifact) -> Result<Candidate, RemapRefusal> {
        self.chosen(self.locate(artifact), artifact)
    }
    /// A held alias comes from the lock, never today's registry table. Catalog
    /// rows are excluded; the same matcher/order/consent/policy still applies.
    pub fn choose_operator_candidate(
        &self,
        artifact: &Artifact,
        held_registry: Option<&str>,
    ) -> Result<Candidate, RemapRefusal> {
        self.chosen(
            self.locate_matches(
                artifact,
                self.matching_bound(artifact, RegistryBinding::Held(held_registry)),
            ),
            artifact,
        )
    }
    fn chosen(
        &self,
        resolution: Resolution,
        artifact: &Artifact,
    ) -> Result<Candidate, RemapRefusal> {
        match resolution {
            Resolution::Candidates(candidates) => candidates
                .into_iter()
                .next()
                .ok_or_else(|| self.refusal(None, None, artifact)),
            Resolution::Denied { row, reason } => Err(self.refusal(row, reason, artifact)),
        }
    }

    /// Build the refusal [`choose_url`](Self::choose_url) returns.
    fn refusal(
        &self,
        row: Option<RowRef>,
        reason: Option<String>,
        artifact: &Artifact,
    ) -> RemapRefusal {
        RemapRefusal::Denied {
            origin: artifact
                .url
                .clone()
                .unwrap_or_else(|| artifact.kind.label().to_owned()),
            row: row.map_or_else(
                || "the active `[policy] unmatched = \"deny\"`".to_owned(),
                |row| {
                    format!(
                        "row {} of `{}` ({})",
                        row.provenance.index + 1,
                        row.provenance.source,
                        row.provenance.layer.label()
                    )
                },
            ),
            reason: reason.unwrap_or_else(|| "no reason given".to_owned()),
        }
    }

    /// The declared origin as a candidate list — empty when the artifact
    /// has no URL (an `sha256`-only input, say).
    fn origin_only(&self, artifact: &Artifact) -> Vec<Candidate> {
        artifact
            .url
            .iter()
            .map(|url| Candidate {
                target: RemapTarget::Url(url.clone()),
                row: None,
            })
            .collect()
    }

    /// Every matching row, sorted into winning order
    /// (`PROP-066#precedence`): most specific key, longest prefix among
    /// prefixes, scoped over unscoped, higher layer, later loaded source, then
    /// later source-local row index. Provenance indexes are never renumbered.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
    fn matching(&self, artifact: &Artifact) -> Vec<Found> {
        self.matching_bound(artifact, RegistryBinding::Current)
    }
    fn matching_bound(&self, artifact: &Artifact, binding: RegistryBinding<'_>) -> Vec<Found> {
        debug_assert_eq!(self.rows.len(), self.source_ordinals.len());
        let mut found: Vec<Found> = self
            .rows
            .iter()
            .zip(&self.source_ordinals)
            .enumerate()
            .filter_map(|(position, (row, ordinal))| {
                if matches!(binding, RegistryBinding::Held(_)) && row.from.catalog {
                    return None;
                }
                self.match_one(position, *ordinal, row, artifact, binding)
            })
            .collect();
        found.sort_by_key(|found| std::cmp::Reverse(found.rank));
        found
    }

    /// Whether one row matches, and what it resolves the artifact to.
    fn match_one(
        &self,
        position: usize,
        source_ordinal: usize,
        row: &RemapRow,
        artifact: &Artifact,
        binding: RegistryBinding<'_>,
    ) -> Option<Found> {
        if !scope_applies(row.scope.as_ref(), artifact) {
            return None;
        }
        // A row limited to one kind of artifact — the built-in single-file
        // endpoints — leaves every other kind (a registry clone, a git
        // dependency, a source tree) to its declared origin.
        if row.only.is_some_and(|kind| kind != artifact.kind) {
            return None;
        }
        let matched_prefix = match &row.key {
            RemapKey::Sha256(hex) => {
                if artifact.digest.as_deref() != Some(hex.as_str()) {
                    return None;
                }
                None
            }
            RemapKey::Package(spelling) => {
                if !package_matches(spelling, artifact.package.as_deref()?) {
                    return None;
                }
                None
            }
            RemapKey::Registry(name) => {
                match binding {
                    RegistryBinding::Held(Some(held)) if held == name => {}
                    RegistryBinding::Held(_) => return None,
                    RegistryBinding::Current => {
                        let url = artifact.url.as_deref()?;
                        let base = self
                            .registries
                            .iter()
                            .find(|(declared, _)| declared == name)
                            .map(|(_, base)| base.as_str())?;
                        if !url.starts_with(base) {
                            return None;
                        }
                        // The registry's base is how the row is *matched* — every
                        // URL that registry resolves to starts with it. It is not
                        // a prefix rewrite: `PROP-066#row` gives prefix
                        // replacement to the `prefix` key alone, and for every
                        // other key the target is the whole new location. That is
                        // also what PROP-002's `[[mirror]]` always meant — a
                        // mirror URL is another org root, and the per-package path
                        // is appended to it by the caller, not by the Locator.
                    }
                }
                None
            }
            RemapKey::Prefix(prefix) => {
                let url = artifact.url.as_deref()?;
                if !url.starts_with(prefix.as_str()) {
                    return None;
                }
                Some(prefix.clone())
            }
        };
        let target = resolve_target(&row.target, matched_prefix.as_deref(), artifact);
        let prefix_len = match &row.key {
            RemapKey::Prefix(prefix) => prefix.len(),
            _ => 0,
        };
        Some(Found {
            rank: (
                row.key.specificity(),
                prefix_len,
                row.scope.as_ref().map_or(0, RemapScope::specificity),
                row.from.layer.rank(),
                source_ordinal,
                row.from.index,
            ),
            row: RowRef {
                provenance: row.from.clone(),
                position,
            },
            key: row.key.clone(),
            target,
            scope: row.scope.clone(),
            reason: row.reason.clone(),
        })
    }
}

/// A matched row plus the sort key that orders it.
struct Found {
    rank: (u8, usize, u8, u8, usize, usize),
    row: RowRef,
    key: RemapKey,
    target: RemapTarget,
    scope: Option<RemapScope>,
    reason: Option<String>,
}

/// Apply a row's target to one artifact. For a key that matched a prefix
/// the matched prefix is replaced literally; for every other key the
/// target is the whole new location.
fn resolve_target(
    target: &RemapTarget,
    matched_prefix: Option<&str>,
    artifact: &Artifact,
) -> RemapTarget {
    match (target, matched_prefix) {
        (RemapTarget::Url(replacement), Some(prefix)) => {
            let url = artifact.url.as_deref().unwrap_or_default();
            RemapTarget::Url(format!("{replacement}{}", &url[prefix.len()..]))
        }
        (other, _) => other.clone(),
    }
}

/// Whether a row's `scope` admits this artifact. An unscoped row applies
/// everywhere; `scope.package` must name the artifact's package, and
/// `scope.step` its step.
fn scope_applies(scope: Option<&RemapScope>, artifact: &Artifact) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    let Some(package) = artifact.package.as_deref() else {
        return false;
    };
    if !package_matches(&scope.package, package) {
        return false;
    }
    match &scope.step {
        Some(step) => artifact.step.as_deref() == Some(step.as_str()),
        None => true,
    }
}

/// Whether a `<group>/<name>[@<req>]` spelling names this package.
///
/// The `group/name` half is compared exactly. A declared `@<req>` is
/// honoured when the artifact carries a version and is **not** a reason to
/// refuse the row when it does not: a row the user wrote to redirect a
/// package should redirect it, and the unchanged digest check above the
/// Locator is what decides whether the bytes are acceptable.
fn package_matches(spelling: &str, artifact_package: &str) -> bool {
    let (want, req) = split_at_version(spelling);
    let (have, version) = split_at_version(artifact_package);
    if want != have {
        return false;
    }
    match (req, version) {
        (Some(req), Some(version)) => match (
            crate::VersionSpec::parse(req),
            semver::Version::parse(version),
        ) {
            (Ok(spec), Ok(version)) => spec.matches(&version),
            // An unparseable requirement or version is not a reason to
            // silently drop the row; the name matched and that is what
            // the user wrote down.
            _ => true,
        },
        _ => true,
    }
}

/// Split `group/name@req` into its halves.
fn split_at_version(spelling: &str) -> (&str, Option<&str>) {
    match spelling.split_once('@') {
        Some((name, req)) => (name, Some(req)),
        None => (spelling, None),
    }
}

/// A fetch refused before any connection (`PROP-066#row`,
/// `PROP-066#policy`) — the error [`Locator::choose_url`] returns.
///
/// ```
/// use vibe_core::remap::RemapRefusal;
///
/// let refusal = RemapRefusal::Denied {
///     origin: "https://github.com/vibevm/vibevm".into(),
///     row: "row 1 of `~/.vibe/settings.toml` (local)".into(),
///     reason: "policy: one artifact store only".into(),
/// };
/// assert!(refusal.to_string().contains("remap_denied"));
/// assert!(refusal.to_string().contains("PROP-066#row"));
/// ```
#[derive(Debug, thiserror::Error)]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-066#row")]
pub enum RemapRefusal {
    #[error(
        "remap_denied: fetching `{origin}` is refused by {row}: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-066#row; \
          fix: drop or narrow that remap row, or point it at a target this host \
          may read — `vibe remap explain {origin}` prints the whole chain)"
    )]
    Denied {
        /// The declared origin, or the artifact kind when it has none.
        origin: String,
        /// The row that refused, or the policy that did.
        row: String,
        /// The row's reason.
        reason: String,
    },
}

#[cfg(test)]
#[path = "locator/publication_tests.rs"]
mod publication_tests;
