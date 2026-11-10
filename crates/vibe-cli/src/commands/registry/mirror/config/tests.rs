//! The `mirror.toml` reader and every refusal decidable from the
//! document alone (PROP-075 `#MIRROR-CONFIG-SCHEMA`).
use super::*;

/// Write `body` as a `mirror.toml` in a fresh directory and read it.
fn read_body(body: &str) -> Result<Validated> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mirror.toml");
    std::fs::write(&path, body).unwrap();
    read(&path)
}

/// The whole refusal chain, because a parse failure names the field in
/// the cause anyhow's `Display` leaves out.
fn refusal(body: &str) -> String {
    format!("{:#}", read_body(body).unwrap_err())
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn a_full_document_parses_every_field() {
    let config = read_body(
        r#"
schema = 1

[select]
all = false
groups = ["org.corp"]
packages = ["org.vibevm.node/node"]
exclude = ["org.corp.legacy", "org.corp/old"]

[[template]]
package = "org.vibevm.node/node"
versions = [">=22", "^20.1"]
channels = ["lts", "lts/jod"]
newest = 2
platforms = ["linux"]
archs = ["x86_64"]
installers = ["linux-x64"]
payloads = true

[payloads]
max_total = "40 GiB"
into = "/srv/payloads"
"#,
    )
    .unwrap();
    assert_eq!(config.select.groups, ["org.corp"]);
    assert_eq!(config.select.packages, ["org.vibevm.node/node"]);
    assert_eq!(config.select.exclude, ["org.corp.legacy", "org.corp/old"]);
    let row = &config.templates[0];
    assert_eq!(row.package(), "org.vibevm.node/node");
    assert_eq!(row.versions.len(), 2);
    assert!(row.versions[0].matches(&semver::Version::new(24, 1, 0)));
    assert_eq!(
        row.channels
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["lts", "lts/jod"]
    );
    assert_eq!(row.newest, Some(2));
    assert_eq!(row.installers, ["linux-x64"]);
    assert!(row.payloads);
    assert_eq!(config.payloads.max_total, Some(40 * 1024 * 1024 * 1024));
    assert_eq!(config.payloads.into, Some(PathBuf::from("/srv/payloads")));
}

#[test]
fn an_absent_file_is_refused_by_path() {
    let dir = tempfile::tempdir().unwrap();
    let message = format!("{:#}", read(&dir.path().join("nope.toml")).unwrap_err());
    assert!(
        message.contains("names a file that does not exist"),
        "{message}"
    );
    assert!(message.contains("nope.toml"), "{message}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn a_schema_other_than_one_is_refused_outright() {
    let message = refusal("schema = 2\n[select]\nall = true\n");
    assert!(
        message.contains("has schema 2, this vibe reads 1"),
        "{message}"
    );
}

#[test]
fn a_missing_schema_is_refused_rather_than_defaulted() {
    let message = refusal("[select]\nall = true\n");
    assert!(message.contains("schema"), "{message}");
}

#[test]
fn an_unknown_key_is_refused_by_name() {
    for body in [
        "schema = 1\nselekt = 1\n",
        "schema = 1\n[select]\nall = true\nnewst = 1\n",
        "schema = 1\n[[template]]\npackage = 'org.a/b'\nversionz = []\n",
        "schema = 1\n[[template]]\npackage = 'org.a/b'\npayloads = true\n[payloads]\nmax = 1\n",
    ] {
        let message = refusal(body);
        assert!(message.contains("unknown field"), "{body}: {message}");
    }
}

#[test]
fn a_package_without_a_group_is_refused() {
    for (body, key) in [
        (
            "schema = 1\n[select]\npackages = ['node']\n",
            "[select] packages",
        ),
        (
            "schema = 1\n[[template]]\npackage = 'node'\n",
            "[[template]] package",
        ),
    ] {
        let message = refusal(body);
        assert!(message.contains(key), "{body}: {message}");
        assert!(message.contains("<group>/<name>"), "{body}: {message}");
    }
}

#[test]
fn a_bad_version_requirement_names_the_spelling() {
    let message =
        refusal("schema = 1\n[[template]]\npackage = 'org.a/b'\nversions = ['not a req']\n");
    assert!(message.contains("`not a req`"), "{message}");
    assert!(message.contains("not a version requirement"), "{message}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn a_bad_channel_name_names_the_grammar() {
    let message = refusal("schema = 1\n[[template]]\npackage = 'org.a/b'\nchannels = ['LTS']\n");
    assert!(message.contains("`LTS`"), "{message}");
    assert!(message.contains("not a channel name"), "{message}");
}

#[test]
fn newest_zero_would_keep_nothing_and_is_refused() {
    let message = refusal("schema = 1\n[[template]]\npackage = 'org.a/b'\nnewest = 0\n");
    assert!(message.contains("would keep no version"), "{message}");
}

#[test]
fn one_template_package_may_not_have_two_rows() {
    let message = refusal(
        "schema = 1\n[[template]]\npackage = 'org.a/b'\n[[template]]\npackage = 'org.a/b'\n",
    );
    assert!(message.contains("two `[[template]]` rows"), "{message}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-BUDGET")]
fn a_budget_reads_a_unit_or_a_bare_count_and_refuses_anything_else() {
    let integer = read_body("schema = 1\n[select]\nall = true\n[payloads]\nmax_total = 4096\n")
        .unwrap()
        .payloads
        .max_total;
    assert_eq!(integer, Some(4096));
    for (text, bytes) in [
        ("1", 1u64),
        ("512B", 512),
        ("2 KiB", 2048),
        ("5 MB", 5_000_000),
        ("1.5 GiB", 1_610_612_736),
        ("1 TiB", 1_099_511_627_776),
    ] {
        assert_eq!(parse_bytes(text), Ok(bytes), "{text}");
    }
    assert!(parse_bytes("40 parsecs").is_err());
    assert!(parse_bytes("lots").is_err());
    let message = refusal("schema = 1\n[select]\nall = true\n[payloads]\nmax_total = true\n");
    assert!(message.contains("not a byte budget"), "{message}");
    let message = refusal("schema = 1\n[select]\nall = true\n[payloads]\nmax_total = '40 jars'\n");
    assert!(message.contains("unknown unit"), "{message}");
    let message = refusal("schema = 1\n[select]\nall = true\n[payloads]\nmax_total = 0\n");
    assert!(message.contains("admits no payload at all"), "{message}");
}

#[test]
fn an_empty_installer_id_is_refused_rather_than_ignored() {
    let message =
        refusal("schema = 1\n[[template]]\npackage = 'org.a/b'\ninstallers = ['linux-x64', ' ']\n");
    assert!(message.contains("carries an empty id"), "{message}");
}
