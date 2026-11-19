use super::*;
use crate::manifest::Manifest;

#[test]
fn layer_declarations_roundtrip_and_default_to_protected() {
    let text = "[project]\nname='demo'\nversion='1.0.0'\n[layers]\nschema=1\n[[layers.targets]]\nid='width'\nsource='spec://org.demo/project/common/STYLE#width'\n";
    let manifest = Manifest::parse_str(text).unwrap();
    assert_eq!(
        manifest.layers.as_ref().unwrap().targets[0].policy,
        LayerPolicy::Protected
    );
    let rendered = manifest.render_with_comments(None).unwrap();
    assert_eq!(Manifest::parse_str(&rendered).unwrap(), manifest);
}
#[test]
fn layer_metadata_refuses_unknown_fields_unsupported_epochs_and_protected_grants() {
    for tail in [
        "schema=2",
        "schema=1\nunknown=true",
        "schema=1\n[[layers.targets]]\nid='a'\nsource='arbitrary/path#x'",
        "schema=1\n[[layers.targets]]\nid='a'\nsource='spec://org.demo/a/common/A#x'\npolicy='overridable'\ncategory='safety'",
    ] {
        let input = format!("[project]\nname='demo'\nversion='1.0.0'\n[layers]\n{tail}\n");
        assert!(Manifest::parse_str(&input).is_err(), "{tail}");
    }
}
