//! Query acceptance matrix: partial-vs-abort, source states, mapping,
//! prefix/truncation, canaries and fences.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use vibe_wire::generated::shared::Timestamp;

use crate::text::render;
use crate::{QueryContext, QueryError, RequirementsQuery, query};

const FIXED_AT: &str = "2026-01-01T00:00:00Z";
pub(crate) const RUN_ID: &str = "0123456789abcdef0123456789abcdef";

fn at() -> Timestamp {
    FIXED_AT.parse().expect("fixed timestamp")
}

pub(crate) fn ctx(root: &Path) -> QueryContext {
    QueryContext {
        selected_root: root.to_path_buf(),
        observed_at: at(),
        lifecycle_run_id: None,
    }
}

pub(crate) fn ctx_with_run(root: &Path, run_id: Option<&str>) -> QueryContext {
    QueryContext {
        selected_root: root.to_path_buf(),
        observed_at: at(),
        lifecycle_run_id: run_id.map(str::to_string),
    }
}

/// A minimal standalone project: host `org.example/demo`, one spec
/// document, no `.vibe`, no lock, no registry.
pub(crate) fn project(body: &str) -> TempDir {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    write_spec(root.path(), "RULE.md", body);
    root
}

fn write_spec(root: &Path, rel: &str, body: &str) {
    let path = root.join(vibe_core::layout::current_specs_root()).join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

/// Write a one-package lock for `org.example/pkg@1.0.0`.
pub(crate) fn write_lock(root: &Path, in_place: bool) {
    let materialization = if in_place { "in-place" } else { "copy" };
    fs::write(
        root.join("vibe.lock"),
        format!(
            "[meta]\ngenerated_by = \"fixture\"\ngenerated_at = \"2026-01-01T00:00:00Z\"\n\
             schema_version = 6\n\n[[package]]\nkind = \"feat\"\nname = \"pkg\"\n\
             group = \"org.example\"\nversion = \"1.0.0\"\n\
             source_url = \"https://example.invalid/pkg.git\"\n\
             content_hash = \"sha256:{}\"\nmaterialization = \"{materialization}\"\n",
            "a".repeat(64)
        ),
    )
    .unwrap();
}

/// The versioned slot path for `org.example/pkg@1.0.0` under `root`.
fn slot(root: &Path) -> PathBuf {
    let group = vibe_core::Group::parse("org.example").unwrap();
    let version = "1.0.0".parse().unwrap();
    vibe_workspace::vibedeps::slot_abs_path(root, &group, "pkg", &version)
}

pub(crate) fn make_slot(root: &Path, body: &str) {
    let slot = slot(root);
    let specs = slot.join(vibe_core::layout::current_specs_root());
    fs::create_dir_all(&specs).unwrap();
    fs::write(specs.join("RULE.md"), body).unwrap();
}

/// One adoption entry for a package address, through the crate's own
/// deterministic writer.
fn adopt(root: &Path, address: &str, status: Option<&str>) {
    let mut registry = vibe_facts::Registry::load(root).unwrap();
    let status = status.map(|value| vibe_facts::FactStatus::parse(value).unwrap());
    let entry = vibe_facts::FactEntry {
        address: address.to_string(),
        origin: vibe_facts::FactOrigin::Package,
        package: Some("org.example/pkg".to_string()),
        status,
        comment: None,
    };
    registry.upsert(root, entry).unwrap();
}

fn tree_snapshot(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    out
}

pub(crate) fn find_source<'a>(
    report: &'a vibe_wire::generated::requirements_report::RequirementsReport,
    package: &str,
) -> &'a vibe_wire::generated::requirements_report::SourceResult {
    report
        .sources
        .iter()
        .find(|source| source.source.package == package)
        .unwrap_or_else(|| panic!("no source result for `{package}`: {report:?}"))
}

#[test]
fn an_unacceptable_query_refuses_before_any_filesystem_access() {
    // The selected root does not exist: had the query touched it first,
    // the failure would be the workspace error, not the query refusal.
    let missing = PathBuf::from("Z:/definitely-not-here");
    for bad in [
        RequirementsQuery::try_new(None, 0, false).unwrap_err(),
        RequirementsQuery::try_new(None, 257, false).unwrap_err(),
        RequirementsQuery::try_new(Some("req-one"), 100, false).unwrap_err(),
        RequirementsQuery::try_new(Some("spec://org.demo/x\n"), 100, false).unwrap_err(),
    ] {
        assert!(matches!(bad, QueryError::InvalidQuery { .. }), "{bad:?}");
    }
    let smuggled = RequirementsQuery {
        limit: 0,
        ..RequirementsQuery::default()
    };
    let error = query(&smuggled, &ctx(&missing), None).unwrap_err();
    assert!(
        matches!(error, QueryError::InvalidQuery { .. }),
        "refused before filesystem: {error:?}"
    );
    assert!(!missing.exists());
}

#[test]
fn a_real_lifecycle_lease_does_not_block_the_read_only_query() {
    let root = project("# Rules\n\n@fact:A a.\n");
    let canonical = fs::canonicalize(root.path()).unwrap();
    let lease = vibe_lifecycle::LifecycleLease::acquire(&canonical)
        .expect("lease acquired in a quiet tempdir");
    let before = tree_snapshot(root.path());
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let after = tree_snapshot(root.path());
    assert_eq!(before, after, "the query created no file or directory");
    assert_eq!(report.rows.len(), 1);
    drop(lease);
}

#[test]
fn an_absent_state_returns_a_partial_host_report_and_creates_nothing() {
    let root = project("# Rules\n\n@fact:A a.\n");
    let before = tree_snapshot(root.path());
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let after = tree_snapshot(root.path());
    assert_eq!(before, after, "the query created no file or directory");

    assert_eq!(report.sources.len(), 1, "host only: {report:?}");
    let host = find_source(&report, "org.example/demo");
    assert_eq!(host.state, source_state("available"));
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].address, "spec://org.example/demo/RULE#A");
    assert!(report.observation.lifecycle_run_id.is_none());
}

pub(crate) fn source_state(
    spelling: &str,
) -> vibe_wire::generated::requirements_report::SourceResultState {
    use vibe_wire::generated::requirements_report::SourceResultState as S;
    match spelling {
        "available" => S::Available,
        "unavailable" => S::Unavailable,
        "invalid" => S::Invalid,
        _ => S::Orphaned,
    }
}

#[test]
fn a_malformed_authored_source_is_invalid_with_a_digest_and_no_rows() {
    let root = project("# Bad\n\n@fact:DUP one\n\n@fact:DUP two\n");
    write_lock(root.path(), false);
    make_slot(root.path(), "# P\n\n@fact:P p.\n");
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let host = find_source(&report, "org.example/demo");
    assert_eq!(host.state, source_state("invalid"));
    assert!(host.digest.as_deref().unwrap().starts_with("sha256:"));
    assert_eq!(host.reason_code.as_deref(), Some("authored-source-invalid"));
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.address.starts_with("spec://org.example/pkg/")),
        "no row from an invalid source: {report:?}"
    );
}

#[test]
fn an_absent_slot_is_unavailable_and_orphaned_wins_when_entries_survive() {
    let root = project("# Rules\n\n@fact:A a.\n");
    write_lock(root.path(), false); // locked, never materialised
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let pkg = find_source(&report, "org.example/pkg");
    assert_eq!(pkg.state, source_state("unavailable"));
    assert!(pkg.digest.is_none());
    assert_eq!(pkg.reason_code.as_deref(), Some("no-materialised-slot"));
    assert!(pkg.adoption_entries.is_none());

    // Now the registry carries entries for the absent slot's coordinate:
    // the positive count is the more informative observation.
    adopt(
        root.path(),
        "spec://org.example/pkg/RULE#X",
        Some("impl/done"),
    );
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let pkg = find_source(&report, "org.example/pkg");
    assert_eq!(pkg.state, source_state("orphaned"));
    assert_eq!(pkg.adoption_entries, Some(1));
    assert_eq!(
        pkg.reason_code.as_deref(),
        Some("no-materialised-slot-with-adoption-entries")
    );
}

#[test]
fn a_registry_only_coordinate_is_an_orphaned_source_observation() {
    let root = project("# Rules\n\n@fact:A a.\n");
    adopt(
        root.path(),
        "spec://org.example/pkg/RULE#X",
        Some("impl/done"),
    );
    adopt(root.path(), "spec://org.example/pkg/RULE#Y", None);
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let orphan = find_source(&report, "org.example/pkg");
    assert_eq!(orphan.state, source_state("orphaned"));
    assert_eq!(orphan.adoption_entries, Some(2));
    assert_eq!(
        orphan.reason_code.as_deref(),
        Some("registry-only-adoption-entries")
    );
    // Never a fake unmarked fact row for the orphan.
    assert!(report.rows.iter().all(|row| row.address.contains("/demo/")));
}

#[test]
fn a_host_with_zero_documents_is_available_with_the_empty_source_digest() {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup = \"org.example\"\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let host = find_source(&report, "org.example/demo");
    assert_eq!(host.state, source_state("available"));
    assert!(host.digest.as_deref().unwrap().starts_with("sha256:"));
    assert!(report.rows.is_empty());
}

#[test]
fn authoring_and_adoption_map_through_all_their_states() {
    let root = project("# Rules\n\n@fact:MARKED m. @status:spec/plan\n\n@fact:UNMARKED u.\n");
    write_lock(root.path(), false);
    make_slot(
        root.path(),
        "# P\n\n@fact:MARKED m. @status:impl/done\n\n@fact:ABSENT a.\n\n@fact:UNSET n.\n",
    );
    adopt(
        root.path(),
        "spec://org.example/pkg/RULE#MARKED",
        Some("impl/done"),
    );
    adopt(root.path(), "spec://org.example/pkg/RULE#UNSET", None);

    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let row = |address: &str| {
        report
            .rows
            .iter()
            .find(|row| row.address == address)
            .unwrap_or_else(|| panic!("missing {address}: {report:?}"))
    };
    // Host: not-applicable regardless of registry content; unmarked stays.
    let host_unmarked = row("spec://org.example/demo/RULE#UNMARKED");
    assert_eq!(
        host_unmarked.authoring.presence,
        authoring_presence("unmarked")
    );
    assert_eq!(
        host_unmarked.adoption.presence,
        adoption_presence("not-applicable")
    );
    // Package: recorded / indeterminate / absent, and marked authoring.
    let recorded = row("spec://org.example/pkg/RULE#MARKED");
    assert_eq!(recorded.adoption.presence, adoption_presence("recorded"));
    assert_eq!(
        recorded.adoption.status.as_ref().map(status_spelling),
        Some("impl/done".to_string())
    );
    let unset = row("spec://org.example/pkg/RULE#UNSET");
    assert_eq!(unset.adoption.presence, adoption_presence("indeterminate"));
    let absent = row("spec://org.example/pkg/RULE#ABSENT");
    assert_eq!(absent.adoption.presence, adoption_presence("absent"));
    assert_eq!(absent.authoring.presence, authoring_presence("unmarked"));
    // Deterministic global address order.
    let addresses: Vec<&str> = report.rows.iter().map(|r| r.address.as_str()).collect();
    let mut sorted = addresses.clone();
    sorted.sort();
    assert_eq!(addresses, sorted);
}

fn authoring_presence(
    spelling: &str,
) -> vibe_wire::generated::requirements_report::AuthoringObservationPresence {
    use vibe_wire::generated::requirements_report::AuthoringObservationPresence as P;
    match spelling {
        "marked" => P::Marked,
        _ => P::Unmarked,
    }
}

fn adoption_presence(
    spelling: &str,
) -> vibe_wire::generated::requirements_report::AdoptionObservationPresence {
    use vibe_wire::generated::requirements_report::AdoptionObservationPresence as P;
    match spelling {
        "not-applicable" => P::NotApplicable,
        "absent" => P::Absent,
        "indeterminate" => P::Indeterminate,
        _ => P::Recorded,
    }
}

fn status_spelling(status: &vibe_wire::generated::requirements_report::FactStatus) -> String {
    use vibe_wire::generated::requirements_report::{FactStatusStage as S, FactStatusState as T};
    let stage = match status.stage {
        S::Unknown => "unknown",
        S::Idea => "idea",
        S::Spec => "spec",
        S::Impl => "impl",
        S::Test => "test",
        S::Doc => "doc",
        S::Freeze => "freeze",
    };
    let state = match status.state {
        T::Hold => "hold",
        T::Plan => "plan",
        T::Work => "work",
        T::Done => "done",
        T::Void => "void",
    };
    format!("{stage}/{state}")
}

#[test]
fn a_prefix_prunes_sources_and_scopes_rows_literally() {
    let root = project("# Rules\n\n@fact:A a.\n");
    write_lock(root.path(), false);
    make_slot(root.path(), "# P\n\n@fact:P p.\n");
    let q = RequirementsQuery::try_new(Some("spec://org.example/pkg"), 100, false).unwrap();
    let report = query(&q, &ctx(root.path()), None).unwrap();
    assert_eq!(report.sources.len(), 1, "host pruned: {report:?}");
    assert_eq!(
        find_source(&report, "org.example/pkg").state,
        source_state("available")
    );
    assert!(
        report
            .rows
            .iter()
            .all(|row| row.address.starts_with("spec://org.example/pkg/"))
    );
    // A prefix that names no full coordinate prunes nothing.
    let q = RequirementsQuery::try_new(Some("spec://org.example"), 100, false).unwrap();
    let report = query(&q, &ctx(root.path()), None).unwrap();
    assert_eq!(report.sources.len(), 2);
    // And the literal row scope still applies post-parse.
    let q = RequirementsQuery::try_new(Some("spec://org.example/pkg/RULE#P"), 100, false).unwrap();
    let report = query(&q, &ctx(root.path()), None).unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].address, "spec://org.example/pkg/RULE#P");
}

#[test]
fn the_row_bound_is_inclusive_and_truncation_is_honest() {
    assert!(RequirementsQuery::try_new(None, 1, false).is_ok());
    assert!(RequirementsQuery::try_new(None, 256, false).is_ok());
    assert!(RequirementsQuery::try_new(None, 0, false).is_err());
    assert!(RequirementsQuery::try_new(None, 257, false).is_err());

    let root = project("# Rules\n\n@fact:B b.\n\n@fact:A a.\n");
    let q = RequirementsQuery::try_new(None, 1, false).unwrap();
    let report = query(&q, &ctx(root.path()), None).unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(
        report.rows[0].address, "spec://org.example/demo/RULE#A",
        "sorted first"
    );
    assert!(report.truncated);
    let q = RequirementsQuery::try_new(None, 2, false).unwrap();
    let report = query(&q, &ctx(root.path()), None).unwrap();
    assert_eq!(report.rows.len(), 2);
    assert!(!report.truncated);
}

#[test]
fn the_clock_alone_does_not_move_the_observation_id_but_the_run_id_does() {
    let root = project("# Rules\n\n@fact:A a.\n");
    let mut later = ctx(root.path());
    later.observed_at = "2030-06-06T06:06:06Z".parse().unwrap();
    let a = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let b = query(&RequirementsQuery::default(), &later, None).unwrap();
    assert_eq!(a.observation.observation_id, b.observation.observation_id);
    assert_ne!(a.observation.observed_at, b.observation.observed_at);

    let with_run = query(
        &RequirementsQuery::default(),
        &ctx_with_run(root.path(), Some(RUN_ID)),
        None,
    )
    .unwrap();
    assert_eq!(
        with_run.observation.lifecycle_run_id.as_deref(),
        Some(RUN_ID)
    );
    assert_ne!(
        a.observation.observation_id, with_run.observation.observation_id,
        "the run join key moves the id"
    );
    let bad = query(
        &RequirementsQuery::default(),
        &ctx_with_run(root.path(), Some("not-hex")),
        None,
    )
    .unwrap_err();
    assert!(matches!(bad, QueryError::InvalidRunId { .. }));
}

#[test]
fn a_registry_only_raw_edit_moves_only_the_scoped_members() {
    let root = project("# Rules\n\n@fact:A a.\n");
    write_lock(root.path(), false);
    make_slot(root.path(), "# P\n\n@fact:P p.\n");
    adopt(
        root.path(),
        "spec://org.example/pkg/RULE#P",
        Some("impl/done"),
    );
    let before = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();

    let registry_file = root
        .path()
        .join(vibe_core::layout::current_vibefacts_root())
        .join("org.example.pkg.toml");
    let mut raw = fs::read(&registry_file).unwrap();
    raw.extend_from_slice(b"# comment-only registry edit\n");
    fs::write(&registry_file, &raw).unwrap();

    let after = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    // Authoring, source results, relations and the run axis: unchanged.
    assert_eq!(after.sources, before.sources);
    assert_eq!(
        after
            .rows
            .iter()
            .map(|row| (row.address.clone(), row.authoring.clone()))
            .collect::<Vec<_>>(),
        before
            .rows
            .iter()
            .map(|row| (row.address.clone(), row.authoring.clone()))
            .collect::<Vec<_>>(),
    );
    // The scoped digests: both move — registry bytes are source bytes.
    assert_ne!(
        before.observation.source_digest,
        after.observation.source_digest
    );
    assert_ne!(
        before.observation.observation_id,
        after.observation.observation_id
    );
}

#[test]
fn the_body_canary_is_absent_from_json_text_and_debug() {
    let canary = "CANARY-QUERY-4d71";
    let root = project(format!("# Rules\n\n@fact:A mentions {canary} in prose.\n").as_str());
    let report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    let json = serde_json::to_string(&report).unwrap();
    for hay in [&json, &render(&report), &format!("{report:?}")] {
        assert!(!hay.contains(canary), "canary leaked into a projection");
    }
}

#[test]
fn a_hand_broken_report_is_red_under_the_p1_validator() {
    let root = project("# Rules\n\n@fact:A a.\n");
    let mut report = query(&RequirementsQuery::default(), &ctx(root.path()), None).unwrap();
    assert!(vibe_wire::behaviour::requirements_report::validate(&report).is_ok());
    report.rows[0].address = "spec://org.example/other/RULE#A".to_string();
    assert!(
        vibe_wire::behaviour::requirements_report::validate(&report).is_err(),
        "a row whose address names another package must be refused"
    );
}

#[test]
fn surfaces_cannot_assemble_reports_and_the_dependency_floor_holds() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Q9: no report STRUCT-LITERAL assembly outside this crate's one
    // function. Thin P3 surfaces may type their return (`RequirementsReport`
    // as a type name); what they may never do is construct the members.
    for surface in ["../vibe-cli/src", "../vibe-mcp/src"] {
        let mut stack = vec![manifest.join(surface)];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let source = fs::read_to_string(&path).unwrap();
                    assert!(
                        !source.contains("RequirementsReport {"),
                        "{} assembles report members",
                        path.display()
                    );
                }
            }
        }
    }
    // Q10: no forbidden dependency in this crate or vibe-facts.
    for cargo in ["Cargo.toml", "../vibe-facts/Cargo.toml"] {
        let text = fs::read_to_string(manifest.join(cargo)).unwrap();
        for forbidden in [
            "vibe-trace",
            "specmap",
            "vibe-llm",
            "vibe-registry",
            "reqwest",
        ] {
            assert!(!text.contains(forbidden), "{cargo} depends on {forbidden}");
        }
    }
    // And no synthetic-verdict vocabulary in the product sources.
    for source in [
        "lib.rs",
        "query.rs",
        "sources.rs",
        "rows.rs",
        "provider.rs",
        "text.rs",
        "digest.rs",
        "error.rs",
    ] {
        let text = fs::read_to_string(manifest.join("src").join(source)).unwrap();
        for word in ["\"unmet\"", "\"fulfilled\"", "\"verified\""] {
            assert!(
                !text.contains(word),
                "{source} mints the verdict word {word}"
            );
        }
    }
}
