//! Unit tests for the output context. Split out of `output.rs` so the
//! production file stays inside the file-length budget. Env mutation
//! goes through `rust_ai_native_env_audit::EnvGuard` — the designated unsafe audit
//! crate (AUD-0016 posture): one guard per test serializes all
//! env-mutating tests process-wide and restores on drop.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#output-format");

use super::*;
use rust_ai_native_env_audit::EnvGuard;

#[test]
fn resolve_returns_default_when_neither_flag_nor_env() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_INVOKED_BY");
    let (v, p) = resolve_invoked_by(None);
    assert_eq!(v, None);
    assert_eq!(p, InvokedByProvenance::Default);
}

#[test]
fn resolve_uses_env_when_flag_absent() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_INVOKED_BY", "opencode");
    let (v, p) = resolve_invoked_by(None);
    assert_eq!(v.as_deref(), Some("opencode"));
    assert_eq!(p, InvokedByProvenance::EnvVar);
}

#[test]
fn resolve_flag_wins_over_env() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_INVOKED_BY", "opencode");
    let (v, p) = resolve_invoked_by(Some("claude-code"));
    assert_eq!(v.as_deref(), Some("claude-code"));
    assert_eq!(p, InvokedByProvenance::CliFlag);
}

#[test]
fn resolve_treats_empty_flag_as_absent() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_INVOKED_BY", "opencode");
    let (v, p) = resolve_invoked_by(Some("   "));
    assert_eq!(v.as_deref(), Some("opencode"));
    assert_eq!(p, InvokedByProvenance::EnvVar);
}

#[test]
fn resolve_treats_empty_env_as_absent() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_INVOKED_BY", "");
    let (v, p) = resolve_invoked_by(None);
    assert_eq!(v, None);
    assert_eq!(p, InvokedByProvenance::Default);
}

#[test]
fn render_json_stamps_invoked_by_on_object_payloads() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_INVOKED_BY");
    let ctx = Context::from_flags(
        false,
        true,
        Some("codex"),
        false,
        crate::cli::AgentModeArg::Auto,
    );
    let payload = serde_json::json!({ "ok": true, "command": "demo" });
    let rendered = ctx.render_json(&payload).unwrap();
    let parsed: Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(parsed["invoked_by"], "codex");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["command"], "demo");
}

#[test]
fn render_json_omits_invoked_by_when_unset() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_INVOKED_BY");
    let ctx = Context::from_flags(false, true, None, false, crate::cli::AgentModeArg::Auto);
    let payload = serde_json::json!({ "ok": true });
    let rendered = ctx.render_json(&payload).unwrap();
    let parsed: Value = serde_json::from_str(&rendered).unwrap();
    assert!(parsed.get("invoked_by").is_none());
}

#[test]
fn unattended_default_false_with_no_flag_no_env() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_UNATTENDED");
    assert!(!resolve_unattended(false));
}

#[test]
fn unattended_cli_flag_true_wins() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_UNATTENDED");
    assert!(resolve_unattended(true));
}

#[test]
fn unattended_env_truthy_values() {
    let mut env = EnvGuard::lock();
    for raw in ["1", "true", "TRUE", " yes ", "On", "yes"] {
        env.set("VIBE_UNATTENDED", raw);
        assert!(
            resolve_unattended(false),
            "VIBE_UNATTENDED={raw:?} must resolve to true"
        );
    }
}

#[test]
fn unattended_env_falsy_values_or_empty_or_unset() {
    let mut env = EnvGuard::lock();
    for raw in ["", "0", "false", "no", "off", "garbage", "  "] {
        env.set("VIBE_UNATTENDED", raw);
        assert!(
            !resolve_unattended(false),
            "VIBE_UNATTENDED={raw:?} must resolve to false"
        );
    }
}

#[test]
fn unattended_cli_flag_overrides_falsy_env() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_UNATTENDED", "0");
    // Flag is true, env is falsy → resolved is true (flag wins by OR).
    assert!(resolve_unattended(true));
}

// --- resolve_offline (PROP-010 §2.5: flag > VIBE_OFFLINE > [net].offline) ----

#[test]
fn offline_default_false_with_no_flag_no_env_no_config() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_OFFLINE");
    assert!(!resolve_offline(false, false));
}

/// Flag over env: the flag rung resolves true even when the env-var
/// is explicitly falsy.
#[test]
fn offline_cli_flag_wins_over_falsy_env() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_OFFLINE", "0");
    assert!(resolve_offline(true, false));
}

/// Env over config: a truthy `VIBE_OFFLINE` carries the posture with
/// no flag and no config.
#[test]
fn offline_env_wins_with_no_flag_no_config() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_OFFLINE", "1");
    assert!(resolve_offline(false, false));
}

/// A falsy env-var does not shadow the config rung — it is absent,
/// not "no": `[net].offline = true` still resolves offline.
#[test]
fn offline_config_applies_when_env_is_falsy() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_OFFLINE", "0");
    assert!(resolve_offline(false, true));
}

#[test]
fn offline_env_truthy_values() {
    let mut env = EnvGuard::lock();
    for raw in ["1", "true", "TRUE", " yes ", "On", "yes"] {
        env.set("VIBE_OFFLINE", raw);
        assert!(
            resolve_offline(false, false),
            "VIBE_OFFLINE={raw:?} must resolve to true"
        );
    }
}

#[test]
fn offline_env_falsy_values_or_empty_or_unset() {
    let mut env = EnvGuard::lock();
    for raw in ["", "0", "false", "no", "off", "garbage", "  "] {
        env.set("VIBE_OFFLINE", raw);
        assert!(
            !resolve_offline(false, false),
            "VIBE_OFFLINE={raw:?} must resolve to false"
        );
    }
    env.unset("VIBE_OFFLINE");
    assert!(!resolve_offline(false, false));
}

#[test]
fn render_json_stamps_unattended_when_true() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_INVOKED_BY");
    env.unset("VIBE_UNATTENDED");
    let ctx = Context::from_flags(false, true, None, true, crate::cli::AgentModeArg::Auto);
    let payload = serde_json::json!({ "ok": true, "command": "demo" });
    let rendered = ctx.render_json(&payload).unwrap();
    let parsed: Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(parsed["unattended"], true);
    assert_eq!(parsed["ok"], true);
}

#[test]
fn render_json_omits_unattended_when_false() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_INVOKED_BY");
    env.unset("VIBE_UNATTENDED");
    let ctx = Context::from_flags(false, true, None, false, crate::cli::AgentModeArg::Auto);
    let payload = serde_json::json!({ "ok": true });
    let rendered = ctx.render_json(&payload).unwrap();
    let parsed: Value = serde_json::from_str(&rendered).unwrap();
    assert!(parsed.get("unattended").is_none());
}

#[test]
fn render_json_preserves_caller_supplied_invoked_by() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_INVOKED_BY");
    let ctx = Context::from_flags(
        false,
        true,
        Some("opencode"),
        false,
        crate::cli::AgentModeArg::Auto,
    );
    let payload = serde_json::json!({
        "ok": true,
        "invoked_by": "explicit-override"
    });
    let rendered = ctx.render_json(&payload).unwrap();
    let parsed: Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(parsed["invoked_by"], "explicit-override");
}

/// `--agent-mode` resolves exactly once per invocation: an explicit value
/// wins outright, and `auto` infers from the ALREADY-resolved invoked-by
/// value rather than re-deriving that ladder. The context is what every
/// lifecycle metadata constructor reads, so both layers are pinned.
#[test]
fn agent_mode_resolves_explicit_over_auto_and_auto_over_nothing() {
    for (flag, invoked_by, expected) in [
        (crate::cli::AgentModeArg::Auto, None, RunAgentMode::Cli),
        (
            crate::cli::AgentModeArg::Auto,
            Some("claude-code"),
            RunAgentMode::Agent,
        ),
        (AgentModeArg::Cli, None, RunAgentMode::Cli),
        // An explicit mode is never overridden by inference, in EITHER
        // direction: this is the pair a "helpful" auto-override would break.
        (AgentModeArg::Cli, Some("claude-code"), RunAgentMode::Cli),
        (AgentModeArg::Agent, None, RunAgentMode::Agent),
        (
            AgentModeArg::Agent,
            Some("claude-code"),
            RunAgentMode::Agent,
        ),
    ] {
        assert_eq!(
            resolve_agent_mode(flag, invoked_by),
            expected,
            "{flag:?} with invoked_by {invoked_by:?}",
        );
    }
}

/// The env rung reaches the mode through the same resolved invoked-by value
/// the JSON stamp uses — one ladder, not two.
#[test]
fn auto_reads_the_env_rung_of_invoked_by_through_the_context() {
    let mut env = EnvGuard::lock();
    env.set("VIBE_INVOKED_BY", "opencode");
    let ctx = Context::from_flags(false, true, None, false, crate::cli::AgentModeArg::Auto);
    assert_eq!(ctx.agent_mode(), RunAgentMode::Agent);
    assert_eq!(ctx.invoked_by(), Some("opencode"));

    let ctx = Context::from_flags(false, true, None, false, AgentModeArg::Cli);
    assert_eq!(
        ctx.agent_mode(),
        RunAgentMode::Cli,
        "an explicit flag still wins over the env-inferred host",
    );

    env.unset("VIBE_INVOKED_BY");
    let ctx = Context::from_flags(false, true, None, false, crate::cli::AgentModeArg::Auto);
    assert_eq!(ctx.agent_mode(), RunAgentMode::Cli);
}

/// A quiet child keeps the invocation's posture: suppressing narration must
/// not silently move a run from the hosted branch to the paid one.
#[test]
fn a_quiet_child_inherits_the_resolved_agent_mode() {
    let ctx = Context::from_flags(
        false,
        false,
        Some("codex"),
        false,
        crate::cli::AgentModeArg::Auto,
    );
    assert_eq!(ctx.agent_mode(), RunAgentMode::Agent);
    assert_eq!(ctx.quiet_child().agent_mode(), RunAgentMode::Agent);
}
