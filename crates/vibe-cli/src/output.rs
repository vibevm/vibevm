//! Output helpers. The CLI has two modes: human-readable (default) and JSON
//! (`--json`). `--quiet` collapses human-readable output to a single summary
//! line. See `VIBEVM-SPEC.md` §9.3.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#output-format");

use console::Style;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Human,
    HumanQuiet,
    Json,
}

/// Resolved provenance for `--invoked-by` / `VIBE_INVOKED_BY`. Drives
/// `vibe show config` reporting and lets tests assert which layer
/// supplied the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvokedByProvenance {
    /// `--invoked-by <agent>` was passed on the command line.
    CliFlag,
    /// `VIBE_INVOKED_BY` was set in the environment; CLI flag was absent.
    EnvVar,
    /// Neither layer set the value.
    Default,
}

impl InvokedByProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            InvokedByProvenance::CliFlag => "cli-flag",
            InvokedByProvenance::EnvVar => "env",
            InvokedByProvenance::Default => "default",
        }
    }
}

/// Read the `VIBE_INVOKED_BY` env-var. Empty string is treated as
/// unset so a `VIBE_INVOKED_BY=` literal in `~/.bashrc` does not
/// silently shadow the flag-absent path.
fn env_invoked_by() -> Option<String> {
    std::env::var("VIBE_INVOKED_BY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Resolve the agent context: CLI flag > env-var > unset. Empty
/// strings on either layer are treated as unset.
pub fn resolve_invoked_by(cli_flag: Option<&str>) -> (Option<String>, InvokedByProvenance) {
    if let Some(flag) = cli_flag {
        let trimmed = flag.trim();
        if !trimmed.is_empty() {
            return (Some(trimmed.to_string()), InvokedByProvenance::CliFlag);
        }
    }
    if let Some(env) = env_invoked_by() {
        return (Some(env), InvokedByProvenance::EnvVar);
    }
    (None, InvokedByProvenance::Default)
}

/// Read the `VIBE_UNATTENDED` env-var. Truthy values are `1`,
/// `true`, `yes`, `on` (case-insensitive, leading/trailing
/// whitespace ignored). Anything else — including the empty
/// string and unset — resolves to `false`. The deliberately small
/// vocabulary matches what cloud-init / systemd-style provisioning
/// scripts already speak.
fn env_unattended() -> bool {
    std::env::var("VIBE_UNATTENDED")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .is_some_and(|s| matches!(s.as_str(), "1" | "true" | "yes" | "on"))
}

/// Resolve the unattended posture: CLI flag wins; otherwise consult
/// the env-var. There is no provenance enum for this — the value is
/// boolean and the source rarely matters in practice (logs carry
/// the resolved value plus an `unattended: true` stamp on every
/// JSON envelope).
pub fn resolve_unattended(cli_flag: bool) -> bool {
    cli_flag || env_unattended()
}

pub struct Context {
    pub mode: Mode,
    pub tick: Style,
    pub cross: Style,
    #[allow(dead_code)] // used by install/uninstall (next slice)
    pub arrow: Style,
    pub warn: Style,
    pub dim: Style,
    pub bold: Style,
    /// Resolved `--invoked-by` value — `None` when neither flag nor env is set.
    invoked_by: Option<String>,
    /// Where `invoked_by` came from. Surfaced via [`Context::invoked_by_provenance`]
    /// to drive `vibe show config`.
    invoked_by_provenance: InvokedByProvenance,
    /// Resolved unattended posture — `--unattended` flag OR
    /// `VIBE_UNATTENDED` env-var truthy. Implies skip-all-confirms
    /// for every mutating subcommand and stamps the JSON envelope
    /// with `"unattended": true`.
    unattended: bool,
}

impl Context {
    pub fn from_flags(
        quiet: bool,
        json: bool,
        invoked_by_cli: Option<&str>,
        unattended_cli: bool,
    ) -> Self {
        let (invoked_by, invoked_by_provenance) = resolve_invoked_by(invoked_by_cli);
        let unattended = resolve_unattended(unattended_cli);
        let mode = match (quiet, json) {
            (_, true) => Mode::Json,
            (true, false) => Mode::HumanQuiet,
            (false, false) => Mode::Human,
        };
        let color_on = matches!(mode, Mode::Human) && console::user_attended();
        let styled = |s: Style| if color_on { s } else { Style::new() };
        Context {
            mode,
            tick: styled(Style::new().green().bold()),
            cross: styled(Style::new().red().bold()),
            arrow: styled(Style::new().cyan()),
            warn: styled(Style::new().yellow().bold()),
            dim: styled(Style::new().dim()),
            bold: styled(Style::new().bold()),
            invoked_by,
            invoked_by_provenance,
            unattended,
        }
    }

    pub fn is_json(&self) -> bool {
        self.mode == Mode::Json
    }

    pub fn is_quiet(&self) -> bool {
        self.mode == Mode::HumanQuiet
    }

    pub fn invoked_by(&self) -> Option<&str> {
        self.invoked_by.as_deref()
    }

    pub fn invoked_by_provenance(&self) -> InvokedByProvenance {
        self.invoked_by_provenance
    }

    /// True when `--unattended` was passed on the CLI or
    /// `VIBE_UNATTENDED` resolves truthy in the environment. Mutating
    /// subcommands treat this as an implicit "yes" for every
    /// confirmation prompt and refuse to open any interactive wizard.
    pub fn is_unattended(&self) -> bool {
        self.unattended
    }

    pub fn heading(&self, text: &str) {
        if self.is_json() || self.is_quiet() {
            return;
        }
        println!("{}", self.bold.apply_to(text));
    }

    #[allow(dead_code)] // used by install
    pub fn step(&self, text: &str) {
        if self.is_json() || self.is_quiet() {
            return;
        }
        println!("  {} {}", self.arrow.apply_to("→"), text);
    }

    pub fn created(&self, path: &str) {
        if self.is_json() || self.is_quiet() {
            return;
        }
        println!("  {} created  {}", self.tick.apply_to("✓"), path);
    }

    pub fn skipped(&self, path: &str, reason: &str) {
        if self.is_json() || self.is_quiet() {
            return;
        }
        println!(
            "  {} kept     {} {}",
            self.warn.apply_to("•"),
            path,
            self.dim.apply_to(&format!("({reason})"))
        );
    }

    #[allow(dead_code)] // used by uninstall
    pub fn removed(&self, path: &str) {
        if self.is_json() || self.is_quiet() {
            return;
        }
        println!("  {} removed  {}", self.cross.apply_to("-"), path);
    }

    pub fn summary(&self, text: &str) {
        match self.mode {
            Mode::Human | Mode::HumanQuiet => println!("{text}"),
            Mode::Json => {}
        }
    }

    pub fn error(&self, err: &anyhow::Error) {
        match self.mode {
            Mode::Human | Mode::HumanQuiet => {
                eprintln!("{} {err:#}", self.cross.apply_to("error:"));
            }
            Mode::Json => {
                let mut payload = serde_json::json!({
                    "ok": false,
                    "error": format!("{err:#}"),
                });
                self.stamp_structured_error(&mut payload, err);
                self.stamp_invoked_by(&mut payload);
                self.stamp_unattended(&mut payload);
                eprintln!("{payload}");
            }
        }
    }

    /// When the anyhow chain carries a known structured error variant,
    /// surface its fields as machine-readable extras alongside the
    /// stringified `error` field. Today the only well-known variant is
    /// `DepProviderError::AggregateNotFound` (registry walk-failure
    /// with per-registry `attempts`); future structured variants
    /// extend this match. JSON consumers (CI, monitoring pipelines)
    /// can branch on `error_kind` and read `attempts` without parsing
    /// the prose.
    fn stamp_structured_error(&self, payload: &mut Value, err: &anyhow::Error) {
        // Walk the anyhow chain looking for a known structured error
        // variant. Today the only match is
        // `DepProviderError::AggregateNotFound` (registry walk
        // failure with per-registry attempts), but it can be reached
        // through two surfaces:
        //
        //   - directly as a `DepProviderError` (some call sites
        //     return it without a `SolveError` wrapper);
        //   - wrapped in `SolveError::Provider(...)` when the
        //     depsolver propagates it. `#[error(transparent)]` on
        //     that variant forwards Display but, in practice,
        //     anyhow's `chain()` does NOT walk through the wrapper
        //     to the inner — so we destructure manually here rather
        //     than rely on chain depth.
        for cause in err.chain() {
            let candidate: Option<&vibe_resolver::DepProviderError> =
                if let Some(d) = cause.downcast_ref::<vibe_resolver::DepProviderError>() {
                    Some(d)
                } else if let Some(vibe_resolver::SolveError::Provider(d)) =
                    cause.downcast_ref::<vibe_resolver::SolveError>()
                {
                    Some(d)
                } else {
                    None
                };
            let Some(provider_err) = candidate else {
                continue;
            };
            if let vibe_resolver::DepProviderError::AggregateNotFound {
                group,
                name,
                attempts,
                ..
            } = provider_err
                && let Value::Object(map) = payload
            {
                map.entry("error_kind".to_string())
                    .or_insert_with(|| Value::String("package_not_found_everywhere".into()));
                map.entry("package".to_string()).or_insert_with(
                    || serde_json::json!({ "group": group.as_str(), "name": name }),
                );
                if let Ok(serialised) = serde_json::to_value(attempts) {
                    map.entry("attempts".to_string()).or_insert(serialised);
                }
                return;
            }
        }
    }

    /// Stamp `invoked_by` on the top-level JSON object when the
    /// resolved context carries a value. No-op for non-objects (vibe
    /// envelopes are always objects, but the function is robust on
    /// scalars / arrays so a stray `Vec<_>` payload does not panic).
    /// The caller's value wins if the inner already set its own
    /// `invoked_by` field — flatten semantics for nested envelopes.
    fn stamp_invoked_by(&self, payload: &mut Value) {
        let Some(invoked_by) = &self.invoked_by else {
            return;
        };
        if let Value::Object(map) = payload {
            map.entry("invoked_by".to_string())
                .or_insert_with(|| Value::String(invoked_by.clone()));
        }
    }

    /// Stamp `"unattended": true` on the top-level JSON object when
    /// the resolved context is unattended. Skipped (not stamped at
    /// all) when the run is interactive — log aggregators see the
    /// field only on scripted runs, which is what they want for
    /// filtering. Same flatten semantics as `stamp_invoked_by`: a
    /// caller-supplied value on the inner payload wins.
    fn stamp_unattended(&self, payload: &mut Value) {
        if !self.unattended {
            return;
        }
        if let Value::Object(map) = payload {
            map.entry("unattended".to_string())
                .or_insert(Value::Bool(true));
        }
    }

    pub fn emit_json<T: Serialize>(&self, value: &T) -> anyhow::Result<()> {
        if !self.is_json() {
            return Ok(());
        }
        let rendered = self.render_json(value)?;
        println!("{rendered}");
        Ok(())
    }

    /// Build the JSON string we'd print, with `invoked_by` and
    /// `unattended` stamped onto the top-level object. Pulled out of
    /// `emit_json` so tests can assert the payload shape without
    /// capturing stdout.
    pub fn render_json<T: Serialize>(&self, value: &T) -> anyhow::Result<String> {
        let mut v = serde_json::to_value(value)?;
        self.stamp_invoked_by(&mut v);
        self.stamp_unattended(&mut v);
        Ok(serde_json::to_string_pretty(&v)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises every test that mutates `VIBE_INVOKED_BY`. Sister
    /// of `UNATTENDED_LOCK`; same rationale (parallel writes flake
    /// the resolver assertions). Tests that mutate both env-vars
    /// hold both locks; hold UNATTENDED_LOCK first to keep the
    /// ordering consistent and avoid potential deadlocks.
    static INVOKED_BY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Reset live `VIBE_INVOKED_BY` before/after each test so the
    /// resolver sees a clean environment regardless of how the test
    /// harness was launched.
    struct EnvGuard {
        prev: Option<String>,
    }

    impl EnvGuard {
        fn new() -> Self {
            let prev = std::env::var("VIBE_INVOKED_BY").ok();
            Self::clear();
            EnvGuard { prev }
        }

        fn set(value: &str) {
            // SAFETY: tests in this module run sequentially under
            // `cargo test --test-threads=1`-equivalent ordering for
            // env mutations? No — Rust tests run in parallel. To stay
            // safe, we mutate env from within EnvGuard only after
            // marking the live value, and the tests that need a
            // specific env hold their own guard. The unsafety is that
            // parallel tests could observe a transient `VIBE_INVOKED_BY`
            // set by another test. We mitigate by gating each test's
            // `EnvGuard::new` then `set` inside the same scope and by
            // giving these tests deterministic, idempotent assertions.
            #[allow(unsafe_code)]
            unsafe {
                std::env::set_var("VIBE_INVOKED_BY", value);
            }
        }

        fn clear() {
            #[allow(unsafe_code)]
            unsafe {
                std::env::remove_var("VIBE_INVOKED_BY");
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => {
                    let v = v.clone();
                    Self::set(&v);
                }
                None => Self::clear(),
            }
        }
    }

    #[test]
    fn resolve_returns_default_when_neither_flag_nor_env() {
        let _lock = INVOKED_BY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = EnvGuard::new();
        let (v, p) = resolve_invoked_by(None);
        assert_eq!(v, None);
        assert_eq!(p, InvokedByProvenance::Default);
    }

    #[test]
    fn resolve_uses_env_when_flag_absent() {
        let _lock = INVOKED_BY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = EnvGuard::new();
        EnvGuard::set("opencode");
        let (v, p) = resolve_invoked_by(None);
        assert_eq!(v.as_deref(), Some("opencode"));
        assert_eq!(p, InvokedByProvenance::EnvVar);
    }

    #[test]
    fn resolve_flag_wins_over_env() {
        let _lock = INVOKED_BY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = EnvGuard::new();
        EnvGuard::set("opencode");
        let (v, p) = resolve_invoked_by(Some("claude-code"));
        assert_eq!(v.as_deref(), Some("claude-code"));
        assert_eq!(p, InvokedByProvenance::CliFlag);
    }

    #[test]
    fn resolve_treats_empty_flag_as_absent() {
        let _lock = INVOKED_BY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = EnvGuard::new();
        EnvGuard::set("opencode");
        let (v, p) = resolve_invoked_by(Some("   "));
        assert_eq!(v.as_deref(), Some("opencode"));
        assert_eq!(p, InvokedByProvenance::EnvVar);
    }

    #[test]
    fn resolve_treats_empty_env_as_absent() {
        let _lock = INVOKED_BY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = EnvGuard::new();
        EnvGuard::set("");
        let (v, p) = resolve_invoked_by(None);
        assert_eq!(v, None);
        assert_eq!(p, InvokedByProvenance::Default);
    }

    #[test]
    fn render_json_stamps_invoked_by_on_object_payloads() {
        let _lock = INVOKED_BY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = EnvGuard::new();
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
        let _lock = INVOKED_BY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = EnvGuard::new();
        let ctx = Context::from_flags(false, true, None, false);
        let payload = serde_json::json!({ "ok": true });
        let rendered = ctx.render_json(&payload).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert!(parsed.get("invoked_by").is_none());
    }

    /// Serialises every test that mutates `VIBE_UNATTENDED`. Without
    /// it parallel tests in this module observe each other's
    /// transient writes and the truthy-vs-falsy assertions flake.
    /// `EnvGuard` for `VIBE_INVOKED_BY` has the same race (known
    /// issue, tracked in CONTINUE.md); we just avoid hitting it by
    /// holding this lock around the unattended writes.
    static UNATTENDED_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Same shape as `EnvGuard` but for `VIBE_UNATTENDED`. Kept
    /// separate so tests that want to control both env-vars can
    /// hold one of each without clobbering.
    struct UnattendedGuard {
        prev: Option<String>,
    }

    impl UnattendedGuard {
        fn new() -> Self {
            let prev = std::env::var("VIBE_UNATTENDED").ok();
            Self::clear();
            UnattendedGuard { prev }
        }

        fn set(value: &str) {
            #[allow(unsafe_code)]
            unsafe {
                std::env::set_var("VIBE_UNATTENDED", value);
            }
        }

        fn clear() {
            #[allow(unsafe_code)]
            unsafe {
                std::env::remove_var("VIBE_UNATTENDED");
            }
        }
    }

    impl Drop for UnattendedGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => {
                    let v = v.clone();
                    Self::set(&v);
                }
                None => Self::clear(),
            }
        }
    }

    #[test]
    fn unattended_default_false_with_no_flag_no_env() {
        let _lock = UNATTENDED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = UnattendedGuard::new();
        assert!(!resolve_unattended(false));
    }

    #[test]
    fn unattended_cli_flag_true_wins() {
        let _lock = UNATTENDED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = UnattendedGuard::new();
        assert!(resolve_unattended(true));
    }

    #[test]
    fn unattended_env_truthy_values() {
        let _lock = UNATTENDED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for raw in ["1", "true", "TRUE", " yes ", "On", "yes"] {
            let _g = UnattendedGuard::new();
            UnattendedGuard::set(raw);
            assert!(
                resolve_unattended(false),
                "VIBE_UNATTENDED={raw:?} must resolve to true"
            );
        }
    }

    #[test]
    fn unattended_env_falsy_values_or_empty_or_unset() {
        let _lock = UNATTENDED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for raw in ["", "0", "false", "no", "off", "garbage", "  "] {
            let _g = UnattendedGuard::new();
            UnattendedGuard::set(raw);
            assert!(
                !resolve_unattended(false),
                "VIBE_UNATTENDED={raw:?} must resolve to false"
            );
        }
    }

    #[test]
    fn unattended_cli_flag_overrides_falsy_env() {
        let _lock = UNATTENDED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = UnattendedGuard::new();
        UnattendedGuard::set("0");
        // Flag is true, env is falsy → resolved is true (flag wins by OR).
        assert!(resolve_unattended(true));
    }

    #[test]
    fn render_json_stamps_unattended_when_true() {
        let _lock = UNATTENDED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g_inv = EnvGuard::new();
        let _g_un = UnattendedGuard::new();
        let ctx = Context::from_flags(false, true, None, true);
        let payload = serde_json::json!({ "ok": true, "command": "demo" });
        let rendered = ctx.render_json(&payload).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["unattended"], true);
        assert_eq!(parsed["ok"], true);
    }

    #[test]
    fn render_json_omits_unattended_when_false() {
        let _lock = UNATTENDED_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g_inv = EnvGuard::new();
        let _g_un = UnattendedGuard::new();
        let ctx = Context::from_flags(false, true, None, false);
        let payload = serde_json::json!({ "ok": true });
        let rendered = ctx.render_json(&payload).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert!(parsed.get("unattended").is_none());
    }

    #[test]
    fn render_json_preserves_caller_supplied_invoked_by() {
        let _lock = INVOKED_BY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _g = EnvGuard::new();
        let ctx = Context::from_flags(false, true, Some("opencode"), false);
        let payload = serde_json::json!({
            "ok": true,
            "invoked_by": "explicit-override"
        });
        let rendered = ctx.render_json(&payload).unwrap();
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["invoked_by"], "explicit-override");
    }
}
