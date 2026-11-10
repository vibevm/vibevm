//! The trust an untrusted dependency's `slot:` contributions need before any
//! of them runs (PROP-065 `##ordinary-trust`), resolved at the CLI boundary
//! because that is where the terminal, the flags and `agents.toml` are.
//!
//! The same file answers the one host fact a slot condition cannot read for
//! itself: whether this run has an agent at all (PROP-065
//! `##ordinary-grammar`'s `agent_configured`, `##needs`' `agent`). That is a
//! selection, not a backend — the lifecycle agent backend is injected per
//! run, so asking it would answer "an object exists", not "a program could
//! deliver this contribution" — and the selection lives exactly here, beside
//! the agent list this file already reads.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#selection");

use std::sync::Arc;

use vibe_app::{AgentsConfig, AppError, ExternalAgent};
use vibe_install::{SlotConsentRequest, SlotJudgmentContext};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use crate::output;

/// The user's `agents.toml`, read once: both halves of this judgment — the
/// trusted groups and the agent selection — come out of that one file, and
/// reading it twice would let them disagree.
fn agents_config() -> Option<AgentsConfig> {
    let settings = crate::commands::app::global::settings_root().ok()?;
    AgentsConfig::load(&settings).map_err(AppError::from).ok()
}

/// The trusted set: the `trusted_groups` of `agents.toml` — the very list
/// `vibe agent list` prints and an agent step already obeys — unioned with
/// PROP-020's default hook allow-list, so a group trusted to run a hook is
/// trusted to run the same act written in the long form.
fn trusted_groups(config: Option<&AgentsConfig>) -> Vec<String> {
    let mut groups: Vec<String> = vibe_workspace::hooks::DEFAULT_ALLOWED_GROUPS
        .iter()
        .map(|group| (*group).to_string())
        .collect();
    for group in config.into_iter().flat_map(|config| &config.trusted_groups) {
        if !groups.contains(group) {
            groups.push(group.clone());
        }
    }
    groups
}

/// Whether an agent could deliver an `agent` contribution in this run.
///
/// The same answer `vibe_app::AgentHooks::available` gives the application
/// plane, from the same two facts and no others: vibe runs inside a hosting
/// agent (the row is parked for it, `PROP-054#AGENT-HANDSHAKE`), or an agent
/// is selected — the external registration of PROP-064
/// `#external-registration`, else `agents.toml`'s `default`. `vibe install`
/// has no `--agent` flag, so the selection is asked without one.
///
/// A `default` that names nothing this build can resolve answers **false**:
/// the question is whether a contribution could be delivered, and an
/// unresolvable name delivers nothing. The refusal is the selecting
/// command's to report, not this judgment's to raise.
///
/// PURE in both inputs, so the law is tested over values rather than over
/// whatever this machine's home happens to hold.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-064#selection")]
fn agent_available(
    hosted: bool,
    config: Option<&AgentsConfig>,
    external: Option<ExternalAgent>,
) -> bool {
    if hosted {
        return true;
    }
    match config {
        Some(config) => {
            vibe_app::select_agent(config, None, external).is_ok_and(|selected| selected.is_some())
        }
        // No readable agent list is no definition and no default; an
        // external registration still selects, because it is the invoking
        // program's own word and needs no file.
        None => external.is_some(),
    }
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
    let config = agents_config();
    SlotJudgmentContext {
        command_present: crate::commands::app::facts::command_on_path,
        env_present: crate::commands::app::facts::env_set,
        attended,
        // The run's real agent posture, read from the selection rather than
        // from the injected backend: a backend is always present, and
        // reporting its existence as «an agent is configured» would make
        // every `agent_configured` condition true on a machine with no
        // agent at all. The seam is this field.
        agent_configured: agent_available(
            ctx.agent_mode() == RunAgentMode::Agent,
            config.as_ref(),
            crate::commands::app::facts::external_agent().ok().flatten(),
        ),
        trusted_groups: trusted_groups(config.as_ref()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn external() -> ExternalAgent {
        ExternalAgent {
            command: "gui-agent".into(),
            args: vec![],
        }
    }

    /// The four ways this plane can have an agent, and the three it cannot.
    /// Before this, the field was the constant `false`, so every
    /// `agent_configured = true` condition skipped on every machine.
    #[test]
    fn an_agent_is_available_when_one_is_hosted_registered_or_defaulted() {
        let empty = AgentsConfig::default();
        let defaulted: AgentsConfig = toml::from_str("default = \"codex\"").unwrap();

        assert!(agent_available(true, Some(&empty), None), "hosted");
        assert!(agent_available(false, Some(&defaulted), None), "default");
        assert!(
            agent_available(false, Some(&empty), Some(external())),
            "the external registration selects without a file",
        );
        assert!(agent_available(false, None, Some(external())), "no file");

        assert!(!agent_available(false, Some(&empty), None), "no agent");
        assert!(!agent_available(false, None, None), "nothing at all");
    }

    /// A `default` naming nothing this build resolves is not an agent that
    /// could deliver a contribution: the condition answers false rather
    /// than promising a delivery that would fail.
    #[test]
    fn an_unresolvable_default_is_not_an_available_agent() {
        let broken: AgentsConfig = toml::from_str("default = \"no-such-agent-preset\"").unwrap();
        assert!(!agent_available(false, Some(&broken), None));
    }

    /// The hook allow-list is the floor of the trusted set, with or without
    /// a readable `agents.toml`, and a group it already carries is not
    /// listed twice.
    #[test]
    fn the_trusted_set_is_the_hook_allow_list_unioned_with_the_agent_list() {
        let none = trusted_groups(None);
        assert!(none.contains(&"org.vibevm".to_string()));

        let config: AgentsConfig =
            toml::from_str("trusted_groups = [\"org.example\", \"org.vibevm\"]").unwrap();
        let both = trusted_groups(Some(&config));
        assert!(both.contains(&"org.example".to_string()));
        assert_eq!(
            both.iter().filter(|group| *group == "org.vibevm").count(),
            1,
            "the union adds, it does not duplicate",
        );
    }
}
