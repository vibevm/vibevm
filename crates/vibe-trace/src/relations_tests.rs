//! A3 adapter oracles: host/current matrix, the carried trust ladder,
//! edge projection, fences.

use std::fs;
use std::path::{Path, PathBuf};

use specmap_core::generated::specmap::{EdgeProvenance, EdgeVerb, Specmap};
use vibe_requirements::{
    ProviderOutcome, ProviderSource, QueryContext, RelationProvider, RelationRequest,
    RequirementsQuery, query,
};
use vibe_wire::generated::requirements_report::{
    RelationSourceProvenance, RelationSourceState, RequirementSourceKind,
};

use crate::SpecmapRelationProvider;
use crate::relations::{FIXTURE_MAP_FILENAME, fixture_edge};

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

fn answer_of<'a>(answer: &'a [(String, ProviderOutcome)], package: &str) -> &'a ProviderOutcome {
    &answer
        .iter()
        .find(|(name, _)| name == package)
        .unwrap_or_else(|| panic!("no outcome for {package}: {answer:?}"))
        .1
}

fn reason_of(outcome: &ProviderOutcome) -> &str {
    outcome.reason().expect("loss outcome carries a reason")
}

// --- Host / current -------------------------------------------------------

/// A workspace root with `specmap.toml` (namespace `ns`), one spec doc
/// carrying the requested fact, and one `#[verifies]` code file.
fn host_tree(root: &Path, ns: &str) -> String {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("specmap.toml"),
        format!(
            "namespace = \"{ns}\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"{}\"]\n",
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
        "## The rule {#req-r}\n`req r1`\n\nIt MUST hold.\n",
    )
    .unwrap();
    let src = root.join("crates/x/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        format!("#[verifies(\"spec://{ns}/D#req-r\")]\nfn t() {{}}\n"),
    )
    .unwrap();
    format!("spec://{ns}/D#req-r")
}

// --- Package / carried trust ladder ---------------------------------------

/// One fully-trusted carried fixture: a real slot record (mixed shape)
/// whose files row owns the map, and the map JSON the row hashes.
fn carried_slot(
    ws: &Path,
    group: &str,
    name: &str,
    version: &str,
    source_hash: &str,
    map: &Specmap,
) -> PathBuf {
    let slot = ws
        .join(vibe_core::layout::current_vibedeps_root())
        .join(format!("{group}.{name}"))
        .join(version);
    fs::create_dir_all(&slot).unwrap();
    let map_json = serde_json::to_string(map).unwrap();
    fs::write(slot.join(FIXTURE_MAP_FILENAME), &map_json).unwrap();
    let record = vibe_workspace::vibedeps::SlotRecord {
        schema: vibe_workspace::vibedeps::SLOT_RECORD_SCHEMA,
        source_hash: vibe_core::ContentHash::parse(source_hash).unwrap(),
        spec_format: vibe_core::manifest::SpecFormat::Mixed,
        converter_recipe: None,
        derived_hash: None,
        overlay_hash: None,
        files: vec![vibe_workspace::vibedeps::SlotFile {
            path: FIXTURE_MAP_FILENAME.to_string(),
            sha256: vibe_workspace::vibedeps::sha256_file(&slot.join(FIXTURE_MAP_FILENAME))
                .unwrap(),
            source: None,
            disposition: None,
        }],
    };
    vibe_workspace::vibedeps::write_slot_record(&slot, &record).unwrap();
    slot
}

pub(crate) fn fixture_map(coordinate: &str) -> Specmap {
    let mut map: Specmap = serde_json::from_str(
        r#"{"schema":3,"spec_units":[],"code_items":[],"edges":[],"suspects":[],"warnings":[]}"#,
    )
    .unwrap();
    map.edges = vec![
        fixture_edge(
            &format!("spec://{coordinate}/RULE#P"),
            EdgeVerb::Verifies,
            EdgeProvenance::Authored,
            "pkg::t",
            "src/lib.rs",
            7,
        ),
        // An edge outside the request's addresses: filtered, not
        // returned for the library to drop.
        fixture_edge(
            &format!("spec://{coordinate}/RULE#OTHER"),
            EdgeVerb::Implements,
            EdgeProvenance::Generated,
            "pkg::u",
            "src/other.rs",
            9,
        ),
    ];
    map
}

pub(crate) const LOCK_HASH: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_HASH: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn the_carried_ladder_walks_every_rung() {
    let ws = tempfile::TempDir::new().unwrap();
    let coordinate = "org.example/pkg";
    let address = format!("spec://{coordinate}/RULE#P");

    // 1: no materialised root.
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        None,
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            &[source],
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "package-slot-absent"
    );

    // 2: missing expected hash.
    let slot = carried_slot(
        ws.path(),
        "org.example",
        "pkg",
        "1.0.0",
        LOCK_HASH,
        &fixture_map(coordinate),
    );
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        None,
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            &[source],
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "lock-content-hash-missing"
    );

    // 3: no slot record at all (the fixture wrote one — a second slot
    // without one stays unavailable; in-place slots land here too).
    let bare = ws
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.example.bare")
        .join("2.0.0");
    fs::create_dir_all(&bare).unwrap();
    let source = provider_source(
        RequirementSourceKind::Package,
        "org.example/bare",
        Some(&bare),
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(ws.path(), &[source], &[]))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, "org.example/bare")),
        "slot-record-unavailable"
    );

    // 4: source-hash mismatch.
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(OTHER_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            &[source],
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "slot-source-hash-mismatch"
    );

    // 5: record owns no map row.
    let mut record = vibe_workspace::vibedeps::read_slot_record(&slot).unwrap();
    record.files.clear();
    vibe_workspace::vibedeps::write_slot_record(&slot, &record).unwrap();
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            &[source],
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "carried-map-not-shipped"
    );
    // Restore the full record for the remaining rungs.
    let mut record = vibe_workspace::vibedeps::read_slot_record(&slot).unwrap();
    record.files = vec![vibe_workspace::vibedeps::SlotFile {
        path: FIXTURE_MAP_FILENAME.to_string(),
        sha256: vibe_workspace::vibedeps::sha256_file(&slot.join(FIXTURE_MAP_FILENAME)).unwrap(),
        source: None,
        disposition: None,
    }];
    vibe_workspace::vibedeps::write_slot_record(&slot, &record).unwrap();

    // 6: the recorded map file is absent.
    fs::remove_file(slot.join(FIXTURE_MAP_FILENAME)).unwrap();
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            &[source],
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "carried-map-unavailable"
    );

    // 7: the map byte was modified after publication.
    let map = fixture_map(coordinate);
    fs::write(
        slot.join(FIXTURE_MAP_FILENAME),
        serde_json::to_string(&map).unwrap(),
    )
    .unwrap();
    fs::write(slot.join(FIXTURE_MAP_FILENAME), "{ tweaked }").unwrap();
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            &[source],
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "carried-map-modified"
    );

    // 8: a MATCHING byte that does not parse — hash the written file
    // with the shared helper so the record row matches exactly.
    let matching_bad = b"{ \"schema\": 3, \"edges\": \"not a list\" }";
    fs::write(slot.join(FIXTURE_MAP_FILENAME), matching_bad).unwrap();
    let digest = vibe_workspace::vibedeps::sha256_file(&slot.join(FIXTURE_MAP_FILENAME)).unwrap();
    let mut record = vibe_workspace::vibedeps::read_slot_record(&slot).unwrap();
    record.files = vec![vibe_workspace::vibedeps::SlotFile {
        path: FIXTURE_MAP_FILENAME.to_string(),
        sha256: digest,
        source: None,
        disposition: None,
    }];
    vibe_workspace::vibedeps::write_slot_record(&slot, &record).unwrap();
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            &[source],
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "carried-map-unparseable"
    );

    // 9: fully matching, valid — the only Available rung.
    let slot = carried_slot(ws.path(), "org.example", "pkg", "1.0.0", LOCK_HASH, &map);
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(ws.path(), &[source], &[address]))
        .unwrap();
    let ProviderOutcome::Available { edges } = answer_of(&answer, coordinate) else {
        panic!("the matching rung must be Available: {answer:?}")
    };
    // Exactly the requested address; coordinates only; the slot-prefixed
    // workspace-relative file; specmap order preserved; no dedup.
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].0, format!("spec://{coordinate}/RULE#P"));
    assert_eq!(edges[0].1.symbol, "pkg::t");
    assert_eq!(edges[0].1.line, 7);
    let expected_prefix = format!(
        "{}/{}/{}/1.0.0",
        vibe_core::layout::current_vibedeps_root()
            .to_string_lossy()
            .replace('\\', "/"),
        "org.example.pkg",
        ""
    )
    .replace("//", "/");
    let expected_file = format!(
        "{}/org.example.pkg/1.0.0/src/lib.rs",
        vibe_core::layout::current_vibedeps_root()
            .to_string_lossy()
            .replace('\\', "/")
    );
    let _ = expected_prefix;
    assert_eq!(edges[0].1.file, expected_file);
}

#[test]
fn one_degraded_map_leaves_the_other_package_available() {
    let ws = tempfile::TempDir::new().unwrap();
    let first = carried_slot(
        ws.path(),
        "org.example",
        "one",
        "1.0.0",
        LOCK_HASH,
        &fixture_map("org.example/one"),
    );
    let second = carried_slot(
        ws.path(),
        "org.example",
        "two",
        "1.0.0",
        LOCK_HASH,
        &fixture_map("org.example/two"),
    );
    // Degrade ONLY the first map's byte.
    fs::write(first.join(FIXTURE_MAP_FILENAME), "tampered").unwrap();
    let addresses = vec![
        "spec://org.example/one/RULE#P".to_string(),
        "spec://org.example/two/RULE#P".to_string(),
    ];
    let sources = [
        provider_source(
            RequirementSourceKind::Package,
            "org.example/one",
            Some(&first),
            Some(LOCK_HASH),
        ),
        provider_source(
            RequirementSourceKind::Package,
            "org.example/two",
            Some(&second),
            Some(LOCK_HASH),
        ),
    ];
    let answer = SpecmapRelationProvider
        .relations(&request(ws.path(), &sources, &addresses))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, "org.example/one")),
        "carried-map-modified"
    );
    assert!(matches!(
        answer_of(&answer, "org.example/two"),
        ProviderOutcome::Available { .. }
    ));
}

#[test]
fn the_landed_query_invokes_the_adapter_and_derives_wire_states() {
    // Host-only project; the adapter is injected into the real query.
    let root = tempfile::TempDir::new().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let address = host_tree(root.path(), "org.example/demo");
    let context = QueryContext {
        selected_root: root.path().to_path_buf(),
        observed_at: "2026-01-01T00:00:00Z".parse().unwrap(),
        lifecycle_run_id: None,
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &context, Some(&SpecmapRelationProvider)).unwrap();
    let relation = &report
        .relation_sources
        .iter()
        .find(|source| source.package == "org.example/demo")
        .unwrap();
    // The LIBRARY derived current/fresh from the base kind.
    assert_eq!(relation.state, RelationSourceState::Current);
    assert_eq!(
        relation.provenance,
        RelationSourceProvenance::FreshProjectMap
    );
    // The edge projection itself is proven by the adapter unit tests
    // (the fixture doc carries a spec unit, not an `@fact:` marker, so
    // the requirements row set is empty by design; the addresses list
    // the adapter filtered against was correspondingly empty).
    let _ = address;
    assert!(report.rows.is_empty());
    // relations=false: not-requested, adapter never consulted (the
    // counter law itself is pinned by the A2 fake-provider suite).
    let plain = query(
        &RequirementsQuery::default(),
        &context,
        Some(&SpecmapRelationProvider),
    )
    .unwrap();
    let relation = &plain.relation_sources[0];
    assert_eq!(relation.state, RelationSourceState::NotRequested);
    assert!(plain.rows.iter().all(|row| row.relations.is_empty()));
    assert!(!root.path().join("specmap.json").exists());
}

#[test]
fn the_adapter_owns_no_second_authority_and_the_dependency_floor_holds() {
    // RED 3: no discover/resolve_foreign/lock/manifest authority in the
    // adapter — the request is its only input.
    let source = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/relations.rs"),
    )
    .unwrap();
    for forbidden in ["discover(", "resolve_foreign", "vibe.lock", "vibe.toml"] {
        assert!(
            !source.contains(forbidden),
            "a second authority appeared in the adapter: {forbidden}"
        );
    }
    // RED 9: the dependency floor — vibe-requirements + vibe-workspace
    // only, no provider/LLM/network edge.
    let cargo =
        fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .unwrap();
    assert!(cargo.contains("vibe-requirements"));
    assert!(cargo.contains("vibe-workspace"));
    for forbidden in [
        "vibe-llm",
        "reqwest",
        "vibe-registry",
        "specmap-core = { package",
    ] {
        assert!(
            !cargo.contains(forbidden),
            "forbidden dependency: {forbidden}"
        );
    }
}

// --- Follow-up corrections ---
