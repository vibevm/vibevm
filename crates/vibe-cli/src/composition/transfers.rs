//! The invocation's transfers (`PROP-068#limit`, `#journal`): the journal
//! under the settings root, the one rate limit of `[transfer] limit`, read
//! once here, and the command's verb every record names. They ride on the
//! output context's progress handle, so every download reaches them
//! through the observation it already receives.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-068#limit");

use clap::CommandFactory;
use vibe_core::progress::transfer::{TransferJournal, TransferLimit, Transfers};
use vibe_core::user_config::TransferConfig;

use crate::cli::Cli;
use crate::output;

/// Select the existing transfer posture at this command composition boundary.
pub(crate) fn attach_for_command(
    ctx: output::Context,
    config: &TransferConfig,
    command: &crate::cli::Command,
) -> output::Context {
    if crate::composition::project_preview(command) {
        attach_preview(ctx, config)
    } else {
        attach(ctx, config)
    }
}

/// Attach the invocation's transfers to `ctx`. A limit that does not read
/// is said once and ignored rather than refusing the command: the setting
/// belongs to the machine, and the command was not asked about it.
pub(crate) fn attach(ctx: output::Context, config: &TransferConfig) -> output::Context {
    attach_with_journal(ctx, config, true)
}

/// Preview retains progress and rate limits without opening a durable journal.
pub(crate) fn attach_preview(ctx: output::Context, config: &TransferConfig) -> output::Context {
    attach_with_journal(ctx, config, false)
}

fn attach_with_journal(
    ctx: output::Context,
    config: &TransferConfig,
    record: bool,
) -> output::Context {
    let limit = TransferLimit::from_config(config).unwrap_or_else(|reason| {
        eprintln!(
            "{}",
            output::language::warning(&format!("[transfer] limit is ignored: {reason}"))
        );
        None
    });
    let journal = if record {
        vibe_core::settings::settings_dir().map(|root| TransferJournal::new(&root))
    } else {
        None
    };
    let verb = verb(std::env::args_os());
    let transfers = Transfers::new(journal, limit, verb);
    // Every clone of the invocation is journaled, whether or not the path
    // that makes it threads the observation (`PROP-068#git`).
    vibe_registry::git_backend::transfers::observe_by_default(
        vibe_core::progress::Progress::default().with_transfers(Some(transfers.clone())),
    );
    ctx.with_transfers(transfers)
}

/// The command line's verb as the journal names it: the subcommand path
/// (`self update`, `cache add`) with `-g` when the command is global
/// (`install -g`). Read from clap's own parse, so it is the name the user
/// typed whatever flags came before it.
fn verb(args: impl IntoIterator<Item = std::ffi::OsString>) -> String {
    let Ok(matches) = Cli::command().try_get_matches_from(args) else {
        return String::new();
    };
    let mut words = Vec::new();
    let mut global = false;
    let mut level = matches.subcommand();
    while let Some((name, sub)) = level {
        words.push(name.to_owned());
        global |= matches!(sub.try_get_one::<bool>("global"), Ok(Some(true)));
        // `install -g PACKAGE` has no subcommand below it; `self update` has.
        level = sub.subcommand();
    }
    if global {
        words.push("-g".to_owned());
    }
    words.join(" ")
}

#[cfg(test)]
#[path = "transfers/coverage_tests.rs"]
mod coverage_tests;

#[cfg(test)]
mod tests {
    use super::verb;

    fn of(line: &str) -> String {
        verb(line.split(' ').map(Into::into))
    }

    #[test]
    fn the_verb_is_the_subcommand_path_with_its_global_flag() {
        assert_eq!(of("vibe --json install -g org.example/x"), "install -g");
        assert_eq!(of("vibe self update"), "self update");
        assert_eq!(of("vibe cache add org.example/x"), "cache add");
        assert_eq!(of("vibe transfers watch --json"), "transfers watch");
    }
}
