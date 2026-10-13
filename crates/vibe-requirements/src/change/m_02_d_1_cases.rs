use super::*;
use crate::complete::CapturedFactSet;
use std::{fs, path::Path, sync::Arc};
use vibe_core::manifest::{
    Manifest,
    change::{ChangeContext, ChangeId},
};
use vibe_workspace::source_query::SourceQueryService;
#[test]
fn complete_origin_builds_bound_local_roles_and_refuses_missing_fact() {
    let fixture = super::m_02_a_1_cases::Fixture::new();
    let inventory = SourceQueryService::prepare_host(&fixture.alpha)
        .unwrap()
        .retain_source_inventory("org.fixture", "reusable-intent")
        .unwrap();
    let facts = CapturedFactSet::observe(Arc::new(inventory)).unwrap();
    let node =
        Manifest::parse_str(&fs::read_to_string(fixture.alpha.join("vibe.toml")).unwrap()).unwrap();
    let context = ChangeContext::for_node(&node, ".", Some(Path::new("docs/custom"))).unwrap();
    let id = ChangeId::parse("chg-0123456789abcdef0123456789abcdef").unwrap();
    let output = prepare_scaffold(
        &context,
        &id,
        "quoted\"slug",
        "Title\n[malicious]",
        &facts.select_all(),
    )
    .unwrap();
    assert_eq!(
        output.files.keys().map(String::as_str).collect::<Vec<_>>(),
        ["acceptance.md", "change.toml", "intent.md", "refinement.md"]
    );
    assert_eq!(output.analysis.manifest.title(), "Title\n[malicious]");
    assert_eq!(
        output.analysis.manifest.origins()[0].package,
        "feat:org.fixture/reusable-intent@=1.0.0"
    );
    assert_eq!(
        output.analysis.manifest.origins()[0]
            .selected_content_hash
            .as_deref(),
        Some(facts.inventory().content_hash().as_str())
    );
    assert_eq!(
        output.selected_addresses,
        facts.facts().keys().cloned().collect::<Vec<_>>()
    );
    assert_eq!(output.analysis.manifest.documents().len(), 3);
    assert!(!output.source_strengths.is_empty());
    assert!(
        output
            .source_strengths
            .values()
            .all(|s| s == "record-consistent")
    );
    let manifest_text = std::str::from_utf8(&output.files["change.toml"]).unwrap();
    let escaped = manifest_text.replacen("path = \"intent.md\"", "path = \"../intent.md\"", 1);
    assert_ne!(escaped, manifest_text);
    let error = vibe_core::manifest::change::ChangeManifest::parse_str(
        &escaped,
        &context,
        &output.directory,
    )
    .unwrap_err();
    assert_eq!(
        error.kind,
        vibe_core::manifest::change::ChangeErrorKind::Path
    );
    assert_eq!(error.field, "documents.path");
    for name in ["acceptance.md", "intent.md", "refinement.md"] {
        let text = std::str::from_utf8(&output.files[name]).unwrap();
        assert!(text.contains("@status:spec"));
        assert!(!text.contains("done"));
    }
    assert!(matches!(
        prepare_scaffold(
            &context,
            &id,
            "local",
            "Local",
            &facts.select_exact(&["spec://org.fixture/reusable-intent/FEAT-001#ABSENT".into()])
        ),
        Err(ScaffoldError::Incomplete)
    ));
    assert!(!fixture.alpha.join(&output.directory).exists());
    fixture.assert_origin_leaves_unchanged();
    println!("SCAFFOLD_FACTS {:?}", output.selected_addresses);
}
