//! The system-prompt paths are reserved, not enrolled — the regression test
//! for the hazard spike S3 demonstrated at `c04bda8a5`.
//!
//! Before this gate existed, a `SYSTEM.md` and a `SYSTEM-TEMPLATE.md` sitting
//! in a boot directory were both admitted by `legacy_entries` as ordinary
//! authored boot — `vibe_specdoc::is_spec_source` accepts by extension alone
//! and only the three generated names were excluded — so INDEX.md named both
//! and every session read the generated prefix a second time while the
//! template's text arrived twice. Measured then: four INDEX rows where two
//! were authored.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SELF-INCLUSION",
    r = 1
);
use super::super::test_helpers::{boot_rel, write};
use super::*;

/// A node whose own boot is discovered by the legacy immediate-children
/// scan: no `[[project.sources]]`, so `policy.source_rows()` is `None`.
///
/// The template is written as PROP-076 says a template is — literal text in
/// no spec dialect — because that is the spelling that must not reach the
/// boot compiler.
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "vibe.toml",
        "[project]\nname='host'\nversion='1.0.0'\n",
    );
    write(root.path(), boot_rel("00-core.md"), "# Core {#root}\n");
    write(root.path(), boot_rel("20-extra.md"), "# Extra {#root}\n");
    write(
        root.path(),
        boot_rel(vibe_core::layout::SYSTEM_TEMPLATE_MD),
        "You are working inside a project. MARKER-TEMPLATE-TEXT.\n",
    );
    write(
        root.path(),
        boot_rel(vibe_core::layout::SYSTEM_MD),
        "# Prefix {#root}\n\nMARKER-PREFIX-TEXT\n",
    );
    write(
        root.path(),
        boot_rel(vibe_core::layout::SYSTEM_META_JSON),
        "{\"protocol\":\"vibe-system-prompt/1\"}\n",
    );
    root
}

fn regenerate(root: &Path) {
    let workspace = Workspace::load(root).unwrap();
    super::super::apply_resolution_with_spec_format(
        &workspace,
        &[],
        vibe_core::user_config::SlotIntegrity::Verify,
        SpecFormat::Xml,
        None,
        None,
    )
    .unwrap();
}

fn index_paths(index: &str) -> Vec<String> {
    let document: toml::Value = toml::from_str(index).unwrap();
    document["entry"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["path"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_boot_directory_holding_all_three_enrols_exactly_the_authored_snippets() {
    let root = fixture();
    regenerate(root.path());
    let index = fs::read_to_string(root.path().join(layout::current_boot_index())).unwrap();
    assert_eq!(
        index_paths(&index),
        vec![
            crate::layout_paths::boot("00-core.md"),
            crate::layout_paths::boot("20-extra.md"),
        ],
        "only the two authored snippets are boot inputs:\n{index}"
    );
    // Said again against the names rather than the whole list, because this
    // is the assertion a future reader checks first.
    for reserved in [
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::SYSTEM_TEMPLATE_MD,
        vibe_core::layout::SYSTEM_META_JSON,
    ] {
        assert!(
            !index.contains(reserved),
            "`{reserved}` must not be named as a boot input:\n{index}"
        );
    }
}

#[test]
fn neither_markers_text_reaches_a_generated_artifact() {
    let root = fixture();
    regenerate(root.path());
    for artifact in [
        layout::current_boot_index(),
        layout::current_boot_static_xml(),
        layout::current_boot_static_md(),
    ] {
        let bytes = fs::read(root.path().join(&artifact)).unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("MARKER-PREFIX-TEXT") && !text.contains("MARKER-TEMPLATE-TEXT"),
            "{} carries reserved-path text",
            artifact.display()
        );
    }
}

#[test]
fn the_three_reserved_files_survive_a_regeneration_untouched() {
    // The template is authored: it is reserved, never generated, and must
    // never be removed as a stale output (PROP-076
    // `#SYS-LAYOUT-RESERVATION`). The two generated names have no producer
    // yet in this phase, so nothing may rewrite them either.
    let root = fixture();
    let watched: Vec<_> = [
        vibe_core::layout::SYSTEM_TEMPLATE_MD,
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::SYSTEM_META_JSON,
    ]
    .into_iter()
    .map(|name| {
        let path = root.path().join(boot_rel(name));
        (path.clone(), fs::read(&path).unwrap())
    })
    .collect();
    regenerate(root.path());
    regenerate(root.path());
    for (path, before) in watched {
        assert_eq!(
            fs::read(&path).unwrap(),
            before,
            "{} changed across two regenerations",
            path.display()
        );
    }
}

#[test]
fn a_declared_source_row_naming_a_reserved_path_refuses_in_any_case_spelling() {
    // Declared mode, the other enrolment route: admission refuses the row
    // before anything is written, and the message names the path.
    for name in [
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::SYSTEM_META_JSON,
        vibe_core::layout::SYSTEM_TEMPLATE_MD,
        vibe_core::layout::INDEX_MD,
    ] {
        for spelling in [
            name.to_string(),
            name.to_ascii_uppercase(),
            name.to_ascii_lowercase(),
        ] {
            let row = crate::layout_paths::boot(&spelling);
            let error = vibe_core::manifest::Manifest::parse_str(&format!(
                "[project]\nname='host'\nversion='1.0.0'\n\
                 [[project.sources]]\nfile='{row}'\nmedia='text/markdown'\n\
                 interpretation='vibe-spec'\nrole='spec'\nloading='scan'\n"
            ))
            .expect_err(&format!("`{row}` must be refused as a declared source"))
            .to_string();
            assert!(
                error.contains(&row) || error.contains(&spelling),
                "the refusal names the path `{row}`: {error}"
            );
        }
    }
}
