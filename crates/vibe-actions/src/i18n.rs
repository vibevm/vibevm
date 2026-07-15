//! Address-keyed message catalogue (PROP-039 §8).
//!
//! A presentation string is a [`MessageKey`] **derived from the address**
//! (`action.<group>/<name>.<field>`) plus an inline English default carried at
//! the declaration site. A [`Catalogue`] resolves a key through a parent chain
//! terminating in the inline default, so a release lookup **can never miss**
//! (no sentinel, no panic). A [`ResolvedLabel`] keeps the localized `value`
//! beside the `original_en`, so Search Everywhere can index both (§8.2).
//!
//! English is the default, mandatory-complete, terminating fallback (§8.3).
//!
//! Spec: [PROP-039 §8](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#i18n).
//
// TODO: Fluent on-disk format (PROP-039 §8.1) — this is a plain in-memory map
// for now; the `locales/<lang>.ftl` loader + `ArcSwap<Catalogue>` locale swap
// land with the surface layer.

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#i18n");

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::address::ActionAddr;

/// The catalogue key for a presentation string — derived one-to-one from the
/// action address, so there is no second key namespace to drift (§8.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageKey(String);

impl MessageKey {
    /// Wrap a raw key string.
    pub fn new(key: impl Into<String>) -> Self {
        MessageKey(key.into())
    }

    /// Derive the key for a `field` (`"name"`, `"description"`, …) of `addr`:
    /// `action.<group>/<name>.<field>`.
    ///
    /// ```
    /// use vibe_actions::{ActionAddr, MessageKey};
    ///
    /// let addr = ActionAddr::parse("action://vibe.tree/copy.markdown").unwrap();
    /// let key = MessageKey::for_action(&addr, "name");
    /// assert_eq!(key.as_str(), "action.vibe.tree/copy.markdown.name");
    /// ```
    pub fn for_action(addr: &ActionAddr, field: &str) -> Self {
        MessageKey(format!("action.{}/{}.{}", addr.group(), addr.name(), field))
    }

    /// The key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MessageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A localized string — the human-facing text a surface displays (e.g. a
/// disabled action's "why disabled" reason, §6.2).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Localized(String);

impl Localized {
    /// Wrap a localized string.
    pub fn new(text: impl Into<String>) -> Self {
        Localized(text.into())
    }

    /// The text as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Localized {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Localized {
    fn from(s: &str) -> Self {
        Localized(s.to_owned())
    }
}

impl From<String> for Localized {
    fn from(s: String) -> Self {
        Localized(s)
    }
}

/// A resolved presentation label — the localized `value` plus the English
/// original the inline default carried (§8.2). Both are indexed by Search
/// Everywhere, so a user typing the English name finds an action under any
/// locale.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ResolvedLabel {
    value: String,
    original_en: String,
}

impl ResolvedLabel {
    /// Construct a resolved label from its parts.
    pub fn new(value: impl Into<String>, original_en: impl Into<String>) -> Self {
        ResolvedLabel {
            value: value.into(),
            original_en: original_en.into(),
        }
    }

    /// The localized value shown to the user.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The English original carried at the declaration site.
    pub fn original_en(&self) -> &str {
        &self.original_en
    }

    /// The localized value as a [`Localized`].
    pub fn localized(&self) -> Localized {
        Localized::new(self.value.clone())
    }
}

/// An address-keyed message catalogue with an optional parent. Resolution walks
/// `self → parent → … → inline default`, so it never misses (§8.1).
#[derive(Debug, Clone)]
pub struct Catalogue {
    locale: String,
    entries: HashMap<MessageKey, String>,
    parent: Option<Arc<Catalogue>>,
}

impl Catalogue {
    /// A root catalogue for `locale` with no parent.
    pub fn new(locale: impl Into<String>) -> Self {
        Catalogue {
            locale: locale.into(),
            entries: HashMap::new(),
            parent: None,
        }
    }

    /// A catalogue for `locale` that falls back to `parent` on a miss.
    pub fn with_parent(locale: impl Into<String>, parent: Arc<Catalogue>) -> Self {
        Catalogue {
            locale: locale.into(),
            entries: HashMap::new(),
            parent: Some(parent),
        }
    }

    /// The locale tag (e.g. `en`, `ru`).
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Insert (or replace) an entry, chaining.
    pub fn insert(&mut self, key: MessageKey, value: impl Into<String>) -> &mut Self {
        self.entries.insert(key, value.into());
        self
    }

    /// Look a key up through this catalogue and its parent chain, returning the
    /// first hit — or `None` if no catalogue in the chain carries it.
    pub fn lookup(&self, key: &MessageKey) -> Option<&str> {
        if let Some(value) = self.entries.get(key) {
            return Some(value);
        }
        match &self.parent {
            Some(parent) => parent.lookup(key),
            None => None,
        }
    }

    /// Resolve `key` to a [`ResolvedLabel`], falling back to `default_en` (the
    /// inline English) when the chain has no entry. Never misses: the returned
    /// `value` is the localized string if present, otherwise `default_en`, and
    /// `original_en` is always `default_en`.
    pub fn resolve(&self, key: &MessageKey, default_en: &str) -> ResolvedLabel {
        let value = self
            .lookup(key)
            .map(str::to_owned)
            .unwrap_or_else(|| default_en.to_owned());
        ResolvedLabel {
            value,
            original_en: default_en.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr() -> ActionAddr {
        ActionAddr::parse("action://vibe.tree/copy.markdown").unwrap()
    }

    #[test]
    fn message_key_is_derived_from_address() {
        let key = MessageKey::for_action(&addr(), "description");
        assert_eq!(key.as_str(), "action.vibe.tree/copy.markdown.description");
    }

    #[test]
    fn resolve_falls_back_to_inline_english() {
        // Empty `en` catalogue: no entry, so the inline default is the value,
        // and it is also the English original.
        let cat = Catalogue::new("en");
        let key = MessageKey::for_action(&addr(), "name");
        let label = cat.resolve(&key, "Copy as Markdown");
        assert_eq!(label.value(), "Copy as Markdown");
        assert_eq!(label.original_en(), "Copy as Markdown");
    }

    #[test]
    fn resolve_prefers_a_catalogue_entry_but_keeps_english_original() {
        let mut cat = Catalogue::new("ru");
        let key = MessageKey::for_action(&addr(), "name");
        cat.insert(key.clone(), "Копировать как Markdown");
        let label = cat.resolve(&key, "Copy as Markdown");
        assert_eq!(label.value(), "Копировать как Markdown");
        assert_eq!(label.original_en(), "Copy as Markdown"); // §8.2
    }

    #[test]
    fn resolve_walks_the_parent_chain() {
        let key = MessageKey::for_action(&addr(), "name");
        let mut en = Catalogue::new("en");
        en.insert(key.clone(), "Copy as Markdown");
        let ru = Catalogue::with_parent("ru", Arc::new(en)); // ru lacks the key

        // ru misses locally, parent `en` hits — value from parent, original the
        // inline default (which never misses).
        let label = ru.resolve(&key, "Copy as Markdown");
        assert_eq!(label.value(), "Copy as Markdown");
    }

    #[test]
    fn lookup_reports_a_true_miss() {
        let cat = Catalogue::new("en");
        assert_eq!(cat.lookup(&MessageKey::new("action.x/y.name")), None);
    }
}
