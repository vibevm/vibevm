//! The provenance-edit clear action (PROP-041 §5 `#provenance-edit`). Splits the
//! `clear_focused` Configurable-lifecycle method out of [`super::lifecycle`] so
//! that file stays under the ≤600-line AI-Native budget. From the provenance
//! view the user can **override at a specific layer** (set L3 without touching
//! L2, or clear L3 to fall back to L2) — direct, layer-aware editing. The
//! "clear this layer" action clears the focused field's value at the chosen
//! write-layer, which is the form's layer selector from S2.
//!
//! `clear_focused` mirrors [`Form::apply`](super::lifecycle::Form::apply)'s
//! persist path with a `remove_dotted` in place of `set_dotted`: load the layer
//! table, remove the dotted path, diff against the schema defaults, and install
//! atomically. The focused field's control + baseline are then updated to the
//! **fallback** value (the next layer down that sets the key, or the built-in
//! default) so the surface immediately reflects the layer that now wins.

specmark::scope!("spec://vibevm/modules/vibe-settings/PROP-041#provenance-edit");

use specmark::spec;
use vibe_settings::loader::load_layer;
use vibe_settings::persist::{diff_from_default, write_layer};
use vibe_settings::resolver::{InspectValue, Origin, ResolvedPrefs};
use vibe_settings::schema::Schema;

use super::Form;
use super::control::build_control;
use super::lifecycle::ApplyError;

impl Form {
    /// Clear the focused field's value at the chosen write-layer (PROP-041 §5
    /// `#provenance-edit` — "clear L3 to fall back to L2"). Removes the dotted
    /// path from the layer file (load → remove → diff-from-default → atomic
    /// write — the same persist path [`apply`](super::lifecycle::Form::apply)
    /// uses, parameterised by the write-layer), then updates the focused field's
    /// control + baseline to the fallback value so the surface immediately
    /// reflects the layer that now wins. Refuses a scope-forbidden layer with
    /// the same typed error as `apply`.
    #[spec(implements = "spec://vibevm/modules/vibe-settings/PROP-041#provenance-edit")]
    pub fn clear_focused(
        &mut self,
        schema: &Schema,
        prefs: &ResolvedPrefs,
    ) -> Result<(), ApplyError> {
        let Some(key) = self.focused_field().map(|f| f.key.clone()) else {
            return Ok(());
        };
        let field_idx = self.focus;
        let scope = self.fields[field_idx].meta.scope;
        // #write-layer-choice — refuse a layer the key's scope forbids (same
        // gate as apply; PROP-040 §7 #scope-matrix).
        if !scope.writable_layers().contains(&self.write_layer) {
            return Err(ApplyError::ScopeForbidden {
                key,
                scope: scope.label().to_owned(),
                layer: self.write_layer.label().to_owned(),
            });
        }

        // Write phase: load → remove_dotted → diff → atomic write.
        let layer = self.write_layer;
        let path = self.write_path();
        let mut table = load_layer(path).map_err(|err| ApplyError::Load {
            layer: layer.label().to_owned(),
            message: err.to_string(),
        })?;
        remove_dotted(&mut table, &key);
        let diffed = diff_from_default(&table, schema);
        write_layer(path, &diffed, layer).map_err(|err| ApplyError::Write {
            layer: layer.label().to_owned(),
            message: err.to_string(),
        })?;

        // Update the focused field to the fallback value — the next layer down
        // that sets the key (or the built-in default). `prefs` is the snapshot
        // from before the clear; its `inspect` data still accurately reflects
        // the layer stack, so the fallback computed from it is the new winner.
        let cleared_origin = match layer {
            vibe_settings::loader::Layer::L1 => Origin::L1,
            vibe_settings::loader::Layer::L2 => Origin::L2,
            vibe_settings::loader::Layer::L3 => Origin::L3,
        };
        let fallback = prefs
            .inspect(&key)
            .and_then(|iv| fallback_after_clearing(&iv, cleared_origin))
            .or_else(|| self.fields[field_idx].meta.default.clone())
            .unwrap_or_else(|| toml::Value::String(String::new()));
        let field = &mut self.fields[field_idx];
        field.control = build_control(&field.meta, Some(&fallback));
        field.baseline = fallback;
        Ok(())
    }
}

/// The value that wins after clearing `cleared` — the highest-precedence layer
/// below the cleared one that sets the path (PROP-041 §5 `#provenance-edit`'s
/// "falls back" semantics). Reads the [`InspectValue`]'s per-layer fields in
/// descending precedence (env → default), skipping the cleared origin.
fn fallback_after_clearing(iv: &InspectValue, cleared: Origin) -> Option<toml::Value> {
    for (origin, value) in [
        (Origin::Env, &iv.env),
        (Origin::Cli, &iv.cli),
        (Origin::L3, &iv.l3),
        (Origin::L2, &iv.l2),
        (Origin::L1, &iv.l1),
        (Origin::Default, &iv.default),
    ] {
        if origin == cleared {
            continue;
        }
        if let Some(v) = value {
            return Some(v.clone());
        }
    }
    None
}

/// Remove a dotted path from a TOML table, pruning now-empty intermediate
/// tables on the way back up (PROP-041 §5 `#provenance-edit` — the clear-this-
/// layer write). Recurses through nested tables; a missing path is a no-op (the
/// layer already does not set it — clearing is idempotent).
fn remove_dotted(table: &mut toml::Table, path: &str) {
    let Some((head, rest)) = path.split_once('.') else {
        table.remove(path);
        return;
    };
    if let Some(toml::Value::Table(child)) = table.get_mut(head) {
        remove_dotted(child, rest);
        if child.is_empty() {
            table.remove(head);
        }
    }
}

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

    fn schema() -> Schema {
        let mut s = Schema::new();
        s.register(
            KeyMeta::new("vibe.tree.flag", KeyType::Bool, Scope::User, "a flag")
                .unwrap()
                .with_default(toml::Value::Boolean(true)),
        )
        .unwrap();
        s
    }

    fn paths(dir: &tempfile::TempDir) -> LayerPaths {
        LayerPaths::new(
            dir.path().join("l1.toml"),
            dir.path().join("l2.toml"),
            dir.path().join("settings.local.toml"),
        )
    }

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
    fn clear_focused_removes_the_key_and_falls_back_to_the_default() {
        // Seed an L3 override, then clear it at L3 → falls back to the default.
        let dir = tempdir().unwrap();
        let l3 = dir.path().join("settings.local.toml");
        fs::write(&l3, "# L3 — user-project.\n[vibe.tree]\nflag = false\n").unwrap();
        let raw = LayeredRaw {
            l1: toml::Table::new(),
            l2: toml::Table::new(),
            l3: toml::from_str("vibe.tree.flag = false\n").unwrap(),
        };
        let prefs = resolve(raw, &schema(), toml::Table::new(), toml::Table::new());
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
        assert_eq!(
            form.focused_field().unwrap().control.current_value(),
            toml::Value::Boolean(false)
        );
        form.clear_focused(&schema(), &prefs).unwrap();
        assert_eq!(
            form.focused_field().unwrap().control.current_value(),
            toml::Value::Boolean(true),
            "clearing L3 fell back to the default"
        );
        let written = fs::read_to_string(&l3).unwrap();
        assert!(
            !written.contains("flag = false"),
            "L3 no longer overrides: {written}"
        );
    }

    #[test]
    fn clear_focused_falls_back_to_a_lower_layer_not_the_default() {
        // L2 sets false, L3 overrides true → clearing L3 falls back to L2 (false).
        let dir = tempdir().unwrap();
        let l3 = dir.path().join("settings.local.toml");
        fs::write(&l3, "# L3 — user-project.\n[vibe.tree]\nflag = true\n").unwrap();
        let raw = LayeredRaw {
            l1: toml::Table::new(),
            l2: toml::from_str("vibe.tree.flag = false\n").unwrap(),
            l3: toml::from_str("vibe.tree.flag = true\n").unwrap(),
        };
        let prefs = resolve(raw, &schema(), toml::Table::new(), toml::Table::new());
        assert_eq!(
            prefs.get("vibe.tree.flag"),
            Some(&toml::Value::Boolean(true))
        );
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
        form.clear_focused(&schema(), &prefs).unwrap();
        assert_eq!(
            form.focused_field().unwrap().control.current_value(),
            toml::Value::Boolean(false),
            "clearing L3 fell back to L2 (false)"
        );
    }

    #[test]
    fn clear_focused_refuses_a_scope_forbidden_layer() {
        let dir = tempdir().unwrap();
        let prefs = resolve(
            LayeredRaw::default(),
            &schema(),
            toml::Table::new(),
            toml::Table::new(),
        );
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
        let err = form.clear_focused(&schema(), &prefs).unwrap_err();
        assert!(matches!(err, ApplyError::ScopeForbidden { .. }));
    }
}
