//! The neutral carrier's laws, driven directly.

use super::*;

#[derive(Debug, thiserror::Error)]
#[error("the handler refused")]
struct Sentinel;

fn row(key: &str, status: &str) -> LifecycleContributionReport {
    LifecycleContributionReport {
        flagged: None,
        handler: "script".into(),
        key: key.into(),
        message: None,
        stderr: None,
        stderr_truncated: None,
        stdout: None,
        stdout_truncated: None,
        phase: "build".into(),
        point: "phase:build".into(),
        provider: "org.demo/x".into(),
        reference: None,
        slot_target: None,
        status: status.into(),
        tier: "dependency".into(),
        version: None,
    }
}

fn lifecycle(rows: Vec<LifecycleContributionReport>, stopped: &str) -> Measurement {
    Measurement::Lifecycle {
        rows,
        stopped_phase: stopped.into(),
        requested: "build".into(),
        chain: vec!["validate".into(), "install".into(), "build".into()],
    }
}

/// The property the carrier exists for: the object comes back out, with its
/// concrete type and its whole context chain intact.
#[test]
fn a_carried_failure_returns_its_exact_error_and_policy() {
    let original = anyhow::Error::new(Sentinel).context("phase `build` stopped");
    let rendered = format!("{original:#}");
    let taken = take(carry(MeasuredFailure {
        original,
        measurement: lifecycle(vec![row("@vibe/exact", "fail")], "build"),
        emit_machine_failure: true,
    }))
    .expect("a carried failure is takeable");
    assert!(taken.emit_machine_failure, "the site's own policy survives");
    assert_eq!(format!("{:#}", taken.original), rendered);
    assert!(taken.original.downcast_ref::<Sentinel>().is_some());
}

/// An uncarried error is returned exactly, so a boundary can branch without
/// consuming what it may have to pass on.
#[test]
fn an_uncarried_error_is_returned_untouched() {
    let error = take(anyhow::anyhow!("planning blew up")).expect_err("not carried");
    assert_eq!(error.to_string(), "planning blew up");
    assert!(!is_measured(&error));
}

/// A GENERIC post-row failure keeps every row measured before it, and stays
/// silent — the historical policy of a stage that never emitted a document.
#[test]
fn a_generic_post_row_error_carries_the_rows_measured_before_it() {
    let measured = vec![row("@vibe/a", "ok"), row("@vibe/b", "cancelled")];
    let carried = carry_measured(
        anyhow::Error::new(Sentinel).context("writing the execution checkpoint"),
        || lifecycle(measured.clone(), "build"),
    );
    let taken = take(carried).expect("carried");
    let Measurement::Lifecycle { rows, .. } = &taken.measurement else {
        panic!("this stage measures lifecycle rows");
    };
    assert_eq!(
        rows.len(),
        2,
        "both earlier rows survived the later failure"
    );
    assert_eq!(rows[1].status, "cancelled");
    assert!(
        !taken.emit_machine_failure,
        "a generic stage failure was historically silent",
    );
    assert_eq!(
        format!("{:#}", taken.original),
        "writing the execution checkpoint: the handler refused",
        "context is neither stripped nor re-added",
    );
}

/// A site that already froze its own, more specific measurement keeps it —
/// including its emission bit.
#[test]
fn carrying_never_overwrites_a_measurement_its_own_site_froze() {
    let specific = carry(MeasuredFailure {
        original: anyhow::Error::new(Sentinel),
        measurement: lifecycle(vec![row("@vibe/exact", "fail")], "build"),
        emit_machine_failure: true,
    });
    let taken = take(carry_measured(specific, || lifecycle(Vec::new(), "build"))).expect("carried");
    let Measurement::Lifecycle { rows, .. } = &taken.measurement else {
        panic!("family");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key, "@vibe/exact");
    assert!(
        taken.emit_machine_failure,
        "and its own emission policy, not the generic one",
    );
}

/// The prefix reaches a LIFECYCLE measurement, in chronological order.
#[test]
fn prepend_puts_earlier_rows_in_front_of_a_lifecycle_measurement() {
    let carried = carry(MeasuredFailure {
        original: anyhow::Error::new(Sentinel),
        measurement: lifecycle(vec![row("@vibe/own", "fail")], "build"),
        emit_machine_failure: true,
    });
    let taken = take(prepend_rows(carried, vec![row("@vibe/earlier", "ok")])).expect("carried");
    let Measurement::Lifecycle { rows, .. } = &taken.measurement else {
        panic!("family");
    };
    assert_eq!(
        rows.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(),
        vec!["@vibe/earlier", "@vibe/own"],
    );
    assert!(taken.emit_machine_failure, "the site's policy is untouched");
}

/// …and never reaches a SLOT measurement, which belongs to a different report
/// family and holds no lifecycle rows at all.
#[test]
fn prepend_leaves_a_slot_measurement_untouched() {
    let carried = carry(MeasuredFailure {
        original: anyhow::Error::new(Sentinel),
        measurement: Measurement::Slot {
            progress: Box::default(),
            reports: Vec::new(),
            packages_resolved: 3,
        },
        emit_machine_failure: false,
    });
    let taken = take(prepend_rows(carried, vec![row("@vibe/earlier", "ok")])).expect("carried");
    let Measurement::Slot {
        reports,
        packages_resolved,
        ..
    } = &taken.measurement
    else {
        panic!("a slot measurement stays a slot measurement");
    };
    assert!(reports.is_empty(), "no lifecycle row leaked into it");
    assert_eq!(*packages_resolved, 3, "every measured field crossed");
}

/// The two slot shapes are NOT interchangeable, and this is the fact that
/// makes them separate variants.
///
/// `InstallBarrier` is the substrate's OWN apply failure: the measuring site
/// already knows it is install-shaped, and every surface reports it in ITS
/// install root — including a phase verb, whose prerequisite install has
/// always emitted a `cli-install-report` with no lifecycle root beside it.
/// `Slot` is the resume transport, whose family only the outer command knows.
///
/// Collapsing them into one variant is the mutation this kills: a phase verb
/// would then absorb a barrier failure into its lifecycle rows and emit the
/// wrong registered root.
#[test]
fn the_frozen_install_barrier_is_a_different_shape_from_the_neutral_resume() {
    let barrier = Measurement::InstallBarrier {
        progress: Box::default(),
        reports: Vec::new(),
        packages_resolved: 2,
    };
    let resume = Measurement::Slot {
        progress: Box::default(),
        reports: Vec::new(),
        packages_resolved: 2,
    };
    assert!(!matches!(barrier, Measurement::Slot { .. }));
    assert!(!matches!(resume, Measurement::InstallBarrier { .. }));

    // Neither is a lifecycle measurement, so neither ever receives a prefix.
    for measurement in [barrier, resume] {
        let carried = carry(MeasuredFailure {
            original: anyhow::Error::new(Sentinel),
            measurement,
            emit_machine_failure: false,
        });
        let after =
            take(prepend_rows(carried, vec![row("@vibe/earlier", "ok")])).expect("still carried");
        match after.measurement {
            Measurement::Slot { reports, .. } | Measurement::InstallBarrier { reports, .. } => {
                assert!(
                    reports.is_empty(),
                    "no lifecycle row leaked into a slot shape"
                );
            }
            Measurement::Lifecycle { .. } => panic!("a slot shape stays a slot shape"),
        }
    }
}

/// The emission bit is a property of the SITE, and it crosses the carrier
/// unchanged in both directions.
///
/// The apply's barrier failure is a machine document under a direct
/// `vibe install --json` (its CHILD observer says so) and silent under a phase
/// verb's suppressed child; the carrier must transport whichever answer that
/// observer gave rather than recomputing one.
#[test]
fn the_emission_bit_crosses_the_carrier_in_both_directions() {
    for expected in [true, false] {
        let carried = carry(MeasuredFailure {
            original: anyhow::Error::new(Sentinel),
            measurement: Measurement::InstallBarrier {
                progress: Box::default(),
                reports: Vec::new(),
                packages_resolved: 0,
            },
            emit_machine_failure: expected,
        });
        assert_eq!(
            take(carried).expect("carried").emit_machine_failure,
            expected,
            "the site's own answer, never a recomputed one",
        );
    }
}
