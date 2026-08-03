use super::*;
use crate::cache::now_utc;
use crate::doc::ParsedDoc;
use crate::parse::parse_document;
use specmark::spec;

/// A baseline carrying every unit of `doc` at `verified_at`, each with
/// the same evidence refs (so `crates` is derived, not hand-written).
fn baseline_of(doc: &ParsedDoc, verified_at: &str, evidence: &[&str]) -> Baseline {
    let mut b = Baseline {
        schema: BASELINE_SCHEMA,
        written_at: now_utc(),
        campaign_id: "t".into(),
        units: BTreeMap::new(),
    };
    for (i, u) in doc.units.iter().enumerate() {
        let addr = unit_addr(doc, i);
        b.units.insert(
            addr.clone(),
            BaselineUnit::new(
                addr,
                u.content_hash.clone(),
                "confirmed",
                evidence.iter().map(|e| (*e).to_string()).collect(),
                verified_at,
                None,
            ),
        );
    }
    b
}

/// Neither code knowledge nor a sample — the lens for asserting on the
/// hash rule alone.
fn plain() -> RescanOptions {
    RescanOptions {
        crate_states: BTreeMap::new(),
        control_rate: 0.0,
    }
}

#[spec(
    deviates = "discipline://rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "test-support helper in a #[cfg(test)] module file: the panic IS the assertion"
)]
fn class_of(rows: &[RescanRow], addr: &str) -> RescanClass {
    rows.iter()
        .find(|r| r.addr == addr)
        .map(|r| r.class.clone())
        .expect("row")
}

/// A doc of `n` trivially-different sections — a corpus big enough that
/// a 5 % sample is a real draw rather than "the only unit there is".
fn wide_doc(n: usize) -> ParsedDoc {
    let mut src = String::new();
    for i in 0..n {
        src.push_str(&format!("# S{i} {{#s{i}}}\n\nbody {i}\n\n"));
    }
    parse_document("a.md", &src)
}

#[test]
fn rescan_classifies_new_changed_carried() {
    let v1 = parse_document("a.md", "# One {#one}\n\nbody v1\n\n# Two {#two}\n\nbody\n");
    let baseline = baseline_of(&v1, &now_utc(), &[]);
    // v2: unit one edited, unit three added.
    let v2 = parse_document(
        "a.md",
        "# One {#one}\n\nbody v2\n\n# Two {#two}\n\nbody\n\n# Three {#three}\n\nnew\n",
    );
    let rows = rescan([&v2], &baseline, &plain());
    assert_eq!(class_of(&rows, "a.md#one"), RescanClass::Changed);
    assert_eq!(class_of(&rows, "a.md#two"), RescanClass::CarriedForward);
    assert_eq!(class_of(&rows, "a.md#three"), RescanClass::New);
}

#[test]
fn crates_derived_from_refs() {
    let doc = parse_document("a.md", "# One {#one}\n\nbody\n");
    let b = baseline_of(
        &doc,
        "2026-07-25T00:00:00Z",
        &["crates/vibe-core/src/x.rs:1", "crates/vibe-cli/src/y.rs:2"],
    );
    let unit = b.units.get("a.md#one").expect("unit");
    assert_eq!(unit.crates, vec!["vibe-cli", "vibe-core"]);
    // Refs that point outside `crates/` add nothing, and a unit with no
    // refs keeps an empty list — the rule just does not apply to it.
    assert!(crates_from_refs(&["spec/modules/x.md#a".to_string()]).is_empty());
    assert!(crates_from_refs(&[]).is_empty());
}

#[test]
fn named_crate_commit_makes_suspect() {
    let doc = parse_document("a.md", "# One {#one}\n\nbody\n");
    let b = baseline_of(
        &doc,
        "2026-07-25T00:00:00Z",
        &["crates/vibe-core/src/x.rs:1"],
    );
    let with = |state: CrateState| RescanOptions {
        crate_states: [("vibe-core".to_string(), state)].into_iter().collect(),
        control_rate: 0.0,
    };

    // T+1: the crate moved after the verdict ⇒ suspect, and the row
    // names which crate did it.
    let rows = rescan(
        [&doc],
        &b,
        &with(CrateState::LastCommit("2026-07-26T00:00:00Z".into())),
    );
    assert_eq!(class_of(&rows, "a.md#one"), RescanClass::Changed);
    assert_eq!(rows[0].crate_moved.as_deref(), Some("vibe-core"));

    // T−1: the crate's last commit predates the verdict ⇒ carry on.
    let rows = rescan(
        [&doc],
        &b,
        &with(CrateState::LastCommit("2026-07-24T00:00:00Z".into())),
    );
    assert_eq!(class_of(&rows, "a.md#one"), RescanClass::CarriedForward);
    assert!(rows[0].crate_moved.is_none());

    // Instants, not strings: `02:00+03:00` sorts *after* `00:00Z`
    // lexically but happens an hour *before* it.
    let rows = rescan(
        [&doc],
        &b,
        &with(CrateState::LastCommit("2026-07-25T02:00:00+03:00".into())),
    );
    assert_eq!(class_of(&rows, "a.md#one"), RescanClass::CarriedForward);

    // A crate that left the tree is the strongest possible move.
    let rows = rescan([&doc], &b, &with(CrateState::Gone));
    assert_eq!(class_of(&rows, "a.md#one"), RescanClass::Changed);
}

#[test]
fn control_sample_is_deterministic() {
    let doc = wide_doc(20);
    let b = baseline_of(&doc, "2026-07-25T00:00:00Z", &[]);
    let sampled = |rate: f64| -> Vec<String> {
        let opts = RescanOptions {
            crate_states: BTreeMap::new(),
            control_rate: rate,
        };
        rescan([&doc], &b, &opts)
            .into_iter()
            .filter(|r| r.class == RescanClass::ControlSample)
            .map(|r| r.addr)
            .collect()
    };

    // Two runs over the same baseline pick the same units.
    let first = sampled(DEFAULT_CONTROL_RATE);
    assert_eq!(first.len(), 1, "5 % of 20 units, rounded up");
    assert_eq!(first, sampled(DEFAULT_CONTROL_RATE));

    // Rate 0 disables the sample outright.
    assert!(sampled(0.0).is_empty());

    // One carried-forward unit at 5 % still gets drawn — rounding up is
    // what keeps a small corpus sampled at all.
    let one = parse_document("a.md", "# One {#one}\n\nbody\n");
    let b1 = baseline_of(&one, "2026-07-25T00:00:00Z", &[]);
    let rows = rescan(
        [&one],
        &b1,
        &RescanOptions {
            crate_states: BTreeMap::new(),
            control_rate: DEFAULT_CONTROL_RATE,
        },
    );
    assert_eq!(class_of(&rows, "a.md#one"), RescanClass::ControlSample);
}

#[test]
fn missing_git_skips_rule_without_failing() {
    let doc = wide_doc(3);
    let b = baseline_of(
        &doc,
        "2026-07-25T00:00:00Z",
        &["crates/vibe-core/src/x.rs:1"],
    );
    // The adapter could not ask anything about any crate — an empty map.
    // Everything the hash carried stays carried; nothing errors.
    let rows = rescan([&doc], &b, &plain());
    assert_eq!(rows.len(), 3);
    assert!(
        rows.iter().all(|r| r.class == RescanClass::CarriedForward),
        "no crate knowledge ⇒ the named-crate rule is simply skipped"
    );
    assert!(rows.iter().all(|r| r.crate_moved.is_none()));
}
