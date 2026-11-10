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
    // The no-facts form: a directory with no declared identity and no lane
    // digest at hand. It names INDEX, says plainly that there is no static
    // lane, and carries the reserved node-id spelling — not an invented one
    // (PROP-076 `#SYS-NO-RECURSION`).
    let r = render_redirect();
    assert!(r.contains("do not edit"));
    assert!(r.contains(&crate::layout_paths::boot("INDEX.md")));
    assert!(r.contains("publishes no static lane"));
    assert!(r.contains(&format!("project {UNDECLARED_NODE_ID}")));
    assert_eq!(
        r,
        render_redirect_with_spec_format(SpecFormat::Markdown),
        "with no lane the target format changes no byte"
    );
    // A node that does publish a lane names the spelling its format selects.
    let facts = |spec_format| RouterFacts {
        node_id: "org.x/demo".to_owned(),
        static_hash12: Some("0123456789ab".to_owned()),
        spec_format,
    };
    assert!(
        render_router(&facts(SpecFormat::Xml)).contains(&crate::layout_paths::boot("STATIC.xml"))
    );
    assert!(
        render_router(&facts(SpecFormat::Markdown))
            .contains(&crate::layout_paths::boot("STATIC.md"))
    );
}

/// Two regenerations with the same facts leave all three copies
/// byte-identical to each other and to the first run — the property that
/// makes this phase's rewrite of every instruction file in every project a
/// one-off diff rather than churn on every install.
#[test]
fn two_regenerations_leave_the_three_copies_identical_and_unwritten() {
    let ws = TempDir::new().unwrap();
    for name in REDIRECT_FILES {
        fs::write(ws.path().join(name), "# Mine\n\nAuthored prose.\n").unwrap();
    }
    let facts = RouterFacts {
        node_id: "org.x/demo".to_owned(),
        static_hash12: Some(crate::system_prompt::segment_hash12(b"LANE")),
        spec_format: SpecFormat::Xml,
    };
    let first = write_redirect_blocks_for(ws.path(), &facts).unwrap();
    assert_eq!(first.len(), 3, "the first run writes all three");
    let bytes = REDIRECT_FILES
        .iter()
        .map(|name| fs::read(ws.path().join(name)).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(bytes[0], bytes[1]);
    assert_eq!(bytes[1], bytes[2]);

    let second = write_redirect_blocks_for(ws.path(), &facts).unwrap();
    assert!(
        second.is_empty(),
        "equal bytes are not rewritten, so there is no second write: {second:?}"
    );
    for (name, before) in REDIRECT_FILES.iter().zip(&bytes) {
        assert_eq!(&fs::read(ws.path().join(name)).unwrap(), before);
        let text = String::from_utf8(before.clone()).unwrap();
        // The block leads and the authored prose follows it, unchanged.
        assert!(text.starts_with("<vibevm>\n"), "{name}");
        assert!(text.contains("\n# Mine\n\nAuthored prose.\n"), "{name}");
    }
}

/// The expected id follows the compiled lane and nothing else: a different
/// lane rewrites the three copies, and the same lane with any other part of
/// the boot changed rewrites none of them. That is what keeps an edit of
/// `90-user.xml` — a dynamic INDEX row, never part of `STATIC.*` — from
/// touching a tracked instruction file (`#SYS-LANE-MARKERS`).
#[test]
fn the_expected_id_follows_the_compiled_lane_and_nothing_else() {
    let ws = TempDir::new().unwrap();
    let with = |lane: &[u8]| RouterFacts {
        node_id: "org.x/demo".to_owned(),
        static_hash12: Some(crate::system_prompt::segment_hash12(lane)),
        spec_format: SpecFormat::Xml,
    };
    write_redirect_blocks_for(ws.path(), &with(b"LANE ONE")).unwrap();
    let first = fs::read_to_string(ws.path().join("AGENTS.md")).unwrap();

    // A dynamic boot file's bytes are not an input to the facts at all, so
    // re-running the identical generation writes nothing.
    fs::create_dir_all(ws.path().join(vibe_core::layout::current_boot_dir())).unwrap();
    fs::write(
        ws.path().join(crate::layout_paths::boot("90-user.xml")),
        "<spec/>",
    )
    .unwrap();
    assert!(
        write_redirect_blocks_for(ws.path(), &with(b"LANE ONE"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fs::read_to_string(ws.path().join("AGENTS.md")).unwrap(),
        first
    );

    // A changed lane does rewrite all three — the accepted cost of D6.
    let rewritten = write_redirect_blocks_for(ws.path(), &with(b"LANE TWO")).unwrap();
    assert_eq!(rewritten.len(), 3);
    let second = fs::read_to_string(ws.path().join("AGENTS.md")).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        first.replace(
            &crate::system_prompt::segment_hash12(b"LANE ONE"),
            &crate::system_prompt::segment_hash12(b"LANE TWO")
        ),
        second,
        "the expected id is the only byte that moved"
    );
}

/// Codex truncates a project instruction file at 32 KiB with no notice, so a
/// block that would land past the limit is a refusal before any write — a
/// project whose own rules silently never reach the model is the one state
/// this gate exists to prevent (owner decision D11).
#[test]
fn a_block_past_the_instruction_file_limit_refuses_and_writes_nothing() {
    let ws = TempDir::new().unwrap();
    let facts = RouterFacts {
        node_id: "org.x/demo".to_owned(),
        static_hash12: Some("0123456789ab".to_owned()),
        spec_format: SpecFormat::Xml,
    };
    // Authored prose far past the limit. Since the block moved to the top
    // this is **no longer** a refusal — which is the whole point of moving
    // it: a truncation now costs authored prose, not the boot router.
    let padding = "x".repeat(PROJECT_DOC_MAX_BYTES);
    let before = format!("# Mine\n\n{padding}\n");
    for name in REDIRECT_FILES {
        fs::write(ws.path().join(name), &before).unwrap();
    }
    let written = write_redirect_blocks_for(ws.path(), &facts)
        .expect("a long authored file no longer loses its router");
    assert_eq!(written.len(), REDIRECT_FILES.len());
    for name in REDIRECT_FILES {
        let content = fs::read_to_string(ws.path().join(name)).unwrap();
        assert!(content.starts_with("<vibevm>\n"), "{name}");
        assert!(content.contains(&padding), "{name}: authored prose is kept");
    }

    // The refusal survives as a guard, reachable only by declaring a limit
    // smaller than the block itself.
    for name in REDIRECT_FILES {
        fs::write(ws.path().join(name), &before).unwrap();
    }
    let error = write_redirect_blocks_bounded(ws.path(), &facts, 64)
        .expect_err("a block past the declared limit refuses");
    let text = error.to_string();
    assert!(text.contains("64"), "{text}");
    assert!(text.contains("truncates"), "{text}");
    assert!(
        text.contains("[system_prompt.harness.codex] doc_max_bytes"),
        "the refusal names the field that raises the limit: {text}"
    );
    for name in REDIRECT_FILES {
        assert_eq!(
            fs::read_to_string(ws.path().join(name)).unwrap(),
            before,
            "{name} must be untouched by a refusal"
        );
    }
}

/// The same file comfortably inside the limit is written — the refusal is a
/// budget and not a ban on authored prose.
#[test]
fn a_block_inside_the_instruction_file_limit_is_written() {
    let ws = TempDir::new().unwrap();
    let facts = RouterFacts {
        node_id: "org.x/demo".to_owned(),
        static_hash12: Some("0123456789ab".to_owned()),
        spec_format: SpecFormat::Xml,
    };
    let before = format!("# Mine\n\n{}\n", "x".repeat(20_000));
    for name in REDIRECT_FILES {
        fs::write(ws.path().join(name), &before).unwrap();
    }
    assert_eq!(
        write_redirect_blocks_for(ws.path(), &facts).unwrap().len(),
        3
    );
    let content = fs::read_to_string(ws.path().join("AGENTS.md")).unwrap();
    assert!(content.contains("# Session boot — project org.x/demo"));
    assert!(content.len() < PROJECT_DOC_MAX_BYTES);
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

#[cfg(test)]
#[path = "block_tests.rs"]
mod block_tests;
