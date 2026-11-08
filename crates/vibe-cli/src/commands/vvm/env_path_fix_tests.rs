//! The consent matrix and the consented repair over both persisters, with
//! no real registry, rc file or terminal involved.

use std::path::Path;

use super::super::registry::{MemoryRegistry, RegistryEnvPersister, RegistryValueKind};
use super::super::{RcFilePersister, Shell};
use super::*;

const SHIMS: &str = r"C:\Users\me\.vibe\bin";

fn consent(flag: Option<bool>, policy: PathFixPolicy, interactive: bool) -> PathFixConsent {
    PathFixConsent {
        flag,
        policy,
        interactive,
        default_answer: false,
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn the_decision_matrix_puts_the_flag_first_and_never_writes_unattended_without_consent() {
    use PathFixDecision::{Apply, Ask, Skip};
    use PathFixPolicy::{Always, Never};
    let ask = PathFixPolicy::Ask;
    let cases = [
        // (flag, policy, interactive) -> decision
        ((Some(true), ask, false), Apply(ConsentSource::Flag)),
        ((Some(true), Never, true), Apply(ConsentSource::Flag)),
        ((Some(false), Always, true), Skip(SkipReason::FlagDeclined)),
        ((Some(false), ask, true), Skip(SkipReason::FlagDeclined)),
        ((None, Always, false), Apply(ConsentSource::Profile)),
        ((None, Always, true), Apply(ConsentSource::Profile)),
        ((None, Never, true), Skip(SkipReason::ProfileNever)),
        ((None, Never, false), Skip(SkipReason::ProfileNever)),
        ((None, ask, true), Ask { default: false }),
        ((None, ask, false), Skip(SkipReason::NotInteractive)),
    ];
    for ((flag, policy, interactive), expected) in cases {
        assert_eq!(
            decide(&consent(flag, policy, interactive)),
            expected,
            "flag={flag:?} policy={policy:?} interactive={interactive}"
        );
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn the_question_defaults_to_yes_on_windows_and_no_elsewhere() {
    let host = PathFixConsent::new(None, PathFixPolicy::Ask, true);
    assert_eq!(host.default_answer, cfg!(windows));
    assert_eq!(
        decide(&host),
        PathFixDecision::Ask {
            default: cfg!(windows)
        }
    );
    let windows = PathFixConsent {
        default_answer: true,
        ..host
    };
    assert_eq!(decide(&windows), PathFixDecision::Ask { default: true });
    assert_eq!(
        decide(&PathFixConsent::granted()),
        PathFixDecision::Apply(ConsentSource::Flag)
    );
}

fn registry_persister(path: &str) -> RegistryEnvPersister<MemoryRegistry> {
    RegistryEnvPersister::new(MemoryRegistry::with(&[(
        "Path",
        path,
        RegistryValueKind::ExpandString,
    )]))
}

fn never_asked(_: &str, _: bool) -> bool {
    panic!("no question may be asked here")
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn an_interactive_yes_applies_through_the_registry_persister() {
    let persister = registry_persister(r"%USERPROFILE%\bin");
    let mut asked = Vec::new();
    let report = ensure_shim_dir_on_durable_path(
        &persister,
        Path::new(SHIMS),
        &PathFixConsent {
            default_answer: true,
            ..consent(None, PathFixPolicy::Ask, true)
        },
        &mut |question, default| {
            asked.push((question.to_string(), default));
            default
        },
    )
    .unwrap();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].0.contains("HKCU\\Environment\\Path"), "{asked:?}");
    assert!(asked[0].1, "the Windows default is yes");
    assert_eq!(report.outcome, PathFixOutcome::Applied);
    assert_eq!(report.consent, Some(ConsentSource::Interactive));
    assert!(!report.was_on_durable_path);
    assert!(report.on_durable_path);
    assert!(report.changed);
    assert!(report.activation_hint.is_some());
    assert_eq!(report.fix_hint, None);
    let written = persister.registry().value("Path").unwrap();
    assert_eq!(written.value, format!(r"{SHIMS};%USERPROFILE%\bin"));
    assert_eq!(written.kind, RegistryValueKind::ExpandString);

    let again = ensure_shim_dir_on_durable_path(
        &persister,
        Path::new(SHIMS),
        &consent(Some(true), PathFixPolicy::Ask, false),
        &mut never_asked,
    )
    .unwrap();
    assert_eq!(again.outcome, PathFixOutcome::AlreadyOnPath);
    assert!(again.was_on_durable_path && again.on_durable_path);
    assert_eq!(
        persister.registry().writes(),
        1,
        "the re-run writes nothing"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn a_declined_or_unconsented_fix_writes_nothing_and_names_the_flag() {
    let persister = registry_persister(r"C:\Tools");
    let declined = ensure_shim_dir_on_durable_path(
        &persister,
        Path::new(SHIMS),
        &consent(None, PathFixPolicy::Ask, true),
        &mut |_, _| false,
    )
    .unwrap();
    assert_eq!(declined.outcome, PathFixOutcome::Skipped);
    assert_eq!(declined.skip_reason, Some(SkipReason::Declined));
    assert!(!declined.on_durable_path);
    assert!(
        declined
            .fix_hint
            .as_deref()
            .is_some_and(|hint| hint.contains("--fix-path"))
    );

    for (flag, policy, reason) in [
        (None, PathFixPolicy::Ask, SkipReason::NotInteractive),
        (None, PathFixPolicy::Never, SkipReason::ProfileNever),
        (Some(false), PathFixPolicy::Always, SkipReason::FlagDeclined),
    ] {
        let report = ensure_shim_dir_on_durable_path(
            &persister,
            Path::new(SHIMS),
            &consent(flag, policy, false),
            &mut never_asked,
        )
        .unwrap();
        assert_eq!(report.skip_reason, Some(reason));
        assert_eq!(report.consent, None);
    }
    assert_eq!(persister.registry().writes(), 0);
    assert_eq!(
        persister.registry().value("Path").unwrap().value,
        r"C:\Tools"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn a_profile_always_policy_fixes_an_unattended_rc_file() {
    let dir = tempfile::tempdir().unwrap();
    let rc = dir.path().join(".bashrc");
    std::fs::write(&rc, "# owner line\n").unwrap();
    let persister = RcFilePersister::new(rc.clone(), Shell::Bash);
    let shims = dir.path().join("shims");
    let report = ensure_shim_dir_on_durable_path(
        &persister,
        &shims,
        &consent(None, PathFixPolicy::Always, false),
        &mut never_asked,
    )
    .unwrap();
    assert_eq!(report.outcome, PathFixOutcome::Applied);
    assert_eq!(report.consent, Some(ConsentSource::Profile));
    assert!(report.on_durable_path);
    let text = std::fs::read_to_string(&rc).unwrap();
    assert!(text.starts_with("# owner line\n"), "{text}");
    assert!(text.contains(&shims.display().to_string()), "{text}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#path", r = 3)]
fn a_persister_failure_cites_the_rule_and_the_fix() {
    let dir = tempfile::tempdir().unwrap();
    let rc = dir.path().join(".bashrc");
    let block = "# >>> vibevm (VVM) — managed, do not edit by hand >>>\nexport PATH='/a':$PATH\nexport PATH='/b':$PATH\n# <<< vibevm (VVM) <<<\n";
    std::fs::write(&rc, block).unwrap();
    let persister = RcFilePersister::new(rc.clone(), Shell::Bash);
    let error = ensure_shim_dir_on_durable_path(
        &persister,
        Path::new("/c"),
        &PathFixConsent::granted(),
        &mut never_asked,
    )
    .map(|report| report.outcome);
    let message = match error {
        Err(error) => error.to_string(),
        Ok(outcome) => panic!("expected a refusal, got {outcome:?}"),
    };
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-019#path"),
        "{message}"
    );
    assert!(message.contains("fix:"), "{message}");
    assert_eq!(
        std::fs::read_to_string(&rc).unwrap(),
        block,
        "nothing written"
    );
}
