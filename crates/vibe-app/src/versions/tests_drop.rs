//! The `in drop` mark of a listing, at the library seam (PROP-066
//! `#drops`): which rows carry it, that `--in-drop` narrows before the
//! limit the way `PATTERN` does, and that a `Domains` which knows nothing
//! of drops marks nothing.
//!
//! The probe itself — the file names an option downloads, the drop's
//! rungs — belongs to the adapter that owns a filesystem
//! (`vibe-cli`'s `commands/app/versions/drop.rs`); here the seam is
//! stated outright, which is what lets the **economy** be asserted by
//! counting the rows it was asked about.

use std::cell::RefCell;

use super::super::*;
use super::{Stated, home, node_domain, request};

/// A domain that also answers the drop probe, counting how many rows were
/// asked — the economy the listing promises is checkable only by counting.
struct Dropped {
    inner: Stated,
    /// The versions the drop holds every input of, with the paths.
    held: &'static [(&'static str, &'static [&'static str])],
    asked: RefCell<Vec<String>>,
}

impl Domains for Dropped {
    fn domain(&self, group: &str, name: &str) -> Result<Domain, String> {
        self.inner.domain(group, name)
    }

    fn in_drop(&self, _group: &str, _name: &str, version: &semver::Version) -> Option<Vec<String>> {
        self.asked.borrow_mut().push(version.to_string());
        self.held
            .iter()
            .find(|(held, _)| *held == version.to_string())
            .map(|(_, paths)| paths.iter().map(|path| (*path).to_owned()).collect())
    }
}

fn dropped(held: &'static [(&'static str, &'static [&'static str])]) -> Dropped {
    Dropped {
        inner: node_domain(),
        held,
        asked: RefCell::default(),
    }
}

/// The drop mark rides on the row it describes, and `--in-drop` narrows
/// **before** the limit, so `total`, `shown` and `more` describe the
/// narrowed set exactly as `PATTERN`'s do (PROP-066 `#drops`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
fn in_drop_narrows_before_the_limit_and_the_mark_rides_on_its_row() {
    let index = home(&[], None);
    let held: &[(&str, &[&str])] = &[
        ("24.22.2", &["/d/org.vibevm.node/node/24.22.2/node.tgz"]),
        ("12.0.0", &["/d/node-12.tgz"]),
    ];
    // Without the filter every row is listed and only the held ones carry
    // the paths.
    let domains = dropped(held);
    let listing = versions_listing(&index, &request("org.vibevm.node/node"), &domains).unwrap();
    let package = &listing.packages[0];
    assert_eq!((package.total, package.shown), (4, 4));
    let marked: Vec<(&str, usize)> = package
        .versions
        .iter()
        .map(|row| (row.version.as_str(), row.drop.len()))
        .collect();
    assert_eq!(
        marked,
        [
            ("26.13.0", 0),
            ("24.22.2", 1),
            ("22.23.4", 0),
            ("12.0.0", 1)
        ]
    );
    assert_eq!(
        package.versions[1].drop,
        ["/d/org.vibevm.node/node/24.22.2/node.tgz"]
    );

    // With the filter the narrowed set is what the counts describe, and a
    // limit below it counts the rest of the *narrowed* set.
    let narrowed = VersionsRequest {
        in_drop: true,
        limit: Some(1),
        ..request("org.vibevm.node/node")
    };
    let domains = dropped(held);
    let listing = versions_listing(&index, &narrowed, &domains).unwrap();
    let package = &listing.packages[0];
    assert_eq!((package.total, package.shown, package.more), (2, 1, 1));
    assert_eq!(package.versions[0].version, "24.22.2");
    assert_eq!(package.versions[0].drop.len(), 1);
    // The filter had to ask every selected row, because its answer decides
    // which rows there are at all.
    assert_eq!(domains.asked.borrow().len(), 4);
}

/// Without `--in-drop` the probe is asked only for the rows that are
/// actually shown: the versions the limit cut are never looked for on
/// disk (PROP-075 `#VERSIONS-LIST-OUTPUT`'s cost budget).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
fn the_drop_is_asked_only_about_the_rows_a_listing_shows() {
    let index = home(&[], None);
    let limited = VersionsRequest {
        limit: Some(2),
        ..request("org.vibevm.node/node")
    };
    let domains = dropped(&[]);
    let listing = versions_listing(&index, &limited, &domains).unwrap();
    assert_eq!(listing.packages[0].total, 4);
    assert_eq!(*domains.asked.borrow(), ["26.13.0", "24.22.2"]);
}

/// A `Domains` that knows nothing of drops answers no mark at all — the
/// default of the seam, so every existing implementor is unchanged.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-066#drops")]
fn a_domain_source_that_knows_no_drop_marks_nothing() {
    let index = home(&["22.23.4"], None);
    let listing = versions_listing(&index, &request("org.vibevm.node/node"), &node_domain())
        .expect("the listing");
    assert!(
        listing.packages[0]
            .versions
            .iter()
            .all(|row| row.drop.is_empty())
    );
    // And the filter over such a source narrows to nothing rather than
    // inventing a mark: the refusal for "no drop configured" is the
    // caller's, before the listing is asked at all.
    let narrowed = VersionsRequest {
        in_drop: true,
        ..request("org.vibevm.node/node")
    };
    let listing = versions_listing(&index, &narrowed, &node_domain()).unwrap();
    assert_eq!(listing.packages[0].total, 0);
}
