//! The argument grammar's reds — its own cell, so `cli.rs` stays the
//! grammar and nothing else.
use clap::Parser;
use specmark::verifies;

use super::{CleanChain, Cli, Command};

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#INVOKE-RUNS-PRIORS")]
fn every_non_install_lifecycle_verb_accepts_global_flags() {
    for verb in [
        "validate", "generate", "build", "test", "create", "verify", "package", "deploy",
    ] {
        let cli = Cli::try_parse_from([
            "vibe",
            verb,
            "--json",
            "--offline",
            "--force",
            "--path",
            ".",
        ])
        .unwrap_or_else(|error| panic!("parse `{verb}`: {error}"));
        assert!(cli.json, "{verb}: --json reaches the root");
        assert!(cli.offline, "{verb}: --offline reaches the root");
        let force = match &cli.command {
            Command::Validate(args)
            | Command::Generate(args)
            | Command::Build(args)
            | Command::Test(args)
            | Command::Create(args)
            | Command::Verify(args)
            | Command::Package(args) => args.force,
            Command::Deploy(args) => args.lifecycle.force,
            _ => false,
        };
        assert!(force, "{verb}: --force reaches lifecycle freshness");
        assert!(matches!(
            (verb, cli.command),
            ("validate", Command::Validate(_))
                | ("generate", Command::Generate(_))
                | ("build", Command::Build(_))
                | ("test", Command::Test(_))
                | ("create", Command::Create(_))
                | ("verify", Command::Verify(_))
                | ("package", Command::Package(_))
                | ("deploy", Command::Deploy(_))
        ));
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
fn non_install_lifecycle_verbs_reject_pkgrefs() {
    for verb in [
        "validate", "generate", "build", "test", "create", "verify", "package", "deploy",
    ] {
        assert!(
            Cli::try_parse_from(["vibe", verb, "flow:org.example/pkg"]).is_err(),
            "{verb} must not accept an install pkgref",
        );
    }
    for argv in [
        vec!["vibe", "build", "--exact"],
        vec!["vibe", "build", "--git", "https://example.invalid/pkg.git"],
    ] {
        assert!(
            Cli::try_parse_from(argv).is_err(),
            "non-install lifecycle verbs must reject install-only flags",
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CHAIN-GENERAL")]
fn clean_accepts_every_default_lifecycle_phase() {
    for verb in [
        "validate", "install", "generate", "build", "test", "create", "verify", "package", "deploy",
    ] {
        let cli = Cli::try_parse_from(["vibe", "clean", verb])
            .unwrap_or_else(|error| panic!("parse `clean {verb}`: {error}"));
        let Command::Clean(args) = cli.command else {
            panic!("`clean {verb}` did not parse as clean");
        };
        let chain = args.chain.expect("a continuation");
        assert!(matches!(
            (verb, chain),
            ("validate", CleanChain::Validate(_))
                | ("install", CleanChain::Install(_))
                | ("generate", CleanChain::Generate(_))
                | ("build", CleanChain::Build(_))
                | ("test", CleanChain::Test(_))
                | ("create", CleanChain::Create(_))
                | ("verify", CleanChain::Verify(_))
                | ("package", CleanChain::Package(_))
                | ("deploy", CleanChain::Deploy(_))
        ));
    }
}

/// The root `--offline` parses before any subcommand (PROP-010
/// §2.5): the posture is a property of the invocation, not of one
/// subcommand.
#[test]
fn offline_flag_parses_on_the_root() {
    let cli =
        Cli::try_parse_from(["vibe", "--offline", "list"]).expect("parse `vibe --offline list`");
    assert!(cli.offline, "--offline reaches the root Cli");
    let Command::List(_) = cli.command else {
        panic!("argv did not parse to `list`");
    };
}

/// Absent the flag, the root posture is online — the default.
#[test]
fn offline_defaults_to_false_on_the_root() {
    let cli = Cli::try_parse_from(["vibe", "list"]).expect("parse `vibe list`");
    assert!(!cli.offline);
}

/// `vibe install --offline` (PROP-030 §3.1) keeps parsing to the
/// subcommand's own flag — the posture absorbs it as one more
/// input, it does not replace it. Note clap's actual mechanics for
/// a global root arg that shares its id with a subcommand arg:
/// they are one argument, so both matches carry the value. That is
/// harmless here — `install::run` resolves the posture as
/// `root_offline || args.offline`.
#[test]
fn install_local_offline_flag_still_parses() {
    let cli = Cli::try_parse_from(["vibe", "install", "--offline"])
        .expect("parse `vibe install --offline`");
    let Command::Install(args) = cli.command else {
        panic!("argv did not parse to `install`");
    };
    assert!(args.offline, "--offline reaches InstallArgs");
    assert!(cli.offline, "the shared id also sets the root field");
}

/// `vibe --offline install` sets the root posture — and, because
/// clap unifies the global root arg with the same-id subcommand
/// arg, `InstallArgs.offline` sees it too. Either way the OR in
/// `install::run` resolves the same posture.
#[test]
fn root_offline_reaches_the_install_command() {
    let cli = Cli::try_parse_from(["vibe", "--offline", "install"])
        .expect("parse `vibe --offline install`");
    assert!(cli.offline);
    let Command::Install(args) = cli.command else {
        panic!("argv did not parse to `install`");
    };
    assert!(args.offline, "the shared id carries the root flag down");
}
