//! The edit form over an open page's keys (PROP-041 §4 `#edit-form`). One
//! [`FormField`] per page key, each carrying its [`KeyMeta`], the editable
//! [`FieldControl`] (toggle / selection / text / not-editable), and the resolved
//! baseline (the value captured at build / apply / reset — `is_modified` compares
//! the control against this). The form owns the field-focus index, the chosen
//! write-layer, and the three layer file paths.
//!
//! The per-type control derivation lives in [`control`]; the Configurable
//! lifecycle (`is_modified` / `apply` / `reset`) in [`lifecycle`]; the render in
//! [`render`]. This module owns the aggregate + the build from [`PrefsApp`] +
//! the focus / write-layer navigation.
//!
//! The form is a **surface** over PROP-040's data layer (PROP-041 §1
//! `#surface-not-engine`): it reads `ResolvedPrefs` + `Schema` and writes
//! through the same load → set-dotted → diff-from-default → atomic-write path
//! P9a's [`TreeSettings`] uses (PROP-037 §9), parameterised by the chosen
//! write-layer.
//!
//! [`TreeSettings`]: crate::commands::tree::tui::settings::TreeSettings

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#edit-form");

pub mod control;
pub mod lifecycle;
pub mod provenance;
pub mod provenance_edit;
pub mod render;
pub mod validation;

use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_settings::loader::Layer;
use vibe_settings::schema::KeyMeta;

use crate::commands::tree::tui::settings::TreeSettings;

use super::state::PrefsApp;
use control::FieldControl;
use control::build_control;

// ── LayerPaths ───────────────────────────────────────────────────────────────

/// The three settings layer file paths (PROP-040 §3), owned by the form so
/// `apply` can write to the chosen layer without touching the process env at
/// write time. Built once from the env (mirroring [`TreeSettings::new`]) for a
/// production form, or from explicit paths for a test form (tempdir-backed).
#[derive(Debug, Clone)]
pub(crate) struct LayerPaths {
    /// L1 — user-machine (`~/.vibe/settings.toml`).
    l1: PathBuf,
    /// L2 — repo-shared (`<repo>/.vibe/settings.toml`).
    l2: PathBuf,
    /// L3 — user-project (`<repo>/.vibe/settings.local.toml`).
    l3: PathBuf,
}

impl LayerPaths {
    /// Build from the three explicit paths (the test entry point — production
    /// builds use [`LayerPaths::from_env`]).
    #[must_use]
    #[cfg(test)]
    pub(crate) fn new(l1: PathBuf, l2: PathBuf, l3: PathBuf) -> Self {
        Self { l1, l2, l3 }
    }

    /// Build from the process env, reusing [`TreeSettings`]'s path logic so the
    /// form and the `vibe tree` TUI agree on where the layers live (PROP-040 §3
    /// fixes L1 at `~/.vibe/`, L2/L3 at `<cwd>/.vibe/`). `pub(crate)` so the
    /// cross-layer lint-all action ([`super::lint`]) can build the same paths
    /// without re-deriving them.
    #[must_use]
    pub(crate) fn from_env() -> Self {
        let ts = TreeSettings::new();
        Self {
            l1: ts.layer_path(Layer::L1).to_owned(),
            l2: ts.layer_path(Layer::L2).to_owned(),
            l3: ts.layer_path(Layer::L3).to_owned(),
        }
    }

    /// The file path for a given layer.
    #[must_use]
    pub(crate) fn path(&self, layer: Layer) -> &Path {
        match layer {
            Layer::L1 => &self.l1,
            Layer::L2 => &self.l2,
            Layer::L3 => &self.l3,
        }
    }
}

// ── FormField ────────────────────────────────────────────────────────────────

/// One editable field in the form (PROP-041 §4). Carries the key's metadata, the
/// editable control, and the resolved baseline (the value at build / last apply
/// / last reset — `is_modified` compares the control's current value against
/// this).
#[derive(Debug, Clone)]
pub struct FormField {
    /// The dotted preference path.
    pub key: String,
    /// The key's declared metadata (type, scope, applies, default, …).
    pub meta: KeyMeta,
    /// The editable control (toggle / selection / text / not-editable).
    pub control: FieldControl,
    /// The resolved value captured at build / apply / reset — the `is_modified`
    /// baseline.
    baseline: toml::Value,
}

impl FormField {
    /// The `applies` badge label for this field (PROP-040 §10, shown per field
    /// per PROP-041 §4 `#apply-indicator`).
    #[must_use]
    pub fn applies_label(&self) -> &'static str {
        self.meta.applies.label()
    }
}

// ── Form ─────────────────────────────────────────────────────────────────────

/// The per-type edit form over an open page's keys (PROP-041 §4). Owns one
/// [`FormField`] per key, the field-focus index, the chosen write-layer, and the
/// three layer file paths. The Configurable lifecycle (`is_modified` / `apply` /
/// `reset`) lives in [`lifecycle`]; the render in [`render`].
pub struct Form {
    /// The open page id.
    #[allow(dead_code)] // introspection: carried for the AIUI / a future jump-to-page.
    pub page_id: String,
    /// The page title (display name).
    #[allow(dead_code)] // introspection: the border title is sourced from PrefsApp today.
    pub title: String,
    /// The page description (rendered at the top of the form).
    pub description: String,
    /// One field per key, in page-declaration order.
    pub fields: Vec<FormField>,
    /// The focused field index.
    pub focus: usize,
    /// The layer `apply` writes to (PROP-041 §4 `#write-layer-choice`). Defaults
    /// to L3 for a project session, L1 for a no-project session; `Tab` cycles.
    pub write_layer: Layer,
    /// Whether the provenance view is open for the focused field (PROP-041 §5
    /// `#provenance-view`). Toggled by `?`; follows the focus — when open, the
    /// block renders under whichever field is focused.
    pub provenance_open: bool,
    /// The three layer file paths.
    paths: LayerPaths,
}

impl Form {
    /// Build the form for the open page over the app's resolved prefs + schema
    /// (PROP-041 §4 `#form-per-type`). One field per page key, derived from the
    /// key's `KeyMeta` + resolved value. The write-layer defaults per the session
    /// context (#write-layer-choice). Returns `None` when no page is open.
    #[spec(implements = "spec://vibevm/modules/vibe-settings/PROP-041#form-per-type")]
    pub fn build(app: &PrefsApp) -> Option<Self> {
        let page_id = app.open_page.as_deref()?;
        let decl = app.registry.pages().iter().find(|d| d.id == page_id)?;
        let paths = LayerPaths::from_env();
        let write_layer = default_write_layer(app.ctx.has_project);
        let fields = decl
            .keys
            .iter()
            .filter_map(|key| {
                let meta = app.schema.get(key)?.clone();
                let resolved = app.prefs.get(key);
                let baseline = resolved
                    .cloned()
                    .or_else(|| meta.default.clone())
                    .unwrap_or_else(|| toml::Value::String(String::new()));
                let control = build_control(&meta, resolved);
                Some(FormField {
                    key: key.clone(),
                    meta,
                    control,
                    baseline,
                })
            })
            .collect();
        Some(Form {
            page_id: page_id.to_owned(),
            title: decl.display_name.clone(),
            description: decl.description.clone(),
            fields,
            focus: 0,
            write_layer,
            provenance_open: false,
            paths,
        })
    }

    /// Test constructor with explicit layer paths (so `apply` can be exercised
    /// against a tempdir without touching the operator's real `~/.vibe/`).
    #[cfg(test)]
    pub(crate) fn for_test(
        title: impl Into<String>,
        description: impl Into<String>,
        fields: Vec<FormField>,
        write_layer: Layer,
        paths: LayerPaths,
    ) -> Self {
        Form {
            page_id: "__test".to_owned(),
            title: title.into(),
            description: description.into(),
            fields,
            focus: 0,
            write_layer,
            provenance_open: false,
            paths,
        }
    }

    /// The focused field, if any.
    #[must_use]
    pub fn focused_field(&self) -> Option<&FormField> {
        self.fields.get(self.focus)
    }

    /// The focused field, mutable, if any.
    pub fn focused_field_mut(&mut self) -> Option<&mut FormField> {
        self.fields.get_mut(self.focus)
    }

    /// The file path for the chosen write-layer.
    #[must_use]
    pub(crate) fn write_path(&self) -> &Path {
        self.paths.path(self.write_layer)
    }

    /// Move the field focus up one field (↑).
    pub fn move_up(&mut self) {
        if !self.fields.is_empty() {
            self.focus = self.focus.saturating_sub(1);
        }
    }

    /// Move the field focus down one field (↓).
    pub fn move_down(&mut self) {
        if !self.fields.is_empty() {
            self.focus = (self.focus + 1).min(self.fields.len() - 1);
        }
    }

    /// Cycle the write-layer L1 → L2 → L3 → L1 (Tab, PROP-041 §4
    /// `#write-layer-choice`).
    pub fn cycle_write_layer(&mut self) {
        self.write_layer = match self.write_layer {
            Layer::L1 => Layer::L2,
            Layer::L2 => Layer::L3,
            Layer::L3 => Layer::L1,
        };
    }

    /// Toggle the provenance view for the focused field (PROP-041 §5
    /// `#provenance-view`, wired to `?`).
    pub fn toggle_provenance(&mut self) {
        self.provenance_open = !self.provenance_open;
    }

    /// Focus the field whose key matches `key`, if any (PROP-041 §6 `#lint-all`'s
    /// jump-to-field: selecting a lint entry opens the owning page and focuses
    /// the offending field). A no-op when the key is not on this page.
    pub fn focus_key(&mut self, key: &str) {
        if let Some(idx) = self.fields.iter().position(|f| f.key == key) {
            self.focus = idx;
        }
    }
}

/// The default write-layer for a session (PROP-041 §4 `#write-layer-choice`): L3
/// for a project session (per-project fine-tuning), L1 for a no-project session
/// (user-machine only — L2/L3 live under a repo that is not active).
fn default_write_layer(has_project: bool) -> Layer {
    if has_project { Layer::L3 } else { Layer::L1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::prefs::tui::state::PrefsCtx;
    use crate::commands::tree::tui::settings::TreeSettings;
    use vibe_settings::loader::LayeredRaw;
    use vibe_settings::resolver::resolve;
    use vibe_settings::schema::{KeyMeta, KeyType, Schema, Scope};

    /// A schema with one bool + one closed-set string (mode) key.
    fn schema() -> Schema {
        let mut s = Schema::new();
        s.register(
            KeyMeta::new("vibe.tree.flag", KeyType::Bool, Scope::User, "a flag")
                .unwrap()
                .with_default(toml::Value::Boolean(true)),
        )
        .unwrap();
        s.register(
            KeyMeta::new("vibe.tree.mode", KeyType::String, Scope::User, "mode")
                .unwrap()
                .with_default(toml::Value::String("all".into())),
        )
        .unwrap();
        s
    }

    fn app() -> PrefsApp {
        let prefs = resolve(
            LayeredRaw::default(),
            &schema(),
            toml::Table::new(),
            toml::Table::new(),
        );
        PrefsApp::new(prefs, schema(), PrefsCtx::new(true))
    }

    #[test]
    fn build_derives_one_field_per_key_with_controls() {
        let mut app = app();
        // Plant an open page with the two keys.
        app.open_page = Some("test".into());
        app.registry = super::super::registry::PageRegistry::from(vec![
            super::super::registry::PageDecl::new("test", "Test", "a test page")
                .with_keys(&["vibe.tree.flag", "vibe.tree.mode"]),
        ]);
        app.rebuild();
        let form = Form::build(&app).unwrap();
        assert_eq!(form.fields.len(), 2);
        assert!(matches!(form.fields[0].control, FieldControl::Toggle(_)));
        assert!(matches!(form.fields[1].control, FieldControl::Selection(_)));
        assert_eq!(form.focus, 0);
    }

    #[test]
    fn default_write_layer_is_l3_with_project_l1_without() {
        assert_eq!(default_write_layer(true), Layer::L3);
        assert_eq!(default_write_layer(false), Layer::L1);
    }

    #[test]
    fn move_up_down_advances_field_focus() {
        let mut app = app();
        app.open_page = Some("test".into());
        app.registry = super::super::registry::PageRegistry::from(vec![
            super::super::registry::PageDecl::new("test", "Test", "a test page")
                .with_keys(&["vibe.tree.flag", "vibe.tree.mode"]),
        ]);
        app.rebuild();
        let mut form = Form::build(&app).unwrap();
        assert_eq!(form.focus, 0);
        form.move_down();
        assert_eq!(form.focus, 1);
        form.move_down(); // clamped
        assert_eq!(form.focus, 1);
        form.move_up();
        assert_eq!(form.focus, 0);
        form.move_up(); // saturating_sub → 0
        assert_eq!(form.focus, 0);
    }

    #[test]
    fn cycle_write_layer_wraps_l1_l2_l3() {
        let mut app = app();
        app.open_page = Some("test".into());
        app.registry = super::super::registry::PageRegistry::from(vec![
            super::super::registry::PageDecl::new("test", "Test", "a test page")
                .with_keys(&["vibe.tree.flag"]),
        ]);
        app.rebuild();
        let mut form = Form::build(&app).unwrap();
        assert_eq!(form.write_layer, Layer::L3); // project session default
        form.cycle_write_layer();
        assert_eq!(form.write_layer, Layer::L1);
        form.cycle_write_layer();
        assert_eq!(form.write_layer, Layer::L2);
        form.cycle_write_layer();
        assert_eq!(form.write_layer, Layer::L3);
    }

    #[test]
    fn layer_paths_from_env_matches_tree_settings() {
        // The form's env-derived paths must agree with TreeSettings' (same L1/L2/L3).
        let ts = TreeSettings::new();
        let paths = LayerPaths::from_env();
        assert_eq!(paths.path(Layer::L1), ts.layer_path(Layer::L1));
        assert_eq!(paths.path(Layer::L2), ts.layer_path(Layer::L2));
        assert_eq!(paths.path(Layer::L3), ts.layer_path(Layer::L3));
    }
}
