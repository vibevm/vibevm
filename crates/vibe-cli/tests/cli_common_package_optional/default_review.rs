//! Qualification stops at the earliest real public-route gap, not a fake review.
use super::*;

#[test]
fn omitted_default_selection_refuses_before_graph_effects_while_explicit_control_works() {
    let fixture = Fixture::new(false);
    let path = fixture.project.join("vibe.toml");
    let original = fs::read_to_string(&path).unwrap();
    let explicit = "configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}}}";
    assert!(original.contains(explicit));
    let automatic = original.replace(
        explicit,
        "configuration={policy='use-declared-defaults',default_features=false}",
    );
    write(&path, &automatic);
    let manifest = Manifest::read(&path).unwrap();
    let input = manifest.requires.configurations.values().next().unwrap();
    assert!(
        input.selections.is_empty(),
        "omitted choice must not be expanded to an explicit answer"
    );
    let output = fixture.install();
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "automatic defaults unexpectedly admitted: {diagnostic}"
    );
    assert!(
        diagnostic.contains("automatic/default/profile basis is not connected"),
        "{diagnostic}"
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), automatic);
    assert!(!fixture.project.join("vibe.lock").exists());
    assert!(!fixture.slot(DRIVER).exists());
    assert!(!fixture.slot(EXTRA).exists());
    assert!(!fixture.project.join("effects.log").exists());
    // Public fresh-process control uses exactly the same definition and selected
    // default meaning, but explicit authored intent. It does not prove review.
    write(&path, original);
    fixture.install_ok();
    assert!(fixture.slot(DRIVER).exists());
    assert!(!fixture.slot(EXTRA).exists());
    let before = fs::read(fixture.project.join("vibe.lock")).unwrap();
    fixture.install_ok();
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), before);
}
