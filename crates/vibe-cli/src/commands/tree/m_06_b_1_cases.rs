use super::*;
#[test]
fn repeated_shared_sources_and_exact_nested_why_chain() {
    let text = "<!-- vibe:static host — boot/a.md -->\n<!-- embed: outer -->\n<!-- embed: inner -->\nx\n<!-- /embed: inner -->\n<!-- /embed: outer -->\n<!-- vibe:static host — boot/a.md -->\ny\n";
    let capture = capture_static("exact-summary-17".into(), text).unwrap();
    assert_eq!(capture.capture_id, "exact-summary-17");
    let graph = capture.graph;
    assert_eq!(graph.contributions.len(), 2);
    assert_eq!(graph.contributions[0].seed, graph.contributions[1].seed);
    let row = &graph.contributions[0];
    // Scanner records closing order: inner then outer.
    assert_eq!(
        vibe_workspace::boot_debugger::why_included(row, 1),
        Some(vec![0, 2, 1])
    );
    assert_eq!(row.occurrences[1].artifact_lines, Some((3, 5)));
    assert_eq!(row.occurrences[1].state, vibe_spec::ReferenceState::Unknown);
}

#[test]
fn shared_dynamic_index_keeps_cli_order_and_conditions() {
    let text = "static = \"STATIC.md\"\n[[entry]]\npath = \"same.md\"\nkind = \"dynamic\"\nwhen = \"os:linux\"\n[[entry]]\npath = \"same.md\"\nkind = \"static\"\n";
    let parsed = read_index(text).unwrap();
    assert_eq!(parsed.static_pointer.as_deref(), Some("STATIC.md"));
    assert_eq!(parsed.entries.len(), 2);
    assert_eq!(parsed.entries[0].path, parsed.entries[1].path);
    assert_eq!(parsed.entries[0].when.as_deref(), Some("os:linux"));
    assert_eq!(parsed.entries[0].kind, IndexKind::Dynamic);
    assert_eq!(parsed.entries[1].kind, IndexKind::Static);
    assert_eq!(parsed.entries[1].order, 1);
}
