//! The three laws this surface still owns about the shared funnel.
//!
//! The funnel's own state machine, bounds and member laws moved down with it
//! and are driven directly in `vibe-orchestrator`
//! (`src/trace/tests/{session,bounds,funnel}.rs`). What CANNOT move is the
//! claim that binds them to this binary: that the shared funnel really is the
//! one this command tree calls, that the exit-code-bearing error type only this
//! crate can name survives it untouched, and that no second copy of any of it
//! grew back here.

use vibe_orchestrator::trace::{CommandExit, finalize, without_workspace};
use vibe_wire::generated::shared::Timestamp;

use super::{RegisteredReportDraft, carry, classify};

/// A workspaceless owner: no filesystem, no lock, no tree — the one
/// `TracePreparation` a unit test can build without a temporary root.
fn preparation(compile_trace: bool) -> vibe_orchestrator::trace::TracePreparation {
    without_workspace(&vibe_lifecycle::RunIdentity {
        run_id: "0".repeat(32),
        started: "2026-08-28T10:00:00Z".to_string(),
        adopted: false,
        compile_trace,
        superseded_trace: None,
    })
}

fn install_draft() -> RegisteredReportDraft {
    RegisteredReportDraft::Install(Box::new(crate::commands::install::InstallDraft::failed(
        std::path::Path::new("/p"),
        vibe_install::InstallProgress::default(),
        Vec::new(),
    )))
}

/// The exit code this binary returns is downcast out of the error object the
/// command handed to the funnel. `vibe-orchestrator` cannot name
/// [`crate::exit_code::InstallError`] — that is the boundary working — so the
/// proof that the REAL variant survives the REAL shared funnel has to live
/// here.
///
/// Deleting the move and re-privatising a funnel in this crate would still
/// compile; importing a funnel that re-wrapped or re-stringified the error
/// would not survive this.
#[test]
fn the_shared_funnel_returns_this_binarys_own_exit_bearing_error_object() {
    let original = anyhow::Error::new(crate::exit_code::InstallError::UserDeclined)
        .context("resolving `@vibe/demo`");
    let rendered = format!("{original:#}");
    let fixed = Timestamp::from_timestamp(0, 0).expect("a fixture instant");

    let finalized = finalize(
        preparation(false),
        CommandExit::Failed {
            report: install_draft(),
            original_error: original,
            emit_when_trace_disabled: false,
        },
        &|| fixed,
    );

    assert!(
        !finalized.emit_report,
        "a historically silent stage stays silent"
    );
    assert!(
        finalized.trace.is_none(),
        "a disabled run carries no member"
    );
    let returned = finalized.original_error.expect("the error comes back");
    assert_eq!(format!("{returned:#}"), rendered, "never re-stringified");
    assert!(
        returned
            .downcast_ref::<crate::exit_code::InstallError>()
            .is_some(),
        "the typed base survives, so `as_exit_code` still finds its variant",
    );
}

/// This surface's registered-draft transport IS the shared generic carrier —
/// not a private twin that happens to behave the same today.
///
/// Proved from the outside: an error built by this crate's [`carry`] is taken
/// apart by `vibe_orchestrator::failure::take` at the shared type. Reintroduce
/// a CLI-only `FailedDraft` struct and this stops compiling; keep the name but
/// give it its own `anyhow` wrapper and the `take` returns `Err`.
#[test]
fn the_registered_draft_rides_the_one_shared_carrier() {
    let carried = carry(
        install_draft(),
        anyhow::Error::new(crate::exit_code::InstallError::UserDeclined),
        true,
    );
    let taken = vibe_orchestrator::failure::take::<RegisteredReportDraft>(carried)
        .unwrap_or_else(|error| panic!("the shared carrier owns it: {error:#}"));
    assert!(matches!(taken.evidence, RegisteredReportDraft::Install(_)));
    assert!(taken.emit_machine_failure, "the site's own bit crosses");
    assert!(
        taken
            .original
            .downcast_ref::<crate::exit_code::InstallError>()
            .is_some(),
    );

    // And the funnel's classifier reads the same carrier back out — the
    // CARRIED family, never the stage fallback's.
    let CommandExit::Failed {
        report,
        emit_when_trace_disabled,
        ..
    } = classify(
        carry(install_draft(), anyhow::Error::msg("refused"), true),
        || {
            RegisteredReportDraft::Lifecycle(Box::new(
                vibe_orchestrator::values::LifecycleValues::failed(
                    "build",
                    Vec::new(),
                    "build",
                    Vec::new(),
                ),
            ))
        },
    )
    else {
        panic!("a carried failure is a failure");
    };
    assert!(matches!(report, RegisteredReportDraft::Install(_)));
    assert!(emit_when_trace_disabled);
}

/// Nothing in this surface DEFINES a trace funnel any more.
///
/// The three failure modes A13 has to keep out are all invisible to a passing
/// behaviour test: the move is reverted and a funnel reappears here; the move
/// lands but a copy is left behind so two owners of one cooperative project
/// lock exist; or the deleted lifecycle draft wrapper grows back beside the
/// shared values it was deleted in favour of. Each of those is a DEFINITION in
/// this crate's production source, so that is what this reads.
///
/// Every needle is spelled in halves: this file is production-adjacent only as
/// a test cell, but the same trap applies — a checker whose own source contains
/// the strings it forbids reports itself and never fails for a real offender.
#[test]
fn the_surface_defines_no_second_funnel() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders: Vec<String> = Vec::new();
    let mut stack = vec![root];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).expect("a readable source directory") {
            let path = entry.expect("a readable entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            // Test cells legitimately NAME the moved types; the fence is on
            // production source, where a definition would really live.
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == "tests.rs" || name.ends_with("_tests.rs") {
                continue;
            }
            if path
                .components()
                .any(|part| part.as_os_str() == std::ffi::OsStr::new("tests"))
            {
                continue;
            }
            let body = std::fs::read_to_string(&path).expect("a readable source file");
            for needle in [
                concat!("enum Trace", "Session"),
                concat!("struct Trace", "Preparation"),
                concat!("enum Command", "Exit"),
                concat!("enum Plan", "Disposition"),
                concat!("struct Finalized", "Command"),
                concat!("struct Bounded", "Diagnostic"),
                concat!("struct Failed", "Draft"),
                concat!("struct Lifecycle", "Draft"),
                concat!("fn final", "ize"),
                concat!("fn super", "sede"),
                // The trace-home JOIN. `RunPrelude::prepare_trace` is the one
                // pairing of "which root may hold a trace" with "which funnel
                // entry opens it", and it is the epoch's, not a surface's. A
                // surface can only re-derive it by naming one of the two arms
                // or by declaring its own `prepare_trace`, so both are refused
                // here — a re-grown trait, a free helper and a hand-rolled
                // `match loaded_root()` all trip on the same needles.
                concat!("trait Prepare", "Trace"),
                concat!("fn prepare_", "trace"),
                concat!("without_", "workspace"),
                concat!("trace::pre", "pare("),
                concat!("compile_trace::pre", "pare("),
            ] {
                if body.contains(needle) {
                    offenders.push(format!("{} defines `{needle}`", path.display()));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the funnel and the deleted draft wrapper live in `vibe-orchestrator`, once: {offenders:#?}",
    );
}
