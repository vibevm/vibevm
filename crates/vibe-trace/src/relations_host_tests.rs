//! A3 adapter host/current oracles: config matrix, namespace gate,
//! in-memory build, member/origin path prefixing.

use std::fs;
use std::path::Path;

use vibe_requirements::{ProviderOutcome, ProviderSource, RelationProvider, RelationRequest};
use vibe_wire::generated::requirements_report::RequirementSourceKind;

use crate::SpecmapRelationProvider;

fn provider_source<'a>(
    kind: RequirementSourceKind,
    package: &'a str,
    root: Option<&'a Path>,
    hash: Option<&'a str>,
) -> ProviderSource<'a> {
    ProviderSource {
        kind,
        package,
        root,
        expected_content_hash: hash,
    }
}

fn request<'a>(
    workspace_root: &'a Path,
    sources: &'a [ProviderSource<'a>],
    addresses: &'a [String],
) -> RelationRequest<'a> {
    RelationRequest {
        selected_root: workspace_root,
        workspace_root,
        sources,
        addresses,
    }
}

fn answer_of<'a>(
    answer: &'a [(String, vibe_requirements::ProviderOutcome)],
    package: &str,
) -> &'a vibe_requirements::ProviderOutcome {
    &answer
        .iter()
        .find(|(name, _)| name == package)
        .unwrap_or_else(|| panic!("no outcome for {package}: {answer:?}"))
        .1
}

/// A host tree with `specmap.toml` (namespace `ns`), one spec doc, and
/// one `#[verifies]` code file; returns the minted address.
fn host_tree(root: &Path, ns: &str) -> String {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("specmap.toml"),
        format!(
            "namespace = \"{ns}\"
scan_roots = [\"crates/*\"]
spec_roots = [\"{}\"]
",
            vibe_core::layout::current_specs_root()
                .to_string_lossy()
                .replace('\\', "/")
        ),
    )
    .unwrap();
    let specs = root.join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("D.md"),
        "## The rule {#req-r}
`req r1`

It MUST hold.
",
    )
    .unwrap();
    let src = root.join("crates/x/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        format!(
            "#[verifies(\"spec://{ns}/D#req-r\")]
fn t() {{}}
"
        ),
    )
    .unwrap();
    format!("spec://{ns}/D#req-r")
}

#[test]
fn host_config_absent_malformed_and_namespace_mismatch_are_typed_losses() {
    let root = tempfile::TempDir::new().unwrap();
    let source = provider_source(
        RequirementSourceKind::Host,
        "org.example/demo",
        Some(root.path()),
        None,
    );
    let addresses: Vec<String> = Vec::new();

    // No specmap.toml at all.
    let answer = SpecmapRelationProvider
        .relations(&request(
            root.path(),
            std::slice::from_ref(&source),
            &addresses,
        ))
        .unwrap();
    assert!(matches!(
        answer_of(&answer, "org.example/demo"),
        ProviderOutcome::Unavailable { reason } if reason == "project-map-config-absent"
    ));

    // Malformed config.
    fs::write(root.path().join("specmap.toml"), "not [ valid toml\n").unwrap();
    let answer = SpecmapRelationProvider
        .relations(&request(
            root.path(),
            std::slice::from_ref(&source),
            &addresses,
        ))
        .unwrap();
    assert!(matches!(
        answer_of(&answer, "org.example/demo"),
        ProviderOutcome::Invalid { reason } if reason == "project-map-config-invalid"
    ));

    // A namespace that is not the host coordinate: never a false
    // zero-edge current.
    let address = host_tree(root.path(), "other/ns");
    let answer = SpecmapRelationProvider
        .relations(&request(
            root.path(),
            std::slice::from_ref(&source),
            &[address],
        ))
        .unwrap();
    assert!(matches!(
        answer_of(&answer, "org.example/demo"),
        ProviderOutcome::Unavailable { reason } if reason == "project-map-namespace-mismatch"
    ));
}

#[test]
fn host_success_builds_edges_in_memory_and_writes_no_specmap_json() {
    let root = tempfile::TempDir::new().unwrap();
    // The namespace IS the host coordinate.
    let specs = root.path().join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("D.md"),
        "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
    )
    .unwrap();
    let src = root.path().join("crates/x/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        "#[verifies(\"spec://org.example/demo/D#req-r\")]\nfn t() {}\n",
    )
    .unwrap();
    fs::write(
        root.path().join("specmap.toml"),
        format!(
            "namespace = \"org.example/demo\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"{}\"]\n",
            vibe_core::layout::current_specs_root().to_string_lossy().replace('\\', "/")
        ),
    )
    .unwrap();
    let address = "spec://org.example/demo/D#req-r".to_string();
    let source = provider_source(
        RequirementSourceKind::Host,
        "org.example/demo",
        Some(root.path()),
        None,
    );

    // Zero requested addresses: honest zero-edge Available (Q14 shape).
    let empty: Vec<String> = Vec::new();
    let answer = SpecmapRelationProvider
        .relations(&request(root.path(), std::slice::from_ref(&source), &empty))
        .unwrap();
    assert!(matches!(
        answer_of(&answer, "org.example/demo"),
        ProviderOutcome::Available { edges } if edges.is_empty()
    ));

    let answer = SpecmapRelationProvider
        .relations(&request(
            root.path(),
            std::slice::from_ref(&source),
            std::slice::from_ref(&address),
        ))
        .unwrap();
    let ProviderOutcome::Available { edges } = answer_of(&answer, "org.example/demo") else {
        panic!("host success must be Available: {answer:?}")
    };
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].0, address);
    assert_eq!(edges[0].1.symbol, "x::t");
    assert_eq!(
        edges[0].1.verb,
        vibe_wire::generated::requirements_report::RequirementRelationVerb::Verifies
    );
    assert!(
        edges[0].1.file.ends_with("crates/x/src/lib.rs"),
        "{}",
        edges[0].1.file
    );
    assert!(
        !root.path().join("specmap.json").exists(),
        "nothing written"
    );
}

#[test]
fn a_selected_member_host_gets_its_prefix_and_an_outside_root_is_invalid() {
    let ws = tempfile::TempDir::new().unwrap();
    let member = ws.path().join("pkg");
    let address = host_tree(&member, "org.example/demo");
    let source = provider_source(
        RequirementSourceKind::Host,
        "org.example/demo",
        Some(&member),
        None,
    );
    let answer = SpecmapRelationProvider
        .relations(&request(ws.path(), &[source], &[address]))
        .unwrap();
    let ProviderOutcome::Available { edges } = answer_of(&answer, "org.example/demo") else {
        panic!("member host must be Available: {answer:?}")
    };
    assert!(edges[0].1.file.starts_with("pkg/"), "{}", edges[0].1.file);

    // A root outside the workspace is a typed invalid for that source.
    let elsewhere = tempfile::TempDir::new().unwrap();
    let _ = host_tree(elsewhere.path(), "org.example/demo");
    let source = provider_source(
        RequirementSourceKind::Host,
        "org.example/demo",
        Some(elsewhere.path()),
        None,
    );
    let answer = SpecmapRelationProvider
        .relations(&request(ws.path(), &[source], &[]))
        .unwrap();
    assert!(matches!(
        answer_of(&answer, "org.example/demo"),
        ProviderOutcome::Invalid { reason } if reason == "relation-root-outside-workspace"
    ));
}

#[test]
fn an_outside_root_refuses_before_any_source_io() {
    // C2's ordering proof: the outside root carries a MALFORMED config
    // (and the package variant a malformed record). Had the adapter
    // read first, the outcome would be config-invalid/record-loss —
    // it must be root-outside with zero source I/O.
    let ws = tempfile::TempDir::new().unwrap();
    let elsewhere = tempfile::TempDir::new().unwrap();
    fs::write(elsewhere.path().join("specmap.toml"), "not [ valid toml\n").unwrap();

    let source = provider_source(
        RequirementSourceKind::Host,
        "org.example/demo",
        Some(elsewhere.path()),
        None,
    );
    let answer = SpecmapRelationProvider
        .relations(&request(ws.path(), std::slice::from_ref(&source), &[]))
        .unwrap();
    assert!(matches!(
        answer_of(&answer, "org.example/demo"),
        ProviderOutcome::Invalid { reason } if reason == "relation-root-outside-workspace"
    ));

    // The package twin: an outside slot with a corrupted record still
    // answers root-outside, never slot-record-unavailable.
    let slot = elsewhere
        .path()
        .join("vibevm/vibedeps/org.example.pkg/1.0.0");
    fs::create_dir_all(&slot).unwrap();
    fs::write(slot.join(".vibe-slot.toml"), "schema = 9\n").unwrap();
    let source = provider_source(
        RequirementSourceKind::Package,
        "org.example/pkg",
        Some(&slot),
        Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(ws.path(), std::slice::from_ref(&source), &[]))
        .unwrap();
    assert!(matches!(
        answer_of(&answer, "org.example/pkg"),
        ProviderOutcome::Invalid { reason } if reason == "relation-root-outside-workspace"
    ));
}
