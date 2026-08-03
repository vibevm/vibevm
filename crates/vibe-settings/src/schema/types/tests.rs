use super::*;
use specmark::spec;

#[spec(
    deviates = "discipline://rust-ai-native-lang/guide#bans-and-escape-hatches",
    reason = "test-support helper in a #[cfg(test)] module file: the panic IS the assertion"
)]
fn key(path: &str, ty: KeyType, scope: Scope) -> KeyMeta {
    KeyMeta::new(path, ty, scope, "a test setting").unwrap()
}

// ── KeyType / Scope / Applies / MergeStrategy metadata ───────────────────

#[test]
fn key_type_labels_match_spec_spellings() {
    assert_eq!(KeyType::Bool.label(), "bool");
    assert_eq!(KeyType::Int.label(), "int");
    assert_eq!(KeyType::String.label(), "string");
    assert_eq!(KeyType::Enum.label(), "enum");
    assert_eq!(KeyType::Array.label(), "array");
    assert_eq!(KeyType::Table.label(), "table");
    assert_eq!(KeyType::Table.to_string(), "table");
}

#[test]
fn scope_writable_layers_encode_the_matrix() {
    // PROP-040 §7 #scope-matrix — encoded once, right here.
    assert_eq!(
        Scope::User.writable_layers(),
        &[Layer::L1, Layer::L2, Layer::L3]
    );
    assert_eq!(Scope::Machine.writable_layers(), &[Layer::L1]);
    assert_eq!(Scope::Project.writable_layers(), &[Layer::L2, Layer::L3]);
    assert_eq!(Scope::TeamOnly.writable_layers(), &[Layer::L2]);
}

#[test]
fn scope_labels_and_display() {
    assert_eq!(Scope::User.label(), "user");
    assert_eq!(Scope::Machine.label(), "machine");
    assert_eq!(Scope::Project.label(), "project");
    assert_eq!(Scope::TeamOnly.label(), "team-only");
    assert_eq!(Scope::TeamOnly.to_string(), "team-only");
}

#[test]
fn applies_default_is_live() {
    assert_eq!(Applies::default(), Applies::Live);
    assert_eq!(Applies::Live.label(), "live");
    assert_eq!(Applies::Reload.label(), "reload");
    assert_eq!(Applies::Restart.label(), "restart");
}

#[test]
fn merge_strategy_default_is_replace() {
    assert_eq!(MergeStrategy::default(), MergeStrategy::Replace);
    assert_eq!(MergeStrategy::Replace.label(), "replace");
    assert_eq!(MergeStrategy::Append.label(), "append");
    assert_eq!(MergeStrategy::Prepend.label(), "prepend");
    assert_eq!(MergeStrategy::MergeByKey.label(), "merge-by-key");
}

// ── Deprecation ────────────────────────────────────────────────────────

#[test]
fn deprecation_constructors() {
    let d = Deprecation::new("retired");
    assert_eq!(d.replaced_by, None);
    assert_eq!(d.message, "retired");

    let d = Deprecation::with_replacement("use b", "tree.b");
    assert_eq!(d.replaced_by.as_deref(), Some("tree.b"));
}

// ── KeyMeta construction ───────────────────────────────────────────────

#[test]
fn key_meta_new_sets_defaults_for_optional_fields() {
    let k = key("tree.palette", KeyType::String, Scope::User);
    assert_eq!(k.path, "tree.palette");
    assert_eq!(k.key_type, KeyType::String);
    assert_eq!(k.scope, Scope::User);
    assert!(k.default.is_none());
    assert_eq!(k.applies, Applies::Live);
    assert_eq!(k.merge, MergeStrategy::Replace);
    assert!(k.deprecated.is_none());
    assert!(!k.restricted);
}

#[test]
fn key_meta_builder_chains() {
    let k = KeyMeta::new("tree.fold", KeyType::Bool, Scope::User, "fold")
        .unwrap()
        .with_default(toml::Value::Boolean(true))
        .with_applies(Applies::Restart)
        .with_merge(MergeStrategy::Append)
        .with_deprecation(Deprecation::new("old"))
        .restricted();
    assert!(matches!(k.default, Some(toml::Value::Boolean(true))));
    assert_eq!(k.applies, Applies::Restart);
    assert_eq!(k.merge, MergeStrategy::Append);
    assert!(k.deprecated.is_some());
    assert!(k.restricted);
}

#[test]
fn key_meta_rejects_empty_description_citing_schema_fields() {
    let err = KeyMeta::new("tree.x", KeyType::Bool, Scope::User, "   ").unwrap_err();
    // Capture the Display string before the match moves `err`'s `path`.
    let msg = err.to_string();
    match err {
        SchemaError::EmptyDescription { path } => {
            assert_eq!(path, "tree.x");
        }
        other => panic!("expected EmptyDescription, got {other:?}"),
    }
    // Diagnostic points at the REQ.
    assert!(msg.contains("schema-fields"));
}

#[test]
fn key_meta_rejects_empty_path() {
    let err = KeyMeta::new("   ", KeyType::Bool, Scope::User, "doc").unwrap_err();
    assert!(matches!(err, SchemaError::EmptyPath));
}
