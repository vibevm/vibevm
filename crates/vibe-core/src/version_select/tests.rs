//! The selection policy over requirement × drop contents × installed ×
//! policy (PROP-075 `#VERSIONS-SELECT`).

use crate::VersionSpec;

use super::{Candidate, Selection, VersionSelect, admitted, pick, select};

/// The Node domain of the owner's own report, as a mirrored template
/// snapshot lists it.
fn domain() -> Vec<semver::Version> {
    ["22.23.4", "22.23.5", "24.22.2", "24.22.3", "26.13.0"]
        .iter()
        .map(|version| version.parse().unwrap())
        .collect()
}

/// The admitted set of `requirement`, each version marked available iff
/// it is in `available`.
fn candidates(requirement: &str, available: &[&str]) -> Vec<Candidate> {
    let spec = VersionSpec::parse(requirement).unwrap();
    admitted(&spec, &domain(), None)
        .into_iter()
        .map(|version| Candidate {
            available: available.iter().any(|a| *a == version.to_string()),
            version,
        })
        .collect()
}

fn version(raw: &str) -> semver::Version {
    raw.parse().unwrap()
}

/// `newest` is nvm's rule and the default: whatever the drop holds, the
/// requirement is left exactly as it was written, under every shape of
/// drop contents.
#[test]
fn newest_never_redirects() {
    for available in [vec![], vec!["22.23.4"], vec!["22.23.4", "22.23.5"]] {
        assert_eq!(
            select(VersionSelect::Newest, &candidates("22", &available)),
            Selection::Unchanged,
            "newest with {available:?}"
        );
    }
}

/// `prefer-drop` picks the highest version of the requirement whose
/// payload is already here — the owner's `node@22 → 22.23.4` case.
#[test]
fn prefer_drop_picks_the_highest_available_version() {
    assert_eq!(
        select(VersionSelect::PreferDrop, &candidates("22", &["22.23.4"])),
        Selection::Redirect {
            version: version("22.23.4"),
            instead_of: version("22.23.5"),
        }
    );
}

/// The two policies agree whenever the drop holds the newest match: the
/// requirement is untouched and no note is printed, because the answer
/// did not move.
#[test]
fn an_available_newest_is_unchanged_under_both_policies() {
    for policy in [VersionSelect::PreferDrop, VersionSelect::DropOnly] {
        assert_eq!(
            select(policy, &candidates("22", &["22.23.4", "22.23.5"])),
            Selection::Unchanged,
            "{policy} with the newest match in the drop"
        );
    }
}

/// An empty drop leaves `prefer-drop` with nothing to prefer, so the
/// requirement resolves as `newest` would and the ordinary refusal about
/// the missing payload is what the operator sees.
#[test]
fn prefer_drop_with_nothing_available_falls_back_to_newest() {
    assert_eq!(
        select(VersionSelect::PreferDrop, &candidates("22", &[])),
        Selection::Unchanged
    );
}

/// `drop-only` refuses instead, naming the version `newest` would have
/// picked — the one whose payload the operator has to carry in.
#[test]
fn drop_only_with_nothing_available_refuses_naming_the_newest_match() {
    assert_eq!(
        select(VersionSelect::DropOnly, &candidates("22", &[])),
        Selection::Refuse {
            newest: Some(version("22.23.5")),
        }
    );
}

/// A requirement nothing in the domain satisfies is the registry's own
/// `NoMatchingVersion`, not a policy verdict: `drop-only` refuses with no
/// version to name rather than inventing one.
#[test]
fn drop_only_over_an_unsatisfiable_requirement_names_no_version() {
    assert_eq!(
        select(VersionSelect::DropOnly, &candidates("23", &["22.23.4"])),
        Selection::Refuse { newest: None }
    );
    assert!(candidates("23", &[]).is_empty());
}

/// An installed version counts as available even when the drop holds
/// nothing of it: installing it downloads no byte, which is the whole
/// property both policies exist to get.
#[test]
fn an_installed_version_is_available_without_the_drop() {
    // `22.23.4` here stands for "installed, not in the drop" — the caller
    // unions the two grounds before it calls, and the policy sees one bit.
    assert_eq!(
        select(VersionSelect::DropOnly, &candidates("22", &["22.23.4"])),
        Selection::Redirect {
            version: version("22.23.4"),
            instead_of: version("22.23.5"),
        }
    );
}

/// Across major lines the requirement still bounds the choice: `^24`
/// never reaches down into 22 however full the drop is there.
#[test]
fn the_requirement_bounds_the_choice_in_every_policy() {
    assert_eq!(
        select(
            VersionSelect::PreferDrop,
            &candidates("^24", &["22.23.4", "24.22.2"])
        ),
        Selection::Redirect {
            version: version("24.22.2"),
            instead_of: version("24.22.3"),
        }
    );
    assert_eq!(
        select(VersionSelect::DropOnly, &candidates("^24", &["22.23.4"])),
        Selection::Refuse {
            newest: Some(version("24.22.3")),
        }
    );
}

/// No requirement at all is the whole domain, so a policy may move the
/// answer across major lines — `vibe install -g node` with only 22 in the
/// drop installs 22, which is exactly the ask.
#[test]
fn an_absent_requirement_admits_the_whole_domain() {
    assert_eq!(admitted(&VersionSpec::Latest, &domain(), None).len(), 5);
    assert_eq!(
        select(VersionSelect::PreferDrop, &candidates("", &["22.23.4"])),
        Selection::Redirect {
            version: version("22.23.4"),
            instead_of: version("26.13.0"),
        }
    );
}

/// The pick rule prefers a stable release and only falls back to a
/// prerelease when the admitted set holds nothing else — the same two
/// steps the registries' own pick walks.
#[test]
fn the_pick_prefers_a_stable_release_over_a_newer_prerelease() {
    let mixed: Vec<semver::Version> = ["1.0.0", "2.0.0-rc.1"]
        .iter()
        .map(|version| version.parse().unwrap())
        .collect();
    assert_eq!(pick(&mixed), Some(&version("1.0.0")));
    let only_pre: Vec<semver::Version> = vec![version("2.0.0-rc.1")];
    assert_eq!(pick(&only_pre), Some(&version("2.0.0-rc.1")));
}

/// A channel's members come from the template and nothing else: supplied,
/// they are the admitted set; withheld, a channel admits nothing, which
/// is the fail-closed reading an unlowered channel gets everywhere else.
#[test]
fn a_channel_admits_its_members_and_nothing_without_them() {
    let spec = VersionSpec::parse("lts").unwrap();
    let members = [version("22.23.4"), version("22.23.5")];
    assert_eq!(admitted(&spec, &domain(), Some(&members)).len(), 2);
    assert!(admitted(&spec, &domain(), None).is_empty());
    let channel: Vec<Candidate> = admitted(&spec, &domain(), Some(&members))
        .into_iter()
        .map(|version| Candidate {
            available: version == self::version("22.23.4"),
            version,
        })
        .collect();
    assert_eq!(
        select(VersionSelect::PreferDrop, &channel),
        Selection::Redirect {
            version: version("22.23.4"),
            instead_of: version("22.23.5"),
        }
    );
}

/// A member the domain no longer carries is not admitted: a trimmed
/// mirror snapshot and a stale channel table cannot together name a
/// version that is not there.
#[test]
fn a_channel_member_outside_the_domain_is_not_admitted() {
    let spec = VersionSpec::parse("lts").unwrap();
    let members = [version("20.1.0"), version("22.23.4")];
    let found = admitted(&spec, &domain(), Some(&members));
    assert_eq!(found, vec![version("22.23.4")]);
}

/// An empty domain decides nothing under any policy: `prefer-drop` leaves
/// the requirement alone and `drop-only` has no version to name.
#[test]
fn an_empty_domain_is_no_policy_verdict() {
    assert_eq!(select(VersionSelect::PreferDrop, &[]), Selection::Unchanged);
    assert_eq!(
        select(VersionSelect::DropOnly, &[]),
        Selection::Refuse { newest: None }
    );
}

/// The stored spellings round-trip through TOML and through `FromStr`,
/// because the same three words are written in `config.toml`, read from
/// `VIBE_VERSIONS_SELECT` and printed by `vibe show config`.
#[test]
fn every_policy_round_trips_through_its_one_spelling() {
    for policy in VersionSelect::ALL {
        let written = toml::to_string(&PolicyRow { select: policy }).unwrap();
        assert!(
            written.contains(policy.as_str()),
            "{policy} written as {written}"
        );
        assert_eq!(policy.as_str().parse::<VersionSelect>().unwrap(), policy);
        assert_eq!(
            policy
                .as_str()
                .to_ascii_uppercase()
                .parse::<VersionSelect>()
                .unwrap(),
            policy,
            "the spelling is read in any letter case"
        );
    }
    assert_eq!(VersionSelect::spellings(), "newest, prefer-drop, drop-only");
}

#[derive(serde::Serialize)]
struct PolicyRow {
    select: VersionSelect,
}
