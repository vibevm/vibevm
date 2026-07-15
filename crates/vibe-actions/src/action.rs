//! The **Action** value (PROP-039 §3): an address, a [`Presentation`], a
//! [`ParamSchema`], a pure enablement predicate, an async `invoke`, a
//! [`Capability`], and [`SearchMeta`] — and nothing else.
//!
//! An [`Action`] is declared through [`Action::builder`], which **requires** a
//! name, a description, and an invoke (§3.3 — the founding human-legibility
//! discipline: name and description are mandatory, non-empty, localizable
//! messages). Presentation strings are [`Msg`]s whose catalogue key is derived
//! from the address (§8.1). A [`ResolvedAction`] is the immutable snapshot a
//! surface renders (§3.2) — change is delivered by re-resolution, not mutation.
//!
//! Spec: [PROP-039 §3](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#action-value).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#action-value");

use std::fmt;

use thiserror::Error;

use crate::address::ActionAddr;
use crate::context::{Ctx, Enablement};
use crate::i18n::{Catalogue, MessageKey, ResolvedLabel};
use crate::invoke::{BoxFuture, InvokeResult};
use crate::params::{ParamSchema, ParamValues};

/// A presentation string: the catalogue [`MessageKey`] (derived from the
/// address) plus the inline English default carried at the declaration site
/// (PROP-039 §8.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Msg {
    key: MessageKey,
    default_en: &'static str,
}

impl Msg {
    /// A message with an explicit key.
    pub fn new(key: MessageKey, default_en: &'static str) -> Self {
        Msg { key, default_en }
    }

    /// A message whose key is derived from `addr` and `field`
    /// (`action.<group>/<name>.<field>`).
    pub fn for_action(addr: &ActionAddr, field: &str, default_en: &'static str) -> Self {
        Msg {
            key: MessageKey::for_action(addr, field),
            default_en,
        }
    }

    /// The catalogue key.
    pub fn key(&self) -> &MessageKey {
        &self.key
    }

    /// The inline English default.
    pub fn default_en(&self) -> &'static str {
        self.default_en
    }

    /// Resolve this message against `catalogue` to a [`ResolvedLabel`].
    pub fn resolve(&self, catalogue: &Catalogue) -> ResolvedLabel {
        catalogue.resolve(&self.key, self.default_en)
    }
}

/// A semantic icon name — a glyph *identifier*, never a rendered glyph, so the
/// crate keeps zero rendering dependencies (§1 `#no-render-dep`). A surface
/// maps the name to its own glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon(String);

impl Icon {
    /// Wrap a semantic icon name (e.g. `"clipboard"`).
    pub fn new(name: impl Into<String>) -> Self {
        Icon(name.into())
    }

    /// The icon name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An action's presentation (PROP-039 §3.3). `name` and `description` are
/// mandatory, non-empty, localizable messages and first-class searchable
/// fields; `icon` and `category` are optional.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presentation {
    name: Msg,
    description: Msg,
    icon: Option<Icon>,
    category: Option<Msg>,
}

impl Presentation {
    /// The action's name message.
    pub fn name(&self) -> &Msg {
        &self.name
    }

    /// The action's description message.
    pub fn description(&self) -> &Msg {
        &self.description
    }

    /// The optional icon.
    pub fn icon(&self) -> Option<&Icon> {
        self.icon.as_ref()
    }

    /// The optional category message.
    pub fn category(&self) -> Option<&Msg> {
        self.category.as_ref()
    }
}

/// Search metadata beyond name/description (PROP-039 §3.1, §10.4): synonyms,
/// aliases, abbreviations, and keywords a Search Everywhere provider indexes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchMeta {
    synonyms: Vec<String>,
    keywords: Vec<String>,
}

impl SearchMeta {
    /// Construct from synonyms and keywords.
    pub fn new(synonyms: Vec<String>, keywords: Vec<String>) -> Self {
        SearchMeta { synonyms, keywords }
    }

    /// The synonyms/aliases/abbreviations.
    pub fn synonyms(&self) -> &[String] {
        &self.synonyms
    }

    /// The keywords.
    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }
}

/// An action's capability class (PROP-039 §7.2), ordered by severity
/// (`Safe < Mutating < Dangerous`) so a [`crate::GrantedScope`] ceiling can
/// permit everything at or below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Capability {
    /// Reads only; no side effects.
    Safe,
    /// Mutates state that can be undone or is low-risk.
    Mutating,
    /// Irreversible or high-risk.
    Dangerous,
}

impl Capability {
    /// The wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Capability::Safe => "safe",
            Capability::Mutating => "mutating",
            Capability::Dangerous => "dangerous",
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The pure enablement predicate stored on an [`Action`] (§6.2).
pub type EnablementFn = Box<dyn Fn(&Ctx) -> Enablement + Send + Sync>;

/// The async body stored on an [`Action`] (§7.1).
pub type InvokeFn =
    Box<dyn Fn(&Ctx, ParamValues) -> BoxFuture<'static, InvokeResult> + Send + Sync>;

/// A first-class, addressable behaviour (PROP-039 §3.1). Carries exactly its
/// address, presentation, parameter schema, enablement predicate, invoke body,
/// capability, and search metadata — no other state.
pub struct Action {
    addr: ActionAddr,
    presentation: Presentation,
    params: ParamSchema,
    enablement: EnablementFn,
    invoke: InvokeFn,
    capability: Capability,
    search_meta: SearchMeta,
}

impl Action {
    /// Start declaring an action at `addr`. Name, description, and invoke are
    /// required at [`ActionBuilder::build`] (§3.3).
    pub fn builder(addr: ActionAddr) -> ActionBuilder {
        ActionBuilder::new(addr)
    }

    /// The action's address.
    pub fn addr(&self) -> &ActionAddr {
        &self.addr
    }

    /// The action's presentation.
    pub fn presentation(&self) -> &Presentation {
        &self.presentation
    }

    /// The action's parameter schema.
    pub fn params(&self) -> &ParamSchema {
        &self.params
    }

    /// The action's capability class.
    pub fn capability(&self) -> Capability {
        self.capability
    }

    /// The action's search metadata.
    pub fn search_meta(&self) -> &SearchMeta {
        &self.search_meta
    }

    /// Evaluate the pure enablement predicate over `ctx` (§6.2).
    pub fn evaluate(&self, ctx: &Ctx) -> Enablement {
        (self.enablement)(ctx)
    }

    /// Produce the async invocation future (§7.1). Prefer the top-level
    /// [`crate::invoke`], which validates parameters and the capability first;
    /// this is the raw body the pipeline awaits.
    pub fn call(&self, ctx: &Ctx, values: ParamValues) -> BoxFuture<'static, InvokeResult> {
        (self.invoke)(ctx, values)
    }

    /// The immutable resolved snapshot a surface renders (§3.2): address plus
    /// resolved name/description and the enablement verdict for `ctx`. Change
    /// is delivered by calling this again, never by mutating the `Action`.
    pub fn resolve(&self, ctx: &Ctx, catalogue: &Catalogue) -> ResolvedAction {
        ResolvedAction {
            addr: self.addr.clone(),
            name: self.presentation.name.resolve(catalogue),
            description: self.presentation.description.resolve(catalogue),
            enablement: self.evaluate(ctx),
        }
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The closures are not `Debug`; project the inspectable state.
        f.debug_struct("Action")
            .field("addr", &self.addr)
            .field("presentation", &self.presentation)
            .field("params", &self.params)
            .field("capability", &self.capability)
            .field("search_meta", &self.search_meta)
            .finish_non_exhaustive()
    }
}

/// The immutable resolved snapshot of an action for a given context (§3.2).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ResolvedAction {
    /// The action's address.
    pub addr: ActionAddr,
    /// The resolved name label.
    pub name: ResolvedLabel,
    /// The resolved description label.
    pub description: ResolvedLabel,
    /// The enablement verdict for the resolving context.
    pub enablement: Enablement,
}

/// Why [`ActionBuilder::build`] rejected a declaration — the human-legibility
/// discipline enforced at construction (§3.3).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-actions/PROP-039#presentation")]
pub enum ActionBuildError {
    /// No name was supplied.
    #[error(
        "action `{addr}` has no name — name is mandatory \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#presentation; \
          fix: call `.name_en(..)` when building the action)"
    )]
    MissingName {
        /// The action being built.
        addr: String,
    },

    /// No description was supplied.
    #[error(
        "action `{addr}` has no description — description is mandatory \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#presentation; \
          fix: call `.description_en(..)` when building the action)"
    )]
    MissingDescription {
        /// The action being built.
        addr: String,
    },

    /// A supplied name or description was empty or whitespace-only.
    #[error(
        "action `{addr}` has an empty {field} — name and description must be non-empty, \
         meaningful text \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#presentation; \
          fix: give `{field}` real, human-legible text)"
    )]
    EmptyPresentation {
        /// The action being built.
        addr: String,
        /// Which field was empty (`name` or `description`).
        field: &'static str,
    },

    /// No invoke body was supplied.
    #[error(
        "action `{addr}` has no invoke body \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#invoke; \
          fix: call `.invoke(..)` when building the action)"
    )]
    MissingInvoke {
        /// The action being built.
        addr: String,
    },
}

/// The ergonomic declaration path for an [`Action`] (§3). Name, description,
/// and invoke are required; everything else defaults (empty params, an
/// always-enabled predicate, [`Capability::Safe`], empty search metadata).
pub struct ActionBuilder {
    addr: ActionAddr,
    name: Option<Msg>,
    description: Option<Msg>,
    icon: Option<Icon>,
    category: Option<Msg>,
    params: ParamSchema,
    enablement: Option<EnablementFn>,
    invoke: Option<InvokeFn>,
    capability: Capability,
    search_meta: SearchMeta,
}

impl ActionBuilder {
    fn new(addr: ActionAddr) -> Self {
        ActionBuilder {
            addr,
            name: None,
            description: None,
            icon: None,
            category: None,
            params: ParamSchema::empty(),
            enablement: None,
            invoke: None,
            capability: Capability::Safe,
            search_meta: SearchMeta::default(),
        }
    }

    /// Set the name from inline English; the catalogue key is derived from the
    /// address (§8.1).
    #[must_use]
    pub fn name_en(mut self, default_en: &'static str) -> Self {
        self.name = Some(Msg::for_action(&self.addr, "name", default_en));
        self
    }

    /// Set the name from an explicit [`Msg`].
    #[must_use]
    pub fn name(mut self, name: Msg) -> Self {
        self.name = Some(name);
        self
    }

    /// Set the description from inline English; the key is derived from the
    /// address (§8.1).
    #[must_use]
    pub fn description_en(mut self, default_en: &'static str) -> Self {
        self.description = Some(Msg::for_action(&self.addr, "description", default_en));
        self
    }

    /// Set the description from an explicit [`Msg`].
    #[must_use]
    pub fn description(mut self, description: Msg) -> Self {
        self.description = Some(description);
        self
    }

    /// Set the optional icon by semantic name.
    #[must_use]
    pub fn icon(mut self, name: impl Into<String>) -> Self {
        self.icon = Some(Icon::new(name));
        self
    }

    /// Set the optional category from inline English.
    #[must_use]
    pub fn category_en(mut self, default_en: &'static str) -> Self {
        self.category = Some(Msg::for_action(&self.addr, "category", default_en));
        self
    }

    /// Set the parameter schema.
    #[must_use]
    pub fn params(mut self, params: ParamSchema) -> Self {
        self.params = params;
        self
    }

    /// Set the pure enablement predicate (§6.2). Defaults to always-enabled.
    #[must_use]
    pub fn enablement<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&Ctx) -> Enablement + Send + Sync + 'static,
    {
        self.enablement = Some(Box::new(predicate));
        self
    }

    /// Set the async invoke body (§7.1). Required.
    #[must_use]
    pub fn invoke<F>(mut self, body: F) -> Self
    where
        F: Fn(&Ctx, ParamValues) -> BoxFuture<'static, InvokeResult> + Send + Sync + 'static,
    {
        self.invoke = Some(Box::new(body));
        self
    }

    /// Set the capability class. Defaults to [`Capability::Safe`].
    #[must_use]
    pub fn capability(mut self, capability: Capability) -> Self {
        self.capability = capability;
        self
    }

    /// Set the search metadata.
    #[must_use]
    pub fn search_meta(mut self, search_meta: SearchMeta) -> Self {
        self.search_meta = search_meta;
        self
    }

    /// Finish, enforcing the §3.3 legibility discipline: name and description
    /// are mandatory and non-empty, and an invoke body is required.
    pub fn build(self) -> Result<Action, ActionBuildError> {
        let addr_str = self.addr.to_string();

        let name = self.name.ok_or_else(|| ActionBuildError::MissingName {
            addr: addr_str.clone(),
        })?;
        if name.default_en().trim().is_empty() {
            return Err(ActionBuildError::EmptyPresentation {
                addr: addr_str,
                field: "name",
            });
        }

        let description = self
            .description
            .ok_or_else(|| ActionBuildError::MissingDescription {
                addr: addr_str.clone(),
            })?;
        if description.default_en().trim().is_empty() {
            return Err(ActionBuildError::EmptyPresentation {
                addr: addr_str,
                field: "description",
            });
        }

        let invoke = self.invoke.ok_or_else(|| ActionBuildError::MissingInvoke {
            addr: addr_str.clone(),
        })?;

        let enablement = self
            .enablement
            .unwrap_or_else(|| Box::new(|_ctx| Enablement::enabled()));

        Ok(Action {
            addr: self.addr,
            presentation: Presentation {
                name,
                description,
                icon: self.icon,
                category: self.category,
            },
            params: self.params,
            enablement,
            invoke,
            capability: self.capability,
            search_meta: self.search_meta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoke::InvokeOutcome;

    fn addr() -> ActionAddr {
        ActionAddr::parse("action://vibe.tree/copy.markdown").unwrap()
    }

    fn minimal() -> ActionBuilder {
        Action::builder(addr())
            .name_en("Copy as Markdown")
            .description_en("Copy the selected node as a Markdown link")
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
    }

    #[test]
    fn builder_produces_a_well_formed_action() {
        let action = minimal().build().unwrap();
        assert_eq!(action.addr(), &addr());
        assert_eq!(
            action.presentation().name().default_en(),
            "Copy as Markdown"
        );
        assert_eq!(action.capability(), Capability::Safe); // default
        assert!(action.params().is_empty());
    }

    #[test]
    fn name_key_is_derived_from_the_address() {
        let action = minimal().build().unwrap();
        assert_eq!(
            action.presentation().name().key().as_str(),
            "action.vibe.tree/copy.markdown.name"
        );
        assert_eq!(
            action.presentation().description().key().as_str(),
            "action.vibe.tree/copy.markdown.description"
        );
    }

    #[test]
    fn build_requires_a_name() {
        let err = Action::builder(addr())
            .description_en("desc")
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap_err();
        assert!(matches!(err, ActionBuildError::MissingName { .. }));
    }

    #[test]
    fn build_requires_a_description() {
        let err = Action::builder(addr())
            .name_en("Copy")
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap_err();
        assert!(matches!(err, ActionBuildError::MissingDescription { .. }));
    }

    #[test]
    fn build_rejects_an_empty_name() {
        let err = Action::builder(addr())
            .name_en("   ")
            .description_en("desc")
            .invoke(|_c, _v| Box::pin(async { Ok(InvokeOutcome::Done) }))
            .build()
            .unwrap_err();
        assert!(matches!(
            err,
            ActionBuildError::EmptyPresentation { field: "name", .. }
        ));
    }

    #[test]
    fn build_requires_an_invoke_body() {
        let err = Action::builder(addr())
            .name_en("Copy")
            .description_en("desc")
            .build()
            .unwrap_err();
        assert!(matches!(err, ActionBuildError::MissingInvoke { .. }));
    }

    #[test]
    fn resolve_produces_an_immutable_snapshot() {
        let action = minimal().build().unwrap();
        let cat = Catalogue::new("en");
        let snap = action.resolve(&Ctx::new(), &cat);
        assert_eq!(snap.addr, addr());
        assert_eq!(snap.name.value(), "Copy as Markdown");
        assert_eq!(snap.name.original_en(), "Copy as Markdown");
        assert!(snap.enablement.enabled); // default predicate
    }

    #[test]
    fn capabilities_are_ordered() {
        assert!(Capability::Safe < Capability::Mutating);
        assert!(Capability::Mutating < Capability::Dangerous);
    }
}
