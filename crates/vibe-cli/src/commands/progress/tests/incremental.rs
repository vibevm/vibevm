//! DRIFT-010 and DRIFT-016 — the cache accelerates a scan and never
//! changes its answer, and the verdict never leaves `cache.json`.
//!
//! Every assertion here is about what a run *answers*: a warm scan and a
//! cold one must agree, byte for byte, on every rendering; an edited file
//! must be reparsed; and `--no-cache` must reach the same place the long
//! way round. The sibling cell `writes` asks the opposite question — what
//! a run *touches* — and the two are kept apart because a test that
//! conflated them would pass while either half was broken.
//!
//! Fixtures, argument builders and the state reader are [`super`]'s, so
//! the tree these tests scan is the same one every other cell scans.

use super::*;

/// The report surface's arguments over [`super::args`] — this cell's own,
/// because the rendering equality it pins is asked nowhere else.
fn report_args(root: &Path, no_cache: bool, md: bool) -> ProgressReportArgs {
    ProgressReportArgs {
        common: args(root, no_cache),
        md,
        view: None,
        audience: None,
    }
}

/// The whole safety argument (DRIFT-010 §4.4): a warm run must answer
/// exactly what a cold run answers. Scan once into an empty campaign
/// zone (cold — every file parsed), scan again over the cache the first
/// run left (warm — every file reused), and compare what a consumer can
/// see: the two state projections and the rendered report.
#[test]
fn warm_and_cold_agree() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    incremental_fixture(root).expect("fixture tree");
    let ctx = crate::output::Context::from_flags(true, false, None, true, AgentModeArg::Auto);

    scan(&ctx, &args(root, false)).expect("cold scan");
    let cold_corpus = read_state(root, "corpus.json");
    let cold_campaign = read_state(root, "campaign.json");
    let cold_report = report_body(
        &ground(&args(root, false)).expect("cold ground"),
        &report_args(root, false, true),
        false,
    )
    .expect("cold report");

    // The cache the cold run left must now be doing the work.
    let g = ground(&args(root, false)).expect("warm ground");
    assert_eq!(g.docs.len(), 2, "both fixture files observed");
    for doc in &g.docs {
        let text = std::fs::read_to_string(root.join(&doc.path)).expect("read fixture");
        assert!(
            g.cache
                .cached_doc(
                    &doc.path,
                    &progress_core::parse::content_hash(&text),
                    &g.payloads
                )
                .is_some(),
            "`{}` is served from the cache on the warm run",
            doc.path
        );
    }

    scan(&ctx, &args(root, false)).expect("warm scan");
    assert_eq!(read_state(root, "corpus.json"), cold_corpus, "corpus.json");
    assert_eq!(
        read_state(root, "campaign.json"),
        cold_campaign,
        "campaign.json"
    );
    let warm_report = report_body(
        &ground(&args(root, false)).expect("warm ground"),
        &report_args(root, false, true),
        false,
    )
    .expect("warm report");
    assert_eq!(warm_report, cold_report, "report output");
    assert!(
        warm_report.contains(&spec_rel("a.md")),
        "a report worth comparing"
    );
}

/// §4.4 says *every* subcommand, so the renderings that put a whole
/// `ParsedDoc` on disk are compared too, not just the report table.
/// `mirror` is the sharpest of them: it serialises each document in full,
/// so a warm mirror equalling a cold one is the round-trip fidelity claim
/// stated in bytes a reviewer can diff.
#[test]
fn warm_and_cold_agree_on_every_rendering() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    incremental_fixture(root).expect("fixture tree");
    let ctx = crate::output::Context::from_flags(true, false, None, true, AgentModeArg::Auto);
    let mirror_dir = root.join("campaigns/progress-test/run/mirror");

    let read_mirror = || -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = std::fs::read_dir(&mirror_dir)
            .expect("mirror dir")
            .map(|e| {
                let p = e.expect("entry").path();
                let name = p.file_name().expect("name").to_string_lossy().into_owned();
                (name, std::fs::read_to_string(&p).expect("read mirror file"))
            })
            .collect();
        out.sort();
        out
    };

    // Cold: nothing cached yet, so every document is freshly parsed.
    mirror(&ctx, &args(root, true)).expect("cold mirror");
    let cold_mirror = read_mirror();
    let cold = ground(&args(root, true)).expect("cold ground");
    let cold_xml = report_body(&cold, &report_args(root, true, false), false).expect("cold xml");
    let cold_json = report_body(&cold, &report_args(root, true, false), true).expect("cold json");
    let cold_digest = weave::weave_digest(cold.docs.iter());
    assert_eq!(cold_mirror.len(), 2, "a mirror worth comparing");

    // Warm: the same renderings, now built from reconstructed documents.
    mirror(&ctx, &args(root, false)).expect("warm mirror");
    let warm = ground(&args(root, false)).expect("warm ground");
    assert_eq!(read_mirror(), cold_mirror, "mirror");
    assert_eq!(
        report_body(&warm, &report_args(root, false, false), false).expect("warm xml"),
        cold_xml,
        "report --xml"
    );
    assert_eq!(
        report_body(&warm, &report_args(root, false, false), true).expect("warm json"),
        cold_json,
        "report --json"
    );
    assert_eq!(weave::weave_digest(warm.docs.iter()), cold_digest, "weave");
}

/// Reuse is keyed on content, so an edit must land and only that file's
/// row may move. The sidecar payload is compared too — a stale payload
/// behind a fresh hash is exactly the failure the content check exists
/// to prevent, and since DRIFT-016 the two halves are separate files
/// that must move together.
#[test]
fn edited_file_is_reparsed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    incremental_fixture(root).expect("fixture tree");
    let ctx = crate::output::Context::from_flags(true, false, None, true, AgentModeArg::Auto);
    let cache_path = root.join("campaigns/progress-test/run/cache.json");

    scan(&ctx, &args(root, false)).expect("cold scan");
    let before = cache::Cache::load(&cache_path).expect("load cache");

    // One new marked item in a.md; b.md is untouched.
    let text = std::fs::read_to_string(root.join(spec_rel("a.md"))).expect("read a");
    std::fs::write(
        root.join(spec_rel("a.md")),
        format!("{text}\n##a5 A newly added claim. @freeze/done\n"),
    )
    .expect("edit a");
    scan(&ctx, &args(root, false)).expect("rescan");
    let after = cache::Cache::load(&cache_path).expect("reload cache");

    let a_before = &before.files[spec_rel("a.md").as_str()];
    let a_after = &after.files[spec_rel("a.md").as_str()];
    assert_ne!(
        a_before.content_hash, a_after.content_hash,
        "the edited file's hash moved"
    );
    assert_eq!(
        a_after.marker_count,
        a_before.marker_count + 1,
        "the new marker is in the record, so the file was re-parsed"
    );
    assert!(
        payload_for(root, &spec_rel("a.md"))
            .expect("sidecar payload for the edited file")
            .markers
            .iter()
            .any(|m| m.stage == progress_core::model::Stage::Freeze),
        "the new marker is in the sidecar payload too"
    );
    assert_eq!(
        serde_json::to_string(&before.files[spec_rel("b.md").as_str()]).expect("before b"),
        serde_json::to_string(&after.files[spec_rel("b.md").as_str()]).expect("after b"),
        "the untouched file's record did not move a byte"
    );
}

/// The campaign map is load-bearing and must not be collateral damage
/// of the reuse path (DRIFT-010 §5): a record carrying verdicts keeps
/// them across a warm run, and keeps them again across the next one.
#[test]
fn campaign_map_survives_incremental() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    incremental_fixture(root).expect("fixture tree");
    let ctx = crate::output::Context::from_flags(true, false, None, true, AgentModeArg::Auto);
    let cache_path = root.join("campaigns/progress-test/run/cache.json");

    scan(&ctx, &args(root, false)).expect("cold scan");

    // A verdict written the way the campaign writes them.
    let verdicts = serde_json::json!({"alpha": {"v": "confirmed", "ev": ["by hand"]}});
    let mut c = cache::Cache::load(&cache_path).expect("load");
    c.files
        .get_mut(&spec_rel("a.md"))
        .expect("record")
        .campaign
        .insert("verdicts".into(), verdicts.clone());
    c.store(&cache_path).expect("store");

    scan(&ctx, &args(root, false)).expect("warm scan");
    scan(&ctx, &args(root, false)).expect("second warm scan");

    let back = cache::Cache::load(&cache_path).expect("reload");
    assert_eq!(
        back.files[spec_rel("a.md").as_str()]
            .campaign
            .get("verdicts"),
        Some(&verdicts),
        "the verdict map rode through both warm runs"
    );
    // …and it reaches the projection the dashboard reads.
    let corpus = read_state(root, "corpus.json");
    assert!(corpus.contains("confirmed"), "corpus.json carries it");
}

/// `--no-cache` must actually distrust the cache. Poison one record's
/// payload — a lie that keeps the record's identity, so nothing else
/// can catch it — and watch the default run repeat the lie while the
/// `--no-cache` run reads the file and tells the truth.
#[test]
fn no_cache_flag_forces_full_parse() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    incremental_fixture(root).expect("fixture tree");
    let ctx = crate::output::Context::from_flags(true, false, None, true, AgentModeArg::Auto);

    scan(&ctx, &args(root, false)).expect("cold scan");
    let mut poisoned = payload_for(root, &spec_rel("a.md")).expect("payload to poison");
    let truthful = poisoned.markers.len();
    assert!(truthful > 1, "a.md has markers to lose");
    poisoned.markers.truncate(1);
    payloads(root).store([&poisoned]);

    // Default: the payload is trusted, so the lie shows through.
    let warm = ground(&args(root, false)).expect("warm ground");
    let warm_markers = warm.docs.iter().map(|d| d.markers.len()).sum::<usize>();

    // `--no-cache`: the file is read and parsed, and the lie is gone.
    let cold = ground(&args(root, true)).expect("no-cache ground");
    let cold_markers = cold.docs.iter().map(|d| d.markers.len()).sum::<usize>();

    assert_eq!(
        warm_markers,
        cold_markers - (truthful - 1),
        "the default run reused the poisoned payload — the cache is really consulted"
    );
    assert!(
        cold_markers > warm_markers,
        "--no-cache parsed the tree instead of believing the cache"
    );

    // And the flag still leaves the campaign's records refreshed: a
    // `--no-cache` run is a full run, not a read-only one.
    scan(&ctx, &args(root, true)).expect("no-cache scan");
    assert_eq!(
        payload_for(root, &spec_rel("a.md"))
            .expect("payload rewritten")
            .markers
            .len(),
        truthful,
        "the run rewrote the payload it refused to trust"
    );
}

// ---- DRIFT-016: the irreplaceable stays in git, the payload leaves ----

/// §5, asserted where it can actually be violated: on the bytes. A
/// verdict lives in the tracked `cache.json` and nowhere else — a verdict
/// that leaked into the sidecar would be a verdict a fresh clone loses
/// without ever knowing it had one.
#[test]
fn verdicts_never_leave_cache_json() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    incremental_fixture(root).expect("fixture tree");
    let ctx = crate::output::Context::from_flags(true, false, None, true, AgentModeArg::Auto);
    let cache_path = root.join("campaigns/progress-test/run/cache.json");

    scan(&ctx, &args(root, false)).expect("cold scan");
    // A verdict written the way the campaign writes them, then a warm run
    // over it — the run that would carry it anywhere it should not go.
    let mut c = cache::Cache::load(&cache_path).expect("load");
    c.files
        .get_mut(&spec_rel("a.md"))
        .expect("record")
        .campaign
        .insert("verdicts".into(), serde_json::json!({"alpha": "confirmed"}));
    c.store(&cache_path).expect("store");
    scan(&ctx, &args(root, false)).expect("warm scan");

    let tracked = std::fs::read_to_string(&cache_path).expect("read cache.json");
    assert!(tracked.contains("confirmed"), "the verdict is in git");

    let store = std::fs::read_to_string(sidecar_dir(root).join(sidecar::PAYLOAD_FILE))
        .expect("the sidecar was written");
    assert!(store.contains(&spec_rel("a.md")), "a store worth searching");
    assert!(!store.contains("campaign"), "no campaign key reaches it");
    assert!(!store.contains("confirmed"), "and no verdict rides along");
}
