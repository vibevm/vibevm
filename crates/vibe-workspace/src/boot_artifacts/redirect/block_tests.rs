//! The managed `<vibevm>` block itself: locating it, creating it, moving it
//! and projecting it (PROP-012, and the block's position under
//! `PROP-076#SYS-HARNESS-DELIVERY`).
//!
//! A child of the redirect tests rather than a sibling, so it inherits their
//! fixtures; split out of them along the one seam that matters here — these
//! are assertions about the block as a *document region*, while the parent's
//! are about the write path that puts it on disk.

use super::*;

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
fn write_managed_block_prepends_preserving_co_tenant_content() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    fs::write(&path, "# My own instructions\n\nKeep me.\n").unwrap();
    write_managed_block(&path).unwrap();
    let content = fs::read_to_string(&path).unwrap();
    // The hand-authored content is preserved verbatim, after the block —
    // which leads, so a harness that truncates the tail keeps the router.
    assert!(content.starts_with("<vibevm>\n"), "{content}");
    assert!(
        content.ends_with("\n# My own instructions\n\nKeep me.\n"),
        "{content}"
    );
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
fn write_managed_block_migrates_a_mid_file_block_to_the_top_once() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    fs::write(
        &path,
        "TOP — mine\n\n<vibevm>\nstale body\n</vibevm>\n\nBOTTOM — mine\n",
    )
    .unwrap();
    write_managed_block(&path).unwrap();
    let content = fs::read_to_string(&path).unwrap();
    // Migrated once: the block leads, the authored text keeps its own order
    // and its own bytes, and the run of newlines that separated the block
    // from that text — vibe's own envelope — travels with the block.
    assert!(content.starts_with("<vibevm>\n"), "{content}");
    assert!(
        content.ends_with("\nTOP — mine\n\nBOTTOM — mine\n"),
        "{content}"
    );
    assert!(!content.contains("stale body"));
    assert!(content.contains("# Session boot"));

    // Idempotent: a second write finds the block at the top and changes
    // nothing at all.
    assert_eq!(write_managed_block(&path).unwrap(), None);
    assert_eq!(fs::read_to_string(&path).unwrap(), content);
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
        let projected = project_managed_block(
            existing.as_deref(),
            &RouterFacts::undeclared(SpecFormat::Mixed),
        )
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
        &RouterFacts::undeclared(SpecFormat::Mixed),
    )
    .expect_err("two marker pairs are malformed");
    assert!(reason.contains("`<vibevm>`"), "{reason}");
}

#[test]
fn the_projection_preserves_every_authored_byte_and_its_order() {
    let before = "# Mine\n\nkeep me\n\n";
    let after = "\n\nkeep me too\n";
    let projected = project_managed_block(
        Some(&format!("{before}<vibevm>\nstale\n</vibevm>{after}")),
        &RouterFacts::undeclared(SpecFormat::Mixed),
    )
    .unwrap();
    assert!(projected.starts_with("<vibevm>\n"), "{projected}");
    // The authored text, in its own order, with only the separator re-made.
    assert!(
        projected.ends_with("\n# Mine\n\nkeep me\n\nkeep me too\n"),
        "{projected}"
    );
    assert!(!projected.contains("stale"), "{projected}");
    // Idempotent: projecting the projection changes nothing.
    assert_eq!(
        project_managed_block(
            Some(&projected),
            &RouterFacts::undeclared(SpecFormat::Mixed)
        )
        .unwrap(),
        projected
    );
}
