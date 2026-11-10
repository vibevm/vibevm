//! `vibe registry mirror --config` projecting a template's payloads into
//! the package drop — campaign phase P4 commit (c), PROP-075
//! `#MIRROR-PAYLOAD-PROJECTION`, `#MIRROR-PAYLOAD-BUDGET`,
//! `#MIRROR-DRY-RUN-OUTPUT`.
//!
//! The fixture declares its own template over its own four-kilobyte
//! payloads and computes the digest documents from them, so the digest
//! every projection verifies is a real digest of real bytes. The one
//! test-shaped thing is a `[[remap]]` row reading the release host off
//! this filesystem — the mechanism a closed network uses anyway (PROP-066
//! `#locator`), so **no run here opens a socket**.
#![cfg(unix)]

mod common;

use std::fs;

use common::UserScratch;
use common::payload_mirror::{Fixture, PACKAGE, PAYLOAD_BYTES, mirror, ok, refusal, vibe, walk};

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
fn a_payload_row_writes_the_inputs_at_the_first_rung_of_the_drop_lookup() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    let config = fixture.one_version("2.4.0", None, "'linux-x64'");
    ok(&mirror(&user, &fixture, &config, &[]));

    // `<into>/<group>/<name>/<version>/<file>`, and nothing else: the row
    // kept one installer option, so the other option's payload is not
    // fetched at all.
    let mut written: Vec<String> = walk(&fixture.drop())
        .iter()
        .map(|path| {
            path.strip_prefix(fixture.drop())
                .unwrap()
                .display()
                .to_string()
        })
        .collect();
    written.sort();
    assert_eq!(written, ["org.corp/tool/2.4.0/tool-2.4.0-linux-x64.tar.gz"]);

    // The bytes are the release host's own, verified against the digest
    // the render produced out of `SHA256SUMS`.
    let projected = fixture.projected("2.4.0", "linux-x64");
    assert_eq!(
        fs::read(&projected).unwrap(),
        fs::read(fixture.dist().join("v2.4.0/tool-2.4.0-linux-x64.tar.gz")).unwrap()
    );
    assert_eq!(projected.metadata().unwrap().len(), PAYLOAD_BYTES);

    // No partial file is left where the drop lookup reads.
    assert!(
        walk(&fixture.drop())
            .iter()
            .all(|path| !path.to_string_lossy().contains(".part")),
        "a partial file was left in the drop"
    );

    // And the snapshot beside it is the ordinary one: payload projection
    // writes outside the registry target and changes nothing inside it.
    assert!(
        fixture
            .target()
            .join("_index/templates")
            .join(PACKAGE)
            .join("digests/2.4.0")
            .is_file()
    );
    assert!(!fixture.target().join("drop").exists());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
fn an_input_already_in_the_drop_is_present_and_a_wrong_one_stops_the_run() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    let config = fixture.one_version("3.1.0", None, "'linux-x64'");
    ok(&mirror(&user, &fixture, &config, &[]));
    let projected = fixture.projected("3.1.0", "linux-x64");
    let written = projected.metadata().unwrap().modified().unwrap();

    // A second run finds it with the right digest, counts it present and
    // does not fetch it again.
    let again = UserScratch::new();
    let report: serde_json::Value =
        serde_json::from_str(&ok(&mirror(&again, &fixture, &config, &[]))).unwrap();
    assert_eq!(report["ok"], true, "{report}");
    assert_eq!(projected.metadata().unwrap().modified().unwrap(), written);

    // A file a human put there with the wrong bytes is never replaced.
    fs::write(&projected, b"not the payload").unwrap();
    let third = UserScratch::new();
    let message = refusal(&mirror(&third, &fixture, &config, &[]));
    assert!(message.contains("digest_mismatch"), "{message}");
    assert!(message.contains("nothing was written"), "{message}");
    assert_eq!(fs::read(&projected).unwrap(), b"not the payload");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-BUDGET")]
fn a_budget_the_projection_does_not_fit_stops_the_run_before_writing_anything() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    // Three versions of one option: 12288 bytes against a 5000-byte
    // budget, so the second version is already beyond it.
    let config = fixture.config(&format!(
        "schema = 1\n[[template]]\npackage = '{PACKAGE}'\ninstallers = ['linux-x64']\n\
         payloads = true\n[payloads]\ninto = '{}'\nmax_total = 5000\n",
        fixture.drop().display()
    ));
    let message = refusal(&mirror(&user, &fixture, &config, &[]));
    assert!(message.contains("payload_budget_exceeded"), "{message}");
    assert!(message.contains("5000 byte(s)"), "{message}");
    assert!(message.contains("12288 byte(s) to fetch"), "{message}");
    // Named in selection order, oldest version first.
    let beyond = message
        .split_once("in selection order: ")
        .unwrap_or_else(|| panic!("{message}"))
        .1;
    assert!(
        beyond.starts_with("org.corp/tool@2.4.0 (linux-x64), org.corp/tool@3.1.0 (linux-x64)"),
        "{message}"
    );

    // Nothing was written: not a payload, not a snapshot, not a version.
    assert!(!fixture.drop().exists(), "the drop must be untouched");
    assert!(!fixture.target().join("_index").exists());
    assert!(!fixture.target().join(PACKAGE).exists());

    // A budget the projection fits is silent, and the run writes.
    let roomy = fixture.config(&format!(
        "schema = 1\n[[template]]\npackage = '{PACKAGE}'\ninstallers = ['linux-x64']\n\
         payloads = true\n[payloads]\ninto = '{}'\nmax_total = \"1 MiB\"\n",
        fixture.drop().display()
    ));
    let next = UserScratch::new();
    ok(&mirror(&next, &fixture, &roomy, &[]));
    assert_eq!(walk(&fixture.drop()).len(), 3);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-DRY-RUN-OUTPUT")]
fn a_dry_run_shows_the_projection_with_its_counts_and_bytes_and_writes_nothing() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    // Both options of one version: one declares its size, the other does
    // not, so the total is a lower bound and says so.
    let config = fixture.one_version("1.9.0", None, "'linux-x64', 'win-x64'");
    let output = vibe(
        &user,
        &fixture.project(),
        &[
            "--remap",
            fixture.root.path().join("remap.toml").to_str().unwrap(),
            "registry",
            "mirror",
            "src",
            "--into",
            "dst",
            "--config",
            config.to_str().unwrap(),
            "--dry-run",
        ],
    );
    let shown = ok(&output) + &String::from_utf8_lossy(&output.stderr);
    assert!(
        shown.contains("1.9.0: 2 payload input(s) — linux-x64, win-x64"),
        "{shown}"
    );
    assert!(
        shown.contains(&format!(
            "payloads: 2 input(s), 0 already in the drop (0 byte(s)), {PAYLOAD_BYTES} byte(s) to \
             fetch, {PAYLOAD_BYTES} byte(s) in all (a lower bound: 1 input(s) declare no size)"
        )),
        "{shown}"
    );
    assert!(
        shown.contains(&format!(
            "Projecting payloads into the drop `{}`",
            fixture.drop().display()
        )),
        "{shown}"
    );

    // A dry run writes nothing, inside the registry or outside it.
    assert!(!fixture.drop().exists());
    assert!(!fixture.target().join("_index").exists());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
fn a_client_inside_the_closed_network_installs_from_the_projected_drop() {
    let user = UserScratch::new();
    let fixture = Fixture::new();
    let config = fixture.one_version("2.4.0", None, "'src'");
    ok(&mirror(&user, &fixture, &config, &[]));
    assert!(fixture.projected("2.4.0", "src").is_file());

    // A fresh home that can see only the mirrored target and the drop,
    // with no `[[remap]]` row to the release host and `--offline`: the
    // drop is the only place the payload can come from.
    let inside = UserScratch::new();
    fixture.configure_drop(&inside);
    let closed = fixture.closed_project();
    ok(&vibe(
        &inside,
        &closed,
        &[
            "--offline",
            "install",
            "-g",
            &format!("{PACKAGE}@=2.4.0"),
            "--registry",
            fixture.target().to_str().unwrap(),
            "--installer",
            "src",
            "--unattended",
        ],
    ));

    // The installed payload is the release host's own bytes, which only
    // the drop could have served.
    let installed = walk(&inside.settings)
        .into_iter()
        .find(|path| path.ends_with("bin/tool"))
        .expect("the installed launcher payload");
    assert_eq!(
        fs::read(&installed).unwrap(),
        fs::read(fixture.dist().join("v2.4.0/tool-2.4.0-src.bin")).unwrap()
    );
}
