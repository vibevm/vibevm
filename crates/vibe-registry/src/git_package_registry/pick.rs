//! The version **selector** of a per-package registry: the pick, and
//! the quarantine that runs before it (PROP-044 §4.5, B-080).
//!
//! Separated from the lookups in [`super::lookup`] because it is a
//! different responsibility: everything there asks a host a question,
//! and everything here decides, over candidates already in hand, which
//! version this reader may act on. Nothing in this file makes contact.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-model");

use super::*;
use crate::index_client::IndexVersion;

/// The one construction of a quarantine skip's warn line, so the
/// shape an operator reads is the shape a test pins (PROP-044 §4.5).
fn skip_warning(
    group: &Group,
    name: &str,
    version: &semver::Version,
    missing: &[String],
) -> String {
    format!(
        "skipping {group}/{name}@{version}: this reader does not understand {missing:?} \
         (must_understand — spec://org.vibevm.core/vibevm/common/PROP-044#machinery)"
    )
}

/// The version selector (PROP-044 §4.5, B-080): pick the version
/// `spec` names from `candidates`, quarantining every candidate whose
/// `must_understand` names a reader capability this build lacks.
///
/// Each quarantine warns ([`skip_warning`]) and the pick runs over
/// what survives. A survivor set with nothing to pick is the honest
/// refusal — when a skipped version WOULD have matched the pick rule,
/// [`RegistryError::AllVersionsUnusable`] names the best of them and
/// what it needs; only a set with no match at all keeps the plain
/// [`RegistryError::NoMatchingVersion`]. Either way the refusal
/// surfaces here, at the point of application — the reader never
/// silently lands on a version it cannot act on.
///
/// `candidates` arrive sorted ascending (both sources sort); the pick
/// walks them in reverse, so "first match" is "newest match".
pub(super) fn pick_version(
    group: &Group,
    name: &str,
    candidates: &[IndexVersion],
    spec: &VersionSpec,
) -> Result<semver::Version, RegistryError> {
    let req_display = spec.requirement_label();
    // One pick rule, applied to whichever list survives: newest
    // non-prerelease under `Latest`; under `Req` the newest
    // non-prerelease that matches, else the newest match of any
    // prerelease-ness.
    let pick_from = |list: &[semver::Version]| -> Option<semver::Version> {
        match spec {
            VersionSpec::Latest => list.iter().rev().find(|v| v.pre.is_empty()).cloned(),
            VersionSpec::Req(req) => list
                .iter()
                .rev()
                .find(|v| req.matches(v) && v.pre.is_empty())
                .or_else(|| list.iter().rev().find(|v| req.matches(v)))
                .cloned(),
            // An unlowered channel picks nothing — the fail-closed
            // invariant of PROP-075 `#CHANNEL-SYNTAX`: a path that forgot
            // to lower must refuse, never resolve something plausible.
            VersionSpec::Channel(_) => None,
        }
    };
    let mut usable: Vec<semver::Version> = Vec::new();
    let mut skipped: Vec<(semver::Version, Vec<String>)> = Vec::new();
    for c in candidates {
        let missing = vibe_core::capabilities::missing_capabilities(&c.must_understand);
        if missing.is_empty() {
            usable.push(c.version.clone());
        } else {
            tracing::warn!(
                target: "vibe_registry",
                "{}",
                skip_warning(group, name, &c.version, &missing)
            );
            skipped.push((c.version.clone(), missing));
        }
    }
    if let Some(version) = pick_from(&usable) {
        return Ok(version);
    }
    // Nothing usable: if the pick rule would have matched a skipped
    // version, the refusal names the best of those and its missing
    // capabilities — the §4.5 point-of-application refusal.
    let skipped_versions: Vec<semver::Version> = skipped.iter().map(|(v, _)| v.clone()).collect();
    let best_pair =
        pick_from(&skipped_versions).and_then(|best| skipped.iter().find(|(v, _)| *v == best));
    if let Some((best, missing)) = best_pair {
        let (best, missing) = (best.clone(), missing.clone());
        return Err(RegistryError::AllVersionsUnusable {
            detail: Box::new(crate::AllVersionsUnusableDetail {
                group: group.clone(),
                name: name.to_string(),
                req: req_display,
                best,
                missing,
            }),
        });
    }
    Err(RegistryError::NoMatchingVersion {
        group: group.clone(),
        name: name.to_string(),
        req: req_display,
    })
}

#[cfg(test)]
mod tests {
    //! B-080 (PROP-044 §4.5): the pick quarantines before it selects.

    use vibe_core::{Group, VersionSpec};

    use super::*;

    fn org() -> Group {
        Group::parse("org.vibevm").unwrap()
    }

    fn candidate(version: &str, caps: &[&str]) -> IndexVersion {
        IndexVersion {
            version: version.parse().unwrap(),
            must_understand: caps.iter().map(|s| s.to_string()).collect(),
            template: false,
        }
    }

    /// The warn that accompanies every quarantine skip: it names the
    /// package, the version, and the missing capabilities — the
    /// operator's actionable facts, in the one construction of the line.
    #[test]
    fn skip_warning_names_package_version_and_capabilities() {
        let line = skip_warning(
            &org(),
            "wal",
            &"2.0.0".parse().unwrap(),
            &["b080-test-capability".to_string()],
        );
        assert!(line.contains("org.vibevm/wal@2.0.0"), "warn: {line}");
        assert!(line.contains("b080-test-capability"), "warn: {line}");
        assert!(line.contains("PROP-044"), "warn: {line}");
    }

    /// Latest over a mixed set skips the quarantinable newest and picks
    /// the newest usable — the pick never lands on a version this reader
    /// cannot act on.
    #[test]
    fn pick_version_latest_skips_quarantinable_newest() {
        let candidates = vec![candidate("1.0.0", &[]), candidate("2.0.0", &["b080-x"])];
        let picked = pick_version(&org(), "wal", &candidates, &VersionSpec::Latest).unwrap();
        assert_eq!(picked.to_string(), "1.0.0");
    }

    /// Git-path candidates declare nothing: the pick over them is exactly
    /// the old behaviour (newest non-prerelease), quarantine never fires.
    #[test]
    fn pick_version_untagged_candidates_behave_as_before() {
        let candidates = vec![
            candidate("0.1.0", &[]),
            candidate("0.2.0", &[]),
            candidate("1.0.0-rc.1", &[]),
        ];
        let picked = pick_version(&org(), "wal", &candidates, &VersionSpec::Latest).unwrap();
        assert_eq!(picked.to_string(), "0.2.0");
    }

    /// Nothing left to pick, and the skipped versions matched the rule:
    /// the refusal is `AllVersionsUnusable` naming the best skip and its
    /// missing capability — not the anonymous `NoMatchingVersion`.
    #[test]
    fn pick_version_all_quarantined_refuses_with_the_capability() {
        let candidates = vec![
            candidate("1.0.0", &["b080-x"]),
            candidate("2.0.0", &["b080-x"]),
        ];
        let err = pick_version(&org(), "wal", &candidates, &VersionSpec::Latest).unwrap_err();
        match err {
            RegistryError::AllVersionsUnusable { detail } => {
                assert_eq!(detail.best.to_string(), "2.0.0");
                assert_eq!(detail.missing, vec!["b080-x".to_string()]);
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    /// A `Req` that matched only quarantined versions gets the same
    /// point-of-application refusal, naming the version it had to skip.
    #[test]
    fn pick_version_req_on_quarantined_only_refuses_with_best_skip() {
        let candidates = vec![candidate("2.0.0", &["b080-x"])];
        let req = VersionSpec::Req("^2.0".parse().unwrap());
        let err = pick_version(&org(), "wal", &candidates, &req).unwrap_err();
        match err {
            RegistryError::AllVersionsUnusable { detail } => {
                assert_eq!(detail.best.to_string(), "2.0.0");
                assert_eq!(detail.req, "^2.0");
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    /// Skipped versions the rule would NOT have matched leave the refusal
    /// as the plain `NoMatchingVersion` — a capability never masks what
    /// is genuinely "no such version".
    #[test]
    fn pick_version_no_match_stays_no_matching_version() {
        let candidates = vec![candidate("1.0.0", &[]), candidate("2.0.0", &["b080-x"])];
        let req = VersionSpec::Req("^9.0".parse().unwrap());
        let err = pick_version(&org(), "wal", &candidates, &req).unwrap_err();
        assert!(matches!(err, RegistryError::NoMatchingVersion { .. }));
    }
}
