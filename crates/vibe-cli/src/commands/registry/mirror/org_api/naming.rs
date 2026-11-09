//! The naming convention as the mirror's package filter
//! (PROP-002 `#registry-mirror`).
//!
//! An organisation's repository listing is just names. Turning one into a
//! package coordinate is only safe in one direction: a manifest states
//! the coordinate, and [`NamingConvention::repo_name`] — the very
//! function the mirror uses to address a package's repository — says
//! whether this registry would have published that coordinate under that
//! name ([`produces`]). A group is never read out of a repository name
//! unless the convention provably put it there ([`name_claim`]).

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-mirror");

use vibe_core::manifest::NamingConvention;
use vibe_core::{Group, PackageKind, PackageName};

/// What a repository's *name* alone says about its coordinate under
/// `naming` — the cheap pre-filter, before anything is fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::commands::registry::mirror) enum NameClaim {
    /// The convention spells the whole coordinate, and this name is one
    /// it could have produced.
    Coordinate(Group, String),
    /// The convention spells the whole coordinate, and this name is not
    /// one it could have produced: not a package repository of this
    /// registry.
    Rejected,
    /// The convention does not carry the group, so the name decides
    /// nothing and only the manifest can.
    Undecidable,
}

/// Read `repo`'s name under `naming`. Only `fqdn` spells the coordinate:
/// it splits at the last dot (the name is one dot-free label, the group
/// carries the rest), validates both halves and composes them back, so
/// the claim is a name this registry would itself have published to.
pub(in crate::commands::registry::mirror) fn name_claim(
    naming: NamingConvention,
    repo: &str,
) -> NameClaim {
    if !matches!(naming, NamingConvention::Fqdn) {
        return NameClaim::Undecidable;
    }
    let claimed = repo
        .rsplit_once('.')
        .and_then(|(group, name)| {
            let group = Group::parse(group).ok()?;
            let name = PackageName::parse(name).ok()?;
            Some((group, name))
        })
        .filter(|(group, name)| {
            naming
                .repo_name(None, group, name.as_str())
                .is_ok_and(|composed| composed == repo)
        });
    match claimed {
        Some((group, name)) => NameClaim::Coordinate(group, name.as_str().to_string()),
        None => NameClaim::Rejected,
    }
}

/// Whether `naming` turns the coordinate a manifest declares into exactly
/// the repository `repo`. This is the one filter every convention shares:
/// the manifest states the coordinate, and the convention confirms that
/// this registry would have published it here.
pub(in crate::commands::registry::mirror) fn produces(
    naming: NamingConvention,
    repo: &str,
    kind: PackageKind,
    group: &Group,
    name: &str,
) -> bool {
    naming
        .repo_name(Some(kind), group, name)
        .is_ok_and(|composed| composed == repo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(text: &str) -> Group {
        Group::parse(text).expect("a group")
    }

    #[test]
    fn an_fqdn_repository_name_claims_its_coordinate_and_the_others_claim_nothing() {
        assert_eq!(
            name_claim(NamingConvention::Fqdn, "org.vibevm.world.wal"),
            NameClaim::Coordinate(group("org.vibevm.world"), "wal".into())
        );
        // Under `fqdn` a name the convention could not have produced is a
        // rejection, not a question for the manifest: no group, an
        // invalid name, an invalid group.
        for rejected in ["index", "org.vibevm.Not-Kebab", ".wal"] {
            assert_eq!(
                name_claim(NamingConvention::Fqdn, rejected),
                NameClaim::Rejected,
                "{rejected}"
            );
        }
        // The conventions that drop the group leave every name to the
        // manifest — including the built-in `vibespecs-gitverse`'s `name`.
        for naming in [
            NamingConvention::Name,
            NamingConvention::KindName,
            NamingConvention::KindSlashName,
        ] {
            assert_eq!(name_claim(naming, "wal"), NameClaim::Undecidable);
            assert_eq!(name_claim(naming, "index"), NameClaim::Undecidable);
        }
    }

    #[test]
    fn the_convention_confirms_a_manifests_coordinate_against_the_repository_name() {
        let world = group("org.vibevm.world");
        // `name`: the repository is the bare name, and a manifest naming
        // another package does not belong in it.
        assert!(produces(
            NamingConvention::Name,
            "wal",
            PackageKind::Flow,
            &world,
            "wal"
        ));
        assert!(!produces(
            NamingConvention::Name,
            "wal",
            PackageKind::Flow,
            &world,
            "zap"
        ));
        // `fqdn`: the whole coordinate, so a differing group is caught too.
        assert!(produces(
            NamingConvention::Fqdn,
            "org.vibevm.world.wal",
            PackageKind::Flow,
            &world,
            "wal"
        ));
        assert!(!produces(
            NamingConvention::Fqdn,
            "org.vibevm.world.wal",
            PackageKind::Flow,
            &group("org.other"),
            "wal"
        ));
        // `kind-name` and `kind/name` need the kind, which a manifest has.
        assert!(produces(
            NamingConvention::KindName,
            "flow-wal",
            PackageKind::Flow,
            &world,
            "wal"
        ));
        assert!(!produces(
            NamingConvention::KindName,
            "feat-wal",
            PackageKind::Flow,
            &world,
            "wal"
        ));
        assert!(produces(
            NamingConvention::KindSlashName,
            "flow/wal",
            PackageKind::Flow,
            &world,
            "wal"
        ));
    }
}
