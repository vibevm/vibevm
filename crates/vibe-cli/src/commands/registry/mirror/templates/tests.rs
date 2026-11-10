//! Which versions a `[[template]]` row selects (PROP-075
//! `#MIRROR-CONFIG-SCHEMA`: `versions`, `channels`, `newest` per major).
use super::*;

fn version(spelling: &str) -> semver::Version {
    spelling.parse().unwrap()
}

/// A Node-shaped domain: three majors, several releases each.
fn domain() -> Vec<semver::Version> {
    [
        "20.1.0", "20.19.5", "22.14.0", "22.23.4", "24.0.0", "24.11.1", "24.22.2",
    ]
    .iter()
    .map(|spelling| version(spelling))
    .collect()
}

fn channels() -> BTreeMap<String, BTreeSet<semver::Version>> {
    BTreeMap::from([
        (
            "lts".to_string(),
            BTreeSet::from([version("20.19.5"), version("22.23.4")]),
        ),
        ("lts/jod".to_string(), BTreeSet::from([version("22.23.4")])),
    ])
}

fn row(versions: &[&str], channel_names: &[&str], newest: Option<u32>) -> TemplateSelection {
    TemplateSelection {
        group: "org.vibevm.node".into(),
        name: "node".into(),
        versions: versions
            .iter()
            .map(|spelling| semver::VersionReq::parse(spelling).unwrap())
            .collect(),
        channels: channel_names.iter().map(|name| name.to_string()).collect(),
        newest,
        platforms: Vec::new(),
        archs: Vec::new(),
        installers: Vec::new(),
        payloads: false,
    }
}

fn selected(row: &TemplateSelection) -> Vec<String> {
    select_versions(row, &domain(), &channels())
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn neither_key_selects_the_whole_domain() {
    assert_eq!(selected(&row(&[], &[], None)).len(), 7);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn version_requirements_are_ored() {
    assert_eq!(
        selected(&row(&[">=24", "~20.1"], &[], None)),
        ["20.1.0", "24.0.0", "24.11.1", "24.22.2"]
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn a_channel_adds_its_members_rather_than_narrowing_the_requirements() {
    // "everything from 22 up, and the LTS line whatever its number" —
    // the union, which is what an operator supplying a closed network
    // actually wants; an intersection here would drop 20.19.5.
    assert_eq!(
        selected(&row(&[">=22"], &["lts"], None)),
        [
            "20.19.5", "22.14.0", "22.23.4", "24.0.0", "24.11.1", "24.22.2"
        ]
    );
    // A channel alone selects exactly its members.
    assert_eq!(selected(&row(&[], &["lts/jod"], None)), ["22.23.4"]);
}

#[test]
fn a_channel_the_template_does_not_declare_selects_nothing_and_does_not_panic() {
    assert!(selected(&row(&[], &["nightly"], None)).is_empty());
    let resolved = resolved_channels(&row(&[], &["nightly", "lts"], None), &channels());
    assert_eq!(resolved[0], ("nightly".to_string(), None));
    // A channel lowers to the newest member of its set.
    assert_eq!(resolved[1], ("lts".to_string(), Some(version("22.23.4"))));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn newest_keeps_n_per_major_not_n_overall() {
    // Two per major, across all three majors — not the two newest
    // versions in the domain.
    assert_eq!(
        selected(&row(&[], &[], Some(2))),
        [
            "20.1.0", "20.19.5", "22.14.0", "22.23.4", "24.11.1", "24.22.2"
        ]
    );
    assert_eq!(
        selected(&row(&[], &[], Some(1))),
        ["20.19.5", "22.23.4", "24.22.2"]
    );
    // More than a major holds is not an error; it keeps what is there.
    assert_eq!(selected(&row(&[], &[], Some(99))).len(), 7);
}

#[test]
fn newest_applies_after_the_other_filters() {
    // `>=24` leaves three, `newest = 1` leaves the newest of them.
    assert_eq!(selected(&row(&[">=24"], &[], Some(1))), ["24.22.2"]);
    // And it applies per major to the union with a channel.
    assert_eq!(
        selected(&row(&[">=24"], &["lts"], Some(1))),
        ["20.19.5", "22.23.4", "24.22.2"]
    );
}

#[test]
fn a_requirement_matching_nothing_selects_nothing_and_is_not_an_error() {
    assert!(selected(&row(&[">=99"], &[], None)).is_empty());
    assert!(selected(&row(&[">=99"], &[], Some(3))).is_empty());
}

#[test]
fn newest_per_major_takes_the_tail_of_each_major() {
    let all: BTreeSet<semver::Version> = domain().into_iter().collect();
    let kept = newest_per_major(&all, 2);
    assert_eq!(
        kept.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [
            "20.1.0", "20.19.5", "22.14.0", "22.23.4", "24.11.1", "24.22.2"
        ]
    );
    // A prerelease sorts below its release, as semver orders it.
    let with_pre: BTreeSet<semver::Version> = ["1.0.0-rc.1", "1.0.0", "1.1.0"]
        .iter()
        .map(|s| version(s))
        .collect();
    assert_eq!(
        newest_per_major(&with_pre, 2)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["1.0.0", "1.1.0"]
    );
}
