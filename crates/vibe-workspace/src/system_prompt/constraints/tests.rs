//! C1 and C4: the shape rule, the proof that it is total in version 1, and
//! the two reachable findings with their clean counterparts.

use std::fs;

use tempfile::TempDir;
use vibe_core::manifest::{HarnessDelivery, Manifest};

use super::*;

/// One node directory carrying the named instruction files with the given
/// managed-block bodies.
fn node(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("a node temp dir");
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("the node manifest");
    for (name, block) in files {
        fs::write(
            dir.path().join(name),
            format!("# demo\n\n<vibevm>\n{block}\n</vibevm>\n\nAuthored prose.\n"),
        )
        .expect("the instruction file");
    }
    dir
}

fn manifest(dir: &TempDir) -> Manifest {
    Manifest::read(dir.path().join("vibe.toml")).expect("the node manifest reads")
}

#[test]
fn every_tier_version_one_can_declare_needs_the_same_block_shape() {
    // C1's shape half is **total** today, and this is the pin that says so
    // rather than a comment claiming it. The day `import` deserialises,
    // `BlockShape::of` stops being exhaustive over one shape and this test
    // fails — which is the signal that C1's refusal has become reachable
    // and needs its own project-level wiring.
    for delivery in [HarnessDelivery::Launch, HarnessDelivery::Read] {
        assert_eq!(
            BlockShape::of(delivery),
            BlockShape::Router,
            "{delivery} needs another block shape, so C1 is now reachable"
        );
    }
    let mixed = "[system_prompt.harness.claude]\ndelivery = \"launch\"\n\n\
                 [system_prompt.harness.codex]\ndelivery = \"read\"\n";
    let dir = TempDir::new().expect("a manifest temp dir");
    fs::write(
        dir.path().join("vibe.toml"),
        format!("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n{mixed}"),
    )
    .expect("the mixed manifest");
    // `[system_prompt]` is admitted only beside the authored template: the
    // template's presence is the whole opt-in.
    let template = dir
        .path()
        .join(vibe_core::layout::current_boot_system_template());
    fs::create_dir_all(template.parent().expect("a parented path")).expect("the boot directory");
    fs::write(&template, "# frame\n").expect("the authored template");
    let manifest = Manifest::read(dir.path().join("vibe.toml")).expect("the mixed manifest reads");
    assert_eq!(
        manifest_block_shape(&manifest),
        Ok(BlockShape::Router),
        "a mixed launch/read declaration is admitted: both tiers share one router block"
    );
}

#[test]
fn c1_refuses_two_tiers_that_need_different_blocks() {
    let refusal = one_block_shape(&[
        ("claude", BlockShape::Router),
        ("gemini", BlockShape::Import),
    ])
    .expect_err("C1 refuses the mixture");
    assert!(refusal.starts_with("C1:"), "{refusal}");
    assert!(
        refusal.contains("system_prompt.harness.claude"),
        "{refusal}"
    );
    assert!(
        refusal.contains("system_prompt.harness.gemini"),
        "{refusal}"
    );
    assert!(refusal.contains("byte-identical"), "{refusal}");
}

#[test]
fn c1_admits_any_number_of_harnesses_that_agree() {
    assert_eq!(one_block_shape(&[]), Ok(BlockShape::Router));
    assert_eq!(
        one_block_shape(&[
            ("claude", BlockShape::Import),
            ("codex", BlockShape::Import),
            ("gemini", BlockShape::Import),
        ]),
        Ok(BlockShape::Import),
        "all-or-none is satisfied by all, not only by the default"
    );
}

#[test]
fn byte_identical_instruction_files_raise_nothing() {
    let dir = node(&[
        ("CLAUDE.md", "the one router"),
        ("AGENTS.md", "the one router"),
        ("GEMINI.md", "the one router"),
    ]);
    let findings = delivery_constraints(dir.path(), &manifest(&dir)).expect("the scan runs");
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn one_instruction_file_alone_raises_nothing() {
    let dir = node(&[("AGENTS.md", "the one router")]);
    let findings = delivery_constraints(dir.path(), &manifest(&dir)).expect("the scan runs");
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn a_divergent_copy_is_c1_and_c4_naming_the_file() {
    let dir = node(&[
        ("CLAUDE.md", "the one router"),
        ("AGENTS.md", "a hand-edited router"),
    ]);
    let findings = delivery_constraints(dir.path(), &manifest(&dir)).expect("the scan runs");
    assert_eq!(findings.len(), 2, "{findings:?}");
    assert!(findings[0].starts_with("C1:"), "{findings:?}");
    assert!(findings[0].contains("AGENTS.md"), "{findings:?}");
    assert!(findings[0].contains("CLAUDE.md"), "{findings:?}");
    assert!(findings[1].contains("AGENTS.md"), "{findings:?}");
    assert!(findings[1].starts_with("C4:"), "{findings:?}");
    assert!(findings[1].contains("redirect_block"), "{findings:?}");
}

#[test]
fn a_malformed_pair_is_left_to_the_redirect_block_cell() {
    // Two openers and no closer: `locate_block` calls it malformed, the
    // write path refuses it and `vibe check` errors on it. Reporting it
    // here as well would name one defect under two constraints.
    let dir = TempDir::new().expect("a node temp dir");
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("the node manifest");
    fs::write(dir.path().join("AGENTS.md"), "<vibevm>\n<vibevm>\n").expect("the broken file");
    fs::write(
        dir.path().join("CLAUDE.md"),
        "# demo\n\n<vibevm>\nthe one router\n</vibevm>\n",
    )
    .expect("the good file");
    let findings = delivery_constraints(dir.path(), &manifest(&dir)).expect("the scan runs");
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn a_node_with_no_instruction_file_raises_nothing() {
    let dir = node(&[]);
    let findings = delivery_constraints(dir.path(), &manifest(&dir)).expect("the scan runs");
    assert!(findings.is_empty(), "{findings:?}");
}
