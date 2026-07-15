//! The headless AIUI surface (PROP-039 §11.3): a thin, render-free API an AI
//! (or any programmatic caller) uses to **drive and observe** actions over a
//! [`Registry`].
//!
//! [`list_actions`] projects the registry into serialisable [`ActionView`]
//! snapshots — each carrying the action's English name/description, its live
//! enablement verdict (`enabled` + the "why disabled" `reason`) over a [`Ctx`],
//! and its parameter names — skipping actions the context hides. [`invoke`]
//! parses a textual address and delegates to the one core
//! [`crate::invoke::invoke`] pipeline (§7.1), so an AIUI call is parameter- and
//! capability-checked exactly as a key press is.
//!
//! Because enablement is pure and introspectable and invocation is
//! address-based, this surface is a thin adapter carrying no rendering types
//! (§1 `#no-render-dep`, §11.2); a future in-process / JSON-RPC / MCP binding
//! realises it (§11.3, DO18).
//!
//! Spec: [PROP-039 §11.3](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#aiui).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#aiui");

use crate::address::ActionAddr;
use crate::context::Ctx;
use crate::invoke::{CancellationToken, GrantedScope, InvokeError, InvokeResult};
use crate::params::ParamValues;
use crate::registry::Registry;

/// The observable snapshot of one action for a headless caller (PROP-039
/// §11.3, §11.2): a pure projection carrying no rendering types, so an AI reads
/// structured state rather than pixels.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ActionView {
    /// The action's address, in textual form.
    pub address: String,
    /// The resolved English presentation name (the inline `default_en`).
    pub name: String,
    /// The resolved English presentation description (the inline `default_en`).
    pub description: String,
    /// Whether the action can currently be invoked over the observed context.
    pub enabled: bool,
    /// The localized reason the action is disabled, if it is.
    pub reason: Option<String>,
    /// The names of the action's declared parameters, in declaration order.
    pub params: Vec<String>,
}

/// Enumerate the registry as observable [`ActionView`]s over `ctx` (PROP-039
/// §11.3). For each action, the English name/description (the inline
/// `default_en`) is read, its enablement predicate is evaluated over `ctx`
/// (yielding `enabled` and the optional `reason`), and its parameter names are
/// listed. Actions whose enablement makes them **not visible** are skipped —
/// the same hide axis a visual surface honours (§6.2).
pub fn list_actions(reg: &Registry, ctx: &Ctx) -> Vec<ActionView> {
    reg.iter()
        .filter_map(|action| {
            let enablement = action.evaluate(ctx);
            if !enablement.visible {
                return None;
            }
            let params = action
                .params()
                .params()
                .iter()
                .map(|spec| spec.name().to_owned())
                .collect();
            let presentation = action.presentation();
            Some(ActionView {
                address: action.addr().to_string(),
                name: presentation.name().default_en().to_owned(),
                description: presentation.description().default_en().to_owned(),
                enabled: enablement.enabled,
                reason: enablement.reason.map(|r| r.as_str().to_owned()),
                params,
            })
        })
        .collect()
}

/// Invoke the action named by the textual `addr` over the core pipeline
/// (PROP-039 §11.3 → §7.1). The address is parsed first — a malformed address
/// yields [`InvokeError::UnknownAction`], as there is no such action to reach —
/// then [`crate::invoke::invoke`] validates `params`, checks the action's
/// capability against `scope`, threads `token`, and awaits the action body. The
/// return is the same typed [`InvokeResult`] every caller shares.
pub async fn invoke(
    reg: &Registry,
    addr: &str,
    params: ParamValues,
    ctx: &Ctx,
    scope: GrantedScope,
    token: &CancellationToken,
) -> InvokeResult {
    let parsed = match ActionAddr::parse(addr) {
        Ok(parsed) => parsed,
        Err(_) => {
            return Err(InvokeError::UnknownAction {
                addr: addr.to_owned(),
            });
        }
    };
    crate::invoke::invoke(reg, &parsed, params, ctx, scope, token).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{Action, Capability};
    use crate::context::Enablement;
    use crate::invoke::InvokeOutcome;
    use crate::params::{ParamSchema, ParamSpec, ParamType, ParamValue};

    /// A typed context marker driving `paste`'s enablement.
    struct Selection(bool);

    fn addr(s: &str) -> ActionAddr {
        ActionAddr::parse(s).unwrap()
    }

    /// A registry with two actions:
    /// - `echo` — always enabled, `Safe`, one required `text` param, echoes it;
    /// - `paste` — disabled (but visible) unless a `Selection(true)` is present.
    fn registry() -> Registry {
        let mut reg = Registry::new();

        let echo = Action::builder(addr("action://vibe.tree/echo"))
            .name_en("Echo")
            .description_en("Echo the text parameter back")
            .params(ParamSchema::empty().with(ParamSpec::required("text", ParamType::String)))
            .invoke(|_ctx, values| {
                let out = match values.get("text") {
                    Some(ParamValue::String(s)) => s.clone(),
                    _ => String::new(),
                };
                Box::pin(async move { Ok(InvokeOutcome::Value(out)) })
            })
            .build()
            .unwrap();
        reg.register(echo).unwrap();

        let paste = Action::builder(addr("action://vibe.tree/paste"))
            .name_en("Paste")
            .description_en("Paste over the current selection")
            .enablement(|ctx: &Ctx| match ctx.get::<Selection>() {
                Some(Selection(true)) => Enablement::enabled(),
                _ => Enablement::disabled("no selection"),
            })
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap();
        reg.register(paste).unwrap();

        reg
    }

    #[test]
    fn list_actions_projects_enablement_and_params() {
        let reg = registry();
        let ctx = Ctx::new(); // no Selection → paste is disabled (but visible)
        let views = list_actions(&reg, &ctx);
        assert_eq!(views.len(), 2);

        // BTreeMap address order: echo precedes paste.
        let echo = &views[0];
        assert_eq!(echo.address, "action://vibe.tree/echo");
        assert_eq!(echo.name, "Echo");
        assert_eq!(echo.description, "Echo the text parameter back");
        assert!(echo.enabled);
        assert_eq!(echo.reason, None);
        assert_eq!(echo.params, vec!["text".to_owned()]);

        let paste = &views[1];
        assert_eq!(paste.address, "action://vibe.tree/paste");
        assert!(!paste.enabled);
        assert_eq!(paste.reason.as_deref(), Some("no selection"));
        assert!(paste.params.is_empty());
    }

    #[test]
    fn list_actions_reflects_an_enabling_context() {
        let reg = registry();
        let ctx = Ctx::new().with(Selection(true));
        let paste = list_actions(&reg, &ctx)
            .into_iter()
            .find(|v| v.address == "action://vibe.tree/paste")
            .expect("paste is listed");
        assert!(paste.enabled);
        assert_eq!(paste.reason, None);
    }

    #[test]
    fn list_actions_skips_invisible_actions() {
        let mut reg = Registry::new();
        let hidden = Action::builder(addr("action://vibe.tree/secret"))
            .name_en("Secret")
            .description_en("Hidden unless a flag is set")
            .enablement(|_c| Enablement::hidden())
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap();
        reg.register(hidden).unwrap();
        assert!(list_actions(&reg, &Ctx::new()).is_empty());
    }

    #[test]
    fn action_view_serialises() {
        let reg = registry();
        let views = list_actions(&reg, &Ctx::new());
        let json = serde_json::to_string(&views).unwrap();
        assert!(json.contains("\"address\":\"action://vibe.tree/echo\""));
        assert!(json.contains("\"enabled\":true"));
    }

    #[tokio::test]
    async fn invoke_runs_an_enabled_action_by_address() {
        let reg = registry();
        let out = invoke(
            &reg,
            "action://vibe.tree/echo",
            ParamValues::new().with("text", "hi"),
            &Ctx::new(),
            GrantedScope::all(),
            &CancellationToken::new(),
        )
        .await;
        assert_eq!(out, Ok(InvokeOutcome::Value("hi".to_owned())));
    }

    #[tokio::test]
    async fn invoke_unknown_address_is_the_unknown_action_error() {
        let reg = registry();
        // Well-formed but unregistered — the core reports it unknown.
        let out = invoke(
            &reg,
            "action://vibe.tree/missing",
            ParamValues::new(),
            &Ctx::new(),
            GrantedScope::all(),
            &CancellationToken::new(),
        )
        .await;
        assert!(matches!(out, Err(InvokeError::UnknownAction { .. })));
    }

    #[tokio::test]
    async fn invoke_malformed_address_is_the_unknown_action_error() {
        let reg = registry();
        let out = invoke(
            &reg,
            "not an address",
            ParamValues::new(),
            &Ctx::new(),
            GrantedScope::all(),
            &CancellationToken::new(),
        )
        .await;
        match out {
            Err(InvokeError::UnknownAction { addr }) => assert_eq!(addr, "not an address"),
            other => panic!("expected UnknownAction, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn invoke_honours_the_core_capability_gate() {
        // A dangerous action is refused under a safe-only scope — the AIUI
        // inherits the core's capability check verbatim (§7.2).
        let mut reg = Registry::new();
        let wipe = Action::builder(addr("action://vibe.tree/wipe"))
            .name_en("Wipe")
            .description_en("A dangerous, irreversible action")
            .capability(Capability::Dangerous)
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap();
        reg.register(wipe).unwrap();

        let out = invoke(
            &reg,
            "action://vibe.tree/wipe",
            ParamValues::new(),
            &Ctx::new(),
            GrantedScope::safe_only(),
            &CancellationToken::new(),
        )
        .await;
        assert!(matches!(out, Err(InvokeError::CapabilityRefused { .. })));
    }
}
