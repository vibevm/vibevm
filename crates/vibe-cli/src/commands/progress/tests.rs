//! Tests for the `vibe progress` adapter (PROP-043 §5).
//!
//! File-backed submodule of [`super`] — split out so the cell stays under
//! the 600-line AI-Native file budget. Every test builds its own fixture
//! tree and campaign zone in a tempdir, so no run here can reach the live
//! campaign's cache or state projections.
//!
//! The same rule binds the payload sidecar, which by default lives under
//! the developer's real per-user home: every fixture `progress.toml` here
//! sets `[progress] cache_dir` inside its own tempdir, which is the escape
//! hatch DRIFT-016 §4.2 exists for. A test that writes a real user
//! location is the defect DRIFT-012 spent a day on. The *default*
//! location — the one a real run takes — is exercised out-of-process in
//! `tests/cli_progress_sidecar.rs`, under a relocated `VIBE_SETTINGS`.

use super::*;

/// DRIFT-017's own cell — whether a write happens at all, asserted on
/// mtimes. Split out so this file stays inside the 600-line budget, and
/// because its subject is different: everything here is about what a run
/// *answers*, everything there about what it *touches*.
mod writes;

/// DRIFT-023's own cell — the baseline round trip through the two
/// subcommands that make the §6 recurrence a loop: write it, read it
/// back, and watch a single edited unit turn suspect.
mod baseline;

/// Where every fixture in this file pins its payload sidecar, relative to
/// the fixture root.
const FIXTURE_CACHE_DIR: &str = "payload-cache";

/// The `progress.toml` body a fixture writes: an include list plus the
/// pinned sidecar. Kept in one place so no fixture can forget the pin.
fn fixture_config(include: &str) -> String {
    format!("include = [\"{include}\"]\n\n[progress]\ncache_dir = \"{FIXTURE_CACHE_DIR}\"\n")
}

/// The fixture specs root as a forward-slashed string — the prefix every
/// fixture path, glob and cache key in this family builds on. Routed
/// through the layout module (PROP-052 L2) so the R4 flip carries the
/// whole family without an edit.
fn specs_prefix() -> String {
    vibe_core::machine_json_path(&vibe_core::layout::current_specs_root())
}

/// One fixture file's project-relative path under the specs root.
fn spec_rel(name: &str) -> String {
    format!("{}/{}", specs_prefix(), name)
}

/// The fixture campaign's payload bucket — `<cache_dir>/<campaign id>`,
/// the leaf `sidecar::resolve_dir` builds.
fn sidecar_dir(root: &Path) -> PathBuf {
    root.join(FIXTURE_CACHE_DIR).join("progress-test")
}

/// The sidecar as it stands on disk right now.
fn payloads(root: &Path) -> sidecar::Payloads {
    sidecar::Payloads::load(Some(sidecar_dir(root)))
}

/// The payload a finished run left for `rel`, by re-reading the file and
/// asking for exactly those bytes.
///
/// `None` covers both "the file is unreadable" and "the store has nothing
/// for these bytes"; the `#[test]` that called it decides whether that is
/// a failure, which is the helper rule's actual point (DRIFT-010 §9).
fn payload_for(root: &Path, rel: &str) -> Option<ParsedDoc> {
    let text = std::fs::read_to_string(root.join(rel)).ok()?;
    payloads(root)
        .get(rel, &progress_core::parse::content_hash(&text))
        .cloned()
}

/// A fixture campaign zone whose journal carries a hand-appended `phase`
/// event: `refresh_state` must derive that phase into `campaign.json`
/// instead of the compiled-in opening phase (DRIFT-003 §4).
#[test]
fn refresh_state_derives_phase_from_journal() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let campaign = tmp.path().join("campaigns").join("progress-test");
    let run = campaign.join("run");
    std::fs::create_dir_all(&run).expect("mkdir run");
    // The exact on-disk event the campaign executor appends by hand.
    std::fs::write(
        run.join("journal.jsonl"),
        "{\"kind\":\"phase\",\"value\":\"B\",\"ts\":\"2026-07-24T00:00:00Z\"}\n",
    )
    .expect("write journal fixture");

    let mut g = Ground {
        root: tmp.path().to_path_buf(),
        docs: Vec::new(),
        campaign: Some(campaign.clone()),
        cache: cache::Cache::load_tolerant(&run.join("cache.json")).0,
        cache_warning: None,
        payloads: sidecar::Payloads::load(Some(tmp.path().join(FIXTURE_CACHE_DIR))),
        // This fixture builds its corpus by hand rather than enumerating
        // one, so no `exclude` glob ever ran over it.
        excluded: 0,
        // …and no file of it entered as an XML source.
        xml_sources: BTreeSet::new(),
    };
    refresh_state(&mut g).expect("refresh_state");

    let text = std::fs::read_to_string(run.join("state").join("campaign.json"))
        .expect("read campaign.json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("parse campaign.json");
    assert_eq!(
        v["phase"], "B",
        "campaign.json carries the journal-derived phase"
    );
}

/// The scope-narrowing prune (DRIFT-001 §4): scan a two-file tree, then
/// narrow `progress.toml` to a single file and rescan. `corpus.json`
/// must carry exactly the observed set — not the union across scans, the
/// stale-row defect §3 records.
#[test]
fn refresh_state_prunes_records_that_leave_scope() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(vibe_core::layout::current_specs_root()))
        .expect("mkdir spec");
    std::fs::write(root.join(spec_rel("a.md")), "@impl a\n").expect("write a");
    std::fs::write(root.join(spec_rel("b.md")), "@impl b\n").expect("write b");
    let campaign = root.join("campaigns").join("progress-test");
    std::fs::create_dir_all(campaign.join("run")).expect("mkdir run");

    let common = ProgressCommonArgs {
        path: root.to_path_buf(),
        campaign: Some(campaign.clone()),
        no_cache: false,
    };

    // Wide scope: both files observed and cached.
    std::fs::write(
        root.join("progress.toml"),
        fixture_config(&format!("{}/**/*.md", specs_prefix())),
    )
    .expect("write wide cfg");
    let mut g = ground(&common).expect("ground wide");
    assert_eq!(g.docs.len(), 2, "both files in scope");
    refresh_state(&mut g).expect("refresh wide");

    // Narrow scope: only a.md observed.
    std::fs::write(
        root.join("progress.toml"),
        fixture_config(&spec_rel("a.md")),
    )
    .expect("write narrow cfg");
    let mut g = ground(&common).expect("ground narrow");
    assert_eq!(g.docs.len(), 1, "only a.md in scope");
    refresh_state(&mut g).expect("refresh narrow");

    // corpus.json rows equal the observed set — the b.md row is gone.
    let corpus = campaign.join("run").join("state").join("corpus.json");
    let text = std::fs::read_to_string(&corpus).expect("read corpus.json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("parse corpus.json");
    let paths: Vec<&str> = v["files"]
        .as_array()
        .expect("files array")
        .iter()
        .map(|f| f["path"].as_str().expect("path str"))
        .collect();
    assert_eq!(paths, vec![spec_rel("a.md")], "corpus.json == observed set");
}

/// The automation seam end to end (DRIFT-008 §4.4): the subcommand
/// writes the record into `campaign.json`, and the scan that follows —
/// which rewrites the whole projection — keeps it.
#[test]
fn progress_gate_cli_records() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(vibe_core::layout::current_specs_root()))
        .expect("mkdir spec");
    std::fs::write(root.join(spec_rel("a.md")), "@impl a\n").expect("write a");
    std::fs::write(
        root.join("progress.toml"),
        fixture_config(&format!("{}/**/*.md", specs_prefix())),
    )
    .expect("write cfg");
    let campaign = root.join("campaigns").join("progress-test");
    std::fs::create_dir_all(campaign.join("run")).expect("mkdir run");
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    let common = || ProgressCommonArgs {
        path: root.to_path_buf(),
        campaign: Some(campaign.clone()),
        no_cache: false,
    };

    // The panel lives in campaign.json, which a scan writes first.
    scan(&ctx, &common()).expect("scan");
    gate(
        &ctx,
        &ProgressGateArgs {
            common: common(),
            name: "floor".into(),
            status: GateStatusArg::Red,
            detail: Some("cli_pkg_cycle::install_from_git_registry (F-055)".into()),
        },
    )
    .expect("gate");
    // … and a following scan does not erase it.
    scan(&ctx, &common()).expect("rescan");

    let text = std::fs::read_to_string(campaign.join("run/state/campaign.json"))
        .expect("read campaign.json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("parse campaign.json");
    let gates = v["gates"].as_array().expect("gates array");
    assert_eq!(gates.len(), 1, "one gate recorded");
    assert_eq!(gates[0]["name"], "floor");
    assert_eq!(gates[0]["status"], "red");
    assert_eq!(
        gates[0]["detail"],
        "cli_pkg_cycle::install_from_git_registry (F-055)"
    );
    assert!(gates[0]["ran_at"].is_string(), "stamped with a UTC time");
}

// ---- DRIFT-010: the subcommands take the incremental path ----------

/// A fixture tree with enough shape that a lossy reuse would show:
/// document and section markers, an anchored paragraph, list items, a
/// table with marked cells, a fenced block that must stay unscanned,
/// and a wrapper fragment.
///
/// Propagates instead of panicking — the decision to fail the run belongs
/// to the `#[test]` that called it, not to a helper.
fn incremental_fixture(root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(root.join(vibe_core::layout::current_specs_root()))?;
    std::fs::write(
        root.join(spec_rel("a.md")),
        "<status stage=\"impl\" state=\"work\"/>\n\n\
         # Alpha {#alpha}\n\n\
         ##a1 The first claim. @test/plan\n\n\
         - ##a2 An item. @doc/done\n\
         - ##a3 Another item. @impl/hold\n\n\
         ```\n@spec/done inside a fence is not a marker\n```\n\n\
         ##a4 A tail with <status stage=\"spec\" state=\"done\" \
         action=\"drift\">a fragment</status> in it.\n",
    )?;
    std::fs::write(
        root.join(spec_rel("b.md")),
        "# Beta {#beta}\n\n\
         <status stage=\"spec\" state=\"plan\"/>\n\n\
         ##b1 A paragraph under the section marker. @spec/work\n\n\
         | h1 | h2 |\n\
         | --- | --- |\n\
         | ##b2 cell one @impl/done | ##b3 cell two @impl/work |\n",
    )?;
    std::fs::write(
        root.join("progress.toml"),
        fixture_config(&format!("{}/**/*.md", specs_prefix())),
    )?;
    std::fs::create_dir_all(root.join("campaigns/progress-test/run"))
}

fn args(root: &Path, no_cache: bool) -> ProgressCommonArgs {
    ProgressCommonArgs {
        path: root.to_path_buf(),
        campaign: Some(root.join("campaigns/progress-test")),
        no_cache,
    }
}

fn report_args(root: &Path, no_cache: bool, md: bool) -> ProgressReportArgs {
    ProgressReportArgs {
        common: args(root, no_cache),
        md,
        view: None,
        audience: None,
    }
}

/// `updated_at` is a wall clock, not content: it differs between any
/// two runs that straddle a second and would make the equality below a
/// coin flip. Everything else in these files is compared byte for byte.
fn blank_stamps(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    for (i, part) in json.split("\"updated_at\": \"").enumerate() {
        if i > 0 {
            out.push_str("\"updated_at\": \"<stamp>");
            match part.find('"') {
                Some(end) => out.push_str(&part[end..]),
                None => out.push_str(part),
            }
        } else {
            out.push_str(part);
        }
    }
    out
}

fn read_state(root: &Path, name: &str) -> String {
    blank_stamps(
        &std::fs::read_to_string(root.join("campaigns/progress-test/run/state").join(name))
            .unwrap_or_else(|e| panic!("read {name}: {e}")),
    )
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
    let ctx = crate::output::Context::from_flags(true, false, None, true);

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
    let ctx = crate::output::Context::from_flags(true, false, None, true);
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
    let ctx = crate::output::Context::from_flags(true, false, None, true);
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
    let ctx = crate::output::Context::from_flags(true, false, None, true);
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
    let ctx = crate::output::Context::from_flags(true, false, None, true);

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
    let ctx = crate::output::Context::from_flags(true, false, None, true);
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

// ---- G-B010: `check` is read-only unless `--write-state` opts the write in ----

/// The `--write-state` flag exists on `check` and is off by default. The
/// default *is* the fix, so the default is the thing to pin (G-B010).
/// Parsed through the real `Cli` so the test exercises the exact surface a
/// user hits, global flags and all.
mod xml_sources;
