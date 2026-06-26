//! Unit tests for the install-hook cell. The two seams ([`InterpreterProbe`]
//! and [`HookRunner`]) are faked so selection, trust, and failure paths are
//! asserted without spawning a real process.

use super::*;
use std::fs;
use std::path::PathBuf;

use specmark::verifies;
use tempfile::tempdir;
use vibe_core::Group;
use vibe_core::manifest::HooksDecl;

/// Probe that reports a fixed set of interpreters as present.
struct FakeProbe {
    present: Vec<String>,
}

impl FakeProbe {
    fn with(progs: &[&str]) -> Self {
        FakeProbe {
            present: progs.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl InterpreterProbe for FakeProbe {
    fn has(&self, program: &str) -> bool {
        self.present.iter().any(|p| p == program)
    }
}

/// Runner that returns a fixed exit code (or spawn error) without spawning.
struct FakeRunner {
    result: Result<i32, String>,
}

impl HookRunner for FakeRunner {
    fn run(
        &self,
        _inv: &HookInvocation,
        _cwd: &Path,
        _env: &[(String, String)],
    ) -> Result<i32, String> {
        self.result.clone()
    }
}

fn org(s: &str) -> Group {
    Group::parse(s).unwrap()
}

fn pre_hook() -> HooksDecl {
    HooksDecl {
        pre_install: Some(PathBuf::from("hooks/prepare")),
        post_install: None,
    }
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#trust-gate", r = 1)]
fn trust_matrix() {
    let allowed = vec!["org.vibevm".to_string()];
    let vibe = org("org.vibevm");
    let other = org("org.other");
    // Allow-listed group runs with no prompt, even non-interactively.
    assert_eq!(
        decide_trust(&vibe, &allowed, false, false),
        HookTrust::Allowed
    );
    // Non-allow-listed needs consent when interactive.
    assert_eq!(
        decide_trust(&other, &allowed, true, false),
        HookTrust::NeedsConsent
    );
    // …and is refused non-interactively (never silent-run).
    assert_eq!(
        decide_trust(&other, &allowed, false, false),
        HookTrust::Refused
    );
    // …unless explicitly opted in with --allow-hooks.
    assert_eq!(
        decide_trust(&other, &allowed, false, true),
        HookTrust::Allowed
    );
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#trust-gate", r = 1)]
fn hook_policy_maps_allowed_to_run_and_the_rest_to_skip() {
    // The pipeline-side policy resolves only run-vs-skip; the CLI already
    // turned a genuine refusal into an abort, so `Refused` never appears here.
    let vibe = org("org.vibevm");
    let other = org("org.other");

    let allowlisted = HookPolicy {
        allowed_groups: vec!["org.vibevm".to_string()],
        allow_hooks: false,
    };
    assert_eq!(allowlisted.trust_for(&vibe), HookTrust::Allowed);
    // A group absent from the policy is skipped — never silently run.
    assert_eq!(allowlisted.trust_for(&other), HookTrust::NeedsConsent);

    // `--allow-hooks` runs every group's hooks regardless of the list.
    let force = HookPolicy {
        allowed_groups: Vec::new(),
        allow_hooks: true,
    };
    assert_eq!(force.trust_for(&other), HookTrust::Allowed);

    // The default policy trusts nothing.
    assert_eq!(
        HookPolicy::default().trust_for(&vibe),
        HookTrust::NeedsConsent
    );
}

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-020#script-selection",
    r = 1
)]
fn unix_selects_sh_via_bash() {
    let slot = tempdir().unwrap();
    fs::create_dir_all(slot.path().join("hooks")).unwrap();
    fs::write(slot.path().join("hooks/prepare.sh"), "#!/bin/sh\n").unwrap();
    let base = Path::new("hooks/prepare");

    let with_bash = FakeProbe::with(&["bash"]);
    let inv = select_invocation(slot.path(), base, Platform::Unix, &with_bash).unwrap();
    assert_eq!(inv.interpreter, "bash");
    assert!(inv.script.ends_with("prepare.sh"));

    // No bash on PATH → nothing usable.
    let no_bash = FakeProbe::with(&[]);
    assert!(select_invocation(slot.path(), base, Platform::Unix, &no_bash).is_none());
}

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-020#script-selection",
    r = 1
)]
fn windows_prefers_sh_then_falls_back_to_ps1() {
    let slot = tempdir().unwrap();
    fs::create_dir_all(slot.path().join("hooks")).unwrap();
    fs::write(slot.path().join("hooks/prepare.sh"), "#!/bin/sh\n").unwrap();
    fs::write(slot.path().join("hooks/prepare.ps1"), "Write-Host hi\n").unwrap();
    let base = Path::new("hooks/prepare");

    // Both interpreters present → Git Bash wins.
    let both = FakeProbe::with(&["bash", "powershell"]);
    let inv = select_invocation(slot.path(), base, Platform::Windows, &both).unwrap();
    assert_eq!(inv.interpreter, "bash");

    // Only PowerShell present → fall back to the .ps1.
    let ps_only = FakeProbe::with(&["powershell"]);
    let inv2 = select_invocation(slot.path(), base, Platform::Windows, &ps_only).unwrap();
    assert_eq!(inv2.interpreter, "powershell");
    assert!(inv2.script.ends_with("prepare.ps1"));

    // Neither → None.
    let none = FakeProbe::with(&[]);
    assert!(select_invocation(slot.path(), base, Platform::Windows, &none).is_none());
}

#[test]
fn not_declared_phase_is_a_noop() {
    let slot = tempdir().unwrap();
    let group = org("org.vibevm");
    let ctx = HookContext {
        group: &group,
        name: "x",
        version: "0.1.0",
        kind: "tool",
        slot: slot.path(),
    };
    let r = run_package_hook(
        HookPhase::PreInstall,
        &HooksDecl::default(),
        &ctx,
        HookTrust::Allowed,
        Platform::Unix,
        &FakeProbe::with(&[]),
        &FakeRunner { result: Ok(0) },
    )
    .unwrap();
    assert_eq!(r.status, "not-declared");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#phases", r = 1)]
fn allowed_hook_runs_on_zero_exit() {
    let slot = tempdir().unwrap();
    fs::create_dir_all(slot.path().join("hooks")).unwrap();
    fs::write(slot.path().join("hooks/prepare.sh"), "#!/bin/sh\n").unwrap();
    let group = org("org.vibevm");
    let ctx = HookContext {
        group: &group,
        name: "x",
        version: "0.1.0",
        kind: "tool",
        slot: slot.path(),
    };
    let r = run_package_hook(
        HookPhase::PreInstall,
        &pre_hook(),
        &ctx,
        HookTrust::Allowed,
        Platform::Unix,
        &FakeProbe::with(&["bash"]),
        &FakeRunner { result: Ok(0) },
    )
    .unwrap();
    assert_eq!(r.status, "ran");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#failure", r = 1)]
fn pre_install_nonzero_aborts() {
    let slot = tempdir().unwrap();
    fs::create_dir_all(slot.path().join("hooks")).unwrap();
    fs::write(slot.path().join("hooks/prepare.sh"), "#!/bin/sh\n").unwrap();
    let group = org("org.vibevm");
    let ctx = HookContext {
        group: &group,
        name: "x",
        version: "0.1.0",
        kind: "tool",
        slot: slot.path(),
    };
    let err = run_package_hook(
        HookPhase::PreInstall,
        &pre_hook(),
        &ctx,
        HookTrust::Allowed,
        Platform::Unix,
        &FakeProbe::with(&["bash"]),
        &FakeRunner { result: Ok(1) },
    )
    .unwrap_err();
    assert!(matches!(err, HookError::PreInstallFailed { code: 1, .. }));
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#failure", r = 1)]
fn post_install_nonzero_installs_but_flags() {
    let slot = tempdir().unwrap();
    fs::create_dir_all(slot.path().join("hooks")).unwrap();
    fs::write(slot.path().join("hooks/finalise.sh"), "#!/bin/sh\n").unwrap();
    let hooks = HooksDecl {
        pre_install: None,
        post_install: Some(PathBuf::from("hooks/finalise")),
    };
    let group = org("org.vibevm");
    let ctx = HookContext {
        group: &group,
        name: "x",
        version: "0.1.0",
        kind: "tool",
        slot: slot.path(),
    };
    let r = run_package_hook(
        HookPhase::PostInstall,
        &hooks,
        &ctx,
        HookTrust::Allowed,
        Platform::Unix,
        &FakeProbe::with(&["bash"]),
        &FakeRunner { result: Ok(2) },
    )
    .unwrap();
    assert_eq!(r.status, "post-install-failed");
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-020#trust-gate", r = 1)]
fn refused_trust_is_an_error() {
    let slot = tempdir().unwrap();
    let group = org("org.untrusted");
    let ctx = HookContext {
        group: &group,
        name: "x",
        version: "0.1.0",
        kind: "tool",
        slot: slot.path(),
    };
    let err = run_package_hook(
        HookPhase::PreInstall,
        &pre_hook(),
        &ctx,
        HookTrust::Refused,
        Platform::Unix,
        &FakeProbe::with(&[]),
        &FakeRunner { result: Ok(0) },
    )
    .unwrap_err();
    assert!(matches!(err, HookError::Untrusted { .. }));
}

#[test]
fn needs_consent_skips_in_library() {
    let slot = tempdir().unwrap();
    let group = org("org.untrusted");
    let ctx = HookContext {
        group: &group,
        name: "x",
        version: "0.1.0",
        kind: "tool",
        slot: slot.path(),
    };
    let r = run_package_hook(
        HookPhase::PreInstall,
        &pre_hook(),
        &ctx,
        HookTrust::NeedsConsent,
        Platform::Unix,
        &FakeProbe::with(&[]),
        &FakeRunner { result: Ok(0) },
    )
    .unwrap();
    assert_eq!(r.status, "skipped-needs-consent");
}

#[test]
#[verifies(
    "spec://vibevm/modules/vibe-workspace/PROP-020#script-selection",
    r = 1
)]
fn declared_but_no_interpreter_errors() {
    let slot = tempdir().unwrap();
    fs::create_dir_all(slot.path().join("hooks")).unwrap();
    fs::write(slot.path().join("hooks/prepare.sh"), "#!/bin/sh\n").unwrap();
    let group = org("org.vibevm");
    let ctx = HookContext {
        group: &group,
        name: "x",
        version: "0.1.0",
        kind: "tool",
        slot: slot.path(),
    };
    // Script is declared and present, but no bash on PATH → hard error,
    // never a silent skip (PROP-020 §2.2).
    let err = run_package_hook(
        HookPhase::PreInstall,
        &pre_hook(),
        &ctx,
        HookTrust::Allowed,
        Platform::Unix,
        &FakeProbe::with(&[]),
        &FakeRunner { result: Ok(0) },
    )
    .unwrap_err();
    assert!(matches!(err, HookError::NoInterpreter { .. }));
}
