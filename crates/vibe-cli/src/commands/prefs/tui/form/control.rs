//! The per-type field controls (PROP-041 §4 `#form-per-type`). Each
//! [`FieldControl`] is the editable shape for one preference key, derived from
//! the key's [`KeyMeta`]: **bool → toggle**, **enum / closed-set string →
//! selection** (renders as a `RadioGroup`, PROP-037 §2.7), **int / string →
//! `TextField`** (PROP-037 §2.8), **array / table → not editable in this form
//! yet** (no built-in `vibe.tree.*` key is Array/Table; §4 names them but S2
//! does not compose a list/sub-form editor).
//!
//! The closed-set `vibe.tree.*` string keys (palette, mode, sort, shape) are
//! declared `KeyType::String` in the schema but take one of a fixed option set
//! — [`known_options`] surfaces that set so they render as selections (the
//! right UX) instead of free-form text. A future `enum_values` field on
//! `KeyMeta` (REVIEW phase 2.4, see PROP-040 §6 `#schema-fields`) will carry
//! this on the metadata itself; until then the table here is the one place the
//! closed sets live for the surface.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#form-per-type");

use specmark::spec;
use vibe_settings::schema::{KeyMeta, KeyType};

use crate::commands::tree::tui::ui::TextField;

// ── known closed-enum option sets for vibe.tree.* string keys ────────────────

/// The closed option set for a string key that is effectively an enum, or
/// `None` for a free-form string. Mirrors the `parse_*` tables in
/// [`crate::commands::tree::tui::settings`] (palette/mode/sort/shape) — the
/// spellings the resolver already accepts, listed once here so the form renders
/// a selection instead of asking the user to type a closed-set value.
fn known_options(key: &str) -> Option<&'static [&'static str]> {
    match key {
        "vibe.tree.palette" => Some(&[
            "rose-pine",
            "catppuccin-mocha",
            "catppuccin-macchiato",
            "catppuccin-frappe",
            "catppuccin-latte",
        ]),
        "vibe.tree.mode" => Some(&["all", "sub-tables", "tabs"]),
        "vibe.tree.sort" => Some(&["topological", "alphabetical"]),
        "vibe.tree.shape" => Some(&["members-as-roots", "load-type-forest", "pruned-tree"]),
        _ => None,
    }
}

/// The short label for a selection group — the last segment of the dotted path
/// (e.g. `vibe.tree.palette` → `palette`). Surfaced as the `RadioGroup` title.
fn short_label(path: &str) -> String {
    path.rsplit('.').next().unwrap_or(path).to_owned()
}

// ── Selection ────────────────────────────────────────────────────────────────

/// A single-choice selection over a closed option set (renders as a `RadioGroup`,
/// PROP-037 §2.7). Backs `KeyType::Enum` and the closed-set string keys.
#[derive(Debug, Clone)]
pub struct Selection {
    /// The group's title (mirrors `RadioGroup`'s label — the field's short name).
    #[allow(dead_code)] // carried so a future modal RadioGroup.render can title itself.
    label: String,
    options: Vec<String>,
    selected: usize,
}

impl Selection {
    /// Build a selection with `label`, `options`, and the initially-selected
    /// index (clamped into range; an empty option list leaves the selection 0).
    #[must_use]
    pub fn new(label: impl Into<String>, options: Vec<String>, selected: usize) -> Self {
        let selected = if options.is_empty() {
            0
        } else {
            selected.min(options.len() - 1)
        };
        Self {
            label: label.into(),
            options,
            selected,
        }
    }

    /// The option labels.
    #[must_use]
    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// The selected option index.
    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// The selected option's label.
    #[must_use]
    pub fn selected_option(&self) -> &str {
        self.options
            .get(self.selected)
            .map(String::as_str)
            .unwrap_or("")
    }

    /// Cycle to the next option, wrapping (Space/Enter, §4 `#form-per-type`).
    pub fn cycle_next(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len();
        }
    }
}

// ── FieldControl ─────────────────────────────────────────────────────────────

/// The text-field flavour — how to parse the typed string back to a TOML value
/// for `apply` / `is_modified`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    /// A free-form UTF-8 string (`KeyType::String` without known options).
    String,
    /// A 64-bit signed integer (`KeyType::Int`).
    Int,
    /// A closed enum with no options carried yet (`KeyType::Enum` before
    /// `enum_values` lands — REVIEW phase 2.4); edited as text for now.
    Enum,
}

/// The editable control for one field, derived from `KeyMeta::key_type` (+ the
/// known closed-enum options for a string key). See [`build_control`].
///
/// `activate` is the Space/Enter verb: a toggle flips, a selection cycles to the
/// next option, text is a no-op (typing is how a text field edits). `current_value`
/// is the TOML value the control holds right now — the baseline `is_modified`
/// compares against and the value `apply` writes.
#[derive(Debug, Clone)]
pub enum FieldControl {
    /// A boolean toggle (`KeyType::Bool`).
    Toggle(bool),
    /// A single-choice selection (`KeyType::Enum`, or a String with known options).
    Selection(Selection),
    /// A free-form / numeric / enum-as-text text entry.
    Text {
        /// The editable text field (PROP-037 §2.8).
        field: TextField,
        /// How to parse the typed string back to a TOML value.
        kind: TextKind,
    },
    /// Array / Table — not editable in this form yet (§4 names them; no built-in
    /// `vibe.tree.*` key is Array/Table). Carries the reason note.
    NotEditable(&'static str),
}

impl FieldControl {
    /// Whether this control accepts typed characters (a `TextField`).
    #[must_use]
    pub fn is_text(&self) -> bool {
        matches!(self, FieldControl::Text { .. })
    }

    /// The Space/Enter verb (§4 `#form-per-type`): a toggle flips, a selection
    /// cycles to the next option, text / not-editable is a no-op.
    pub fn activate(&mut self) {
        match self {
            FieldControl::Toggle(b) => *b = !*b,
            FieldControl::Selection(s) => s.cycle_next(),
            FieldControl::Text { .. } | FieldControl::NotEditable(_) => {}
        }
    }

    /// The current TOML value of the control (for `is_modified` / `apply`). An
    /// `Int` text field whose typed string does not yet parse as an integer
    /// yields the raw `String` — so `is_modified` reads true mid-edit and
    /// `apply` can surface the invalid value as a typed error.
    #[must_use]
    pub fn current_value(&self) -> toml::Value {
        match self {
            FieldControl::Toggle(b) => toml::Value::Boolean(*b),
            FieldControl::Selection(s) => toml::Value::String(s.selected_option().to_owned()),
            FieldControl::Text { field, kind } => text_value(field.value(), *kind),
            // A non-editable field yields its captured baseline (passed through
            // by the form — `apply` skips unmodified fields regardless).
            FieldControl::NotEditable(_) => toml::Value::String(String::new()),
        }
    }

    /// Whether the control's editable value parses cleanly for its declared type
    /// (an `Int` text field whose string is not an integer is invalid). Used by
    /// `apply` to refuse a malformed value before writing (PROP-041 §6
    /// `#validation-feedback`'s gate; the inline-error rendering is a later
    /// phase).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            FieldControl::Text {
                field,
                kind: TextKind::Int,
            } => field.value().parse::<i64>().is_ok(),
            _ => true,
        }
    }
}

/// Render a typed text string + kind back to a TOML value (best-effort for Int;
/// a non-numeric string yields `Value::String` so the difference is visible).
fn text_value(text: &str, kind: TextKind) -> toml::Value {
    match kind {
        TextKind::Int => text
            .parse::<i64>()
            .map(toml::Value::Integer)
            .unwrap_or_else(|_| toml::Value::String(text.to_owned())),
        TextKind::String | TextKind::Enum => toml::Value::String(text.to_owned()),
    }
}

/// The TOML value rendered as an editable string (for pre-filling a text field).
fn value_to_edit_string(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Float(f) => f.to_string(),
        _ => String::new(),
    }
}

/// Derive the field control from the key's metadata + resolved value (PROP-041
/// §4 `#form-per-type`). A string key with a known closed-enum option set
/// ([`known_options`]) renders as a selection regardless of its declared
/// `KeyType::String` — the surface's per-key knowledge of the closed set, until
/// `enum_values` is carried on the metadata (REVIEW phase 2.4).
#[spec(implements = "spec://vibevm/modules/vibe-settings/PROP-041#form-per-type")]
pub fn build_control(meta: &KeyMeta, resolved: Option<&toml::Value>) -> FieldControl {
    // A closed-set string key → selection (the vibe.tree.* palette/mode/sort/shape
    // keys are declared String but are really closed enums).
    if let Some(options) = known_options(&meta.path) {
        let opts: Vec<String> = options.iter().map(|s| (*s).to_owned()).collect();
        let current = resolved
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| options.first().copied().unwrap_or(""));
        let selected = opts.iter().position(|o| o == current).unwrap_or(0);
        return FieldControl::Selection(Selection::new(short_label(&meta.path), opts, selected));
    }
    match meta.key_type {
        KeyType::Bool => {
            FieldControl::Toggle(resolved.and_then(toml::Value::as_bool).unwrap_or(false))
        }
        // enum_values not carried yet → fall back to text (the closed-set path
        // above handles the vibe.tree.* string-enums; a raw Enum without options
        // is edited as text until REVIEW phase 2.4 lands the option list).
        KeyType::Enum => FieldControl::Text {
            field: TextField::new()
                .with_value(resolved.and_then(toml::Value::as_str).unwrap_or("")),
            kind: TextKind::Enum,
        },
        KeyType::Int => FieldControl::Text {
            field: TextField::new()
                .with_value(resolved.map(value_to_edit_string).unwrap_or_default()),
            kind: TextKind::Int,
        },
        KeyType::String => FieldControl::Text {
            field: TextField::new().with_value(
                resolved
                    .and_then(toml::Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
            ),
            kind: TextKind::String,
        },
        KeyType::Array | KeyType::Table => {
            FieldControl::NotEditable("not editable in this form yet")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_settings::schema::Scope;

    fn meta(path: &str, ty: KeyType) -> KeyMeta {
        KeyMeta::new(path, ty, Scope::User, "a test setting").unwrap()
    }

    #[test]
    fn bool_field_builds_a_toggle_and_flips() {
        let m = meta("vibe.tree.flag", KeyType::Bool);
        let mut c = build_control(&m, Some(&toml::Value::Boolean(true)));
        assert!(matches!(c, FieldControl::Toggle(true)));
        c.activate();
        assert_eq!(c.current_value(), toml::Value::Boolean(false));
        assert!(c.is_valid());
    }

    #[test]
    fn closed_set_string_builds_a_selection_that_cycles() {
        // vibe.tree.mode is declared String but is a closed enum {all, sub-tables, tabs}.
        let m = meta("vibe.tree.mode", KeyType::String);
        let mut c = build_control(&m, Some(&toml::Value::String("all".into())));
        let sel = match &c {
            FieldControl::Selection(s) => s,
            other => panic!("expected Selection, got {other:?}"),
        };
        assert_eq!(sel.selected_option(), "all");
        assert_eq!(sel.options().len(), 3);
        c.activate(); // cycle all → sub-tables
        assert_eq!(c.current_value(), toml::Value::String("sub-tables".into()));
        c.activate(); // → tabs
        c.activate(); // → all (wraps)
        assert_eq!(c.current_value(), toml::Value::String("all".into()));
    }

    #[test]
    fn free_form_string_builds_a_text_field_that_types() {
        let m = meta("vibe.tree.freeform", KeyType::String);
        let mut c = build_control(&m, Some(&toml::Value::String("hi".into())));
        match &mut c {
            FieldControl::Text { field, kind } => {
                assert_eq!(*kind, TextKind::String);
                assert_eq!(field.value(), "hi");
                field.type_char('!');
                assert_eq!(field.value(), "hi!");
            }
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(c.current_value(), toml::Value::String("hi!".into()));
        assert!(c.is_text());
    }

    #[test]
    fn int_field_builds_a_text_field_and_parses_when_valid() {
        let m = meta("vibe.tree.tier", KeyType::Int);
        let mut c = build_control(&m, Some(&toml::Value::Integer(3)));
        match &c {
            FieldControl::Text { field, kind } => {
                assert_eq!(*kind, TextKind::Int);
                assert_eq!(field.value(), "3");
            }
            other => panic!("expected Text, got {other:?}"),
        }
        assert_eq!(c.current_value(), toml::Value::Integer(3));
        assert!(c.is_valid());
        // Type a non-digit → current_value falls back to String, is_valid false.
        if let FieldControl::Text { field, .. } = &mut c {
            field.type_char('a');
        }
        assert_eq!(c.current_value(), toml::Value::String("3a".into()));
        assert!(!c.is_valid());
    }

    #[test]
    fn array_and_table_are_not_editable() {
        let arr = build_control(&meta("tags", KeyType::Array), None);
        let tab = build_control(&meta("node", KeyType::Table), None);
        assert!(matches!(arr, FieldControl::NotEditable(_)));
        assert!(matches!(tab, FieldControl::NotEditable(_)));
    }

    #[test]
    fn known_options_covers_the_builtin_string_enums() {
        assert_eq!(known_options("vibe.tree.palette").map(|o| o.len()), Some(5));
        assert_eq!(known_options("vibe.tree.mode").map(|o| o.len()), Some(3));
        assert_eq!(known_options("vibe.tree.sort").map(|o| o.len()), Some(2));
        assert_eq!(known_options("vibe.tree.shape").map(|o| o.len()), Some(3));
        assert!(known_options("vibe.tree.tier").is_none());
    }
}
