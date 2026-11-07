//! The existing caller, not a path heuristic, knows embedded-source selection.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-EVIDENCE");
use super::*;
use vibe_wire::generated::shared::{SourceAcquisition, SourceSelectionKind};

fn seed(root: &Path, version: &str) {
    let package = root.join(format!("org.example/demo/v{version}"));
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("vibe.toml"), format!("[package]\ngroup='org.example'\nname='demo'\nkind='app'\nversion='{version}'\n[application]\nid='demo'\n")).unwrap();
}

#[test]
fn embedded_source_evidence_keeps_explicit_registry_precedence() {
    let scratch = tempfile::tempdir().unwrap();
    let embedded = scratch.path().join("embedded");
    let explicit = scratch.path().join("explicit");
    seed(&embedded, "1.0.0");
    seed(&explicit, "2.0.0");
    let settings = SettingsRoot::new(&scratch.path().join("settings"));
    let result =
        resolve_package(&settings, None, Some(&embedded), "org.example/demo", true).unwrap();
    assert_eq!(result.package.version, "1.0.0");
    let evidence = result.source_evidence.unwrap();
    assert_eq!(evidence.source_kind, Some(SourceSelectionKind::Embedded));
    assert_eq!(evidence.acquisition, Some(SourceAcquisition::Directory));
    assert_eq!(evidence.registry, None);
    let result = resolve_package(
        &settings,
        Some(&explicit),
        Some(&embedded),
        "org.example/demo",
        true,
    )
    .unwrap();
    assert_eq!(result.package.version, "2.0.0");
    assert_eq!(
        result.source_evidence.unwrap().source_kind,
        Some(SourceSelectionKind::Registry)
    );
    assert!(
        !settings.path().exists(),
        "a carried local package must not initialize a global catalog"
    );
}
