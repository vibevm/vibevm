//! `LocalCompositeProvider` — a `DepProvider` over an ordered list of
//! [`LocalRegistryProvider`]s, the structural answer to "project-local
//! packages + vibe-embedded packages" composing at the same tier of
//! resolution (PROP-030 §3.3, the project-local sources extension).
//!
//! The pre-§3.3 `EmbeddedProvider` carried a single `LocalRegistryProvider`
//! for the embedded registry; adding project-local as a second source forced
//! either N-way logic inside `EmbeddedProvider` (touching its `ordered()`,
//! `resolve_first`, `union_versions`, `first_served_versions`, plus four
//! clash/short-circuit tests) or this composite — a `DepProvider` over many
//! local-registry providers, exposing the same `VersionEnumerator` surface
//! `EmbeddedProvider` consumed, so it composes unchanged with the declared
//! walk. The composite owns the inner ordering of its locals (project-local
//! first, then vibe-embedded, per the developer-in-project precedence) and is
//! transparent to the layer above.
//!
//! Conformance twin: spec://vibevm/modules/vibe-registry/PROP-030#project-local
//! (the products-side normative anchor for the project-local extension).

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-030#project-local");

use specmark::{cell, spec};
use vibe_core::manifest::Manifest;
use vibe_core::{Group, PackageRef};

use crate::{DepProvider, DepProviderError, LocalRegistryProvider, VersionEnumerator};

/// A `DepProvider` over an ordered list of [`LocalRegistryProvider`]s. The
/// providers are consulted in the order given (first wins; later providers
/// are absent-fall-through only), and version enumeration unions across all
/// of them. An empty composite is a programming error — at least one local
/// source is required (the only caller constructs it from a non-empty list,
/// see `InstallResolver::Embedded` in `vibe-cli`).
#[cell(seam = "DepProvider", variant = "local-composite", flag = "provider")]
pub struct LocalCompositeProvider<'a> {
    /// Ordered: first provider wins a clash. The caller (the install
    /// resolver) builds this as `[project_local, vibe_embedded]` so
    /// project-local takes precedence inside the local family.
    providers: Vec<LocalRegistryProvider<'a>>,
}

impl<'a> LocalCompositeProvider<'a> {
    /// Compose an ordered list of local-registry providers. The list MUST be
    /// non-empty — an empty composite answers nothing for any coordinate and
    /// surfaces the same "consulted no providers" sentinel the embedded
    /// composition does.
    pub fn new(providers: Vec<LocalRegistryProvider<'a>>) -> Self {
        Self { providers }
    }

    /// The providers in consultation order, as `&dyn` for the combinator
    /// helpers (`resolve_first` / `union_versions` / `first_served_versions`)
    /// that take `&[&dyn VersionEnumerator]`.
    fn ordered(&self) -> Vec<&dyn VersionEnumerator> {
        self.providers
            .iter()
            .map(|p| p as &dyn VersionEnumerator)
            .collect()
    }
}

impl<'a> DepProvider for LocalCompositeProvider<'a> {
    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-030#project-local")]
    fn resolve_version(&self, pkgref: &PackageRef) -> Result<semver::Version, DepProviderError> {
        resolve_first(&self.ordered(), |p| p.resolve_version(pkgref))
    }

    #[spec(implements = "spec://vibevm/modules/vibe-registry/PROP-030#project-local")]
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        resolve_first(&self.ordered(), |p| p.fetch_manifest(group, name, version))
    }
}

impl<'a> VersionEnumerator for LocalCompositeProvider<'a> {
    /// Project-local and vibe-embedded both answer locally (no network), so
    /// the union is cheap and is what the candidate-enumerating solver needs
    /// — it sees every locally-available version for a coordinate, then the
    /// fetch picks the precedence-first one (project-local, per the ordering).
    /// The `--embedded-short-circuit` knob, when it applies, is enforced by
    /// the layer above (`EmbeddedProvider`); this composite always unions.
    #[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-017#provider-enrichment")]
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        union_versions(&self.ordered(), group, name)
    }
}

// ---------------------------------------------------------------------------
// Combinator helpers — mirrors of `embedded_provider`'s, factored so this
// cell is unit-testable in isolation. The `is_absent` classification is
// shared with the embedded composition (same `DepProviderError` taxonomy).
// ---------------------------------------------------------------------------

/// Does this error mean "this provider does not serve the coordinate"? On
/// these the combiner falls through to the next provider; any other error (a
/// real provider failure) propagates immediately rather than being masked by
/// a fall-through.
fn is_absent(err: &DepProviderError) -> bool {
    matches!(
        err,
        DepProviderError::UnknownPackage { .. }
            | DepProviderError::NoMatchingVersion { .. }
            | DepProviderError::AggregateNotFound { .. }
    )
}

/// Run `op` against each provider in order, returning the first success.
/// Absent providers are skipped; a real failure short-circuits. If every
/// provider is absent the last absence is returned so the caller sees a
/// genuine "not found" rather than a fabricated one. `providers` MUST be
/// non-empty (see [`LocalCompositeProvider::ordered`]).
fn resolve_first<T>(
    providers: &[&dyn VersionEnumerator],
    op: impl Fn(&dyn VersionEnumerator) -> Result<T, DepProviderError>,
) -> Result<T, DepProviderError> {
    let mut last_absent = None;
    for &p in providers {
        match op(p) {
            Ok(value) => return Ok(value),
            Err(e) if is_absent(&e) => last_absent = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last_absent.unwrap_or_else(|| {
        DepProviderError::Other("local composite consulted no providers".into())
    }))
}

/// The union of every provider's versions for `(group, name)`, sorted and
/// de-duplicated. Absent providers contribute nothing; only when *every*
/// provider is absent does the union report "not found" (the last absence).
fn union_versions(
    providers: &[&dyn VersionEnumerator],
    group: &Group,
    name: &str,
) -> Result<Vec<semver::Version>, DepProviderError> {
    let mut all = Vec::new();
    let mut last_absent = None;
    let mut any_served = false;
    for &p in providers {
        match p.list_versions(group, name) {
            Ok(mut versions) => {
                any_served = true;
                all.append(&mut versions);
            }
            Err(e) if is_absent(&e) => last_absent = Some(e),
            Err(e) => return Err(e),
        }
    }
    if !any_served {
        return Err(last_absent.unwrap_or_else(|| {
            DepProviderError::Other("local composite consulted no providers".into())
        }));
    }
    all.sort();
    all.dedup();
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalRegistryProvider;
    use semver::Version;
    use specmark::verifies;
    use std::fs;
    use vibe_core::Group;
    use vibe_registry::LocalRegistry;

    /// Seed `<root>/<group>/<name>/v<ver>/vibe.toml` and return the
    /// `LocalRegistry` over `root`. The package carries `label` in its
    /// description so a test can tell which provider answered.
    fn seed(
        root: &std::path::Path,
        group: &str,
        name: &str,
        ver: &str,
        label: &str,
    ) -> LocalRegistry {
        let dir = root.join(group).join(name).join(format!("v{ver}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(Manifest::FILENAME),
            format!(
                "[package]\ngroup=\"{group}\"\nname=\"{name}\"\nkind=\"flow\"\n\
                 version=\"{ver}\"\ndescription=\"{label}\"\n"
            ),
        )
        .unwrap();
        LocalRegistry::new(root).unwrap()
    }

    fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    /// The first local provider (project-local) wins a coordinate both carry
    /// — project-local is source-of-truth for a developer in their own
    /// project. The composite resolves `wal` and `fetch_manifest` reports the
    /// project-local label, not the embedded one.
    #[test]
    #[verifies("spec://vibevm/modules/vibe-registry/PROP-030#project-local", r = 1)]
    fn project_local_wins_over_embedded_on_a_clash() {
        let project_tmp = tempfile::tempdir().unwrap();
        let embedded_tmp = tempfile::tempdir().unwrap();
        let project = seed(
            project_tmp.path(),
            "org.vibevm",
            "wal",
            "0.2.0",
            "project-local",
        );
        let embedded = seed(
            embedded_tmp.path(),
            "org.vibevm",
            "wal",
            "0.2.0",
            "vibe-embedded",
        );

        let composite = LocalCompositeProvider::new(vec![
            LocalRegistryProvider::new(&project),
            LocalRegistryProvider::new(&embedded),
        ]);
        let pkgref = PackageRef::parse("org.vibevm/wal@0.2.0").unwrap();
        let v = composite.resolve_version(&pkgref).unwrap();
        assert_eq!(v, Version::parse("0.2.0").unwrap());

        let manifest = composite.fetch_manifest(&org(), "wal", &v).unwrap();
        assert_eq!(
            manifest.package.as_ref().unwrap().description.as_deref(),
            Some("project-local"),
            "project-local (first in the ordered list) wins the clash"
        );
    }

    /// When project-local does NOT carry a coordinate, the composite falls
    /// through to vibe-embedded — the absence is not fatal. This is the
    /// load-bearing fall-through behaviour the embedded composition relied on
    /// with a single provider; the composite preserves it across N.
    #[test]
    #[verifies("spec://vibevm/modules/vibe-registry/PROP-030#project-local", r = 1)]
    fn absent_in_first_falls_through_to_second() {
        let project_tmp = tempfile::tempdir().unwrap();
        let embedded_tmp = tempfile::tempdir().unwrap();
        // project-local carries `redb`; embedded carries `wal`.
        let project = seed(
            project_tmp.path(),
            "org.vibevm",
            "redb",
            "0.1.0",
            "project-local",
        );
        let embedded = seed(
            embedded_tmp.path(),
            "org.vibevm",
            "wal",
            "0.1.0",
            "vibe-embedded",
        );

        let composite = LocalCompositeProvider::new(vec![
            LocalRegistryProvider::new(&project),
            LocalRegistryProvider::new(&embedded),
        ]);
        let pkgref = PackageRef::parse("org.vibevm/wal@0.1.0").unwrap();
        let v = composite.resolve_version(&pkgref).unwrap();
        let manifest = composite.fetch_manifest(&org(), "wal", &v).unwrap();
        assert_eq!(
            manifest.package.as_ref().unwrap().description.as_deref(),
            Some("vibe-embedded"),
            "absent in project-local falls through to vibe-embedded"
        );
    }

    /// Version enumeration unions across both providers — the solver sees
    /// every locally-available version for a coordinate. For `wal`, which
    /// only embedded carries, only embedded's versions surface; for `redb`
    /// only project's; for a coordinate neither carries, an absence.
    #[test]
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-017#provider-enrichment")]
    fn list_versions_unions_across_both_providers() {
        let project_tmp = tempfile::tempdir().unwrap();
        let embedded_tmp = tempfile::tempdir().unwrap();
        // both carry `wal` at different versions
        let project = seed(project_tmp.path(), "org.vibevm", "wal", "0.3.0", "project");
        let embedded = seed(
            embedded_tmp.path(),
            "org.vibevm",
            "wal",
            "0.2.0",
            "embedded",
        );

        let composite = LocalCompositeProvider::new(vec![
            LocalRegistryProvider::new(&project),
            LocalRegistryProvider::new(&embedded),
        ]);
        let versions = composite.list_versions(&org(), "wal").unwrap();
        assert_eq!(
            versions.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
            vec!["0.2.0", "0.3.0"],
            "union + sort + dedup across both locals"
        );
    }

    /// A real provider failure (a malformed manifest at a coordinate a
    /// provider DOES carry) propagates immediately — the composite never
    /// masks a genuine error as a fall-through.
    #[test]
    fn a_real_failure_propagates_rather_than_falling_through() {
        let project_tmp = tempfile::tempdir().unwrap();
        let embedded_tmp = tempfile::tempdir().unwrap();
        let project = seed(project_tmp.path(), "org.vibevm", "wal", "0.1.0", "project");
        let embedded = seed(
            embedded_tmp.path(),
            "org.vibevm",
            "wal",
            "0.1.0",
            "embedded",
        );
        // Corrupt the project-local manifest so reading it fails hard.
        let path = project_tmp
            .path()
            .join("org.vibevm/wal/v0.1.0")
            .join(Manifest::FILENAME);
        fs::write(&path, "not = valid = toml =").unwrap();

        let composite = LocalCompositeProvider::new(vec![
            LocalRegistryProvider::new(&project),
            LocalRegistryProvider::new(&embedded),
        ]);
        let v = Version::parse("0.1.0").unwrap();
        let err = composite.fetch_manifest(&org(), "wal", &v).unwrap_err();
        assert!(
            matches!(err, DepProviderError::Other(_)),
            "a real read failure is not masked by fall-through; got {err:?}"
        );
    }

    /// A coordinate no provider carries surfaces the last absence, not a
    /// fabricated "consulted no providers" — the caller sees a genuine
    /// UnknownPackage from the layer above.
    #[test]
    fn absent_everywhere_returns_the_last_absence() {
        let project_tmp = tempfile::tempdir().unwrap();
        let embedded_tmp = tempfile::tempdir().unwrap();
        let project = LocalRegistry::new(project_tmp.path()).unwrap();
        let embedded = LocalRegistry::new(embedded_tmp.path()).unwrap();
        let composite = LocalCompositeProvider::new(vec![
            LocalRegistryProvider::new(&project),
            LocalRegistryProvider::new(&embedded),
        ]);
        let pkgref = PackageRef::parse("org.vibevm/nope").unwrap();
        let err = composite.resolve_version(&pkgref).unwrap_err();
        assert!(
            matches!(err, DepProviderError::UnknownPackage { .. }),
            "absent everywhere → UnknownPackage, not Other; got {err:?}"
        );
    }
}
