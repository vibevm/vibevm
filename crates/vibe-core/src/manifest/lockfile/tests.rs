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

const FIXTURE: &str = r#"
[meta]
generated_by = "vibe 0.1.0-dev"
generated_at = "2026-05-21T12:00:00Z"
schema_version = 5
solver = "resolvo-0.x"
root_dependencies = ["org.vibevm.world/wal", "org.vibevm/rust-cli"]

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
dependencies = ["org.vibevm.world/atomic-commits@=0.1.0"]

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
"#;

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#lockfile", r = 1)]
#[verifies("spec://vibevm/modules/vibe-registry/PROP-008#identity", r = 1)]
fn parses_fully() {
    let lf: Lockfile = toml::from_str(FIXTURE).unwrap();
    assert_eq!(lf.meta.schema_version, 5);
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
        "org.vibevm.world/atomic-commits"
    );
    assert_eq!(wal.source_kind, Some(SourceKind::Registry));
    assert!(!wal.overridden);
}

#[test]
fn roundtrip() {
    let lf: Lockfile = toml::from_str(FIXTURE).unwrap();
    let rendered = toml::to_string_pretty(&lf).unwrap();
    let back: Lockfile = toml::from_str(&rendered).unwrap();
    assert_eq!(lf, back);
}

#[test]
fn empty_lockfile_has_v5_defaults() {
    let lf = Lockfile::empty("vibe 0.1.0-dev", "2026-05-21T00:00:00Z");
    assert_eq!(lf.meta.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(CURRENT_SCHEMA_VERSION, 5);
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
    assert_eq!(lf.meta.schema_version, 5);
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#lockfile", r = 1)]
fn read_rejects_non_current_version() {
    // A pre-v5 lockfile is rejected outright — no legacy reader, no
    // migration. The fix is to regenerate with `vibe install`.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("vibe.lock");
    std::fs::write(
        &path,
        "[meta]\ngenerated_by = \"old\"\ngenerated_at = \"x\"\nschema_version = 4\n",
    )
    .unwrap();
    let err = Lockfile::read(&path).unwrap_err();
    assert!(
        matches!(
            err,
            crate::error::Error::UnsupportedLockfile {
                found: 4,
                expected: 5
            }
        ),
        "{err}"
    );
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#lockfile", r = 1)]
fn path_source_kind_round_trips() {
    // A path-source member: source_kind = "path", and source_url is the
    // workspace-root-relative path, not a URL. PROP-007 §2.5.
    let raw = r#"
[meta]
generated_by = "vibe"
generated_at = "2026-05-21T00:00:00Z"
schema_version = 5

[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version ="0.1.0"
source_url = "packages/flow-wal"
content_hash = "sha256:abc"
source_kind = "path"
"#;
    let lf: Lockfile = toml::from_str(raw).unwrap();
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
    let mut lf: Lockfile = toml::from_str(FIXTURE).unwrap();
    assert_eq!(lf.packages.len(), 2);
    let removed = lf.remove(&org(), "wal").unwrap();
    assert_eq!(removed.name, "wal");
    assert_eq!(lf.packages.len(), 1);
    assert!(lf.find(&org(), "wal").is_none());
}

#[test]
fn override_flag_round_trips() {
    let raw = r#"
[meta]
generated_by = "vibe 0.1.0-dev"
generated_at = "2026-05-21T00:00:00Z"
schema_version = 5

[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version ="0.3.0"
source_url = "git@mycompany:forks/wal"
source_ref = "my-fix"
content_hash = "sha256:xyz"
source_kind = "override"
overridden = true
"#;
    let lf: Lockfile = toml::from_str(raw).unwrap();
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
    "spec://vibevm/modules/vibe-workspace/PROP-022#destructive-guard",
    r = 1
)]
fn materialization_round_trips() {
    // An `in-place` package records its mode so uninstall / the destructive
    // guard (PROP-022 §2.6) recognise the slot as a non-vendored git clone.
    let raw = r#"
[meta]
generated_by = "vibe"
generated_at = "2026-05-21T00:00:00Z"
schema_version = 5

[[package]]
kind = "feat"
name = "giant"
group = "org.vibevm"
version ="1.0.0"
source_url = "https://example.test/giant.git"
content_hash = "sha256:abc"
source_kind = "git"
materialization = "in-place"
"#;
    let lf: Lockfile = toml::from_str(raw).unwrap();
    let p = lf.find(&org(), "giant").unwrap();
    assert!(p.materialization.is_in_place());
    assert!(!p.materialization.is_default());

    let rendered = toml::to_string_pretty(&lf).unwrap();
    assert!(rendered.contains("materialization = \"in-place\""));
    let back: Lockfile = toml::from_str(&rendered).unwrap();
    assert_eq!(lf, back);

    // The default `snapshot` is skipped on serialize, so every lockfile
    // written before this field landed round-trips unchanged.
    let mut snap = lf.clone();
    snap.packages[0].materialization = Default::default();
    let rendered2 = toml::to_string_pretty(&snap).unwrap();
    assert!(!rendered2.contains("materialization"));
}

#[test]
fn rejects_unknown_package_field() {
    let raw = r#"
[meta]
generated_by = "vibe"
generated_at = "2026-05-21T00:00:00Z"
schema_version = 5

[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version ="0.1.0"
source_url = "file:///x"
content_hash = "sha256:abc"
mystery = true
"#;
    assert!(toml::from_str::<Lockfile>(raw).is_err());
}
