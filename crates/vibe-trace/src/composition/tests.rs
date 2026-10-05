use super::*;
use vibe_requirements::{ProviderOutcome, ProviderSource, RelationProvider, RelationRequest};
use vibe_wire::generated::requirements_report::RequirementSourceKind;

const SUBJECT: &str = "spec://org.example/demo/Protocol#identity";

pub(super) fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("crates/demo/src")).unwrap();
    fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    fs::write(
        root.join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/demo/Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(root.join("crates/demo/src/lib.rs"), format!("#![feature(register_tool)]\n#![register_tool(specmark)]\n#[specmark::spec(implements = \"{SUBJECT}\", r = 1)]\npub fn identity(n: u64) -> u64 {{ n }}\n#[test]\nfn identity_cases() {{ assert_eq!(identity(0), 0); assert_eq!(identity(u64::MAX), u64::MAX); }}\n")).unwrap();
    fs::write(root.join("vibevm/vibespecs/Protocol.md"), "# Protocol {#root}\n\n## Identity {#identity}\n`req r1`\n\nIdentity preserves its input.\n").unwrap();
    fs::write(
        root.join("Example.lean"),
        "import Std\nnamespace Example\ntheorem identity (n : Nat) : n = n := rfl\nend Example\n",
    )
    .unwrap();
    fs::write(root.join("lean-proof.toml"), format!("schema = 1\nnamespace = \"org.example/demo\"\ntoolchain = \"leanprover/lean4:v4.34.1\"\nsources = [\"Example.lean\"]\nallowed_axioms = []\n[[proof]]\nsubject = \"{SUBJECT}\"\nspec_file = \"vibevm/vibespecs/Protocol.md\"\nspec_revision = 1\nmodule = \"Example\"\ntheorem = \"Example.identity\"\ngrade = \"g1_model\"\n")).unwrap();
    fs::write(root.join("specmap.toml"), format!("namespace = \"org.example/demo\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"vibevm/vibespecs\"]\n[composition]\nschema = 1\nkind = \"rust_lean\"\nsubject = \"{SUBJECT}\"\nrust_symbol = \"demo::identity\"\nlean_carrier = \"lean-proof.toml\"\n")).unwrap();
    temp
}

#[test]
fn combined_write_and_fresh_relations_preserve_both_same_subject_edges() {
    let temp = fixture();
    let root = temp.path();
    let loaded = load(root).unwrap().unwrap();
    let map = write(root, &loaded, "specmap.json", None).unwrap();
    let edges: Vec<_> = map
        .edges
        .iter()
        .filter(|edge| edge.uri == SUBJECT)
        .collect();
    assert_eq!(edges.len(), 2);
    assert!(
        edges
            .iter()
            .any(|e| e.fromSymbol == "demo::identity" && matches!(e.verb, EdgeVerb::Implements))
    );
    assert!(
        edges
            .iter()
            .any(|e| e.fromSymbol == "lean::Example.identity"
                && matches!(e.verb, EdgeVerb::Verifies))
    );
    let before = fs::read(root.join("specmap.json")).unwrap();
    let sources = [ProviderSource {
        kind: RequirementSourceKind::Host,
        package: "org.example/demo",
        root: Some(root),
        expected_content_hash: None,
    }];
    let addresses = [SUBJECT.to_owned()];
    let answer = crate::SpecmapRelationProvider
        .relations(&RelationRequest {
            selected_root: root,
            workspace_root: root,
            sources: &sources,
            addresses: &addresses,
        })
        .unwrap();
    let ProviderOutcome::Available { edges } = &answer[0].1 else {
        panic!("{answer:?}");
    };
    assert_eq!(edges.len(), 2);
    assert_eq!(
        fs::read(root.join("specmap.json")).unwrap(),
        before,
        "fresh query does not write"
    );
}

#[test]
fn legacy_closed_config_refuses_before_partial_write() {
    let temp = fixture();
    let root = temp.path();
    fs::write(root.join("specmap.json"), b"sentinel-complete-map").unwrap();
    let error = Config::load(root).unwrap_err();
    assert!(error.to_string().contains("parsing") || format!("{error:#}").contains("composition"));
    assert_eq!(
        fs::read(root.join("specmap.json")).unwrap(),
        b"sentinel-complete-map"
    );
}

#[test]
fn malformed_duplicate_foreign_and_missing_sources_never_replace_map() {
    let temp = fixture();
    let root = temp.path();
    let original = fs::read_to_string(root.join("specmap.toml")).unwrap();
    fs::write(root.join("specmap.json"), b"sentinel").unwrap();
    for bad in [
        format!("{original}\n[composition]\nschema=1\n"),
        original.replace("kind = \"rust_lean\"", "kind = \"unknown\""),
        original.replace(SUBJECT, "spec://org.other/demo/Protocol#identity"),
    ] {
        fs::write(root.join("specmap.toml"), bad).unwrap();
        assert!(load(root).is_err());
        assert_eq!(fs::read(root.join("specmap.json")).unwrap(), b"sentinel");
    }
    fs::write(root.join("specmap.toml"), original).unwrap();
    fs::remove_file(root.join("Example.lean")).unwrap();
    let loaded = load(root).unwrap().unwrap();
    assert!(write(root, &loaded, "specmap.json", None).is_err());
    assert_eq!(fs::read(root.join("specmap.json")).unwrap(), b"sentinel");
}

#[test]
fn rust_only_map_remains_the_core_map() {
    let temp = fixture();
    let root = temp.path();
    let original = fs::read_to_string(root.join("specmap.toml")).unwrap();
    fs::write(
        root.join("specmap.toml"),
        original.split("[composition]").next().unwrap(),
    )
    .unwrap();
    let loaded = load(root).unwrap().unwrap();
    let old = specmap_core::index::build(root, &loaded.config);
    let new = build(root, &loaded, None).unwrap();
    assert_eq!(
        specmap_core::index::to_canonical_bytes(&old).unwrap(),
        specmap_core::index::to_canonical_bytes(&new).unwrap()
    );
}

#[test]
fn fresh_read_refuses_missing_scanner_inputs_and_foreign_proof_coverage() {
    let temp = fixture();
    let root = temp.path();
    let loaded = load(root).unwrap().unwrap();
    write(root, &loaded, "specmap.json", None).unwrap();
    let saved = fs::read(root.join("specmap.json")).unwrap();
    let sources = [ProviderSource {
        kind: RequirementSourceKind::Host,
        package: "org.example/demo",
        root: Some(root),
        expected_content_hash: None,
    }];
    let addresses = [SUBJECT.to_owned()];
    let query = || {
        crate::SpecmapRelationProvider
            .relations(&RelationRequest {
                selected_root: root,
                workspace_root: root,
                sources: &sources,
                addresses: &addresses,
            })
            .unwrap()
    };
    fs::remove_file(root.join("Example.lean")).unwrap();
    let answer = query();
    assert!(
        matches!(&answer[0].1, ProviderOutcome::Unavailable { reason } if reason == "project-map-composition-incomplete")
    );
    let config = fs::read_to_string(root.join("specmap.toml")).unwrap();
    fs::write(
        root.join("specmap.toml"),
        config.replace(SUBJECT, "spec://org.other/demo/Protocol#identity"),
    )
    .unwrap();
    let answer = query();
    assert!(
        matches!(&answer[0].1, ProviderOutcome::Unavailable { reason } if reason == "cross-package-proof-coverage-unsupported")
    );
    assert_eq!(fs::read(root.join("specmap.json")).unwrap(), saved);
}
