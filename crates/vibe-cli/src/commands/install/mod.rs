//! `vibe install <kind>:<name>[@version] …` — the CLI's share of the install
//! transaction (VIBEVM-SPEC §5.6, §9.1, §11.1).
//!
//! The pipeline itself is `vibe-install`; the whole-command algorithm above it
//! — the empty/fresh/ready decision, the Ready apply, the slot continuation and
//! the post-durability world — is the shared application service in
//! `vibe-orchestrator`. What remains here is exactly the CLI's own share:
//! argument grammar and input normalisation (pkgref parsing, PROP-008 §2.6
//! short-name qualification), registry cell construction behind the
//! [`vibe_install::InstallSource`] seam (R-001 — the registry module builds the
//! cells), the interactive confirmation, terminal/JSON rendering, and the
//! registered report family every failure is classified into.
//!
//! ## Where the compile trace enters
//!
//! The shared core BORROWS `Option<&TraceRun>` and hands it to the traced
//! sibling of every API that compiles. It never opens, finishes or clones a
//! recorder — the owner is the command boundary here ([`direct::run`] for
//! `vibe install`, `lifecycle::execute` for a phase verb), and a second owner of
//! the project's cooperative lock would be a second answer to "is this
//! workspace being traced right now".

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

mod closure_diff;
mod confirm;
mod direct;
mod draft;
mod events;
mod observer;
mod project_local;
mod report;
mod resolver;

pub(crate) use closure_diff::{emit_closure_diff, lane_sizes};
pub(crate) use confirm::CliConfirmGate;
pub(crate) use direct::run as run_direct;
pub(crate) use draft::InstallDraft;
pub(crate) use observer::{CliInstallObserver, CliRegistryEnvironment, LifecycleSlotObserver};
pub(crate) use project_local::project_packages_root;
pub(crate) use report::{HookReportPresentation, LifecycleHookView};
pub(crate) use resolver::{
    CliGitSourceMutation, CliPackageSourceFactory, InstallResolver, build_install_resolver,
};
pub(crate) use vibe_install::exact_pinned_pkgref;

/// The shared application service's own names, re-exported at the seam every
/// CLI command already imports. One definition, one crate: nothing here
/// re-implements a moved algorithm.
pub(crate) use vibe_orchestrator::failure::{
    MeasuredFailure, Measurement, take as take_measured_failure,
};
pub(crate) use vibe_orchestrator::ports::{NoAfterDurableWorld, NoManifestMutation};
pub(crate) use vibe_orchestrator::{
    InstallDisposition, InstallExecution, InstallRun, InstallRunContext, PreparedSelection,
    ResumeOutcome, ResumeRequest, SelectedManifest, WorldCallbackOutcome, WorldCallbackSummary,
    acquire_lease, execute_prepared, own_resume, resolve_project_root, resolve_spec_format,
    resume_slot_continuation,
};
