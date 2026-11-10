//! The trust an untrusted dependency's `slot:` contributions need before any
//! of them runs (PROP-065 `##ordinary-trust`), resolved at the CLI boundary
//! because that is where the terminal, the flags and `agents.toml` are.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust");

use std::sync::Arc;

use vibe_install::{SlotConsentRequest, SlotJudgmentContext};

use crate::output;

/// The trusted set: the `trusted_groups` of `agents.toml` — the very list
/// `vibe agent list` prints and an agent step already obeys — unioned with
/// PROP-020's default hook allow-list, so a group trusted to run a hook is
/// trusted to run the same act written in the long form.
fn trusted_groups() -> Vec<String> {
    let mut groups: Vec<String> = vibe_workspace::hooks::DEFAULT_ALLOWED_GROUPS
        .iter()
        .map(|group| (*group).to_string())
        .collect();
    if let Ok(settings) = crate::commands::app::global::settings_root()
        && let Ok(config) = vibe_app::AgentsConfig::load(&settings)
    {
        for group in config.trusted_groups {
            if !groups.contains(&group) {
                groups.push(group);
            }
        }
    }
    groups
}

/// The one question of `##ordinary-trust`, asked once per package and only
/// about a dependency whose group the operator has not trusted.
fn question(request: &SlotConsentRequest) -> String {
    format!(
        "Run {} installation contribution{} of `{}` (group `{}` is not trusted)?",
        request.contributions.len(),
        if request.contributions.len() == 1 {
            ""
        } else {
            "s"
        },
        request.package,
        request.group,
    )
}

/// The host facts and the trust policy of one install pass.
///
/// `assume_yes` is `--assume-yes`, which `--unattended` implies: the per-run
/// escape agent steps already use, reused rather than reinvented.
pub(crate) fn judgment(ctx: &output::Context, assume_yes: bool) -> SlotJudgmentContext {
    let attended = !ctx.is_json() && !ctx.is_unattended() && console::user_attended();
    let notice = ctx.clone();
    SlotJudgmentContext {
        command_present: crate::commands::app::facts::command_on_path,
        env_present: crate::commands::app::facts::env_set,
        attended,
        // The lifecycle agent backend is injected per run rather than read
        // from a selection here, so this plane reports «no agent» and an
        // `agent_configured = true` condition skips. Recorded rather than
        // guessed; the seam is the field, not a branch.
        agent_configured: false,
        trusted_groups: trusted_groups(),
        assume_yes: assume_yes || ctx.is_unattended(),
        confirm: attended.then(|| {
            let asker: vibe_install::SlotConsentAsk = Arc::new(move |request| {
                for line in &request.contributions {
                    notice.step(&format!("contribution: {line}"));
                }
                crate::output::prompts::confirm(&question(request), false)
                    .map_err(|error| error.to_string())
            });
            asker
        }),
    }
}
