//! Invocation — the primary interface an action runs through (PROP-039 §7).
//!
//! [`invoke`] is *the* way an action runs: a key press, a menu click, a Search
//! Everywhere selection, and an AIUI call are all thin callers of it. It looks
//! the action up (unknown → typed error), validates parameters (§5.2), checks
//! the declared [`Capability`] against the caller's [`GrantedScope`] (§7.2),
//! threads a [`CancellationToken`], then awaits the action's own async body.
//! The result is a first-class typed [`InvokeResult`].
//!
//! Spec: [PROP-039 §7](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#invocation).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#invocation");

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use thiserror::Error;

use crate::action::Capability;
use crate::address::ActionAddr;
use crate::context::Ctx;
use crate::params::{self, ParamError, ParamValues};
use crate::registry::Registry;

/// A boxed, `Send`, owned future — the shape an action's async body returns.
///
/// Locally defined (rather than pulling the `futures` crate) so `vibe-actions`
/// keeps a minimal dependency graph; the definition matches
/// `futures::future::BoxFuture`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What a successful invocation produced (PROP-039 §7.1). Deliberately small
/// and `#[non_exhaustive]` so richer typed payloads can be added without a
/// breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvokeOutcome {
    /// The action performed its effect and produced no value.
    Done,
    /// The action produced a textual value (e.g. content copied to a buffer).
    Value(String),
}

/// The typed result of an invocation (PROP-039 §7.1).
pub type InvokeResult = Result<InvokeOutcome, InvokeError>;

/// Why an invocation failed (PROP-039 §7). The action body may also return
/// [`InvokeError::Failed`] for its own domain failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-actions/PROP-039#invoke")]
pub enum InvokeError {
    /// No action is registered at (or aliased to) the address.
    #[error(
        "no action registered at `{addr}` \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#invoke; \
          fix: register the action or correct the address)"
    )]
    UnknownAction {
        /// The unresolved address.
        addr: String,
    },

    /// The supplied parameters failed schema validation (§5.2).
    #[error(transparent)]
    InvalidParams(#[from] ParamError),

    /// The action's capability exceeds the caller's granted scope (§7.2).
    #[error(
        "action `{addr}` requires capability `{required}` but the caller was granted \
         only up to `{granted}` \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#capabilities; \
          fix: grant a wider scope or invoke a lower-capability action)"
    )]
    CapabilityRefused {
        /// The refused action's address.
        addr: String,
        /// The capability the action declares.
        required: Capability,
        /// The ceiling the caller was granted.
        granted: Capability,
    },

    /// The invocation was cancelled before the action body completed.
    #[error(
        "invocation of `{addr}` was cancelled \
         (spec://vibevm/modules/vibe-actions/PROP-039#invoke)"
    )]
    Cancelled {
        /// The cancelled action's address.
        addr: String,
    },

    /// The action body reported a domain failure.
    #[error(
        "action `{addr}` failed: {message} \
         (spec://vibevm/modules/vibe-actions/PROP-039#invoke)"
    )]
    Failed {
        /// The failing action's address.
        addr: String,
        /// The body's failure message.
        message: String,
    },
}

/// The capability ceiling a caller was granted (PROP-039 §7.2). Inert for the
/// trusted local TUI (`all`), but the seam a networked surface or an AIUI uses
/// to refuse an out-of-scope action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantedScope {
    max: Capability,
}

impl GrantedScope {
    /// Grant everything up to and including [`Capability::Dangerous`] — the
    /// trusted local surface.
    pub const fn all() -> Self {
        GrantedScope {
            max: Capability::Dangerous,
        }
    }

    /// Grant only [`Capability::Safe`] actions.
    pub const fn safe_only() -> Self {
        GrantedScope {
            max: Capability::Safe,
        }
    }

    /// Grant everything up to and including `max`.
    pub const fn up_to(max: Capability) -> Self {
        GrantedScope { max }
    }

    /// The granted ceiling.
    pub const fn max(self) -> Capability {
        self.max
    }

    /// Whether an action needing `needed` is permitted.
    pub fn permits(self, needed: Capability) -> bool {
        needed <= self.max
    }
}

impl Default for GrantedScope {
    /// Least privilege — [`GrantedScope::safe_only`].
    fn default() -> Self {
        GrantedScope::safe_only()
    }
}

/// A minimal cooperative cancellation flag threaded through [`invoke`]
/// (PROP-039 §7.1). Cloneable and shared: cancelling one handle cancels all.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// A fresh, un-cancelled token.
    pub fn new() -> Self {
        CancellationToken::default()
    }

    /// Request cancellation. Idempotent.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Invoke the action at `addr` (PROP-039 §7.1).
///
/// The pipeline, in order: honour an already-cancelled token; look the action
/// up (unknown → [`InvokeError::UnknownAction`]); validate `values` against the
/// schema (§5.2); check the action's [`Capability`] against `granted` (§7.2);
/// re-check cancellation; then await the action's async body and return its
/// typed [`InvokeResult`].
pub async fn invoke(
    registry: &Registry,
    addr: &ActionAddr,
    values: ParamValues,
    ctx: &Ctx,
    granted: GrantedScope,
    cancel: &CancellationToken,
) -> InvokeResult {
    if cancel.is_cancelled() {
        return Err(InvokeError::Cancelled {
            addr: addr.to_string(),
        });
    }

    let action = registry
        .get(addr)
        .ok_or_else(|| InvokeError::UnknownAction {
            addr: addr.to_string(),
        })?;

    params::validate(action.params(), &values)?;

    let required = action.capability();
    if !granted.permits(required) {
        return Err(InvokeError::CapabilityRefused {
            addr: addr.to_string(),
            required,
            granted: granted.max(),
        });
    }

    if cancel.is_cancelled() {
        return Err(InvokeError::Cancelled {
            addr: addr.to_string(),
        });
    }

    action.call(ctx, values).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::context::Enablement;
    use crate::params::{ParamSchema, ParamSpec, ParamType};

    fn addr(s: &str) -> ActionAddr {
        ActionAddr::parse(s).unwrap()
    }

    /// A registry with one `Safe` action that echoes its `text` parameter and
    /// one `Dangerous` action, for the capability test.
    fn registry() -> Registry {
        let mut reg = Registry::new();

        let echo = Action::builder(addr("action://vibe.tree/echo"))
            .name_en("Echo")
            .description_en("Echo the text parameter back")
            .params(ParamSchema::empty().with(ParamSpec::required("text", ParamType::String)))
            .invoke(|_ctx, values| {
                let out = match values.get("text") {
                    Some(crate::params::ParamValue::String(s)) => s.clone(),
                    _ => String::new(),
                };
                Box::pin(async move { Ok(InvokeOutcome::Value(out)) })
            })
            .build()
            .unwrap();
        reg.register(echo).unwrap();

        let wipe = Action::builder(addr("action://vibe.tree/wipe"))
            .name_en("Wipe")
            .description_en("A dangerous, mutating action")
            .capability(Capability::Dangerous)
            .invoke(|_ctx, _values| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap();
        reg.register(wipe).unwrap();

        reg
    }

    #[tokio::test]
    async fn happy_path_returns_the_body_value() {
        let reg = registry();
        let values = ParamValues::new().with("text", "hello");
        let out = invoke(
            &reg,
            &addr("action://vibe.tree/echo"),
            values,
            &Ctx::new(),
            GrantedScope::all(),
            &CancellationToken::new(),
        )
        .await;
        assert_eq!(out, Ok(InvokeOutcome::Value("hello".to_owned())));
    }

    #[tokio::test]
    async fn unknown_address_is_a_typed_error() {
        let reg = registry();
        let out = invoke(
            &reg,
            &addr("action://vibe.tree/missing"),
            ParamValues::new(),
            &Ctx::new(),
            GrantedScope::all(),
            &CancellationToken::new(),
        )
        .await;
        assert!(matches!(out, Err(InvokeError::UnknownAction { .. })));
    }

    #[tokio::test]
    async fn invalid_params_are_rejected_before_the_body() {
        let reg = registry();
        // `text` is required and missing.
        let out = invoke(
            &reg,
            &addr("action://vibe.tree/echo"),
            ParamValues::new(),
            &Ctx::new(),
            GrantedScope::all(),
            &CancellationToken::new(),
        )
        .await;
        assert!(matches!(
            out,
            Err(InvokeError::InvalidParams(
                ParamError::MissingRequired { .. }
            ))
        ));
    }

    #[tokio::test]
    async fn capability_is_refused_when_scope_is_too_narrow() {
        let reg = registry();
        let out = invoke(
            &reg,
            &addr("action://vibe.tree/wipe"),
            ParamValues::new(),
            &Ctx::new(),
            GrantedScope::safe_only(),
            &CancellationToken::new(),
        )
        .await;
        match out {
            Err(InvokeError::CapabilityRefused {
                required, granted, ..
            }) => {
                assert_eq!(required, Capability::Dangerous);
                assert_eq!(granted, Capability::Safe);
            }
            other => panic!("expected CapabilityRefused, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_pre_cancelled_token_short_circuits() {
        let reg = registry();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let out = invoke(
            &reg,
            &addr("action://vibe.tree/echo"),
            ParamValues::new().with("text", "hi"),
            &Ctx::new(),
            GrantedScope::all(),
            &cancel,
        )
        .await;
        assert!(matches!(out, Err(InvokeError::Cancelled { .. })));
    }

    #[test]
    fn granted_scope_orders_capabilities() {
        assert!(GrantedScope::all().permits(Capability::Dangerous));
        assert!(GrantedScope::safe_only().permits(Capability::Safe));
        assert!(!GrantedScope::safe_only().permits(Capability::Mutating));
        assert!(GrantedScope::up_to(Capability::Mutating).permits(Capability::Safe));
        assert!(!GrantedScope::up_to(Capability::Mutating).permits(Capability::Dangerous));
    }

    #[test]
    fn enablement_predicate_over_ctx_is_pure() {
        // A sanity check that an action's enablement reads the typed context.
        struct HasSelection(bool);
        let action = Action::builder(addr("action://vibe.tree/copy"))
            .name_en("Copy")
            .description_en("Copy the selection")
            .enablement(|ctx: &Ctx| match ctx.get::<HasSelection>() {
                Some(HasSelection(true)) => Enablement::enabled(),
                _ => Enablement::disabled("no selection"),
            })
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap();

        let empty = Ctx::new();
        assert!(!action.evaluate(&empty).enabled);
        let with = Ctx::new().with(HasSelection(true));
        assert!(action.evaluate(&with).enabled);
    }
}
