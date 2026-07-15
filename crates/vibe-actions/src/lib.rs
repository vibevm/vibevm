//! `vibe-actions` — the frontend-agnostic, addressable action system.
//!
//! The behaviour-layer twin of `spec://`: `spec://` addresses facts,
//! `action://` addresses **behaviour** (PROP-039). Everything a UI can *do* is
//! a first-class, addressable [`Action`] — an [`ActionAddr`], a mandatory
//! human-legible [`Presentation`], a typed [`ParamSchema`], a pure enablement
//! predicate over a typed [`Ctx`], an async `invoke`, a [`Capability`], and
//! [`SearchMeta`]. Actions live in a collision-erroring, enumerable
//! [`Registry`], are localized through an address-keyed [`Catalogue`], and run
//! through the single [`invoke`] entry point.
//!
//! # The frontend-agnostic invariant (§1 `#no-render-dep`)
//!
//! This crate has **zero rendering dependencies** — no `ratatui`, no
//! `crossterm`, no UI toolkit, and no dependency on any surface or consumer
//! crate. That invariant is what makes every surface (the TUI, a future web UI)
//! and the headless AIUI possible.
//!
//! # Scope
//!
//! This crate implements the **plumbing core**, PROP-039 §§1–8: addressing,
//! the Action value, the registry, parameters, context + enablement,
//! invocation, and i18n — plus the **Search Everywhere** engine (§10, the
//! [`search`] module: the provider trait, the one shared matcher, and the
//! tabbed/grouped/recency-weighted engine). Concrete providers live in
//! consumer crates. The keymap (§9) and the Surface / AIUI layer (§11) are
//! implemented separately and are intentionally absent here.
//!
//! Spec: [PROP-039](../../../spec/modules/vibe-actions/PROP-039-action-system.md);
//! design: [`spec/design/action-system.md`](../../../spec/design/action-system.md).

#![forbid(unsafe_code)]
specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#root");

pub mod action;
pub mod address;
pub mod context;
pub mod i18n;
pub mod invoke;
pub mod params;
pub mod registry;
pub mod search;

pub use action::{
    Action, ActionBuildError, ActionBuilder, Capability, EnablementFn, Icon, InvokeFn, Msg,
    Presentation, ResolvedAction, SearchMeta,
};
pub use address::{ActionAddr, AddrError, QueryPairs};
pub use context::{Ctx, Enablement};
pub use i18n::{Catalogue, Localized, MessageKey, ResolvedLabel};
pub use invoke::{
    BoxFuture, CancellationToken, GrantedScope, InvokeError, InvokeOutcome, InvokeResult, invoke,
};
pub use params::{
    ParamError, ParamSchema, ParamSpec, ParamType, ParamValue, ParamValues, validate,
};
pub use registry::{Registry, RegistryError};
pub use search::{
    Candidate, Hit, ItemRef, Modifiers, ProviderId, Query, SearchEngine, SearchProvider, SearchRow,
    Selected, Tab,
};
