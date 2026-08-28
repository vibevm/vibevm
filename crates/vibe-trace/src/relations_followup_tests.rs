//! Follow-up correction REDs: the one-read byte seam, the regular-file
//! floor, and the real-query integration.

use std::fs;

use specmap_core::generated::specmap::{EdgeProvenance, EdgeVerb};
use vibe_requirements::{
    ProviderOutcome, ProviderSource, QueryContext, RelationProvider, RelationRequest,
    RequirementsQuery, query,
};
use vibe_wire::generated::requirements_report::{
    RelationSourceProvenance, RelationSourceState, RequirementSourceKind,
};

use crate::SpecmapRelationProvider;
use crate::relations::{FIXTURE_MAP_FILENAME, fixture_edge};
use crate::relations_tests::{LOCK_HASH, fixture_map};

fn provider_source<'a>(
    kind: RequirementSourceKind,
    package: &'a str,
    root: Option<&'a std::path::Path>,
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
    workspace_root: &'a std::path::Path,
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

#[test]
fn the_accepted_edges_come_from_the_hashed_byte_not_the_disk() {
    // C1's decisive seam: `carried_outcome_from_bytes` is pure over the
    // bytes it is handed. The slot on disk carries a DIFFERENT valid
    // map; the bytes passed in are the recorded ones — the accepted
    // edges must be the recorded byte's edges. An implementation that
    // re-read the disk would return the other map's edges (or stale).
    let ws = tempfile::TempDir::new().unwrap();
    let coordinate = "org.example/pkg";
    let recorded = fixture_map(coordinate);
    let recorded_bytes = serde_json::to_string(&recorded).unwrap().into_bytes();
    let row_sha = vibe_workspace::vibedeps::sha256_bytes(&recorded_bytes);

    // The disk holds a DIFFERENT, equally valid map with a different
    // single edge (symbol `disk::swapped`).
    let mut swapped = fixture_map(coordinate);
    swapped.edges = vec![fixture_edge(
        &format!("spec://{coordinate}/RULE#P"),
        EdgeVerb::Implements,
        EdgeProvenance::Proposed,
        "disk::swapped",
        "src/swapped.rs",
        1,
    )];
    let slot = ws
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.example.pkg")
        .join("1.0.0");
    fs::create_dir_all(&slot).unwrap();
    fs::write(
        slot.join(FIXTURE_MAP_FILENAME),
        serde_json::to_string(&swapped).unwrap(),
    )
    .unwrap();

    let address = format!("spec://{coordinate}/RULE#P");
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(LOCK_HASH),
    );
    let outcome = crate::relations::carried_outcome_from_bytes(
        &recorded_bytes,
        &row_sha,
        &source,
        &request(
            ws.path(),
            std::slice::from_ref(&source),
            std::slice::from_ref(&address),
        ),
        "vibevm/vibedeps/org.example.pkg/1.0.0",
    );
    let ProviderOutcome::Available { edges } = outcome else {
        panic!("the recorded byte must be Available: {outcome:?}")
    };
    assert_eq!(edges.len(), 1);
    assert_eq!(
        edges[0].1.symbol, "pkg::t",
        "edges come from the BYTE, not the disk"
    );
}

#[test]
fn a_non_regular_map_entry_is_refused_before_the_read() {
    // C4: a directory (or symlink/reparse) wearing the map's name is
    // unavailable, before any read is attempted.
    let ws = tempfile::TempDir::new().unwrap();
    let coordinate = "org.example/pkg";
    let slot = ws
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.example.pkg")
        .join("1.0.0");
    fs::create_dir_all(slot.join(FIXTURE_MAP_FILENAME)).unwrap();
    let record = vibe_workspace::vibedeps::SlotRecord {
        schema: vibe_workspace::vibedeps::SLOT_RECORD_SCHEMA,
        source_hash: vibe_core::ContentHash::parse(LOCK_HASH).unwrap(),
        spec_format: vibe_core::manifest::SpecFormat::Mixed,
        converter_recipe: None,
        derived_hash: None,
        overlay_hash: None,
        files: vec![vibe_workspace::vibedeps::SlotFile {
            path: FIXTURE_MAP_FILENAME.to_string(),
            sha256: vibe_workspace::vibedeps::sha256_bytes(b"whatever"),
            source: None,
            disposition: None,
        }],
    };
    vibe_workspace::vibedeps::write_slot_record(&slot, &record).unwrap();
    let address = format!("spec://{coordinate}/RULE#P");
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            std::slice::from_ref(&source),
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "carried-map-unavailable"
    );
}

#[test]
fn a_hardlinked_map_is_not_an_owned_carried_byte() {
    let ws = tempfile::TempDir::new().unwrap();
    let coordinate = "org.example/pkg";
    let slot = ws
        .path()
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.example.pkg")
        .join("1.0.0");
    fs::create_dir_all(&slot).unwrap();
    let bytes = serde_json::to_vec(&fixture_map(coordinate)).unwrap();
    let alias = ws.path().join("map-alias.json");
    fs::write(&alias, &bytes).unwrap();
    fs::hard_link(&alias, slot.join(FIXTURE_MAP_FILENAME)).unwrap();
    let record = vibe_workspace::vibedeps::SlotRecord {
        schema: vibe_workspace::vibedeps::SLOT_RECORD_SCHEMA,
        source_hash: vibe_core::ContentHash::parse(LOCK_HASH).unwrap(),
        spec_format: vibe_core::manifest::SpecFormat::Mixed,
        converter_recipe: None,
        derived_hash: None,
        overlay_hash: None,
        files: vec![vibe_workspace::vibedeps::SlotFile {
            path: FIXTURE_MAP_FILENAME.to_string(),
            sha256: vibe_workspace::vibedeps::sha256_bytes(&bytes),
            source: None,
            disposition: None,
        }],
    };
    vibe_workspace::vibedeps::write_slot_record(&slot, &record).unwrap();
    let address = format!("spec://{coordinate}/RULE#P");
    let source = provider_source(
        RequirementSourceKind::Package,
        coordinate,
        Some(&slot),
        Some(LOCK_HASH),
    );
    let answer = SpecmapRelationProvider
        .relations(&request(
            ws.path(),
            std::slice::from_ref(&source),
            std::slice::from_ref(&address),
        ))
        .unwrap();
    assert_eq!(
        reason_of(answer_of(&answer, coordinate)),
        "carried-map-unavailable"
    );
    assert!(alias.exists(), "the refusal never mutates the alias");
}

#[test]
fn the_real_query_attaches_a_real_edge_to_a_real_fact_row() {
    // C3: an addressed `@fact` row and a code edge citing that EXACT
    // full address, through the actual landed query.
    let root = tempfile::TempDir::new().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        root.path().join("specmap.toml"),
        format!(
            "namespace = \"org.example/demo\"\nscan_roots = [\"crates/*\"]\nspec_roots = [\"{}\"]\n",
            vibe_core::layout::current_specs_root()
                .to_string_lossy()
                .replace('\\', "/")
        ),
    )
    .unwrap();
    let specs = root.path().join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(
        specs.join("D.md"),
        "## The rule {#req-r}\n\n@fact:RULE The rule itself. @status:impl/done\n",
    )
    .unwrap();
    let src = root.path().join("crates/x/src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.rs"),
        "#[verifies(\"spec://org.example/demo/D#RULE\")]\nfn t() {}\n",
    )
    .unwrap();
    let address = "spec://org.example/demo/D#RULE".to_string();

    let context = QueryContext {
        selected_root: root.path().to_path_buf(),
        observed_at: "2026-01-01T00:00:00Z".parse().unwrap(),
        lifecycle_run_id: None,
    };
    let q = RequirementsQuery::try_new(None, 100, true).unwrap();
    let report = query(&q, &context, Some(&SpecmapRelationProvider)).unwrap();

    // One requirement row exists, and its edge is attached with exact
    // coordinates.
    assert_eq!(
        report.rows.len(),
        1,
        "{:?}",
        report
            .rows
            .iter()
            .map(|row| row.address.clone())
            .collect::<Vec<_>>()
    );
    let row = &report.rows[0];
    assert_eq!(row.address, address);
    assert_eq!(row.relations.len(), 1, "{row:?}");
    let edge = &row.relations[0];
    assert_eq!(
        edge.verb,
        vibe_wire::generated::requirements_report::RequirementRelationVerb::Verifies
    );
    assert_eq!(edge.symbol, "x::t");
    assert!(edge.file.ends_with("crates/x/src/lib.rs"), "{}", edge.file);
    assert!(edge.line >= 1);

    // The relation source is Current/FreshProjectMap, derived by the
    // LIBRARY from the base kind.
    let relation = &report.relation_sources[0];
    assert_eq!(relation.state, RelationSourceState::Current);
    assert_eq!(
        relation.provenance,
        RelationSourceProvenance::FreshProjectMap
    );

    // relations=false: the same base row, zero edges, NotRequested/None.
    let plain = query(
        &RequirementsQuery::default(),
        &context,
        Some(&SpecmapRelationProvider),
    )
    .unwrap();
    assert_eq!(plain.rows.len(), 1);
    assert!(plain.rows[0].relations.is_empty());
    let relation = &plain.relation_sources[0];
    assert_eq!(relation.state, RelationSourceState::NotRequested);
    assert_eq!(relation.provenance, RelationSourceProvenance::None);

    // Nothing was written.
    assert!(!root.path().join("specmap.json").exists());
}
