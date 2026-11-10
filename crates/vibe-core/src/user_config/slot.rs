//! `[slot.<slot>.extras.<ecosystem>]` — the user's own per-slot,
//! per-ecosystem list of what every new version of a runtime should carry
//! in that runtime's own ecosystem (PROP-075 `#SLOT-EXTRAS`).
//!
//! This is the generalisation of nvm's `$NVM_DIR/default-packages`: not
//! "default npm packages" but "what this user wants in every new version
//! of this slot's runtime". The declaration is **the user's**, which is
//! why it lives here and not beside `applications/index.json` — the owner
//! decided the list is settings, and settings are `~/.vibe/config.toml`.
//!
//! Nothing in this module knows one ecosystem word. A slot id, an
//! ecosystem id and an item string are all opaque: they are read, held
//! and handed on verbatim, never parsed, split, quoted, escaped,
//! validated or reordered. A `drop` entry is the one exception to
//! "opaque" only in *kind* — it names a **recommendation id**, not an
//! item, because naming the author's row rather than re-typing their
//! string is what keeps the two sides from drifting.
//!
//! An unknown slot, an unknown ecosystem id or an unknown recommendation
//! id is **not** a parse error: a hand-written settings file outlives the
//! packages it mentions, so refusing an install over a stale line would
//! be the wrong trade. The reporting-and-ignoring happens where the list
//! is consumed (`vibe-app`'s extras arithmetic), not here.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS");

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specmark::spec;

/// One `[slot.<slot>]` table of the user config.
///
/// It carries `extras` today and is a table rather than the extras map
/// itself so that a later per-slot user setting — there is no second one
/// yet — does not have to move the user's extras list to reach the file.
///
/// ```
/// use vibe_core::user_config::UserConfig;
///
/// let cfg: UserConfig = toml::from_str(
///     "[slot.node.extras.npm]\nadd = [\"typescript\", \"@scope/pkg@^1.2\"]\ndrop = [\"corepack\"]\n",
/// )
/// .unwrap();
/// let extras = cfg.extras_of("node", "npm").unwrap();
/// assert_eq!(extras.add, ["typescript", "@scope/pkg@^1.2"]);
/// assert_eq!(extras.drop, ["corepack"]);
/// // A slot or an ecosystem the file never mentions is simply absent.
/// assert!(cfg.extras_of("node", "cargo").is_none());
/// assert!(cfg.extras_of("python", "npm").is_none());
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub struct SlotConfig {
    /// `[slot.<slot>.extras.<ecosystem>]`, keyed by the adapter `id` the
    /// slot's package declares. An id no package declares is kept — the
    /// consumer reports it once and ignores it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extras: BTreeMap<String, SlotExtras>,
}

impl SlotConfig {
    /// `true` for the all-defaults table — lets the serializer skip a
    /// `[slot.<slot>]` the user never filled.
    pub fn is_default(&self) -> bool {
        *self == SlotConfig::default()
    }
}

/// One `[slot.<slot>.extras.<ecosystem>]` table: what the user wants added
/// to every new version of that slot in that ecosystem, and which of the
/// author's recommendations they decline.
///
/// ```
/// use vibe_core::user_config::SlotExtras;
///
/// let extras: SlotExtras = toml::from_str("add = [\"typescript\"]").unwrap();
/// assert_eq!(extras.add, ["typescript"]);
/// assert!(extras.drop.is_empty());
/// assert!(SlotExtras::default().is_default());
/// // An item is opaque: whatever a shell would make of it, vibe does not.
/// let extras: SlotExtras = toml::from_str("add = [\"; rm -rf $HOME\"]").unwrap();
/// assert_eq!(extras.add, ["; rm -rf $HOME"]);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub struct SlotExtras {
    /// The user's own items, in their own order. Opaque strings: they
    /// reach the adapter's `install` verb as separate argv elements,
    /// exactly as written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add: Vec<String>,
    /// The **recommendation ids** the user declines — not items. A drop is
    /// honoured in every case, `--no-extras` included: a flag that says
    /// "do not install my extras" must never cause *more* to be installed
    /// than the user asked for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drop: Vec<String>,
}

impl SlotExtras {
    /// `true` for the all-defaults table.
    pub fn is_default(&self) -> bool {
        *self == SlotExtras::default()
    }

    /// Append `items` to [`Self::add`], keeping the file's own order and
    /// skipping an item the list already holds verbatim. Answers how many
    /// were new, because `vibe app extras add` reports that and nothing
    /// else.
    ///
    /// ```
    /// use vibe_core::user_config::SlotExtras;
    ///
    /// let mut extras = SlotExtras::default();
    /// assert_eq!(extras.add_items(["typescript".to_owned(), "pnpm".to_owned()]), 2);
    /// // Exact strings, first occurrence winning: a repeat adds nothing.
    /// assert_eq!(extras.add_items(["pnpm".to_owned()]), 0);
    /// assert_eq!(extras.add, ["typescript", "pnpm"]);
    /// ```
    pub fn add_items(&mut self, items: impl IntoIterator<Item = String>) -> usize {
        let mut added = 0;
        for item in items {
            if !self.add.contains(&item) {
                self.add.push(item);
                added += 1;
            }
        }
        added
    }

    /// Append `ids` to [`Self::drop`] the same way.
    pub fn drop_ids(&mut self, ids: impl IntoIterator<Item = String>) -> usize {
        let mut added = 0;
        for id in ids {
            if !self.drop.contains(&id) {
                self.drop.push(id);
                added += 1;
            }
        }
        added
    }
}

/// One item per line, `#` comments and blank lines ignored — the format
/// `$NVM_DIR/default-packages` is in, and the one an adapter's `list` verb
/// already writes (PROP-075 `#SLOT-EXTRAS`).
///
/// A line is trimmed of surrounding whitespace and otherwise untouched: an
/// item is opaque, so a `#` inside a line that does not *start* with one
/// is part of the item.
///
/// ```
/// use vibe_core::user_config::read_item_lines;
///
/// assert_eq!(
///     read_item_lines("# npm globals\n\n  typescript  \n@scope/pkg@^1.2\n\n# done\n"),
///     ["typescript", "@scope/pkg@^1.2"]
/// );
/// // Only a leading `#` is a comment marker.
/// assert_eq!(read_item_lines("pkg#tag\n"), ["pkg#tag"]);
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
pub fn read_item_lines(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}
