//! DRIFT-023 — the baseline round trip, through the two subcommands the
//! §6 recurrence is made of.
//!
//! The claim under test is not "the writer writes a file". It is that
//! what the writer puts down is what the reader picks up: run
//! `progress baseline` over a campaign's verdicts, feed the result
//! straight back to `progress rescan` over the same tree, and every unit
//! carries forward — no `new`, no `changed`, no marker reported as
//! diverged. A single row of any other class means the two halves
//! disagree about a unit's address, its hash, or its marker, and the
//! artifact is measuring something other than what the reader reads.
//!
//! The controls around it are the other half of the evidence: an edit
//! must turn exactly one row suspect (a gate never seen to fire is not
//! known to work), and a second run over an unchanged campaign must
//! leave the file untouched, byte for byte.

use super::super::{baseline::baseline_cmd, rescan::rescan_cmd};
use super::*;
use progress_core::baseline::{Baseline, RescanClass, RescanOptions, RescanRow, rescan};

use crate::cli::{ProgressBaselineArgs, ProgressRescanArgs};

/// The fixture campaign with verdicts in its cache, written the way a
/// verification pass writes them: `verified_at` plus `verdicts{anchor →
/// {v, ev[]}}` in the per-file `campaign` map (PROP-043 §7.1).
///
/// The `a.md` fixture disagrees with itself on purpose — one drifting
/// item among three confirmed facts — so the unit's rolled-up verdict has
/// something to be worst-of.
fn seeded(root: &Path, ctx: &crate::output::Context) -> Result<()> {
    incremental_fixture(root)?;
    scan(ctx, &args(root, false))?;
    let cache_path = root.join("campaigns/progress-test/run/cache.json");
    let mut c = cache::Cache::load(&cache_path)?;
    let seed = |c: &mut cache::Cache, path: &str, verdicts: serde_json::Value| {
        if let Some(record) = c.files.get_mut(path) {
            record.campaign.insert(
                "verified_at".into(),
                serde_json::json!("2026-07-25T00:00:00Z"),
            );
            record.campaign.insert("verdicts".into(), verdicts);
        }
    };
    // An evidence ref into another spec document, on the live layout.
    let module_x_ev = format!("{}#a", spec_rel("modules/x.md"));
    seed(
        &mut c,
        &spec_rel("a.md"),
        serde_json::json!({
            "a1": {"v": "confirmed", "ev": ["crates/vibe-core/src/x.rs:1"]},
            "a2": {"v": "drift", "ev": [module_x_ev]},
            "a3": {"v": "confirmed", "ev": []},
            "a4": {"v": "confirmed", "ev": []},
            "_elements": {"v": "confirmed", "ev": []},
        }),
    );
    seed(
        &mut c,
        &spec_rel("b.md"),
        serde_json::json!({
            "b1": {"v": "confirmed", "ev": []},
            "b2": {"v": "confirmed", "ev": []},
            "b3": {"v": "unverifiable", "ev": []},
        }),
    );
    c.store(&cache_path)?;
    Ok(())
}

fn baseline_args(root: &Path) -> ProgressBaselineArgs {
    ProgressBaselineArgs {
        common: args(root, false),
        out: None,
    }
}

fn baseline_path(root: &Path) -> PathBuf {
    root.join("campaigns/progress-test/baseline.json")
}

/// Classify the tree as it stands right now against `base`, with the
/// control sample off — the §6 lens, in which every row's class is a
/// statement about that unit and nothing else.
fn classify(root: &Path, base: &Baseline) -> Vec<RescanRow> {
    let g = ground(&args(root, false)).unwrap_or_else(|e| panic!("ground the fixture tree: {e:#}"));
    rescan(
        g.docs.iter(),
        base,
        &RescanOptions {
            crate_states: BTreeMap::new(),
            control_rate: 0.0,
        },
    )
}

/// §6's headline control: write the baseline, read it back against the
/// unchanged tree, and every row carries forward with its marker intact.
#[test]
fn the_baseline_round_trips_to_carried_forward() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    seeded(root, &ctx).expect("a seeded fixture campaign");

    baseline_cmd(&ctx, &baseline_args(root)).expect("write the baseline");
    let base = Baseline::load(&baseline_path(root)).expect("load the baseline back");

    // The projection first: the addresses and the rolled-up verdicts.
    assert_eq!(base.campaign_id, "progress-test");
    assert_eq!(base.units.len(), 2, "one unit per fixture file");
    let alpha = base
        .units
        .get(&format!("{}#alpha", spec_rel("a.md")))
        .expect("alpha");
    assert_eq!(
        alpha.verdict, "drift",
        "one drifting item among three confirmed facts wins the unit"
    );
    assert_eq!(
        alpha.crates,
        vec!["vibe-core"],
        "derived from the evidence refs, so the named-crate rule can fire"
    );
    assert_eq!(
        alpha.marker.as_deref(),
        Some("impl/work"),
        "a.md's document marker governs its only section"
    );
    assert_eq!(
        base.units
            .get(&format!("{}#beta", spec_rel("b.md")))
            .and_then(|u| u.marker.as_deref()),
        Some("spec/plan"),
        "b.md's section marker beats the fallback"
    );

    // …and then the whole point: the reader agrees with the writer.
    let rows = classify(root, &base);
    assert_eq!(rows.len(), 2, "one row per unit in the observed tree");
    for r in &rows {
        assert_eq!(
            r.class,
            RescanClass::CarriedForward,
            "`{}` did not carry forward",
            r.addr
        );
        assert!(!r.marker_diverged, "`{}` reports a moved marker", r.addr);
        assert!(r.crate_moved.is_none());
    }

    // The subcommand itself runs the same comparison end to end — it also
    // asks git about the crates the baseline names, which a tempdir
    // cannot answer, and that must be a skipped rule rather than a
    // failed run.
    rescan_cmd(
        &ctx,
        &ProgressRescanArgs {
            common: args(root, false),
            baseline: baseline_path(root),
            control_rate: 0.0,
        },
    )
    .expect("rescan against the baseline just written");
}

/// The negative control: a gate never seen to fire is not known to work.
/// One unit's text moves; exactly that row turns suspect and its
/// neighbour does not.
#[test]
fn an_edited_unit_is_the_only_row_that_moves() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    seeded(root, &ctx).expect("a seeded fixture campaign");
    baseline_cmd(&ctx, &baseline_args(root)).expect("write the baseline");
    let base = Baseline::load(&baseline_path(root)).expect("load");

    let before = std::fs::read_to_string(root.join(spec_rel("a.md"))).expect("read a");
    std::fs::write(
        root.join(spec_rel("a.md")),
        before.replace("The first claim.", "The first claim, reworded."),
    )
    .expect("edit a");

    let rows = classify(root, &base);
    let class = |addr: &str| {
        rows.iter()
            .find(|r| r.addr == addr)
            .map(|r| r.class.clone())
            .unwrap_or_else(|| panic!("no row for {addr}"))
    };
    assert_eq!(
        class(&format!("{}#alpha", spec_rel("a.md"))),
        RescanClass::Changed
    );
    assert_eq!(
        class(&format!("{}#beta", spec_rel("b.md"))),
        RescanClass::CarriedForward
    );

    // Reverted, the row goes back to carrying forward — the suspicion was
    // about the text, not about the run.
    std::fs::write(root.join(spec_rel("a.md")), &before).expect("revert a");
    let rows = classify(root, &base);
    assert!(rows.iter().all(|r| r.class == RescanClass::CarriedForward));
}

/// The determinism control: two runs over an unchanged campaign produce
/// the same bytes, and the second one does not write them again.
///
/// Asserted on the modification time as well as the content, because the
/// content would match even if the file were rewritten — and a baseline
/// rewritten on every run is a line of `git diff` per run, which is how a
/// close-out artifact turns into noise nobody reads.
#[test]
fn a_second_run_writes_nothing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    seeded(root, &ctx).expect("a seeded fixture campaign");
    let path = baseline_path(root);

    baseline_cmd(&ctx, &baseline_args(root)).expect("first");
    let first = std::fs::read_to_string(&path).expect("read");
    let stamped = std::fs::metadata(&path).expect("meta").modified().ok();

    baseline_cmd(&ctx, &baseline_args(root)).expect("second");
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        first,
        "byte-identical, stamp included"
    );
    assert_eq!(
        std::fs::metadata(&path).expect("meta").modified().ok(),
        stamped,
        "the second run did not touch the file"
    );

    // A moved verdict, on the other hand, must land: the skip is keyed on
    // content, and the campaign's verdicts are the content.
    let cache_path = root.join("campaigns/progress-test/run/cache.json");
    let mut c = cache::Cache::load(&cache_path).expect("load cache");
    c.files
        .get_mut(&spec_rel("b.md"))
        .expect("record")
        .campaign
        .insert(
            "verdicts".into(),
            serde_json::json!({"b1": {"v": "drift", "ev": []}}),
        );
    c.store(&cache_path).expect("store cache");
    baseline_cmd(&ctx, &baseline_args(root)).expect("third");
    assert_ne!(
        std::fs::read_to_string(&path).expect("read"),
        first,
        "the moved verdict reached the baseline"
    );
}

/// §4.3's error paths, which are the two ways a baseline can lie: written
/// without a campaign zone to take verdicts from, or written from a cache
/// that failed to load. Both refuse; neither produces a file.
#[test]
fn a_baseline_is_refused_rather_than_truncated() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    seeded(root, &ctx).expect("a seeded fixture campaign");

    // No campaign zone: `--campaign` points nowhere and `campaigns/` holds
    // more than one candidate, so nothing resolves.
    let no_zone = ProgressBaselineArgs {
        common: ProgressCommonArgs {
            path: root.to_path_buf(),
            campaign: None,
            no_cache: false,
        },
        out: None,
    };
    std::fs::create_dir_all(root.join("campaigns/other")).expect("second zone");
    let err = baseline_cmd(&ctx, &no_zone).expect_err("no campaign zone");
    assert!(
        format!("{err:#}").contains("needs a campaign zone"),
        "{err:#}"
    );

    // An unreadable cache: every other subcommand degrades to a cold run,
    // this one refuses — a truncated baseline reads as knowledge.
    std::fs::write(
        root.join("campaigns/progress-test/run/cache.json"),
        [0xff, 0xfe, b'{'],
    )
    .expect("clobber the cache");
    let err = baseline_cmd(&ctx, &baseline_args(root)).expect_err("unreadable cache");
    assert!(
        format!("{err:#}").contains("refusing to write a baseline"),
        "{err:#}"
    );
    assert!(
        !baseline_path(root).exists(),
        "nothing was written on either refusal"
    );
}
