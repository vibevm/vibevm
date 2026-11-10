//! The listing `vibe app versions` draws: the union of a domain with this
//! home's installations, the origin marks, the channel memberships, the
//! local state, the limit and the two refusals of an unresolvable
//! argument (PROP-075 `#VERSIONS-LIST-OUTPUT`).

use std::collections::BTreeMap;

use super::*;
use crate::index::{Location, Method, PackageCoordinate, SlotState};

/// A domain stated outright, standing in for whatever a registry answers.
struct Stated {
    versions: Vec<(&'static str, Origin)>,
    channels: &'static [(&'static str, &'static [&'static str])],
    template: bool,
}

impl Domains for Stated {
    fn domain(&self, group: &str, name: &str) -> Result<Domain, String> {
        let mut domain = Domain {
            versions: self
                .versions
                .iter()
                .map(|(version, origin)| DomainVersion {
                    version: version.parse().expect("a version"),
                    origin: *origin,
                })
                .collect(),
            ..Domain::default()
        };
        for (channel, members) in self.channels {
            domain.channels.insert(
                (*channel).to_owned(),
                members
                    .iter()
                    .map(|member| member.parse().expect("a version"))
                    .collect(),
            );
        }
        if self.template {
            domain.template = Some(TemplateFacts {
                coordinate: format!("{group}/{name}"),
                revision: "1.0.0".to_owned(),
                fetched_at: Some("2026-11-10T00:00:00Z".to_owned()),
                ..TemplateFacts::default()
            });
        }
        Ok(domain)
    }
}

/// A domain source that refuses, the way a registry that cannot be read
/// does.
struct Denied;

impl Domains for Denied {
    fn domain(&self, _group: &str, _name: &str) -> Result<Domain, String> {
        Err("the test denies every registry read".to_owned())
    }
}

/// The empty domain of a home with nothing to consult.
struct Nothing;

impl Domains for Nothing {
    fn domain(&self, _group: &str, _name: &str) -> Result<Domain, String> {
        Ok(Domain::default())
    }
}

fn installation(slot: &str, group: &str, name: &str, version: &str) -> (String, Installation) {
    let package = PackageCoordinate {
        group: group.to_owned(),
        name: name.to_owned(),
        version: version.to_owned(),
    };
    let key = package.key();
    (
        key,
        Installation {
            source_evidence: None,
            secret_origins: BTreeMap::new(),
            configuration: None,
            pending: None,
            slot: slot.to_owned(),
            package,
            option: "main".to_owned(),
            platform: "linux".to_owned(),
            arch: "x86_64".to_owned(),
            method: Method::TarGz,
            location: Location::Owned {
                library: "default".to_owned(),
                directory: "d".to_owned(),
            },
            portable: true,
            launchers: Vec::new(),
            public_commands: Vec::new(),
            path_dirs: vec!["bin".to_owned()],
            version_file: Vec::new(),
            template: None,
            migrations: Vec::new(),
            migrated: Vec::new(),
            extras: Vec::new(),
            commands: Default::default(),
            features: Vec::new(),
            parameters: BTreeMap::new(),
            license: None,
            icon: None,
            menu: Default::default(),
            steps: Vec::new(),
            lifecycle: Default::default(),
        },
    )
}

/// A home with `node` provided by `org.vibevm.node/node` at `versions`,
/// defaulting to `default`.
fn home(versions: &[&str], default: Option<&str>) -> Index {
    let mut index = Index::default();
    for version in versions {
        let (key, record) = installation("node", "org.vibevm.node", "node", version);
        index.installations.insert(key, record);
    }
    index.slots.insert(
        "node".to_owned(),
        SlotState {
            default: default.map(str::to_owned),
            off: false,
            path_env: Vec::new(),
        },
    );
    index
}

fn request<'a>(target: &'a str) -> VersionsRequest<'a> {
    VersionsRequest {
        target,
        ..VersionsRequest::default()
    }
}

fn node_domain() -> Stated {
    Stated {
        versions: vec![
            ("26.13.0", Origin::Template),
            ("24.22.2", Origin::Overrides),
            ("22.23.4", Origin::Template),
            ("12.0.0", Origin::Template),
        ],
        channels: &[("lts", &["26.13.0", "22.23.4"]), ("lts/jod", &["22.23.4"])],
        template: true,
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn every_version_is_listed_newest_first_with_its_origin_and_channels() {
    let index = home(&["22.23.4"], Some("org.vibevm.node/node@22.23.4"));
    let listing = versions_listing(&index, &request("node"), &node_domain()).unwrap();
    assert_eq!(listing.protocol, VERSIONS_PROTOCOL);
    assert_eq!(listing.slot.as_deref(), Some("node"));
    assert_eq!(
        listing.default.as_deref(),
        Some("org.vibevm.node/node@22.23.4")
    );
    assert!(!listing.off);
    let package = &listing.packages[0];
    assert_eq!(package.package, "org.vibevm.node/node");
    assert_eq!(package.total, 4);
    assert_eq!(package.shown, 4);
    assert_eq!(package.more, 0);
    let versions: Vec<&str> = package
        .versions
        .iter()
        .map(|row| row.version.as_str())
        .collect();
    assert_eq!(versions, ["26.13.0", "24.22.2", "22.23.4", "12.0.0"]);
    // The origin of each row is the domain's verdict, including the
    // published row that overrides a produced version.
    assert_eq!(package.versions[0].origin, wire::Origin::Template);
    assert_eq!(package.versions[1].origin, wire::Origin::Overrides);
    // The channels a version belongs to, in name order.
    assert_eq!(package.versions[0].channels, ["lts"]);
    assert_eq!(package.versions[2].channels, ["lts", "lts/jod"]);
    assert!(package.versions[1].channels.is_empty());
    // The resolved channel table, newest member first.
    assert_eq!(
        package.channels.get("lts").map(Vec::as_slice),
        Some(["26.13.0".to_owned(), "22.23.4".to_owned()].as_slice())
    );
    // The template provenance a `--json` client reads.
    let template = package.template.as_ref().expect("a template");
    assert_eq!(template.coordinate, "org.vibevm.node/node");
    assert_eq!(template.revision, "1.0.0");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn the_local_state_of_each_version_is_the_home_s_own() {
    let mut index = home(
        &["26.13.0", "24.22.2", "22.23.4"],
        Some("org.vibevm.node/node@26.13.0"),
    );
    index
        .installations
        .get_mut("org.vibevm.node/node@24.22.2")
        .unwrap()
        .lifecycle
        .hidden = true;
    index
        .installations
        .get_mut("org.vibevm.node/node@22.23.4")
        .unwrap()
        .lifecycle
        .held = true;
    let listing = versions_listing(&index, &request("node"), &node_domain()).unwrap();
    let states: Vec<&wire::State> = listing.packages[0]
        .versions
        .iter()
        .map(|row| &row.state)
        .collect();
    assert_eq!(
        states,
        [
            &wire::State::InstalledDefault,
            &wire::State::InstalledHidden,
            &wire::State::InstalledHeld,
            // v12.0.0 is nothing this home holds.
            &wire::State::Available,
        ]
    );
    assert_eq!(
        listing.packages[0].versions[0].key.as_deref(),
        Some("org.vibevm.node/node@26.13.0")
    );
    assert!(listing.packages[0].versions[3].key.is_none());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_slot_that_is_off_says_so_in_the_trailer() {
    let mut index = home(&["22.23.4"], Some("org.vibevm.node/node@22.23.4"));
    index.slots.get_mut("node").unwrap().off = true;
    let listing = versions_listing(&index, &request("node"), &node_domain()).unwrap();
    assert!(listing.off);
    // The remembered default is still named: `vibe app use node --default`
    // is what puts it back (PROP-059 `#slot-off`).
    assert_eq!(
        listing.default.as_deref(),
        Some("org.vibevm.node/node@22.23.4")
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn the_limit_shows_the_newest_rows_and_counts_the_rest() {
    let index = home(&[], None);
    let limited = VersionsRequest {
        limit: Some(2),
        ..request("org.vibevm.node/node")
    };
    let listing = versions_listing(&index, &limited, &node_domain()).unwrap();
    let package = &listing.packages[0];
    assert_eq!((package.total, package.shown, package.more), (4, 2, 2));
    assert_eq!(package.versions[0].version, "26.13.0");
    // `--all` shows every one of them and counts nothing out.
    let all = VersionsRequest {
        all: true,
        limit: Some(2),
        ..request("org.vibevm.node/node")
    };
    let listing = versions_listing(&index, &all, &node_domain()).unwrap();
    assert_eq!(
        (listing.packages[0].shown, listing.packages[0].more),
        (4, 0)
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn a_channel_narrows_the_listing_and_an_undeclared_one_is_refused_by_name() {
    let index = home(&[], None);
    let lts = VersionsRequest {
        channel: Some("lts/jod"),
        ..request("org.vibevm.node/node")
    };
    let listing = versions_listing(&index, &lts, &node_domain()).unwrap();
    assert_eq!(listing.packages[0].total, 1);
    assert_eq!(listing.packages[0].versions[0].version, "22.23.4");
    let unknown = VersionsRequest {
        channel: Some("lts/argon"),
        ..request("org.vibevm.node/node")
    };
    let error = versions_listing(&index, &unknown, &node_domain()).unwrap_err();
    assert_eq!(error.code(), "unknown_channel");
    let message = error.to_string();
    assert!(
        message.contains("lts/argon") && message.contains("`lts/jod`"),
        "{message}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn installed_keeps_only_what_this_home_holds() {
    let index = home(&["22.23.4", "12.0.0"], Some("org.vibevm.node/node@22.23.4"));
    let installed = VersionsRequest {
        installed: true,
        ..request("node")
    };
    let listing = versions_listing(&index, &installed, &node_domain()).unwrap();
    let versions: Vec<&str> = listing.packages[0]
        .versions
        .iter()
        .map(|row| row.version.as_str())
        .collect();
    assert_eq!(versions, ["22.23.4", "12.0.0"]);
    // And with nothing at all to consult the answer is the same, which is
    // what makes `--installed` free of a registry (PROP-075
    // `#VERSIONS-LIST-OUTPUT`).
    let listing = versions_listing(&index, &installed, &Nothing).unwrap();
    let versions: Vec<&str> = listing.packages[0]
        .versions
        .iter()
        .map(|row| row.version.as_str())
        .collect();
    assert_eq!(versions, ["22.23.4", "12.0.0"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST")]
fn an_installation_the_domain_no_longer_carries_is_still_listed() {
    let index = home(&["18.20.0"], Some("org.vibevm.node/node@18.20.0"));
    let listing = versions_listing(&index, &request("node"), &node_domain()).unwrap();
    let package = &listing.packages[0];
    assert_eq!(package.total, 5);
    let row = package
        .versions
        .iter()
        .find(|row| row.version == "18.20.0")
        .expect("the installed version nothing publishes any more");
    assert_eq!(row.state, wire::State::InstalledDefault);
    assert_eq!(row.origin, wire::Origin::Published);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_slot_with_two_packages_lists_each_under_its_own_heading() {
    let mut index = home(&["22.23.4"], Some("org.vibevm.node/node@22.23.4"));
    let (key, record) = installation("node", "org.example", "nodejs", "20.11.0");
    index.installations.insert(key, record);
    let listing = versions_listing(&index, &request("node"), &Nothing).unwrap();
    let headings: Vec<&str> = listing
        .packages
        .iter()
        .map(|package| package.package.as_str())
        .collect();
    assert_eq!(headings, ["org.example/nodejs", "org.vibevm.node/node"]);
    assert_eq!(listing.packages[0].total, 1);
    assert_eq!(listing.packages[1].total, 1);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_bare_name_that_is_neither_a_slot_nor_one_package_is_refused_by_name() {
    let mut index = home(&["22.23.4"], None);
    // Two groups publishing a `tool`, both installed in another slot.
    for group in ["org.example", "org.other"] {
        let (key, record) = installation("tool", group, "tool", "1.0.0");
        index.installations.insert(key, record);
    }
    // `tool` is a slot here, so it lists both headings rather than refusing.
    let listing = versions_listing(&index, &request("tool"), &Nothing).unwrap();
    assert_eq!(listing.packages.len(), 2);

    // A name that is only a package name, held by two groups, is the
    // refusal that names the spellings.
    let mut index = Index::default();
    for group in ["org.example", "org.other"] {
        let (key, record) = installation(&format!("{group}-tool"), group, "tool", "1.0.0");
        index.installations.insert(key, record);
    }
    let error = versions_listing(&index, &request("tool"), &Nothing).unwrap_err();
    assert_eq!(error.code(), "ambiguous_target");
    let message = error.to_string();
    assert!(
        message.contains("`org.example/tool`") && message.contains("`org.other/tool`"),
        "{message}"
    );

    // And a word that is nothing at all names the slots that do exist.
    let index = home(&["22.23.4"], None);
    let error = versions_listing(&index, &request("nodejs"), &Nothing).unwrap_err();
    assert_eq!(error.code(), "unknown_target");
    assert!(error.to_string().contains("`node`"), "{error}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST")]
fn a_domain_that_cannot_be_read_is_a_refusal_naming_the_package() {
    let index = home(&["22.23.4"], None);
    let error = versions_listing(&index, &request("node"), &Denied).unwrap_err();
    assert_eq!(error.code(), "version_domain_unreadable");
    let message = error.to_string();
    assert!(
        message.contains("org.vibevm.node/node") && message.contains("denies"),
        "{message}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_coordinate_names_one_package_whether_or_not_it_is_installed() {
    let index = Index::default();
    let listing =
        versions_listing(&index, &request("org.vibevm.node/node"), &node_domain()).unwrap();
    assert_eq!(listing.packages.len(), 1);
    // Nothing installed means no slot and no default, and every row is
    // `available`.
    assert!(listing.slot.is_none() && listing.default.is_none());
    assert!(
        listing.packages[0]
            .versions
            .iter()
            .all(|row| row.state == wire::State::Available)
    );
    // A version pinned onto the coordinate names the same package — and
    // narrows it, because the `@VER` a user already writes is the same
    // filter the positional pattern is (PROP-075 `#VERSIONS-LIST-OUTPUT`).
    let listing =
        versions_listing(&index, &request("org.vibevm.node/node@22"), &node_domain()).unwrap();
    assert_eq!(listing.packages[0].package, "org.vibevm.node/node");
    assert_eq!(listing.packages[0].total, 1);
    assert_eq!(listing.packages[0].versions[0].version, "22.23.4");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSIONS-LIST-OUTPUT")]
fn a_pattern_narrows_the_listing_by_prefix_and_a_bad_one_is_refused_by_name() {
    let index = home(&["22.23.4", "12.0.0"], Some("org.vibevm.node/node@22.23.4"));
    let listing = |pattern: &str, installed: bool| {
        let narrowed = VersionsRequest {
            pattern: Some(pattern),
            installed,
            ..request("node")
        };
        versions_listing(&index, &narrowed, &node_domain()).map(|listing| {
            listing.packages[0]
                .versions
                .iter()
                .map(|row| row.version.clone())
                .collect::<Vec<_>>()
        })
    };
    // The prefix rule a version file's value follows, over both listings.
    assert_eq!(listing("22", false).unwrap(), ["22.23.4"]);
    assert_eq!(listing("22.23", false).unwrap(), ["22.23.4"]);
    assert_eq!(listing("22.23.4", false).unwrap(), ["22.23.4"]);
    assert_eq!(listing("24", false).unwrap(), ["24.22.2"]);
    assert!(listing("23", false).unwrap().is_empty());
    assert_eq!(listing("22", true).unwrap(), ["22.23.4"]);
    assert!(listing("24", true).unwrap().is_empty());
    // A requirement works as written, and a channel name narrows exactly as
    // `--channel` does.
    assert_eq!(listing("^22", false).unwrap(), ["22.23.4"]);
    assert_eq!(listing("lts", false).unwrap(), ["26.13.0", "22.23.4"]);
    assert_eq!(listing("lts/jod", false).unwrap(), ["22.23.4"]);
    // A pattern that is neither refuses by name, before any version is
    // listed.
    let error = listing("22..", false).unwrap_err();
    assert_eq!(error.code(), "bad_version_pattern");
    assert!(error.to_string().contains("22.."), "{error}");
    // And a channel the package does not declare is the same refusal the
    // flag gives, because a typo must not read as an empty domain.
    let error = listing("lts/argon", false).unwrap_err();
    assert_eq!(error.code(), "unknown_channel");
    assert!(error.to_string().contains("lts/argon"), "{error}");
    // The positional pattern wins over a target's `@VER` suffix, so the one
    // a user typed last decides.
    let both = VersionsRequest {
        pattern: Some("24"),
        ..request("org.vibevm.node/node@22")
    };
    let listing = versions_listing(&index, &both, &node_domain()).unwrap();
    assert_eq!(listing.packages[0].versions[0].version, "24.22.2");
}
