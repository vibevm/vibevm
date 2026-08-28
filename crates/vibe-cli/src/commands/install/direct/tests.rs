//! The `vibe install` boundary's reds: the preparation ORDER it owes, and
//! the double hop a neutral resume failure makes on its way to this
//! command's own registered root.
//!
//! Its own cell because the boundary source is at the file budget: the reds
//! here drive real carriers and a real funnel, and they grow with the laws
//! rather than with the code.

use super::*;
use crate::commands::compile_trace::{CommandExit, classify};
use crate::commands::install::{MeasuredFailure, Measurement};

/// A fully-defaulted `InstallArgs` for the preparation-boundary red —
/// every flag off, exactly one field (the path) set by the caller.
fn args(path: std::path::PathBuf) -> InstallArgs {
    InstallArgs {
        packages: Vec::new(),
        path,
        registry: None,
        assume_yes: true,
        language: None,
        features: Vec::new(),
        no_default_features: false,
        all_features: false,
        exact: false,
        auth_required: false,
        solver: None,
        git: None,
        tag: None,
        branch: None,
        rev: None,
        git_auth: None,
        git_token_env: None,
        force: false,
        prefer_embedded: false,
        no_prefer_embedded: false,
        no_default_registry: false,
        offline: true,
        embedded_short_circuit: false,
        prefer_local: false,
        no_prefer_local: false,
        trace_compile: false,
    }
}

fn quiet_ctx() -> output::Context {
    output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Cli)
}

/// The pre-lease-snapshot barrier, pinned for real (R7.4 §2.1): the
/// safefs `before_lock` race hook fires between `Project::open` and the
/// OS lock — exactly the window in which a concurrent editor rewrites
/// the selected manifest — and rewrites a valid manifest into a
/// SEMANTICALLY DIFFERENT one (a standing `[compile] trace` request).
///
/// The correct order consumes the POST-hook file: the prepared identity
/// carries the post-hook activation. An order that read the manifest
/// before acquiring would freeze the pre-hook bytes and this test fails
/// — which is the discrimination the planted change buys: the file was
/// valid before AND after, so only the ORDER decides which semantics
/// the command runs with.
#[test]
fn the_selected_manifest_snapshot_is_taken_after_the_lease_acquisition() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]
name = \"demo\"
version = \"0.1.0\"
",
    )
    .unwrap();
    let manifest_path = dir.path().join("vibe.toml");
    let rewritten = manifest_path.clone();
    vibe_safefs::arm_before_lock(Some(Box::new(move |_, name| {
        if name == "lifecycle.lock" {
            std::fs::write(
                &rewritten,
                "[project]
name = \"demo\"
version = \"0.1.0\"

[compile]
trace = true
",
            )
            .unwrap();
        }
    })));
    // The hook is one-shot: the lifecycle acquisition consumes it, and
    // the compile-trace lock `prepare_trace` may take below finds
    // nothing armed.
    let prepared = prepare_direct_install(&quiet_ctx(), &args(dir.path().to_path_buf()), true)
        .expect("the preparation completes against the post-acquisition tree");
    assert!(
        prepared.metadata.trace_compile,
        "the identity carries the POST-hook activation — the manifest snapshot was \
         taken after the lease, not before it",
    );
    assert!(
        std::fs::read_to_string(&manifest_path)
            .unwrap()
            .contains("trace = true"),
        "the hook really fired inside the acquire window",
    );
}

#[derive(Debug, thiserror::Error)]
#[error("the resumed row refused")]
struct Sentinel;

fn row(point: &str, status: &str) -> vibe_install::SlotLifecycleReport {
    vibe_install::SlotLifecycleReport {
        key: format!("org.demo/tools#{point}"),
        point: point.into(),
        handler: "builtin".into(),
        provider: "org.demo/tools".into(),
        tier: "dependency".into(),
        status: status.into(),
        message: None,
        version: None,
        reference: "spec://org.demo/tools".into(),
        flagged: false,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
        slot_target: None,
    }
}

/// The exact rendering the substrate's error already had, so the double hop
/// can be checked against a value and not against "it still contains the
/// word".
const CHAIN: &str = "finishing the parked slot run: the resumed row refused";

/// The neutral transport the substrate really builds, with the site's
/// emission bit as a parameter — the bit is frozen at the measuring site and
/// has to survive both hops unchanged, in either position.
fn transported_with(emit_machine_failure: bool) -> anyhow::Error {
    vibe_orchestrator::failure::carry(MeasuredFailure {
        original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
        emit_machine_failure,
        evidence: Measurement::Slot {
            progress: Box::new(vibe_install::InstallProgress {
                complete: true,
                fresh: false,
                materialised: vec!["vibedeps/org.demo.tools/0.1.0".into()],
                skipped: Vec::new(),
                pruned: Vec::new(),
                nodes_regenerated: vec![".".into()],
            }),
            reports: vec![
                row("slot:pre-install", "ok"),
                row("slot:post-install", "fail"),
            ],
            packages_resolved: 4,
        },
    })
}

/// The WHOLE double hop, on the real path: the substrate's neutral
/// `Carried<Measurement>` is taken, mapped to this command's registered
/// family, re-carried as `Carried<RegisteredReportDraft>`, classified and
/// finalised — and at the far end the operator gets the same error OBJECT,
/// rendered exactly as it always was, with the emission bit the measuring
/// site froze and no carrier of either shape left around it.
///
/// Every step is asserted rather than assumed, because each is separately
/// wrong in a way the others hide:
///
/// * the transport really IS `Carried<Measurement>` going in — otherwise
///   `absorb_resume_failure` is a no-op and the rest proves nothing;
/// * after the hop it is `Carried<RegisteredReportDraft>` and NO LONGER
///   `Carried<Measurement>`: one carrier is REPLACED, never nested inside
///   another, which is what keeps the outer `classify` total;
/// * reducing the transport to `failure.original` — what this site used to
///   do — makes the fallback report `InstallProgress::default()` and zero
///   rows over a run that had already finished somebody's parked slot work;
/// * the error is MOVED, not formatted: the `Sentinel` downcast the exit
///   code is read from survives, and `{:#}` is byte-identical to what the
///   substrate handed over;
/// * neither carrier escapes to `main` — not as the returned object, and
///   not hiding as a link in its source chain.
///
/// Run for BOTH emission bits, because "frozen at the measuring site" is a
/// claim about transport, not about the value `false`.
#[test]
fn a_neutral_resume_failure_becomes_a_measured_install_root() {
    for frozen in [false, true] {
        let root = std::path::PathBuf::from("/p");

        // Hop 0 — what the substrate really hands this surface.
        let transport = transported_with(frozen);
        assert!(
            is_measurement_carrier(&transport),
            "the premise: the substrate transports a NEUTRAL measurement",
        );
        assert!(
            !is_draft_carrier(&transport),
            "and names no registered family — it cannot know one",
        );
        assert_eq!(
            format!("{transport:#}"),
            CHAIN,
            "the carrier is transparent"
        );

        // Hop 1 — the REAL surface mapping under test.
        let absorbed = absorb_resume_failure(transport, &root);
        assert!(
            is_draft_carrier(&absorbed),
            "the family is chosen HERE and travels as this surface's carrier",
        );
        assert!(
            !is_measurement_carrier(&absorbed),
            "the neutral carrier is REPLACED, never nested inside the draft one",
        );
        assert_eq!(
            format!("{absorbed:#}"),
            CHAIN,
            "and the second carrier is transparent too",
        );

        // Hop 2 — the funnel's classifier takes this surface's carrier apart.
        let CommandExit::Failed {
            report,
            original_error,
            emit_when_trace_disabled,
        } = classify(absorbed, || panic!("the carrier decides, not the fallback"))
        else {
            panic!("a failure is a failure");
        };
        assert_eq!(
            emit_when_trace_disabled, frozen,
            "the bit the MEASURING site froze crossed both hops unchanged",
        );

        let RegisteredReportDraft::Install(draft) = report else {
            panic!("this command's own family");
        };
        let built = draft.into_report(None);
        assert!(!built.ok);
        assert_eq!(built.materialised, ["vibedeps/org.demo.tools/0.1.0"]);
        let statuses: Vec<&str> = built
            .contributions
            .iter()
            .map(|row| row.status.as_str())
            .collect();
        assert_eq!(statuses, ["ok", "fail"], "both rows, in order");

        // Hop 3 — the shared funnel, exactly as the command returns.
        let finalized = vibe_orchestrator::trace::finalize(
            disabled_owner(),
            CommandExit::Failed {
                report: RegisteredReportDraft::Install(Box::new(InstallDraft::failed(
                    &root,
                    vibe_install::InstallProgress::default(),
                    Vec::new(),
                ))),
                original_error,
                emit_when_trace_disabled,
            },
            &fixed_instant,
        );
        assert_eq!(
            finalized.emit_report, frozen,
            "and the funnel's emission decision is that same frozen bit",
        );
        let returned = finalized.original_error.expect("the error comes back");
        assert!(
            returned.downcast_ref::<Sentinel>().is_some(),
            "the ORIGINAL object — the type `as_exit_code` downcasts through",
        );
        assert_eq!(
            format!("{returned:#}"),
            CHAIN,
            "rendered exactly as the substrate handed it over: never re-wrapped, \
             never rebuilt from its own Display",
        );
        assert_eq!(returned.chain().count(), 2, "the context chain is intact");
        assert!(
            !is_measurement_carrier(&returned) && !is_draft_carrier(&returned),
            "neither carrier escapes to `main`",
        );
        assert!(
            returned
                .chain()
                .all(|link| !format!("{link:?}").contains("Carried")),
            "and none is hiding as a source link either",
        );
    }
}

fn is_measurement_carrier(error: &anyhow::Error) -> bool {
    vibe_orchestrator::failure::is_carried::<Measurement>(error)
}

fn is_draft_carrier(error: &anyhow::Error) -> bool {
    vibe_orchestrator::failure::is_carried::<RegisteredReportDraft>(error)
}

fn fixed_instant() -> vibe_wire::generated::shared::Timestamp {
    vibe_wire::generated::shared::Timestamp::from_timestamp(0, 0)
        .expect("a fixture instant is representable")
}

/// A trace owner for a run that asked for nothing: no filesystem, no lock,
/// no tree — so the assertions above are about the CARRIER and the error,
/// with the recorder deliberately out of the way. Built through the epoch's
/// own `prepare_trace`, so this red also exercises the one join.
fn disabled_owner() -> vibe_orchestrator::trace::TracePreparation {
    vibe_orchestrator::RunPrelude {
        identity: vibe_lifecycle::RunIdentity {
            run_id: "0".repeat(32),
            started: "2026-08-28T10:00:00Z".to_string(),
            adopted: false,
            compile_trace: false,
            superseded_trace: None,
        },
        selection: crate::commands::install::SelectedManifest::read(std::path::Path::new(
            "/definitely/absent",
        ))
        .prepare(),
        lease: vibe_test_support::retained_lifecycle_lease(),
        selected: ".".to_string(),
    }
    .prepare_trace(&fixed_instant)
}

/// Anything else is left exactly as it arrived, so the carried-draft
/// classifier still sees its own carriers.
#[test]
fn an_ordinary_error_passes_through_untouched() {
    let error = absorb_resume_failure(
        anyhow::anyhow!("planning blew up"),
        std::path::Path::new("/p"),
    );
    assert_eq!(error.to_string(), "planning blew up");
}
