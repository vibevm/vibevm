//! The Configurable lifecycle — `is_modified` / `apply` / `reset` (PROP-041 §4
//! `#configurable-lifecycle`, `#write-layer-choice`). The clean-room IntelliJ
//! `Configurable` contract: a cheap `is_modified()` (form vs the captured
//! baseline), `apply()` (writes via the persist path P9a's `TreeSettings` uses,
//! to the chosen write-layer; throws a typed error on a scope-forbidden layer or
//! an invalid value), `reset()` (prefs → form). `apply` is gated on
//! `is_modified()` — the form never writes a no-op. The `clear_focused`
//! provenance-edit method (§5 `#provenance-edit`) lives in [`super::provenance_edit`].
//!
//! ## Write path (#write-layer-choice)
//!
//! `apply` mirrors [`TreeSettings::try_set`] exactly, parameterised by the
//! chosen layer: load the layer table ([`load_layer`]), set each modified dotted
//! path ([`set_dotted`]), diff against the schema defaults
//! ([`diff_from_default`] — a key set to its default is dropped from the file),
//! and install atomically ([`write_layer`] — sibling `.tmp` + rename). Writing
//! to a layer the key's `scope` forbids is refused up front with a typed error
//! citing PROP-040 §7 (the VSCode `.vscode-overwrites-contributors` pain).
//!
//! [`TreeSettings::try_set`]: crate::commands::tree::tui::settings::TreeSettings::try_set
//! [`load_layer`]: vibe_settings::loader::load_layer
//! [`set_dotted`]: crate::commands::tree::tui::settings::set_dotted
//! [`diff_from_default`]: vibe_settings::persist::diff_from_default
//! [`write_layer`]: vibe_settings::persist::write_layer

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#configurable-lifecycle");

use specmark::spec;
use vibe_settings::loader::load_layer;
use vibe_settings::persist::{diff_from_default, write_layer};
use vibe_settings::resolver::ResolvedPrefs;
use vibe_settings::schema::Schema;

use crate::commands::tree::tui::settings::set_dotted;

use super::Form;
use super::control::TextKind;

impl Form {
    /// Whether any field's editable value differs from its resolved baseline
    /// (PROP-041 §4 `#configurable-lifecycle`). Cheap: compares each control's
    /// current value to the captured baseline; no resolver call, no I/O.
    #[spec(implements = "spec://vibevm/modules/vibe-settings/PROP-041#configurable-lifecycle")]
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.fields
            .iter()
            .any(|f| f.control.current_value() != f.baseline)
    }

    /// Write every modified key to the chosen write-layer (PROP-041 §4
    /// `#configurable-lifecycle`, `#write-layer-choice`). Refuses a
    /// scope-forbidden layer or an invalid value with a typed error **before**
    /// writing anything (atomic intent — the form never half-applies), then
    /// writes through the same load → set-dotted → diff-from-default →
    /// atomic-write path P9a's `TreeSettings` uses. On success, updates each
    /// field's baseline so `is_modified` reads false post-apply.
    #[spec(implements = "spec://vibevm/modules/vibe-settings/PROP-041#write-layer-choice")]
    pub fn apply(&mut self, schema: &Schema) -> Result<(), ApplyError> {
        // #configurable-lifecycle — apply is gated on is_modified; never write a no-op.
        if !self.is_modified() {
            return Ok(());
        }
        // Validate every modified field up front (scope + value shape) so the
        // write phase cannot fail mid-way and leave a partial layer on disk.
        for field in &self.fields {
            if field.control.current_value() == field.baseline {
                continue; // unmodified — skipped.
            }
            // #write-layer-choice — refuse a layer the key's scope forbids
            // (PROP-040 §7 #scope-matrix).
            if !field
                .meta
                .scope
                .writable_layers()
                .contains(&self.write_layer)
            {
                return Err(ApplyError::ScopeForbidden {
                    key: field.key.clone(),
                    scope: field.meta.scope.label().to_owned(),
                    layer: self.write_layer.label().to_owned(),
                });
            }
            // An Int text field whose string is not an integer is invalid (§6
            // #validation-feedback gates apply; the inline-error render is a
            // later phase — the typed error carries the reason).
            if !field.control.is_valid() {
                let value = match &field.control {
                    super::control::FieldControl::Text { field, .. } => field.value().to_owned(),
                    _ => String::new(),
                };
                return Err(ApplyError::InvalidValue {
                    key: field.key.clone(),
                    value,
                    kind: int_kind_label(&field.control),
                });
            }
        }

        // Write phase — mirror TreeSettings::try_set against the chosen layer.
        let layer = self.write_layer;
        let path = self.write_path();
        let mut table = load_layer(path).map_err(|err| ApplyError::Load {
            layer: layer.label().to_owned(),
            message: err.to_string(),
        })?;
        for field in &self.fields {
            let current = field.control.current_value();
            if current != field.baseline {
                set_dotted(&mut table, &field.key, current);
            }
        }
        let diffed = diff_from_default(&table, schema);
        write_layer(path, &diffed, layer).map_err(|err| ApplyError::Write {
            layer: layer.label().to_owned(),
            message: err.to_string(),
        })?;

        // Update baselines so is_modified reads false post-apply (the form's
        // model is now the applied values — the Configurable contract).
        for field in &mut self.fields {
            field.baseline = field.control.current_value();
        }
        Ok(())
    }

    /// Reload every field from the resolved prefs (PROP-041 §4
    /// `#configurable-lifecycle` — model → form). Rebuilds each control from the
    /// key's metadata + the fresh resolved value and resets the focus to the
    /// first field.
    #[spec(implements = "spec://vibevm/modules/vibe-settings/PROP-041#configurable-lifecycle")]
    pub fn reset(&mut self, prefs: &ResolvedPrefs) {
        for field in &mut self.fields {
            let resolved = prefs.get(&field.key);
            field.control = super::control::build_control(&field.meta, resolved);
            field.baseline = resolved
                .cloned()
                .or_else(|| field.meta.default.clone())
                .unwrap_or_else(|| toml::Value::String(String::new()));
        }
        self.focus = 0;
    }
}

/// The kind label for an invalid-value error (right now only `Int` can be
/// invalid; kept as a fn so adding a kind is one line).
fn int_kind_label(control: &super::control::FieldControl) -> &'static str {
    match control {
        super::control::FieldControl::Text {
            kind: TextKind::Int,
            ..
        } => "int",
        super::control::FieldControl::Text {
            kind: TextKind::String,
            ..
        } => "string",
        super::control::FieldControl::Text {
            kind: TextKind::Enum,
            ..
        } => "enum",
        _ => "value",
    }
}

// ── ApplyError ───────────────────────────────────────────────────────────────

/// Why a form `apply` failed (PROP-041 §4 `#write-layer-choice`,
/// `#configurable-lifecycle`). Each variant cites the governing REQ anchor so a
/// command-edge diagnostic can point the reader at the contract clause. Hand-
/// rolled `Display`/`Error` to match the tree TUI's `SetError` style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// The chosen write-layer is forbidden for the key's `scope` (PROP-040 §7
    /// `#scope-matrix`, PROP-041 §4 `#write-layer-choice`). Carries the key, the
    /// scope label, and the layer label.
    ScopeForbidden {
        /// The dotted preference path.
        key: String,
        /// The key's scope label (e.g. `"machine"`).
        scope: String,
        /// The refused layer label (e.g. `"L3"`).
        layer: String,
    },

    /// A text field's typed string does not parse for its declared type (an `Int`
    /// field that is not an integer). PROP-041 §6 `#validation-feedback` gates
    /// apply on a valid value; the inline-error render is a later phase.
    InvalidValue {
        /// The dotted preference path.
        key: String,
        /// The typed string that failed to parse.
        value: String,
        /// The expected kind label (`"int"` / `"string"` / `"enum"`).
        kind: &'static str,
    },

    /// Loading the target layer file failed (a present-but-unreadable file; a
    /// missing file is an empty table, never this error — PROP-040 §3
    /// `#missing-is-default`).
    Load {
        /// The layer label.
        layer: String,
        /// The underlying error's `Display`.
        message: String,
    },

    /// Writing the diffed layer file failed (I/O or serialisation).
    Write {
        /// The layer label.
        layer: String,
        /// The underlying error's `Display`.
        message: String,
    },
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::ScopeForbidden { key, scope, layer } => write!(
                f,
                "cannot write `{key}` to layer {layer}: scope `{scope}` forbids it \
                 (violates spec://vibevm/modules/vibe-settings/PROP-040#scope-matrix; \
                 fix: switch the write-layer to one the scope allows, or change the key's scope)"
            ),
            ApplyError::InvalidValue { key, value, kind } => write!(
                f,
                "invalid value for `{key}`: `{value}` is not a valid {kind} \
                 (violates spec://vibevm/modules/vibe-settings/PROP-041#validation; \
                 fix: enter a value of the declared type)"
            ),
            ApplyError::Load { layer, message } => write!(
                f,
                "could not load the {layer} settings file: {message} \
                 (violates spec://vibevm/modules/vibe-settings/PROP-040#diff-from-default)"
            ),
            ApplyError::Write { layer, message } => write!(
                f,
                "could not write the {layer} settings file: {message} \
                 (violates spec://vibevm/modules/vibe-settings/PROP-040#diff-from-default)"
            ),
        }
    }
}

impl std::error::Error for ApplyError {}

#[cfg(test)]
mod tests {
    use super::super::control::{FieldControl, build_control};
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use vibe_settings::loader::{Layer, LayeredRaw};
    use vibe_settings::resolver::resolve;
    use vibe_settings::schema::{KeyMeta, KeyType, Scope};

    use super::super::LayerPaths;

    /// A schema with a Bool key (Scope::User) + an Int key (Scope::User).
    fn schema() -> Schema {
        let mut s = Schema::new();
        s.register(
            KeyMeta::new("vibe.tree.flag", KeyType::Bool, Scope::User, "a flag")
                .unwrap()
                .with_default(toml::Value::Boolean(true)),
        )
        .unwrap();
        s.register(
            KeyMeta::new("vibe.tree.tier", KeyType::Int, Scope::User, "tier")
                .unwrap()
                .with_default(toml::Value::Integer(3)),
        )
        .unwrap();
        s
    }

    fn prefs() -> ResolvedPrefs {
        resolve(
            LayeredRaw::default(),
            &schema(),
            toml::Table::new(),
            toml::Table::new(),
        )
    }

    fn paths(dir: &tempfile::TempDir) -> LayerPaths {
        LayerPaths::new(
            dir.path().join("l1.toml"),
            dir.path().join("l2.toml"),
            dir.path().join("settings.local.toml"),
        )
    }

    /// A field built from a meta + resolved value (test helper).
    fn field(
        key: &str,
        ty: KeyType,
        scope: Scope,
        resolved: Option<&toml::Value>,
    ) -> super::super::FormField {
        let meta = KeyMeta::new(key, ty, scope, "a test setting")
            .unwrap()
            .with_default(resolved.cloned().unwrap_or(toml::Value::Boolean(false)));
        let baseline = resolved
            .cloned()
            .or_else(|| meta.default.clone())
            .unwrap_or_else(|| toml::Value::String(String::new()));
        let control = build_control(&meta, resolved);
        super::super::FormField {
            key: key.to_owned(),
            meta,
            control,
            baseline,
        }
    }

    #[test]
    fn is_modified_is_false_initially_true_after_an_edit() {
        let dir = tempdir().unwrap();
        let mut form = Form::for_test(
            "Palette",
            "a page",
            vec![field(
                "vibe.tree.flag",
                KeyType::Bool,
                Scope::User,
                Some(&toml::Value::Boolean(true)),
            )],
            Layer::L3,
            paths(&dir),
        );
        assert!(!form.is_modified(), "not modified at build");
        if let FieldControl::Toggle(b) = &mut form.fields[0].control {
            *b = false;
        }
        assert!(form.is_modified(), "modified after an edit");
    }

    #[test]
    fn apply_writes_through_persist_to_the_chosen_layer() {
        let dir = tempdir().unwrap();
        let l3 = dir.path().join("settings.local.toml");
        let mut form = Form::for_test(
            "Palette",
            "a page",
            vec![field(
                "vibe.tree.flag",
                KeyType::Bool,
                Scope::User,
                Some(&toml::Value::Boolean(true)),
            )],
            Layer::L3,
            paths(&dir),
        );
        if let FieldControl::Toggle(b) = &mut form.fields[0].control {
            *b = false;
        }
        assert!(form.is_modified());
        form.apply(&schema()).unwrap();
        let written = fs::read_to_string(&l3).unwrap();
        assert!(written.contains("flag = false"), "L3 carries: {written}");
        assert!(written.contains("L3"), "role-marker present");
        assert!(!form.is_modified(), "baseline updated post-apply");
    }

    #[test]
    fn apply_is_a_noop_when_not_modified() {
        let dir = tempdir().unwrap();
        let l3 = dir.path().join("settings.local.toml");
        let mut form = Form::for_test(
            "Palette",
            "a page",
            vec![field(
                "vibe.tree.flag",
                KeyType::Bool,
                Scope::User,
                Some(&toml::Value::Boolean(true)),
            )],
            Layer::L3,
            paths(&dir),
        );
        form.apply(&schema()).unwrap();
        assert!(!l3.exists(), "no no-op write");
    }

    #[test]
    fn apply_refuses_a_scope_forbidden_layer_citing_section_7() {
        let dir = tempdir().unwrap();
        let mut form = Form::for_test(
            "Machine",
            "a page",
            vec![field(
                "vibe.tree.machine_path",
                KeyType::String,
                Scope::Machine,
                Some(&toml::Value::String("/usr/bin".into())),
            )],
            Layer::L3,
            paths(&dir),
        );
        if let FieldControl::Text { field, .. } = &mut form.fields[0].control {
            field.type_char('!');
        }
        let err = form.apply(&schema()).unwrap_err();
        let msg = err.to_string();
        match err {
            ApplyError::ScopeForbidden { key, scope, layer } => {
                assert_eq!(key, "vibe.tree.machine_path");
                assert_eq!(scope, "machine");
                assert_eq!(layer, "L3");
            }
            other => panic!("expected ScopeForbidden, got {other:?}"),
        }
        assert!(msg.contains("scope-matrix"), "cites the REQ: {msg}");
    }

    #[test]
    fn apply_refuses_an_invalid_int_value() {
        let dir = tempdir().unwrap();
        let mut form = Form::for_test(
            "Tier",
            "a page",
            vec![field(
                "vibe.tree.tier",
                KeyType::Int,
                Scope::User,
                Some(&toml::Value::Integer(3)),
            )],
            Layer::L3,
            paths(&dir),
        );
        if let FieldControl::Text { field, .. } = &mut form.fields[0].control {
            field.backspace();
            field.type_char('x');
        }
        let err = form.apply(&schema()).unwrap_err();
        assert!(matches!(err, ApplyError::InvalidValue { .. }));
        assert!(err.to_string().contains("vibe.tree.tier"));
    }

    #[test]
    fn reset_reverts_edits_to_the_resolved_values() {
        let prefs = prefs();
        let dir = tempdir().unwrap();
        let mut form = Form::for_test(
            "Palette",
            "a page",
            vec![field(
                "vibe.tree.flag",
                KeyType::Bool,
                Scope::User,
                Some(&toml::Value::Boolean(true)),
            )],
            Layer::L3,
            paths(&dir),
        );
        if let FieldControl::Toggle(b) = &mut form.fields[0].control {
            *b = false;
        }
        assert!(form.is_modified());
        form.reset(&prefs);
        assert!(!form.is_modified(), "reset reverted the edit");
        assert!(matches!(form.fields[0].control, FieldControl::Toggle(true)));
    }

    #[test]
    fn closed_set_selection_resets_to_the_resolved_choice() {
        let dir = tempdir().unwrap();
        let mut s = Schema::new();
        s.register(
            KeyMeta::new("vibe.tree.mode", KeyType::String, Scope::User, "mode")
                .unwrap()
                .with_default(toml::Value::String("all".into())),
        )
        .unwrap();
        let prefs = resolve(
            LayeredRaw::default(),
            &s,
            toml::Table::new(),
            toml::Table::new(),
        );
        let mut form = Form::for_test(
            "Mode",
            "a page",
            vec![field(
                "vibe.tree.mode",
                KeyType::String,
                Scope::User,
                Some(&toml::Value::String("all".into())),
            )],
            Layer::L3,
            paths(&dir),
        );
        form.fields[0].control.activate();
        assert_eq!(
            form.fields[0].control.current_value(),
            toml::Value::String("sub-tables".into())
        );
        assert!(form.is_modified());
        form.reset(&prefs);
        assert_eq!(
            form.fields[0].control.current_value(),
            toml::Value::String("all".into()),
            "reset restored 'all'"
        );
        assert!(!form.is_modified());
    }

    #[test]
    fn diff_drops_a_key_set_back_to_its_default() {
        let dir = tempdir().unwrap();
        let l3 = dir.path().join("settings.local.toml");
        fs::write(&l3, "# L3 — user-project.\n[vibe.tree]\nflag = false\n").unwrap();
        let mut form = Form::for_test(
            "Palette",
            "a page",
            vec![field(
                "vibe.tree.flag",
                KeyType::Bool,
                Scope::User,
                Some(&toml::Value::Boolean(false)),
            )],
            Layer::L3,
            paths(&dir),
        );
        if let FieldControl::Toggle(b) = &mut form.fields[0].control {
            *b = true;
        }
        form.apply(&schema()).unwrap();
        let written = fs::read_to_string(&l3).unwrap();
        assert!(
            !written.contains("flag = true"),
            "default-valued key dropped: {written}"
        );
    }
}
