//! `vibe registry mirror --config` snapshotting a template's external
//! inputs into the target's `_index/`, and a client inside a closed
//! network rendering from that snapshot — campaign phase P4 commit (b)
//! (`AV-FMT-008`), PROP-075 `#MIRROR-TEMPLATE-SNAPSHOT`,
//! `#MIRROR-SNAPSHOT-LAYOUT`.
//!
//! The template is the production `org.vibevm.node/node` manifest and the
//! digest documents are upstream's own `SHASUMS256.txt`, both the same
//! frozen fixtures P3's render-fidelity suite uses. The version list here
//! is a **four-record** subset of upstream's own index, built from the
//! same frozen document: this suite is about selection and snapshotting,
//! and every selected version is fully rendered, so the set must be the
//! versions whose digest documents the fixtures carry. P3's
//! `cli_template_node` keeps the whole 410-record window for the
//! questions that need it.
//!
//! The one test-shaped thing is a `[[remap]]` row reading
//! `https://nodejs.org/dist/` off this filesystem — the mechanism a
//! closed network uses anyway (PROP-066 `#locator`), so **no run here
//! opens a socket**. The network-denied client has no such row and runs
//! `--offline`: the only place its inputs can come from is the snapshot.
#![cfg(unix)]

mod common;

use std::fs;

use common::UserScratch;
use common::template_mirror::{
    DIGESTS, Fixture, PACKAGE, REVISION, mirror, ok, refusal, snapshot_versions, sync_last, vibe,
    walk,
};

const SELECT_24_AND_LTS: &str = r#"schema = 1
[[template]]
package = "org.vibevm.node/node"
versions = [">=24"]
channels = ["lts"]
"#;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-SNAPSHOT-LAYOUT")]
fn a_selected_template_writes_exactly_the_expected_snapshot_tree() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    let config = fixture.config(SELECT_24_AND_LTS);
    ok(&mirror(&user, &fixture, &config, &[]));

    // `>=24` is 26.13.0 and 24.22.2; the `lts` channel adds 22.23.4 and
    // leaves 12.0.0 out, because upstream says `lts: false` for it.
    assert_eq!(
        snapshot_versions(&fixture),
        ["22.23.4", "24.22.2", "26.13.0"]
    );

    // The three files of `#MIRROR-SNAPSHOT-LAYOUT`, and nothing else.
    let mut written: Vec<String> = walk(&fixture.snapshot())
        .iter()
        .map(|path| {
            path.strip_prefix(fixture.snapshot())
                .unwrap()
                .display()
                .to_string()
        })
        .collect();
    written.sort();
    assert_eq!(
        written,
        [
            "digests/22.23.4",
            "digests/24.22.2",
            "digests/26.13.0",
            "snapshot.toml",
            "versions.json",
        ]
    );

    // The digest documents are upstream's own bytes, verbatim.
    for (version, document) in DIGESTS {
        let path = fixture.snapshot().join("digests").join(version);
        if path.exists() {
            assert_eq!(fs::read_to_string(&path).unwrap(), document, "{version}");
        }
    }

    // `versions.json` is the client cache's envelope, restricted to the
    // selected versions, with its `sha256` over the FILTERED document.
    let envelope: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture.snapshot().join("versions.json")).unwrap(),
    )
    .unwrap();
    let document = envelope["document"].as_str().unwrap();
    let records: Vec<serde_json::Value> = serde_json::from_str(document).unwrap();
    let kept: Vec<&str> = records
        .iter()
        .map(|record| record["version"].as_str().unwrap())
        .collect();
    assert_eq!(kept, ["v26.13.0", "v24.22.2", "v22.23.4"]);
    assert_eq!(
        envelope["sha256"].as_str().unwrap(),
        format!(
            "{:x}",
            <sha2::Sha256 as sha2::Digest>::digest(document.as_bytes())
        )
    );
    // A kept record is byte-verbatim: upstream's own `lts` codename.
    assert_eq!(records[2]["lts"], "Jod");

    // `[source]` carries the urls as the template DECLARES them, before
    // remap — the remap row is this operator's route, not the template's.
    let record: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.snapshot().join("snapshot.toml")).unwrap())
            .unwrap();
    assert_eq!(record["schema"].as_integer(), Some(1));
    assert_eq!(record["source"]["package"].as_str(), Some(PACKAGE));
    assert_eq!(record["source"]["revision"].as_str(), Some(REVISION));
    assert_eq!(
        record["source"]["version_list_url"].as_str(),
        Some("https://nodejs.org/dist/index.json")
    );
    assert!(
        record["source"]["digest_url"]
            .as_str()
            .unwrap()
            .starts_with("https://nodejs.org/dist/"),
        "{record:?}"
    );
    // `[sync]` is there for `vibe app versions` to report.
    assert!(record["sync"]["last"].as_str().unwrap().contains('T'));

    // The template package itself was copied beside its snapshot, with
    // its whole tree: a client needs the recipes and documents too.
    assert!(
        fixture
            .target()
            .join(PACKAGE)
            .join(format!("v{REVISION}"))
            .join("README.md")
            .is_file()
    );

    // `_index/templates/` does not belong to the listing, which is
    // rendered over the version directories alone — so the snapshot
    // cannot make the index read stale. The plan carried the opposite as
    // a risk; `#P4-AS-BUILT` amendment 6 withdraws it, and the identical
    // re-run below is the end-to-end half of that closure (the focused
    // half is `vibe_core::layout_registry`'s own test).
    let listing = fs::read_to_string(fixture.target().join("_index/listing.txt")).unwrap();
    assert!(!listing.contains("templates"), "{listing}");

    let again = UserScratch::new();
    let repeat: serde_json::Value =
        serde_json::from_str(&ok(&mirror(&again, &fixture, &config, &[]))).unwrap();
    assert_eq!(repeat["facts_recorded"], 0, "{repeat}");
    assert_eq!(repeat["index_refreshed"], false, "{repeat}");
    assert_eq!(
        snapshot_versions(&fixture),
        ["22.23.4", "24.22.2", "26.13.0"]
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-TEMPLATE-SNAPSHOT"
)]
fn a_client_with_only_the_target_and_no_network_renders_from_the_snapshot() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    let config = fixture.config(SELECT_24_AND_LTS);
    ok(&mirror(&user, &fixture, &config, &[]));

    // A fresh home: nothing of the mirror run's own template cache is
    // shared with the client, so the snapshot is the only place its
    // inputs can come from.
    let inside = UserScratch::new();
    let closed = fixture.closed_project();
    // No `--remap` row and `--offline`: upstream is unreachable by
    // construction, exactly as it is inside a closed network.
    let listing = ok(&vibe(
        &inside,
        &closed,
        &[
            "--offline",
            "--json",
            "app",
            "versions",
            PACKAGE,
            "--registry",
            fixture.target().to_str().unwrap(),
            "--all",
        ],
    ));
    let listing: serde_json::Value = serde_json::from_str(&listing).unwrap();
    let package = &listing["packages"][0];
    let versions: Vec<&str> = package["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["version"].as_str().unwrap())
        .collect();
    assert_eq!(versions, ["26.13.0", "24.22.2", "22.23.4"], "{listing}");

    // The snapshot's `[sync] last` is reported, which is the field P5b
    // declared in the wire and left for this phase to fill.
    let synced = listing["snapshot_synced"]
        .as_str()
        .unwrap_or_else(|| panic!("no snapshot_synced in {listing}"));
    assert!(synced.contains('T'), "{synced}");
    assert_eq!(
        package["template"]["snapshot_synced"].as_str(),
        Some(synced)
    );
    assert_eq!(package["template"]["revision"].as_str(), Some(REVISION));

    // And a full render of a snapshotted version succeeds with no
    // network at all: the plan resolves the template, renders the
    // manifest and names the payload it would fetch.
    let plan = ok(&vibe(
        &inside,
        &closed,
        &[
            "--offline",
            "--json",
            "install",
            "-g",
            &format!("{PACKAGE}@=24.22.2"),
            "--registry",
            fixture.target().to_str().unwrap(),
            "--plan-only",
        ],
    ));
    assert!(plan.contains("24.22.2"), "{plan}");
    // The rendered manifest is in the client's own cache, carrying the
    // digest the snapshot's SHASUMS256.txt names.
    let rendered = inside
        .settings
        .join("cache/templates")
        .join(PACKAGE)
        .join("rendered/v24.22.2/vibe.toml");
    let rendered = fs::read_to_string(&rendered)
        .unwrap_or_else(|error| panic!("{}: {error}", rendered.display()));
    assert!(rendered.contains("version = \"24.22.2\""), "{rendered}");
    // Sixty-four zeros is `vibe_template::SKELETON_DIGEST`, the declared
    // placeholder a skeleton render leaves behind; a full render has none.
    assert!(
        !rendered.contains(&"0".repeat(64)),
        "a full render carries real digests, never the skeleton placeholder"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
fn a_changed_upstream_digest_for_a_snapshotted_version_stops_before_writing() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    let config = fixture.config(SELECT_24_AND_LTS);
    ok(&mirror(&user, &fixture, &config, &[]));
    let before = fs::read_to_string(fixture.snapshot().join("snapshot.toml")).unwrap();

    // Upstream replaced a released version's checksums — which may never
    // happen. The document still names every file the template needs, so
    // the version renders and the run reaches the comparison: one digest
    // moved, which is exactly the case the alarm exists for. A document
    // missing a file would fail earlier, as a render error.
    let real = DIGESTS
        .iter()
        .find(|(version, _)| *version == "24.22.2")
        .map(|(_, document)| *document)
        .unwrap();
    let tampered: String = real
        .lines()
        .map(|line| match line.contains("-linux-x64.tar.gz") {
            true => format!("{}{}\n", "deadbeef".repeat(8), &line[64..]),
            false => format!("{line}\n"),
        })
        .collect();
    assert_ne!(tampered, real, "the tampered document must differ");
    fs::write(fixture.dist().join("v24.22.2/SHASUMS256.txt"), &tampered).unwrap();
    // A fresh home, so the alarm is raised against the snapshot rather
    // than against the first run's own cached document (a cached digest
    // document is immutable and is never refetched).

    let again = UserScratch::new();
    let message = refusal(&mirror(&again, &fixture, &config, &[]));
    assert!(
        message.contains("has changed since this registry snapshotted it"),
        "{message}"
    );
    assert!(message.contains("24.22.2"), "{message}");
    assert!(message.contains("nothing was written"), "{message}");

    // Nothing was written: the snapshot is byte-identical to before.
    assert_eq!(
        fs::read_to_string(fixture.snapshot().join("snapshot.toml")).unwrap(),
        before
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-SNAPSHOT-LAYOUT")]
fn a_rerun_adds_versions_and_leaves_the_existing_ones_untouched() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    ok(&mirror(
        &user,
        &fixture,
        &fixture.config(
            "schema = 1\n[[template]]\npackage = 'org.vibevm.node/node'\nversions = ['=26.13.0']\n",
        ),
        &[],
    ));
    assert_eq!(snapshot_versions(&fixture), ["26.13.0"]);
    let first = fs::read_to_string(fixture.snapshot().join("digests/26.13.0")).unwrap();
    let synced_first = sync_last(&fixture);

    // A widened selection adds the new versions and keeps the old row.
    let again = UserScratch::new();
    ok(&mirror(
        &again,
        &fixture,
        &fixture.config(SELECT_24_AND_LTS),
        &[],
    ));
    assert_eq!(
        snapshot_versions(&fixture),
        ["22.23.4", "24.22.2", "26.13.0"]
    );
    assert_eq!(
        fs::read_to_string(fixture.snapshot().join("digests/26.13.0")).unwrap(),
        first
    );
    // `[sync] first` is kept and `last` moves.
    let record: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.snapshot().join("snapshot.toml")).unwrap())
            .unwrap();
    assert_eq!(
        record["sync"]["first"].as_str(),
        Some(synced_first.0.as_str())
    );
    assert!(record["sync"]["last"].as_str().unwrap() >= synced_first.1.as_str());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn a_row_selecting_nothing_is_reported_and_writes_no_snapshot() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    // A requirement no produced version satisfies.
    let config = fixture.config(
        "schema = 1\n[[template]]\npackage = 'org.vibevm.node/node'\nversions = ['>=99']\n",
    );
    ok(&mirror(&user, &fixture, &config, &[]));
    assert!(
        !fixture.snapshot().join("snapshot.toml").exists(),
        "a row that selected nothing has no snapshot to write"
    );
    // The template package itself is still copied: the row selects it.
    assert!(
        fixture
            .target()
            .join(PACKAGE)
            .join(format!("v{REVISION}"))
            .join("vibe.toml")
            .is_file()
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-DRY-RUN-OUTPUT")]
fn a_dry_run_prints_the_projection_and_writes_nothing() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    let config = fixture.config(SELECT_24_AND_LTS);
    let report: serde_json::Value =
        serde_json::from_str(&ok(&mirror(&user, &fixture, &config, &["--dry-run"]))).unwrap();
    assert_eq!(report["dry_run"], true);
    // A snapshot the target does not hold is a write the batch would
    // make, so the index is reported as one that would be refreshed.
    assert_eq!(report["index_refreshed"], true, "{report}");
    assert!(!fixture.snapshot().exists(), "a dry run writes nothing");
    assert!(!fixture.target().join(PACKAGE).exists());
}

#[test]
fn a_template_row_naming_a_package_with_no_template_revision_refuses_by_name() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    // An ordinary package, published concretely, with no revision.
    let dir = fixture.source().join("org.corp/plain/v1.0.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        "[package]\ngroup='org.corp'\nname='plain'\nkind='feat'\nversion='1.0.0'\n\
         description='The quokkaword plain package'\nkeywords=['quokkaword']\n",
    )
    .unwrap();
    let config = fixture.config("schema = 1\n[[template]]\npackage = 'org.corp/plain'\n");
    let message = refusal(&mirror(&user, &fixture, &config, &[]));
    assert!(message.contains("org.corp/plain"), "{message}");
    assert!(
        message.contains("publishes no template revision"),
        "{message}"
    );
}

/// The `[[template]]` configs the `version_list` cases use: a wide first
/// run, then a narrowed second one with each spelling of the knob.
const SELECT_24_UP: &str = r#"schema = 1
[[template]]
package = "org.vibevm.node/node"
versions = [">=24"]
"#;

const SELECT_26_LIST_ALL: &str = r#"schema = 1
[[template]]
package = "org.vibevm.node/node"
versions = [">=26"]
version_list = "all"
"#;

const SELECT_26_LIST_SELECTED: &str = r#"schema = 1
[[template]]
package = "org.vibevm.node/node"
versions = [">=26"]
version_list = "selected"
"#;

/// The versions the snapshot's `versions.json` carries, newest first as
/// upstream writes them — the document a client's whole domain is
/// computed from.
fn listed_versions(fixture: &Fixture) -> Vec<String> {
    let envelope: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture.snapshot().join("versions.json")).unwrap(),
    )
    .unwrap();
    let document: serde_json::Value =
        serde_json::from_str(envelope["document"].as_str().unwrap()).unwrap();
    document
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row["version"]
                .as_str()
                .unwrap()
                .trim_start_matches('v')
                .to_owned()
        })
        .collect()
}

/// `version_list = "all"` is the behaviour before the key existed: a
/// re-run that selects less leaves the published list alone, because the
/// record still names those versions and a client parses one document.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn version_list_all_keeps_every_version_the_record_still_names() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    ok(&mirror(&user, &fixture, &fixture.config(SELECT_24_UP), &[]));
    assert_eq!(listed_versions(&fixture), ["26.13.0", "24.22.2"]);

    ok(&mirror(
        &user,
        &fixture,
        &fixture.config(SELECT_26_LIST_ALL),
        &[],
    ));
    assert_eq!(
        snapshot_versions(&fixture),
        ["24.22.2", "26.13.0"],
        "the record keeps what it had: no --prune was asked for"
    );
    assert_eq!(
        listed_versions(&fixture),
        ["26.13.0", "24.22.2"],
        "and so does the list a client resolves within"
    );
}

/// `version_list = "selected"` trims the published list to what the row
/// now selects, with no `--prune`: the document is rewritten in place, so
/// a client behind the mirror sees — and resolves within, and computes
/// channels over — exactly the set the operator chose to carry.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn version_list_selected_trims_the_list_a_client_resolves_within() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    ok(&mirror(&user, &fixture, &fixture.config(SELECT_24_UP), &[]));
    assert_eq!(listed_versions(&fixture), ["26.13.0", "24.22.2"]);

    ok(&mirror(
        &user,
        &fixture,
        &fixture.config(SELECT_26_LIST_SELECTED),
        &[],
    ));
    assert_eq!(
        snapshot_versions(&fixture),
        ["24.22.2", "26.13.0"],
        "the record is untouched: the knob is about the list, not the renders"
    );
    assert_eq!(
        listed_versions(&fixture),
        ["26.13.0"],
        "the list is trimmed to this run's selection"
    );
    // The digest documents follow the record, not the trim, so flipping
    // the knob back needs no repair run.
    for (version, _) in DIGESTS {
        let digest = fixture.snapshot().join("digests").join(version);
        assert_eq!(
            digest.is_file(),
            ["26.13.0", "24.22.2"].contains(&version),
            "digest of {version}: {}",
            digest.display()
        );
    }

    // The envelope still describes its own document, as the client's
    // reader checks (`#MIRROR-SNAPSHOT-LAYOUT`).
    let envelope: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture.snapshot().join("versions.json")).unwrap(),
    )
    .unwrap();
    let digest = sha256_hex(envelope["document"].as_str().unwrap().as_bytes());
    assert_eq!(envelope["sha256"].as_str().unwrap(), digest);

    // And the client: a fresh home, the target alone, `--offline`, so the
    // snapshot is the only place a domain can come from.
    let inside = UserScratch::new();
    let closed = fixture.closed_project();
    let listing = ok(&vibe(
        &inside,
        &closed,
        &[
            "--offline",
            "--json",
            "app",
            "versions",
            PACKAGE,
            "--registry",
            fixture.target().to_str().unwrap(),
            "--all",
        ],
    ));
    let listing: serde_json::Value = serde_json::from_str(&listing).unwrap();
    let package = &listing["packages"][0];
    let versions: Vec<&str> = package["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["version"].as_str().unwrap())
        .collect();
    assert_eq!(
        versions,
        ["26.13.0"],
        "the client sees only what the mirror published"
    );
    // Channels are recomputed from that same document, so `lts` holds the
    // one member the mirror carried and nothing it did not.
    let lts: Vec<&str> = package["versions"][0]["channels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|name| name.as_str().unwrap())
        .collect();
    assert!(lts.contains(&"lts"), "the trimmed channel table: {lts:?}");
}

/// The hex SHA-256 of `bytes`, the one the snapshot's envelope carries.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The knob is recorded like every other selection key, so a bare re-run
/// with no `--config` reproduces the operator's choice instead of
/// silently widening the list again.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn the_recorded_selection_carries_the_version_list_knob() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    ok(&mirror(&user, &fixture, &fixture.config(SELECT_24_UP), &[]));
    let record =
        fs::read_to_string(fixture.target().join("_index/mirrors").join("src.toml")).unwrap();
    assert!(
        !record.contains("version_list"),
        "the default is skipped, so every record written before the key existed is unchanged:\n{record}"
    );

    ok(&mirror(
        &user,
        &fixture,
        &fixture.config(SELECT_26_LIST_SELECTED),
        &[],
    ));
    let record =
        fs::read_to_string(fixture.target().join("_index/mirrors").join("src.toml")).unwrap();
    assert!(
        record.contains("version_list = \"selected\""),
        "the declared value is recorded:\n{record}"
    );
}

/// A value no spelling of the knob answers to is refused by naming the
/// two, rather than taken as the default.
#[test]
fn an_unknown_version_list_spelling_is_refused() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    let config = fixture.config(
        "schema = 1\n[[template]]\npackage = \"org.vibevm.node/node\"\nversion_list = \"trimmed\"\n",
    );
    let message = refusal(&mirror(&user, &fixture, &config, &[]));
    assert!(message.contains("version_list"), "{message}");
    assert!(message.contains("selected"), "{message}");
}
