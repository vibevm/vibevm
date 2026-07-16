//! Per-field validation over the form's current edits (PROP-041 §6
//! `#validation-feedback`). Surfaces schema violations inline next to the
//! offending field — a wrong type (an `Int` text field whose string does not
//! parse), or a deprecated key in use — in the warning style with the rule
//! cited. A blocking diagnostic (an invalid value) blocks `apply` for that
//! field; a non-blocking one (deprecation) only warns.
//!
//! The form is a **surface** over PROP-040's data layer (§1
//! `#surface-not-engine`): it reads the key's declared [`KeyMeta`] and the
//! control's [`is_valid`](super::control::FieldControl::is_valid) flag. The
//! schema's own `validate` fn (unknown keys / deprecated) is used for the
//! cross-layer lint-all action ([`super::super::lint`]), not here — a form only
//! edits keys the schema already declares, so unknown-key diagnostics cannot
//! arise from a form edit.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#validation");

use super::Form;

// ── DiagnosticLevel ─────────────────────────────────────────────────────────

/// Whether a field diagnostic blocks `apply` (PROP-041 §6 `#validation-feedback`:
/// "A field in error blocks `apply` for that field"). An invalid value blocks;
/// a deprecation only warns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    /// A non-blocking warning — surfaced inline but does not gate `apply`
    /// (a deprecated key is still legal to set).
    Warning,
    /// A blocking error — `apply` refuses to write this field until the value
    /// parses for its declared type (PROP-041 §6 `#validation-feedback`).
    Error,
}

// ── FieldDiagnostic ─────────────────────────────────────────────────────────

/// One validation diagnostic for one form field (PROP-041 §6
/// `#validation-feedback`). Carries the field index (so the render can attach
/// it to the right row), the dotted path, the level, a human-readable message
/// citing the rule, and the REQ anchor for an inline citation.
#[derive(Debug, Clone)]
pub struct FieldDiagnostic {
    /// The index of the offending field in [`Form::fields`].
    pub field_idx: usize,
    /// The dotted preference path.
    pub key: String,
    /// Whether this blocks `apply`.
    pub level: DiagnosticLevel,
    /// The human-readable message (already cites the migration target or the
    /// expected type).
    pub message: String,
    /// The REQ anchor cited (e.g. `"#validation-feedback"`, `"#deprecation"`).
    pub rule: &'static str,
}

impl Form {
    /// Compute every field diagnostic for the form's current state (PROP-041 §6
    /// `#validation-feedback`). Cheap — iterates the fields once, checking the
    /// control's `is_valid` flag and the key's deprecation metadata. Empty when
    /// every field parses cleanly and no key is deprecated.
    pub fn diagnostics(&self) -> Vec<FieldDiagnostic> {
        let mut out = Vec::new();
        for (idx, field) in self.fields.iter().enumerate() {
            // #validation-feedback — an invalid value (wrong type) is a blocking
            // error. Right now only an Int text field can be invalid; the
            // control's is_valid flag is the single seam.
            if !field.control.is_valid() {
                let value = match &field.control {
                    super::control::FieldControl::Text { field, .. } => field.value().to_owned(),
                    _ => String::from("(current value)"),
                };
                let kind = int_kind_label(&field.control);
                out.push(FieldDiagnostic {
                    field_idx: idx,
                    key: field.key.clone(),
                    level: DiagnosticLevel::Error,
                    message: format!(
                        "`{value}` is not a valid {kind} — enter a value of the declared type"
                    ),
                    rule: "#validation-feedback",
                });
            }
            // #deprecation — a deprecated key in use is a non-blocking warning
            // (the surface guides the user to the replacement; setting the key
            // is still legal).
            if let Some(dep) = &field.meta.deprecated {
                let target = dep.replaced_by.as_deref().unwrap_or("(no replacement)");
                out.push(FieldDiagnostic {
                    field_idx: idx,
                    key: field.key.clone(),
                    level: DiagnosticLevel::Warning,
                    message: format!("deprecated: {} (replaced by `{target}`)", dep.message),
                    rule: "#deprecation",
                });
            }
        }
        out
    }

    /// Whether any field carries a blocking (`Error`) diagnostic (PROP-041 §6
    /// `#validation-feedback` gates `apply`). `apply` itself re-checks each
    /// modified field up front; this is the cheap predicate the render + the
    /// keymap use to show "blocked" without running the full apply path.
    #[must_use]
    pub fn has_blocking_error(&self) -> bool {
        self.diagnostics()
            .iter()
            .any(|d| d.level == DiagnosticLevel::Error)
    }
}

/// The kind label for an invalid-value diagnostic (right now only `Int` can be
/// invalid; mirrors [`super::lifecycle`]'s same-named helper so the wording is
/// identical at the apply gate and the inline warning).
fn int_kind_label(control: &super::control::FieldControl) -> &'static str {
    use super::control::{FieldControl, TextKind};
    match control {
        FieldControl::Text {
            kind: TextKind::Int,
            ..
        } => "int",
        FieldControl::Text {
            kind: TextKind::String,
            ..
        } => "string",
        FieldControl::Text {
            kind: TextKind::Enum,
            ..
        } => "enum",
        _ => "value",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::prefs::tui::form::control::{FieldControl, build_control};
    use crate::commands::prefs::tui::form::{FormField, LayerPaths};
    use std::path::PathBuf;
    use vibe_settings::loader::Layer;
    use vibe_settings::schema::{Deprecation, KeyMeta, KeyType, Scope};

    fn paths() -> LayerPaths {
        LayerPaths::new(
            PathBuf::from("/tmp/l1.toml"),
            PathBuf::from("/tmp/l2.toml"),
            PathBuf::from("/tmp/l3.toml"),
        )
    }

    fn field(key: &str, ty: KeyType, scope: Scope, resolved: Option<&toml::Value>) -> FormField {
        let meta = KeyMeta::new(key, ty, scope, "a test setting")
            .unwrap()
            .with_default(resolved.cloned().unwrap_or(toml::Value::Boolean(false)));
        let baseline = resolved
            .cloned()
            .or_else(|| meta.default.clone())
            .unwrap_or_else(|| toml::Value::String(String::new()));
        let control = build_control(&meta, resolved);
        FormField {
            key: key.to_owned(),
            meta,
            control,
            baseline,
        }
    }

    fn form_with(fields: Vec<FormField>) -> Form {
        Form::for_test("Page", "a page", fields, Layer::L3, paths())
    }

    #[test]
    fn clean_form_yields_no_diagnostics() {
        let form = form_with(vec![field(
            "vibe.tree.flag",
            KeyType::Bool,
            Scope::User,
            Some(&toml::Value::Boolean(true)),
        )]);
        assert!(form.diagnostics().is_empty());
        assert!(!form.has_blocking_error());
    }

    #[test]
    fn invalid_int_field_is_a_blocking_error() {
        let mut f = field(
            "vibe.tree.tier",
            KeyType::Int,
            Scope::User,
            Some(&toml::Value::Integer(3)),
        );
        // Type a non-numeric value into the Int field.
        if let FieldControl::Text { field, .. } = &mut f.control {
            field.backspace();
            field.type_char('x');
        }
        let form = form_with(vec![f]);
        let diags = form.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, DiagnosticLevel::Error);
        assert_eq!(diags[0].rule, "#validation-feedback");
        assert!(diags[0].message.contains("int"));
        assert!(form.has_blocking_error());
    }

    #[test]
    fn deprecated_key_is_a_non_blocking_warning() {
        let mut f = field(
            "node.sort",
            KeyType::String,
            Scope::User,
            Some(&toml::Value::String("name".into())),
        );
        f.meta = f
            .meta
            .clone()
            .with_deprecation(Deprecation::with_replacement("use tree.sort", "tree.sort"));
        let form = form_with(vec![f]);
        let diags = form.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, DiagnosticLevel::Warning);
        assert_eq!(diags[0].rule, "#deprecation");
        assert!(diags[0].message.contains("tree.sort"));
        assert!(
            !form.has_blocking_error(),
            "deprecation does not block apply"
        );
    }

    #[test]
    fn blocking_and_warning_can_coexist_for_different_fields() {
        let mut bad_int = field(
            "vibe.tree.tier",
            KeyType::Int,
            Scope::User,
            Some(&toml::Value::Integer(3)),
        );
        if let FieldControl::Text { field, .. } = &mut bad_int.control {
            field.type_char('x');
        }
        let mut dep = field(
            "node.sort",
            KeyType::String,
            Scope::User,
            Some(&toml::Value::String("name".into())),
        );
        dep.meta = dep
            .meta
            .clone()
            .with_deprecation(Deprecation::with_replacement("use tree.sort", "tree.sort"));
        let form = form_with(vec![bad_int, dep]);
        let diags = form.diagnostics();
        assert_eq!(diags.len(), 2);
        assert!(form.has_blocking_error());
    }
}
