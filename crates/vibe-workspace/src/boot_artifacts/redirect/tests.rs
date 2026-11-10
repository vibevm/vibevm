//! Tests for the managed redirect block ([`super`], PROP-012) and the
//! redirect-write path of [`super::super::write_boot_artifacts`]. Split out
//! of the lane tests along the redirect seam; the lane tests keep the
//! `STATIC.md` / `INDEX.md` coverage.

use super::super::{SelfCoordinate, write_boot_artifacts, write_boot_artifacts_with_spec_format};
use super::*;
use crate::boot::{BootBand, BootEntry, EffectiveBoot};
use specmark::verifies;
use tempfile::TempDir;
use vibe_core::manifest::{LinkType, SpecFormat};

/// The host self coordinate these redirect tests stand in for (B-031). The
/// redirect-path tests carry no static entries, so the coordinate is never
/// exercised — it is threaded for signature conformance only.
#[cfg(test)]
fn coord() -> SelfCoordinate {
    SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into())
}

#[cfg(test)]
fn entry(path: impl Into<String>, link: LinkType, origin: &str) -> BootEntry {
    BootEntry {
        path: path.into(),
        band: BootBand::Dependency,
        link,
        when: None,
        origin: origin.to_string(),
        provenance: crate::boot::fixture_provenance(origin),
        use_ref: false,
        format: Default::default(),
        unit_substituted: false,
        elided: false,
    }
}

#[cfg(test)]
fn boot(entries: Vec<BootEntry>) -> EffectiveBoot {
    EffectiveBoot { entries }
}

#[test]
fn render_redirect_points_at_the_boot_files() {
    let r = render_redirect();
    assert!(r.contains("do not edit"));
    assert!(r.contains(&crate::layout_paths::boot("STATIC.md")));
    assert!(r.contains(&crate::layout_paths::boot("INDEX.md")));
    assert_eq!(
        r,
        render_redirect_with_spec_format(SpecFormat::Markdown),
        "the Markdown redirect bytes remain unchanged"
    );
    assert!(
        render_redirect_with_spec_format(SpecFormat::Xml)
            .contains(&crate::layout_paths::boot("STATIC.xml"))
    );
}

#[test]
fn write_boot_artifacts_writes_index_and_redirects() {
    let ws = TempDir::new().unwrap();
    let b = boot(vec![entry(
        crate::layout_paths::boot("00-core.md"),
        LinkType::Dynamic,
        ".",
    )]);
    let written = write_boot_artifacts(ws.path(), ws.path(), &coord(), &b).unwrap();

    assert!(written.index.is_file());
    assert!(written.static_lane.is_none());
    assert!(fs::symlink_metadata(ws.path().join(crate::layout_paths::boot("STATIC.md"))).is_err());
    assert_eq!(written.redirects.len(), 3);
    for name in REDIRECT_FILES {
        assert!(ws.path().join(name).is_file(), "{name} must be written");
    }
}

#[test]
fn write_boot_artifacts_writes_inline_when_present() {
    let ws = TempDir::new().unwrap();
    let crit = ws.path().join(crate::layout_paths::vibedeps(
        "org.vibevm.crit/1.0.0/boot.md",
    ));
    fs::create_dir_all(crit.parent().unwrap()).unwrap();
    fs::write(&crit, "# discipline").unwrap();

    let b = boot(vec![entry(
        crate::layout_paths::vibedeps("org.vibevm.crit/1.0.0/boot.md"),
        LinkType::Static,
        "flow:crit",
    )]);
    let written = write_boot_artifacts(ws.path(), ws.path(), &coord(), &b).unwrap();
    assert!(written.static_lane.is_some());
    assert!(
        ws.path()
            .join(crate::layout_paths::boot("STATIC.md"))
            .is_file()
    );
}

#[test]
fn write_boot_artifacts_removes_a_stale_inline() {
    let ws = TempDir::new().unwrap();
    let crit = ws.path().join(crate::layout_paths::vibedeps(
        "org.vibevm.crit/1.0.0/boot.md",
    ));
    fs::create_dir_all(crit.parent().unwrap()).unwrap();
    fs::write(&crit, "# discipline").unwrap();

    // First generation has an static contribution.
    let with_inline = boot(vec![entry(
        crate::layout_paths::vibedeps("org.vibevm.crit/1.0.0/boot.md"),
        LinkType::Static,
        "flow:crit",
    )]);
    write_boot_artifacts(ws.path(), ws.path(), &coord(), &with_inline).unwrap();
    assert!(fs::symlink_metadata(ws.path().join(crate::layout_paths::boot("STATIC.md"))).is_ok());

    // A later generation has none — the stale STATIC.md must go.
    let without = boot(vec![entry(
        crate::layout_paths::boot("00-core.md"),
        LinkType::Dynamic,
        ".",
    )]);
    let written = write_boot_artifacts(ws.path(), ws.path(), &coord(), &without).unwrap();
    assert!(written.static_lane.is_none());
    assert!(fs::symlink_metadata(ws.path().join(crate::layout_paths::boot("STATIC.md"))).is_err());
}

#[test]
fn switching_markdown_to_xml_removes_the_owned_stale_name() {
    let ws = TempDir::new().unwrap();
    let source = ws.path().join(crate::layout_paths::vibedeps(
        "org.example.rules/1.0.0/rules.md",
    ));
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(
        &source,
        "# Rules {#root}\n\n@fact:RULE one @status:impl/done\n",
    )
    .unwrap();
    let b = boot(vec![entry(
        crate::layout_paths::vibedeps("org.example.rules/1.0.0/rules.md"),
        LinkType::Static,
        "org.example/rules",
    )]);

    write_boot_artifacts_with_spec_format(ws.path(), ws.path(), &coord(), &b, SpecFormat::Markdown)
        .unwrap();
    assert!(
        ws.path()
            .join(crate::layout_paths::boot("STATIC.md"))
            .is_file()
    );

    write_boot_artifacts_with_spec_format(ws.path(), ws.path(), &coord(), &b, SpecFormat::Xml)
        .unwrap();
    assert!(fs::symlink_metadata(ws.path().join(crate::layout_paths::boot("STATIC.md"))).is_err());
    assert!(
        ws.path()
            .join(crate::layout_paths::boot("STATIC.xml"))
            .is_file()
    );
    let redirect = fs::read_to_string(ws.path().join("AGENTS.md")).unwrap();
    assert!(redirect.contains(&crate::layout_paths::boot("STATIC.xml")));
}

// ----- the managed <vibevm> block (PROP-012) -----

#[test]
fn locate_block_absent_when_no_markers() {
    assert_eq!(
        locate_block("# just a file\n\nno markers here"),
        BlockLocation::Absent
    );
    assert_eq!(locate_block(""), BlockLocation::Absent);
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#markers",
    r = 1
)]
fn locate_block_well_formed_pair() {
    let content = "before\n<vibevm>\nbody\n</vibevm>\nafter\n";
    match locate_block(content) {
        BlockLocation::Present { start, end } => {
            assert_eq!(&content[start..end], "<vibevm>\nbody\n</vibevm>\n");
        }
        other => panic!("expected Present, got {other:?}"),
    }
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#markers",
    r = 1
)]
fn locate_block_two_openers_is_malformed() {
    let content = "<vibevm>\na\n</vibevm>\n<vibevm>\nb\n</vibevm>\n";
    match locate_block(content) {
        // The drill's precision: the report names each marker's line, so
        // the operator repairing by hand does not have to search for them.
        BlockLocation::Malformed(reason) => {
            assert!(
                reason.contains("at line(s) 1, 4") && reason.contains("at line(s) 3, 6"),
                "the malformed report must name the marker lines; got: {reason}"
            );
        }
        other => panic!("expected Malformed, got {other:?}"),
    }
}

#[test]
fn locate_block_unbalanced_is_malformed() {
    assert!(matches!(
        locate_block("<vibevm>\nbody\n"),
        BlockLocation::Malformed(_)
    ));
    assert!(matches!(
        locate_block("body\n</vibevm>\n"),
        BlockLocation::Malformed(_)
    ));
}

#[test]
fn locate_block_close_before_open_is_malformed() {
    assert!(matches!(
        locate_block("</vibevm>\nbody\n<vibevm>\n"),
        BlockLocation::Malformed(_)
    ));
}

#[test]
fn write_managed_block_creates_a_missing_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    let written = write_managed_block(&path).unwrap();
    assert_eq!(written.as_deref(), Some(path.as_path()));
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.starts_with("<vibevm>\n"));
    assert!(content.trim_end().ends_with("</vibevm>"));
    assert!(content.contains(&crate::layout_paths::boot("INDEX.md")));
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#create",
    r = 1
)]
fn write_managed_block_appends_preserving_co_tenant_content() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    fs::write(&path, "# My own instructions\n\nKeep me.\n").unwrap();
    write_managed_block(&path).unwrap();
    let content = fs::read_to_string(&path).unwrap();
    // The hand-authored content is preserved verbatim, ahead of the block.
    assert!(content.starts_with("# My own instructions\n\nKeep me.\n"));
    assert!(content.contains("<vibevm>") && content.contains("</vibevm>"));
    assert!(matches!(
        locate_block(&content),
        BlockLocation::Present { .. }
    ));
}

#[test]
#[verifies(
    "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-012#create",
    r = 1
)]
fn write_managed_block_splices_in_place_preserving_surroundings() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    fs::write(
        &path,
        "TOP — mine\n\n<vibevm>\nstale body\n</vibevm>\n\nBOTTOM — mine\n",
    )
    .unwrap();
    write_managed_block(&path).unwrap();
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.starts_with("TOP — mine\n\n"));
    assert!(content.trim_end().ends_with("BOTTOM — mine"));
    assert!(!content.contains("stale body"));
    assert!(content.contains("# Session boot"));
}

#[test]
fn write_managed_block_migrates_the_old_whole_file_redirect() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    // A file that is wholly the pre-PROP-012 generated redirect.
    fs::write(
        &path,
        format!("{OLD_GENERATED_HEADER}\n\n# Session boot\n\nold body\n"),
    )
    .unwrap();
    write_managed_block(&path).unwrap();
    let content = fs::read_to_string(&path).unwrap();
    // Reclaimed wholesale as a block — no leftover old body.
    assert!(content.starts_with("<vibevm>\n"));
    assert!(!content.contains("old body"));
    assert!(matches!(
        locate_block(&content),
        BlockLocation::Present { .. }
    ));
}

#[test]
fn write_managed_block_is_a_noop_when_block_is_identical() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    write_managed_block(&path).unwrap();
    // A second write finds a byte-identical block — nothing to do.
    assert!(write_managed_block(&path).unwrap().is_none());
}

#[test]
fn write_managed_block_errors_on_a_malformed_block() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    fs::write(&path, "<vibevm>\na\n</vibevm>\n<vibevm>\nb\n</vibevm>\n").unwrap();
    let err = write_managed_block(&path).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::MalformedRedirectBlock { .. }),
        "{err}"
    );
}

/// The manifest's `[system_prompt] entrypoint` enum and this module's
/// [`REDIRECT_FILES`] are two spellings of one closed set, in two crates
/// that cannot share a constant (vibe-core cannot depend on vibe-workspace).
/// This is the fence that keeps them from drifting apart — PROP-076
/// `#SYS-ENTRYPOINT-ONE` admits exactly the set vibe owns a block inside.
#[test]
fn the_entrypoint_choice_set_is_exactly_the_redirect_file_set() {
    let declared: Vec<&str> = vibe_core::manifest::EntrypointFile::ALL
        .iter()
        .map(|file| file.as_str())
        .collect();
    assert_eq!(declared, REDIRECT_FILES.to_vec());
    assert!(REDIRECT_FILES.contains(&vibe_core::manifest::EntrypointFile::DEFAULT.as_str()));
}

// --- PROP-076 `#SYS-PROSPECTIVE-ENTRYPOINT` — one projection, two callers --

/// The risk the prospective projection introduces is a second renderer: a
/// value computed one way and bytes written another, drifting apart at the
/// first wording change. This is the fence — for every shape the write path
/// distinguishes, the file the write leaves behind is byte-equal to what the
/// projection returned for the same input.
#[test]
fn the_projection_equals_the_bytes_the_write_leaves_behind() {
    let dir = TempDir::new().unwrap();
    let cases = [
        ("missing", None),
        ("empty", Some(String::new())),
        (
            "co-tenant with no markers",
            Some("# Mine\n\nAuthored prose.\n".to_string()),
        ),
        (
            "co-tenant without a trailing newline",
            Some("# Mine\n\nAuthored prose, unterminated.".to_string()),
        ),
        (
            "an existing block with text on both sides",
            Some("# Mine\n\nbefore\n\n<vibevm>\nstale body\n</vibevm>\n\nafter\n".to_string()),
        ),
        (
            "the pre-PROP-012 whole-file redirect",
            Some(format!("{OLD_GENERATED_HEADER}\n\nold body\n")),
        ),
    ];
    for (label, existing) in cases {
        let path = dir.path().join("AGENTS.md");
        match &existing {
            Some(content) => fs::write(&path, content).unwrap(),
            None => {
                let _ = fs::remove_file(&path);
            }
        }
        let projected = project_managed_block(existing.as_deref(), SpecFormat::Mixed)
            .unwrap_or_else(|reason| panic!("{label}: {reason}"));
        write_managed_block(&path).unwrap();
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            projected,
            "{label}: the witness and the write must come from one function"
        );
    }
}

#[test]
fn the_projection_reports_a_malformed_marker_pair_without_repairing_it() {
    // A reading caller gets the same refusal reason a writing caller does,
    // and neither invents a repair of a co-tenant document.
    let reason = project_managed_block(
        Some("<vibevm>\na\n</vibevm>\n<vibevm>\nb\n</vibevm>\n"),
        SpecFormat::Mixed,
    )
    .expect_err("two marker pairs are malformed");
    assert!(reason.contains("`<vibevm>`"), "{reason}");
}

#[test]
fn the_projection_preserves_every_byte_outside_the_block() {
    let before = "# Mine\n\nkeep me\n\n";
    let after = "\n\nkeep me too\n";
    let projected = project_managed_block(
        Some(&format!("{before}<vibevm>\nstale\n</vibevm>{after}")),
        SpecFormat::Mixed,
    )
    .unwrap();
    assert!(projected.starts_with(before), "{projected}");
    assert!(projected.ends_with(after), "{projected}");
    assert!(!projected.contains("stale"), "{projected}");
}
