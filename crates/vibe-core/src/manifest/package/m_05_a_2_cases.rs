use crate::manifest::{FeaturesTable, Manifest};
use specmark::verifies;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-SYNTAX")]
fn qualified_activation_strings_roundtrip_unchanged() {
    let raw = "default = [\"dep:org.example/dep\", \"org.example/dep/fast\", \"org.example/dep?/fast\"]\na = []\nb = []\n[exclusive]\nchoice = [\"a\", \"b\"]";
    let table: FeaturesTable = toml::from_str(raw).unwrap();
    assert_eq!(
        table.defaults(),
        [
            "dep:org.example/dep",
            "org.example/dep/fast",
            "org.example/dep?/fast"
        ]
    );
    assert_eq!(table.exclusive["choice"], ["a", "b"]);
    assert_eq!(
        table,
        toml::from_str::<FeaturesTable>(&toml::to_string(&table).unwrap()).unwrap()
    );
}

fn manifest(body: &str) -> String {
    format!(
        "[package]\ngroup = \"org.example\"\nname = \"a\"\nkind = \"flow\"\nversion = \"1.0.0\"\n{body}"
    )
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-SYNTAX")]
fn qualified_dependency_key_preserves_identity_and_version() {
    let m = Manifest::parse_str(&manifest(
        "[requires.packages]\n\"org.example/dep\" = \"^2\"",
    ))
    .unwrap();
    let dep = &m.requires.packages[0];
    assert_eq!(dep.group.as_ref().unwrap().as_str(), "org.example");
    assert_eq!(dep.name, "dep");
    assert!(dep.version.matches(&semver::Version::new(2, 0, 0)));
    assert!(!dep.version.matches(&semver::Version::new(1, 0, 0)));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-SYNTAX")]
fn inline_dependency_feature_options_are_unsupported() {
    for option in [
        "features = [\"fast\"]",
        "optional = true",
        "default-features = false",
    ] {
        let raw = manifest(&format!(
            "[requires.packages]\n\"org.example/dep\" = {{ version = \"^2\", {option} }}"
        ));
        assert!(
            Manifest::parse_str(&raw).is_err(),
            "unexpectedly accepted {option}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-CONDITIONAL")]
fn negated_target_and_requires_any_preserve_constraints() {
    let m = Manifest::parse_str(&manifest("[[requires_any]]\none_of = [\"org.example/b@^2\", \"org.example/c\"]\n[target.\"context(not flow:b)\".dependencies.packages]\n\"org.example/b\" = \"^2\"")).unwrap();
    assert_eq!(m.requires_any[0].one_of.len(), 2);
    let wire = toml::Value::try_from(&m).unwrap();
    let target: crate::manifest::ConditionalTarget = wire["target"]["context(not flow:b)"]
        .clone()
        .try_into()
        .unwrap();
    let dep = &target.dependencies.packages[0];
    assert_eq!(dep.name, "b");
    assert!(!dep.version.matches(&semver::Version::new(1, 0, 0)));
}
