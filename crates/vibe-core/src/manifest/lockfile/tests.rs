//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the
//! inline form. Non-`#[test]` helpers carry `#[cfg(test)]` so
//! file-grain scanners (the conform frontend) scope their `unwrap`s
//! as test code.

use specmark::verifies;

use super::*;

/// The canonical group every fixture package in these tests belongs to.
#[cfg(test)]
fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

/// The two-package lockfile three tests below parse. The schema number
/// is read from [`CURRENT_SCHEMA_VERSION`] rather than written out: a
/// fixture that spells today's version as a digit is a fixture that
/// turns red on the day the version moves, for no reason connected to
/// what it tests.
#[cfg(test)]
fn fixture() -> String {
    format!(
        r#"
[meta]
generated_by = "vibe 0.1.0-dev"
generated_at = "2026-05-21T12:00:00Z"
schema_version = {CURRENT_SCHEMA_VERSION}
solver = "resolvo-0.x"
root_dependencies = ["org.vibevm/wal", "org.vibevm/rust-cli"]

[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version ="0.3.0"
registry = "vibespecs"
source_url = "git@gitverse.ru:vibespecs/flow-wal.git"
source_ref = "v0.3.0"
resolved_commit = "abc123def456"
content_hash = "sha256:abc"
source_kind = "registry"
boot_snippet = "10-flow-wal.md"
files_written = [
"spec/flows/wal/WAL-PROTOCOL.md",
"spec/boot/10-flow-wal.md",
]
dependencies = ["org.vibevm/atomic-commits@=0.1.0"]
admitted_by = "friends-chain"
via_override = "org.vibevm/root"

[[package]]
kind = "stack"
name = "rust-cli"
group = "org.vibevm"
version ="0.1.0"
registry = "vibespecs"
source_url = "git@gitverse.ru:vibespecs/stack-rust-cli.git"
source_ref = "v0.1.0"
resolved_commit = "999888777666"
content_hash = "sha256:def"
source_kind = "registry"
"#
    )
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#lockfile",
    r = 1
)]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity",
    r = 1
)]
fn parses_fully() {
    let lf: Lockfile = toml::from_str(&fixture()).unwrap();
    assert_eq!(lf.meta.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(lf.meta.solver.as_deref(), Some("resolvo-0.x"));
    assert_eq!(lf.meta.root_dependencies.len(), 2);
    assert_eq!(lf.packages.len(), 2);

    let wal = lf.find(&org(), "wal").unwrap();
    assert_eq!(wal.version.to_string(), "0.3.0");
    assert_eq!(wal.registry.as_deref(), Some("vibespecs"));
    assert_eq!(wal.source_url, "git@gitverse.ru:vibespecs/flow-wal.git");
    assert_eq!(wal.source_ref.as_deref(), Some("v0.3.0"));
    assert_eq!(wal.resolved_commit.as_deref(), Some("abc123def456"));
    assert_eq!(wal.dependencies.len(), 1);
    assert_eq!(
        wal.dependencies[0].qualified_name(),
        "org.vibevm/atomic-commits"
    );
    assert_eq!(wal.source_kind, Some(SourceKind::Registry));
    assert_eq!(wal.admitted_by.as_deref(), Some("friends-chain"));
    assert_eq!(wal.via_override.as_deref(), Some("org.vibevm/root"));
    assert!(!wal.overridden);
}

#[test]
fn roundtrip() {
    let lf: Lockfile = toml::from_str(&fixture()).unwrap();
    let rendered = toml::to_string_pretty(&lf).unwrap();
    let back: Lockfile = toml::from_str(&rendered).unwrap();
    assert_eq!(lf, back);
}

#[test]
fn embedded_source_provenance_round_trips_without_machine_state() {
    let raw = format!(
        r#"
[meta]
generated_by = "vibe"
generated_at = "2026-09-11T00:00:00Z"
schema_version = {CURRENT_SCHEMA_VERSION}

[[package]]
kind = "tool"
name = "adapter"
group = "org.example"
version = "1.0.0"
authors = ["Bridge Package Maintainer"]
source_url = "https://github.com/vibespecs/adapter.git"
content_hash = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[package.embedded_source]]
name = "upstream"
source_url = "https://github.com/example/upstream.git"
source_ref = "refs/tags/v1.2.3"
resolved_commit = "0123456789abcdef0123456789abcdef01234567"
tree_oid = "89abcdef0123456789abcdef0123456789abcdef"
content_hash = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
upstream_authors = ["Example Upstream Authors"]
upstream_license = "MIT"
license_path = "LICENSE"
license_url = "https://github.com/example/upstream/blob/0123456789abcdef0123456789abcdef01234567/LICENSE"
license_file_sha256 = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
"#
    );
    let lock: Lockfile = toml::from_str(&raw).unwrap();
    let source = &lock.packages[0].embedded_sources[0];
    assert_eq!(lock.packages[0].authors, ["Bridge Package Maintainer"]);
    assert_eq!(source.name, "upstream");
    assert_eq!(
        source.resolved_commit,
        "0123456789abcdef0123456789abcdef01234567"
    );
    assert_eq!(source.upstream_authors, ["Example Upstream Authors"]);
    assert_eq!(source.license_path, std::path::PathBuf::from("LICENSE"));

    let rendered = toml::to_string_pretty(&lock).unwrap();
    assert!(
        rendered.contains("[[package.embedded_source]]"),
        "{rendered}"
    );
    assert!(!rendered.contains("cache_path"), "{rendered}");
    assert!(!rendered.contains("\nauth ="), "{rendered}");
    let back: Lockfile = toml::from_str(&rendered).unwrap();
    assert_eq!(lock, back);

    let missing_authors = raw.replace("upstream_authors = [\"Example Upstream Authors\"]\n", "");
    assert!(toml::from_str::<Lockfile>(&missing_authors).is_err());
}

#[test]
fn embedded_source_lock_rejects_cache_paths_and_credentials() {
    let raw = format!(
        r#"
[meta]
generated_by = "vibe"
generated_at = "2026-09-11T00:00:00Z"
schema_version = {CURRENT_SCHEMA_VERSION}

[[package]]
kind = "tool"
name = "adapter"
group = "org.example"
version = "1.0.0"
authors = ["Bridge Package Maintainer"]
source_url = "https://github.com/vibespecs/adapter.git"
content_hash = "sha256:aaaa"

[[package.embedded_source]]
name = "upstream"
source_url = "https://github.com/example/upstream.git"
resolved_commit = "0123456789abcdef0123456789abcdef01234567"
tree_oid = "89abcdef0123456789abcdef0123456789abcdef"
content_hash = "sha256:bbbb"
upstream_authors = ["Example Upstream Authors"]
upstream_license = "MIT"
license_path = "LICENSE"
license_url = "https://github.com/example/upstream/blob/0123456789abcdef0123456789abcdef01234567/LICENSE"
license_file_sha256 = "sha256:cccc"
cache_path = "C:/secret/cache"
"#
    );
    assert!(toml::from_str::<Lockfile>(&raw).is_err());
    assert!(toml::from_str::<Lockfile>(&raw.replace("cache_path", "auth")).is_err());
}

#[test]
fn absent_visibility_provenance_is_not_serialized() {
    let raw = format!(
        r#"
[meta]
generated_by = "vibe"
generated_at = "2026-08-23T00:00:00Z"
schema_version = {CURRENT_SCHEMA_VERSION}

[[package]]
kind = "flow"
name = "plain"
group = "org.vibevm"
version = "1.0.0"
source_url = "file:///plain"
content_hash = "sha256:abc"
"#
    );
    let lock: Lockfile = toml::from_str(&raw).unwrap();
    assert!(lock.packages[0].admitted_by.is_none());
    assert!(lock.packages[0].via_override.is_none());
    let rendered = toml::to_string_pretty(&lock).unwrap();
    assert!(!rendered.contains("admitted_by"));
    assert!(!rendered.contains("via_override"));
}

#[test]
fn empty_lockfile_has_v7_defaults() {
    let lf = Lockfile::empty("vibe 0.1.0-dev", "2026-05-21T00:00:00Z");
    assert_eq!(lf.meta.schema_version, LEGACY_SCHEMA_VERSION);
    assert_eq!(LEGACY_SCHEMA_VERSION, 7);
    assert_eq!(CURRENT_SCHEMA_VERSION, 11);
    assert!(lf.meta.solver.is_none());
    assert!(lf.packages.is_empty());

    let rendered = toml::to_string_pretty(&lf).unwrap();
    let back: Lockfile = toml::from_str(&rendered).unwrap();
    assert_eq!(lf, back);
}

#[test]
fn read_accepts_current_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vibe.lock");
    Lockfile::empty("vibe", "2026-05-21T00:00:00Z")
        .write(&path)
        .unwrap();
    let lf = Lockfile::read(&path).unwrap();
    assert_eq!(lf.meta.schema_version, 7);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#lockfile",
    r = 1
)]
fn read_rejects_pre_seven_version() {
    // A pre-v7 lockfile is rejected outright — no legacy reader, no
    // migration. The fix is to regenerate with `vibe install`.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vibe.lock");
    std::fs::write(
        &path,
        "[meta]\ngenerated_by = \"old\"\ngenerated_at = \"x\"\nschema_version = 6\n",
    )
    .unwrap();
    let err = Lockfile::read(&path).unwrap_err();
    assert!(
        matches!(
            err,
            crate::error::Error::UnsupportedLockfile {
                found: 6,
                expected: CURRENT_SCHEMA_VERSION
            }
        ),
        "{err}"
    );
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#lockfile",
    r = 1
)]
fn path_source_kind_round_trips() {
    // A path-source member: source_kind = "path", and source_url is the
    // workspace-root-relative path, not a URL. PROP-007 §2.5.
    let raw = format!(
        r#"
[meta]
generated_by = "vibe"
generated_at = "2026-05-21T00:00:00Z"
schema_version = {CURRENT_SCHEMA_VERSION}

[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version ="0.1.0"
source_url = "packages/flow-wal"
content_hash = "sha256:abc"
source_kind = "path"
"#
    );
    let lf: Lockfile = toml::from_str(&raw).unwrap();
    let wal = lf.find(&org(), "wal").unwrap();
    assert_eq!(wal.source_kind, Some(SourceKind::Path));
    assert_eq!(wal.source_url, "packages/flow-wal");
    let rendered = toml::to_string_pretty(&lf).unwrap();
    assert!(rendered.contains("source_kind = \"path\""));
    let back: Lockfile = toml::from_str(&rendered).unwrap();
    assert_eq!(lf, back);
}

#[test]
fn rejects_missing_schema_version() {
    // schema_version is a required field — no default.
    let raw = "[meta]\ngenerated_by = \"vibe\"\ngenerated_at = \"x\"\n";
    assert!(toml::from_str::<Lockfile>(raw).is_err());
}

#[test]
fn remove_drops_entry() {
    let mut lf: Lockfile = toml::from_str(&fixture()).unwrap();
    assert_eq!(lf.packages.len(), 2);
    let removed = lf.remove(&org(), "wal").unwrap();
    assert_eq!(removed.name, "wal");
    assert_eq!(lf.packages.len(), 1);
    assert!(lf.find(&org(), "wal").is_none());
}

#[test]
fn override_flag_round_trips() {
    let raw = format!(
        r#"
[meta]
generated_by = "vibe 0.1.0-dev"
generated_at = "2026-05-21T00:00:00Z"
schema_version = {CURRENT_SCHEMA_VERSION}

[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version ="0.3.0"
source_url = "git@mycompany:forks/wal"
source_ref = "my-fix"
content_hash = "sha256:abc"
source_kind = "override"
overridden = true
"#
    );
    let lf: Lockfile = toml::from_str(&raw).unwrap();
    assert!(lf.packages[0].overridden);

    let rendered = toml::to_string_pretty(&lf).unwrap();
    assert!(rendered.contains("overridden = true"));
    // The false case (default) is skipped on serialize.
    let mut lf2 = lf.clone();
    lf2.packages[0].overridden = false;
    let rendered2 = toml::to_string_pretty(&lf2).unwrap();
    assert!(!rendered2.contains("overridden"));
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-022#destructive-guard",
    r = 1
)]
fn materialization_round_trips() {
    // An `in-place` package records its mode so uninstall / the destructive
    // guard (PROP-022 §2.6) recognise the slot as a non-vendored git clone.
    let raw = format!(
        r#"
[meta]
generated_by = "vibe"
generated_at = "2026-05-21T00:00:00Z"
schema_version = {CURRENT_SCHEMA_VERSION}

[[package]]
kind = "feat"
name = "giant"
group = "org.vibevm"
version ="1.0.0"
source_url = "https://example.test/giant.git"
content_hash = "sha256:abc"
source_kind = "git"
materialization = "in-place"
"#
    );
    let lf: Lockfile = toml::from_str(&raw).unwrap();
    let p = lf.find(&org(), "giant").unwrap();
    assert!(p.materialization.is_in_place());
    assert!(!p.materialization.is_default());

    let rendered = toml::to_string_pretty(&lf).unwrap();
    assert!(rendered.contains("materialization = \"in-place\""));
    let back: Lockfile = toml::from_str(&rendered).unwrap();
    assert_eq!(lf, back);

    // The default `copy` is skipped on serialize, so every lockfile
    // written before this field landed round-trips unchanged.
    let mut snap = lf.clone();
    snap.packages[0].materialization = Default::default();
    let rendered2 = toml::to_string_pretty(&snap).unwrap();
    assert!(!rendered2.contains("materialization"));
}

#[test]
fn rejects_unknown_package_field() {
    let raw = format!(
        r#"
[meta]
generated_by = "vibe"
generated_at = "2026-05-21T00:00:00Z"
schema_version = {CURRENT_SCHEMA_VERSION}

[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version ="0.1.0"
source_url = "file:///x"
content_hash = "sha256:abc"
mystery = true
"#
    );
    assert!(toml::from_str::<Lockfile>(&raw).is_err());
}

/// One `[[package]]` row's TOML, with `extra` appended inside it.
fn lock_text(schema: u32, extra: &str) -> String {
    format!(
        "[meta]\ngenerated_by = \"vibe\"\ngenerated_at = \"2026-11-10T00:00:00Z\"\n\
         schema_version = {schema}\n\n\
         [[package]]\nkind = \"app\"\nname = \"node\"\ngroup = \"org.vibevm.node\"\n\
         version = \"24.22.2\"\nsource_url = \"file:///r\"\n\
         content_hash = \"sha256:abc\"\n{extra}"
    )
}

const TEMPLATE_ROW: &str = "via_channel = \"lts\"\n\n\
     [package.template]\nrevision = \"0.3.0\"\n\
     rendered_sha256 = \"aa\"\ndigest_source_sha256 = \"bb\"\n\
     source_url = \"https://nodejs.org/dist/index.json\"\n\
     digest_url = \"https://nodejs.org/dist/v24.22.2/SHASUMS256.txt\"\n";

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
fn schema_ten_round_trips_the_template_record_and_the_channel() {
    let lock = Lockfile::parse_str(&lock_text(TEMPLATE_SCHEMA_VERSION, TEMPLATE_ROW)).unwrap();
    let row = &lock.packages[0];
    let template = row.template.as_ref().expect("the row carries a template");
    assert_eq!(template.revision.to_string(), "0.3.0");
    assert_eq!(template.rendered_sha256, "aa");
    assert_eq!(template.digest_source_sha256.as_deref(), Some("bb"));
    assert_eq!(row.via_channel.as_deref(), Some("lts"));
    // The writer keeps the epoch, because the row needs it.
    let written = lock.for_write().unwrap();
    assert_eq!(written.meta.schema_version, TEMPLATE_SCHEMA_VERSION);
    let back = Lockfile::parse_str(&toml::to_string_pretty(&written).unwrap()).unwrap();
    assert_eq!(back.packages[0].template, row.template);
    assert_eq!(back.packages[0].via_channel, row.via_channel);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
fn nine_still_reads_and_the_epoch_above_the_newest_refuses() {
    // A schema-9 lock with a remap source binding is read exactly as
    // before: the 9 → 10 step did not raise that floor, and neither
    // did the 10 → 11 step of PROP-075 `#TEMPLATE-NO-DIGEST`.
    for schema in [
        LEGACY_SCHEMA_VERSION,
        CONFIGURATION_SCHEMA_VERSION,
        9,
        10,
        11,
    ] {
        assert!(
            Lockfile::parse_str(&lock_text(schema, "")).is_ok(),
            "schema{schema} must read"
        );
    }
    let error =
        Lockfile::parse_str(&lock_text(DIGESTLESS_TEMPLATE_SCHEMA_VERSION + 1, "")).unwrap_err();
    assert!(error.to_string().contains("12"), "{error}");
}

/// The digest-less template record: round-tripped, writing epoch11, and
/// corruption under any lower epoch (PROP-075 `#TEMPLATE-NO-DIGEST`,
/// AV-FMT-033).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-NO-DIGEST")]
fn a_template_record_with_no_digest_document_round_trips_at_epoch_eleven() {
    const NO_DIGEST_ROW: &str = "[package.template]\nrevision = \"1.0.0\"\n\
         rendered_sha256 = \"aa\"\n";
    let lock = Lockfile::parse_str(&lock_text(
        DIGESTLESS_TEMPLATE_SCHEMA_VERSION,
        NO_DIGEST_ROW,
    ))
    .unwrap();
    let template = lock.packages[0]
        .template
        .as_ref()
        .expect("the row carries a template");
    // Absent, never an empty string: the record says "there is no such
    // document", which is what makes the alarm meaningful.
    assert!(template.digest_source_sha256.is_none());
    assert!(template.digest_url.is_none());

    let written = lock.for_write().unwrap();
    assert_eq!(
        written.meta.schema_version, DIGESTLESS_TEMPLATE_SCHEMA_VERSION,
        "the row needs the epoch, so the writer claims it"
    );
    let back = Lockfile::parse_str(&toml::to_string_pretty(&written).unwrap()).unwrap();
    assert_eq!(back.packages[0].template, lock.packages[0].template);

    // Under schema10 the same row is corruption, not legacy input: a
    // 10 reader requires `digest_source_sha256`.
    let error =
        Lockfile::parse_str(&lock_text(TEMPLATE_SCHEMA_VERSION, NO_DIGEST_ROW)).unwrap_err();
    assert!(error.to_string().contains("schema11"), "{error}");
}

/// A template record that *does* name its digest document keeps writing
/// epoch10 — the ladder gained a rung, not a floor.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-NO-DIGEST")]
fn a_template_record_with_a_digest_document_still_writes_epoch_ten() {
    let lock =
        Lockfile::parse_str(&lock_text(DIGESTLESS_TEMPLATE_SCHEMA_VERSION, TEMPLATE_ROW)).unwrap();
    assert_eq!(
        lock.for_write().unwrap().meta.schema_version,
        TEMPLATE_SCHEMA_VERSION
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
fn a_template_record_under_an_older_epoch_is_corruption_not_legacy_input() {
    let error = Lockfile::parse_str(&lock_text(9, TEMPLATE_ROW)).unwrap_err();
    assert!(error.to_string().contains("schema10"), "{error}");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
fn a_lock_with_no_template_keeps_writing_the_epoch_it_always_did() {
    // The writer writes the lowest epoch that carries what the lock
    // holds, so nothing about a project without a template moves.
    let lock = Lockfile::parse_str(&lock_text(TEMPLATE_SCHEMA_VERSION, "")).unwrap();
    assert_eq!(
        lock.for_write().unwrap().meta.schema_version,
        LEGACY_SCHEMA_VERSION
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-LOCK-RECORD")]
fn the_alarm_names_the_digest_fields_that_differ() {
    let before = "[[installer_options.steps]]\nid = \"unpack\"\nsha256 = \"aa\"\nsize = 10\n";
    let after = "[[installer_options.steps]]\nid = \"unpack\"\nsha256 = \"bb\"\nsize = 10\n";
    let fields = crate::manifest::differing_digest_fields(before, after);
    assert_eq!(fields, vec!["sha256 = \"aa\" → \"bb\""]);
    assert!(crate::manifest::differing_digest_fields(before, before).is_empty());

    let alarm = crate::manifest::TemplateRenderChanged {
        package: "org.vibevm.node/node".to_string(),
        version: "24.22.2".parse().unwrap(),
        recorded_revision: "0.3.0".parse().unwrap(),
        computed_revision: "0.4.0".parse().unwrap(),
        recorded_sha256: "a".repeat(64),
        computed_sha256: "b".repeat(64),
        fields,
    };
    let message = alarm.to_string();
    assert!(message.contains("template_render_changed"), "{message}");
    assert!(message.contains("org.vibevm.node/node"), "{message}");
    assert!(message.contains("0.3.0"), "{message}");
    assert!(message.contains("0.4.0"), "{message}");
    assert!(message.contains("--trust-template"), "{message}");
}
