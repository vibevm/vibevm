//! Unit tests for the output context. Split out of `output.rs` so the
//! production file stays inside the file-length budget. Env mutation
//! goes through `env_audit::EnvGuard` — the designated unsafe audit
//! crate (AUD-0016 posture): one guard per test serializes all
//! env-mutating tests process-wide and restores on drop.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#output-format");

use super::*;
use env_audit::EnvGuard;

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
    let ctx = Context::from_flags(false, true, Some("codex"), false);
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
    let ctx = Context::from_flags(false, true, None, false);
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

#[test]
fn render_json_stamps_unattended_when_true() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_INVOKED_BY");
    env.unset("VIBE_UNATTENDED");
    let ctx = Context::from_flags(false, true, None, true);
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
    let ctx = Context::from_flags(false, true, None, false);
    let payload = serde_json::json!({ "ok": true });
    let rendered = ctx.render_json(&payload).unwrap();
    let parsed: Value = serde_json::from_str(&rendered).unwrap();
    assert!(parsed.get("unattended").is_none());
}

#[test]
fn render_json_preserves_caller_supplied_invoked_by() {
    let mut env = EnvGuard::lock();
    env.unset("VIBE_INVOKED_BY");
    let ctx = Context::from_flags(false, true, Some("opencode"), false);
    let payload = serde_json::json!({
        "ok": true,
        "invoked_by": "explicit-override"
    });
    let rendered = ctx.render_json(&payload).unwrap();
    let parsed: Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(parsed["invoked_by"], "explicit-override");
}
