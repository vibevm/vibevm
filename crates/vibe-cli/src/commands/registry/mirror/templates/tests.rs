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
        version_list: VersionList::default(),
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

/// `version_list = "all"` reports the source's own channel table, which
/// is what a client computing over the untrimmed document will find.
#[test]
fn version_list_all_reports_the_sources_channel_table() {
    let channels = channels();
    let selected = vec![version("24.22.2")];
    assert_eq!(
        listed_channels(VersionList::All, &channels, &selected),
        channels
    );
}

/// `version_list = "selected"` reports the channel table over the trimmed
/// list — the one a client behind the mirror will actually recompute — so
/// the row an operator reads cannot promise a member the mirror did not
/// carry. A channel left with nothing is kept as an empty set, which
/// `resolved_channels` then reports as "nothing".
#[test]
fn version_list_selected_reports_the_channel_table_over_the_trimmed_list() {
    let channels = channels();
    let selected = vec![version("22.23.4")];
    let trimmed = listed_channels(VersionList::Selected, &channels, &selected);
    assert_eq!(
        trimmed["lts"]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["22.23.4"]
    );
    assert!(
        trimmed.contains_key("lts/jod"),
        "a channel the trim emptied is still named: {trimmed:?}"
    );
    let row = row(&["=22.23.4"], &["lts"], None);
    assert_eq!(
        resolved_channels(&row, &trimmed),
        vec![("lts".to_owned(), Some(version("22.23.4")))]
    );
}

/// The case the intersection exists for: `newest` is applied **after** a
/// channel's members are unioned in, so it can drop the channel's newest
/// member back out — and then the reported head is the one the mirror
/// kept, not upstream's.
#[test]
fn newest_can_drop_a_channel_member_and_the_report_follows_the_trim() {
    let mut channels = channels();
    channels.insert(
        "lts".to_owned(),
        ["22.23.4", "22.23.9"]
            .iter()
            .map(|spelling| version(spelling))
            .collect(),
    );
    let domain = vec![version("22.23.4"), version("22.23.9")];
    let row = row(&[], &["lts"], Some(1));
    let selected = select_versions(&row, &domain, &channels);
    assert_eq!(selected, vec![version("22.23.9")]);
    let trimmed = listed_channels(VersionList::Selected, &channels, &selected);
    assert_eq!(
        resolved_channels(&row, &trimmed),
        vec![("lts".to_owned(), Some(version("22.23.9")))]
    );
}
