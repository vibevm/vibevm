//! The channel grammar, the semver-first precedence and the fail-closed
//! invariant of PROP-075 `#CHANNEL-SYNTAX`.

use super::*;
use crate::ChannelName;

struct Fixture {
    domain: Vec<semver::Version>,
    lts: Vec<semver::Version>,
    broken: bool,
}

impl Fixture {
    fn new() -> Self {
        Fixture {
            domain: ["22.9.0", "24.1.0", "25.0.0-rc.1"]
                .iter()
                .map(|v| v.parse().unwrap())
                .collect(),
            lts: ["20.1.0", "22.9.0"]
                .iter()
                .map(|v| v.parse().unwrap())
                .collect(),
            broken: false,
        }
    }
}

impl ChannelResolver for Fixture {
    fn domain(&self, _group: &Group, _name: &str) -> Result<Vec<semver::Version>, String> {
        if self.broken {
            return Err("the version list could not be read".to_string());
        }
        Ok(self.domain.clone())
    }

    fn members(
        &self,
        _group: &Group,
        _name: &str,
        channel: &ChannelName,
    ) -> Result<Option<Vec<semver::Version>>, String> {
        if self.broken {
            return Err("the version list could not be read".to_string());
        }
        Ok(match channel.as_str() {
            "lts" => Some(self.lts.clone()),
            "lts/jod" => Some(vec!["22.9.0".parse().unwrap()]),
            "nightly" => Some(Vec::new()),
            _ => None,
        })
    }
}

fn pkgref(spelling: &str) -> PackageRef {
    PackageRef::parse(spelling).unwrap()
}

#[test]
fn semver_wins_over_the_channel_grammar() {
    // Every one of these parses as a `semver::VersionReq` today and must
    // keep doing so, or an existing manifest changes meaning.
    for spelling in ["^22", "=1.2.3", "1.x", "~0.3.1", ">=0.2, <1.0", "0.3.0"] {
        let spec = VersionSpec::parse(spelling).unwrap();
        assert!(
            matches!(spec, VersionSpec::Req(_)),
            "`{spelling}` must stay a version requirement"
        );
    }
}

#[test]
fn a_channel_name_that_is_no_requirement_is_a_channel() {
    for spelling in ["lts", "lts/jod", "nightly", "latest", "stable"] {
        let spec = VersionSpec::parse(spelling).unwrap();
        assert_eq!(
            spec.channel().map(|c| c.as_str()),
            Some(spelling),
            "`{spelling}` must be a channel"
        );
        assert_eq!(spec.to_string(), spelling, "a channel round-trips");
    }
}

#[test]
fn neither_a_requirement_nor_a_channel_is_still_a_refusal() {
    for spelling in ["LTS", "lts/jod/extra", "not a channel", "@@"] {
        assert!(
            VersionSpec::parse(spelling).is_err(),
            "`{spelling}` must refuse"
        );
    }
}

#[test]
fn an_unlowered_channel_matches_nothing() {
    // The fail-closed invariant: a path that forgot to lower resolves
    // nothing rather than something plausible.
    let spec = VersionSpec::parse("lts").unwrap();
    for version in ["22.9.0", "24.1.0", "0.0.1"] {
        assert!(!spec.matches(&version.parse().unwrap()));
    }
    assert!(!spec.is_exact_pin());
}

#[test]
fn lowering_pins_the_newest_member_and_reports_the_channel() {
    let mut reference = pkgref("org.vibevm.node/node@lts");
    let via = lower_channel(&mut reference, &Fixture::new()).unwrap();
    assert_eq!(via.unwrap().as_str(), "lts");
    assert_eq!(reference.version.to_string(), "=22.9.0");
    assert!(reference.version.is_exact_pin());
}

#[test]
fn a_second_segment_narrows_the_set() {
    let mut reference = pkgref("org.vibevm.node/node@lts/jod");
    lower_channel(&mut reference, &Fixture::new()).unwrap();
    assert_eq!(reference.version.to_string(), "=22.9.0");
}

#[test]
fn latest_is_the_newest_of_the_domain_and_stable_skips_a_prerelease() {
    let mut newest = pkgref("org.vibevm.node/node@latest");
    lower_channel(&mut newest, &Fixture::new()).unwrap();
    assert_eq!(newest.version.to_string(), "=25.0.0-rc.1");

    let mut stable = pkgref("org.vibevm.node/node@stable");
    lower_channel(&mut stable, &Fixture::new()).unwrap();
    assert_eq!(stable.version.to_string(), "=24.1.0");
}

#[test]
fn an_undeclared_channel_refuses_by_name() {
    let mut reference = pkgref("org.vibevm.node/node@canary");
    let error = lower_channel(&mut reference, &Fixture::new()).unwrap_err();
    assert!(matches!(error, ChannelError::UnknownChannel { .. }));
    assert!(error.to_string().contains("canary"));
    assert!(error.to_string().contains("org.vibevm.node/node"));
}

#[test]
fn a_declared_but_empty_channel_refuses_apart_from_an_unknown_one() {
    let mut reference = pkgref("org.vibevm.node/node@nightly");
    let error = lower_channel(&mut reference, &Fixture::new()).unwrap_err();
    assert!(matches!(error, ChannelError::EmptyChannel { .. }));
}

#[test]
fn a_resolver_failure_is_reported_with_its_own_reason() {
    let mut fixture = Fixture::new();
    fixture.broken = true;
    let mut reference = pkgref("org.vibevm.node/node@lts");
    let error = lower_channel(&mut reference, &fixture).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("the version list could not be read")
    );
}

#[test]
fn a_short_reference_with_a_channel_refuses_rather_than_guessing() {
    let mut reference = pkgref("node@lts");
    let error = lower_channel(&mut reference, &Fixture::new()).unwrap_err();
    assert!(matches!(error, ChannelError::Unqualified { .. }));
}

#[test]
fn a_requirement_that_names_no_channel_is_left_alone() {
    let mut reference = pkgref("org.vibevm.node/node@^22");
    assert!(
        lower_channel(&mut reference, &Fixture::new())
            .unwrap()
            .is_none()
    );
    assert_eq!(reference.version.to_string(), "^22");

    let mut latest = pkgref("org.vibevm.node/node");
    assert!(
        lower_channel(&mut latest, &Fixture::new())
            .unwrap()
            .is_none()
    );
    assert_eq!(latest.version, VersionSpec::Latest);
}
