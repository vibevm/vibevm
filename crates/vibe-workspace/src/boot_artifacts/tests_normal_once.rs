//! G1 probe: ordinary normal directives across one complete XML consumer lane.
//! These tests call the real workspace/compiler boundary and filesystem resolver;
//! they do not claim a CLI install or native Windows execution.
#![cfg(test)]
use super::super::tests::{boot, coord, entry_normal};
use super::super::*;
use std::fs;
use tempfile::TempDir;
use vibe_core::manifest::SpecFormat;

#[cfg(test)]
fn document(root: &Path, package: &str, relative: &str, body: &str) -> String {
    let slot = crate::layout_paths::vibedeps(format!("org.g1.{package}/1.0.0"));
    let relative = crate::layout_paths::slot_specs(slot, relative);
    let path = root.join(&relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
    relative
}

#[cfg(test)]
fn observe(label: &str, xml: &str, markers: &[&str]) -> Vec<usize> {
    vibe_spec::minify_emitted_xml(xml).expect("ordinary emitted lane must remain valid XML");
    let counts: Vec<_> = markers
        .iter()
        .map(|marker| xml.matches(marker).count())
        .collect();
    let table = markers
        .iter()
        .zip(&counts)
        .map(|(marker, count)| format!("{marker}\t{count}\n"))
        .collect::<String>();
    eprintln!("{label}:\n{table}");
    if let Some(directory) = std::env::var_os("NST_G1_EVIDENCE") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(format!("{label}.xml")), xml).unwrap();
        fs::write(directory.join(format!("{label}.counts.tsv")), table).unwrap();
    }
    counts
}

#[cfg(test)]
fn render(root: &Path, effective: &EffectiveBoot) -> String {
    render_static_with_spec_format(effective, root, &coord(), SpecFormat::Xml)
        .unwrap()
        .expect("the fixture declares static entries")
}

#[cfg(test)]
fn umbrella(root: &Path) -> EffectiveBoot {
    let umbrella = document(
        root,
        "umbrella",
        "boot/entry.md",
        "# Umbrella {#root}\n#use spec://org.g1/alpha/contract/root#root\n\nG1_UMBRELLA_TEXT\n",
    );
    let alpha = document(
        root,
        "alpha",
        "contract/root.md",
        "# Alpha {#root}\n#use spec://org.g1/beta/contract/solution#solution\n\nG1_ALPHA_TEXT\n# Uncovered {#uncovered}\nG1_ALPHA_UNCOVERED_TEXT\n",
    );
    let beta = document(
        root,
        "beta",
        "contract/solution.md",
        "# Beta {#solution}\n#source spec://org.g1/beta/source/solution#implementation\n\nG1_BETA_CONTRACT_TEXT\n",
    );
    document(
        root,
        "beta",
        "source/solution.md",
        "# Implementation {#implementation}\n\nG1_BETA_IMPLEMENTATION_TEXT\n",
    );
    boot(vec![
        entry_normal(umbrella, "org.g1/umbrella"),
        entry_normal(alpha, "org.g1/alpha"),
        entry_normal(beta, "org.g1/beta"),
    ])
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#NORMAL-CLOSURE-ONCE"
)]
fn xml_whole_consumer_normal_use_and_source_emit_shared_documents_once() {
    let ws = TempDir::new().unwrap();
    let effective = umbrella(ws.path());
    let markers = [
        "G1_UMBRELLA_TEXT",
        "G1_ALPHA_TEXT",
        "G1_ALPHA_UNCOVERED_TEXT",
        "G1_BETA_CONTRACT_TEXT",
        "G1_BETA_IMPLEMENTATION_TEXT",
    ];
    // Individual ordinary compiler calls establish reach and tree shaking;
    // whole-lane coverage must not become shared state between calls.
    for (index, expected) in [[1, 1, 0, 1, 1], [0, 1, 1, 1, 1], [0, 0, 0, 1, 1]]
        .into_iter()
        .enumerate()
    {
        let unit = render(ws.path(), &boot(vec![effective.entries[index].clone()]));
        assert_eq!(observe(&format!("unit-{index}"), &unit, &markers), expected);
    }
    let lane = render(ws.path(), &effective);
    assert_eq!(
        observe("consumer-whole", &lane, &markers),
        [1, 1, 1, 1, 1],
        "same source sections repeated across normal contributions"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#NORMAL-CLOSURE-ONCE"
)]
fn xml_partial_then_whole_then_direct_preserves_preamble_and_uncovered_sibling_once() {
    let ws = TempDir::new().unwrap();
    let partial = document(
        ws.path(),
        "partial",
        "boot/entry.md",
        "# Partial {#root}\n#use spec://org.g1/shared/contract/root#solution\nG1_PARTIAL_CALLER\n",
    );
    let whole = document(
        ws.path(),
        "whole",
        "boot/entry.md",
        "# Whole {#root}\n#use spec://org.g1/shared/contract/root\nG1_WHOLE_CALLER\n",
    );
    let shared = document(
        ws.path(),
        "shared",
        "contract/root.md",
        "G1_AUTHORED_PREAMBLE\n# Shared {#root}\n## Solution {#solution}\nG1_NESTED_SOLUTION\n## Sibling {#sibling}\nG1_NESTED_SIBLING\n",
    );
    let lane = render(
        ws.path(),
        &boot(vec![
            entry_normal(partial, "org.g1/partial"),
            entry_normal(whole, "org.g1/whole"),
            entry_normal(shared, "org.g1/shared"),
        ]),
    );
    assert_eq!(
        observe(
            "partial-whole-direct",
            &lane,
            &[
                "G1_AUTHORED_PREAMBLE",
                "G1_NESTED_SOLUTION",
                "G1_NESTED_SIBLING",
                "G1_PARTIAL_CALLER",
                "G1_WHOLE_CALLER"
            ]
        ),
        [1, 1, 1, 1, 1]
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#NORMAL-CLOSURE-ONCE"
)]
fn xml_equal_prose_from_distinct_documents_remains_two_contributions() {
    let ws = TempDir::new().unwrap();
    let body = "# Shared wording {#root}\nG1_IDENTICAL_PROSE\n";
    let first = document(ws.path(), "first", "boot/entry.md", body);
    let second = document(ws.path(), "second", "boot/entry.md", body);
    let lane = render(
        ws.path(),
        &boot(vec![
            entry_normal(first, "org.g1/first"),
            entry_normal(second, "org.g1/second"),
        ]),
    );
    assert_eq!(
        observe("distinct-documents", &lane, &["G1_IDENTICAL_PROSE"]),
        [2]
    );
    assert!(lane.contains("org-g1--first--root"), "{lane}");
    assert!(lane.contains("org-g1--second--root"), "{lane}");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#COMPUTED-NOT-COPIED"
)]
fn normal_coverage_is_local_to_each_consumer_compilation() {
    let ws = TempDir::new().unwrap();
    let shared = document(
        ws.path(),
        "shared",
        "boot/entry.md",
        "# Shared {#root}\nG1_EVERY_CONSUMER_NEEDS_THIS\n",
    );
    let effective = boot(vec![entry_normal(shared, "org.g1/shared")]);
    for index in 0..2 {
        let lane = render(ws.path(), &effective);
        assert_eq!(
            observe(
                &format!("independent-consumer-{index}"),
                &lane,
                &["G1_EVERY_CONSUMER_NEEDS_THIS"]
            ),
            [1]
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#SOURCE-DIRECTIVE-PLACEMENT"
)]
fn xml_normal_source_is_recognized_before_heading_and_inside_section_with_following_text() {
    let ws = TempDir::new().unwrap();
    let contract = document(
        ws.path(),
        "placement",
        "contract/root.md",
        "  #source spec://org.g1/placement/source/first#first\n# Contract {#root}\nG1_BEFORE_SECOND_SOURCE\n  #source spec://org.g1/placement/source/second#second\nG1_AFTER_SECOND_SOURCE\n",
    );
    document(
        ws.path(),
        "placement",
        "source/first.md",
        "# First {#first}\nG1_FIRST_SOURCE_TEXT\n",
    );
    document(
        ws.path(),
        "placement",
        "source/second.md",
        "# Second {#second}\nG1_SECOND_SOURCE_TEXT\n",
    );
    let lane = render(
        ws.path(),
        &boot(vec![entry_normal(contract, "org.g1/placement")]),
    );
    assert_eq!(
        observe(
            "source-placement",
            &lane,
            &[
                "G1_BEFORE_SECOND_SOURCE",
                "G1_AFTER_SECOND_SOURCE",
                "G1_FIRST_SOURCE_TEXT",
                "G1_SECOND_SOURCE_TEXT"
            ]
        ),
        [1, 1, 1, 1]
    );
    assert!(!lane.contains("#source spec://org.g1/placement/"));
}

#[test]
fn xml_unresolved_normal_use_refuses_before_replacing_committed_boot_artifacts() {
    let ws = TempDir::new().unwrap();
    let bad = document(
        ws.path(),
        "bad",
        "boot/entry.md",
        "# Bad {#root}\n#use spec://org.g1/missing/contract/root#root\n",
    );
    let directory = ws.path().join(crate::layout_paths::boot(""));
    fs::create_dir_all(&directory).unwrap();
    let index = directory.join(INDEX_FILE);
    let lane = directory.join(STATIC_XML_FILE);
    fs::write(&index, b"previous-index").unwrap();
    fs::write(&lane, b"previous-static\xff").unwrap();
    let error = write_boot_artifacts_with_spec_format(
        ws.path(),
        ws.path(),
        &coord(),
        &boot(vec![entry_normal(bad, "org.g1/bad")]),
        SpecFormat::Xml,
    )
    .unwrap_err();
    assert!(
        matches!(&error,WorkspaceError::InlineCompile {reason,..} if reason.contains("org.g1/bad") && reason.contains("org.g1/missing")),
        "{error}"
    );
    assert_eq!(fs::read(index).unwrap(), b"previous-index");
    assert_eq!(fs::read(lane).unwrap(), b"previous-static\xff");
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#NORMAL-CLOSURE-ONCE"
)]
fn xml_source_add_continued_heading_keeps_both_facts_and_suppresses_only_repeat() {
    let ws = TempDir::new().unwrap();
    let caller = document(
        ws.path(),
        "caller",
        "boot/entry.md",
        "# Caller {#root}\n#use spec://org.g1/continued/contract/root#root\nCALLER_CONTINUED\n",
    );
    let contract = document(
        ws.path(),
        "continued",
        "contract/root.md",
        "# Contract {#root}\n#source spec://org.g1/continued/source/implementation#root\n##CONTRACT_FACT G1_CONTRACT_CONTINUED\n",
    );
    document(
        ws.path(),
        "continued",
        "source/implementation.md",
        "# Implementation {#root} :add\n##IMPLEMENTATION_FACT G1_SOURCE_CONTINUED\n",
    );
    let lane = render(
        ws.path(),
        &boot(vec![
            entry_normal(caller, "org.g1/caller"),
            entry_normal(contract, "org.g1/continued"),
        ]),
    );
    assert_eq!(
        observe(
            "continued-heading",
            &lane,
            &[
                "G1_CONTRACT_CONTINUED",
                "G1_SOURCE_CONTINUED",
                "CALLER_CONTINUED"
            ]
        ),
        [1, 1, 1]
    );
}
