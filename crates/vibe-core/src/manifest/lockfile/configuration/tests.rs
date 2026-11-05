//! Lock8 stores original requests separately and validates selected associations.
use super::*;
use crate::manifest::{ConfigurationIntentDecl, Manifest};
use std::collections::BTreeMap;

fn rich() -> Lockfile {
    let manifest=Manifest::parse_str("[package]\ngroup='org.test'\nname='owner'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'org.test/dep'={version='^1',optional=true}\n[[configuration.features]]\nid='enable'\nname='Enable'\neffects=[{kind='dependency',target='org.test/dep'}]").unwrap();
    let input: ConfigurationIntentDecl =
        toml::from_str("policy='require-selection'\ndefault_features=false\nfeatures=['enable']")
            .unwrap();
    let configuration =
        PackageConfiguration::evaluate(&manifest, &BTreeMap::from([("./vibe.toml".into(), input)]))
            .unwrap();
    let mut lock:Lockfile=toml::from_str("[meta]\nschema_version=8\ngenerated_by='test'\ngenerated_at='fixed'\nroot_dependencies=['org.test/owner@^1']\n[[package]]\nkind='flow'\ngroup='org.test'\nname='owner'\nversion='1.0.0'\nsource_url='file:///fixture'\ncontent_hash='sha256:aa'\nfeatures=['enable']\ndependencies=['org.test/dep@=1.0.0']\n[[package]]\nkind='flow'\ngroup='org.test'\nname='dep'\nversion='1.0.0'\nsource_url='file:///fixture'\ncontent_hash='sha256:bb'").unwrap();
    lock.configurations
        .insert("org.test/owner".into(), configuration);
    lock
}
fn read(lock: &Lockfile) -> Result<Lockfile> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("vibe.lock");
    std::fs::write(&path, toml::to_string(lock).unwrap()).unwrap();
    Lockfile::read(path)
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn rich_roundtrip_preserves_original_and_uses_eight_while_legacy_stays_seven() {
    let lock = rich();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("vibe.lock");
    lock.write(&path).unwrap();
    assert_eq!(Lockfile::read(&path).unwrap(), lock);
    let legacy = Lockfile::empty("test", "fixed");
    legacy.write(&path).unwrap();
    assert_eq!(Lockfile::read(&path).unwrap(), legacy);
    assert_eq!(legacy.meta.schema_version, 7);
    assert!(
        !std::fs::read_to_string(path)
            .unwrap()
            .contains("configurations")
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn original_version_and_kind_constraints_bind_the_exact_selected_target() {
    let original = rich();
    let mut constraint = original.clone();
    constraint
        .configurations
        .get_mut("org.test/owner")
        .unwrap()
        .declarations
        .packages[0]
        .version = VersionSpec::parse("^2").unwrap();
    assert!(
        read(&constraint)
            .unwrap_err()
            .to_string()
            .contains("original version/kind")
    );
    let mut kind = original.clone();
    kind.configurations
        .get_mut("org.test/owner")
        .unwrap()
        .declarations
        .packages[0]
        .kind = Some(PackageKind::App);
    assert!(
        read(&kind)
            .unwrap_err()
            .to_string()
            .contains("original version/kind")
    );
    let mut effect_kind = original.clone();
    effect_kind
        .configurations
        .get_mut("org.test/owner")
        .unwrap()
        .definition
        .features[0]
        .effects[0] = crate::manifest::ConfigurationEffect::Dependency {
        target: "app:org.test/dep".into(),
    };
    assert!(
        read(&effect_kind)
            .unwrap_err()
            .to_string()
            .contains("effect kind")
    );
    let mut unselected = original.clone();
    let definition = &mut unselected
        .configurations
        .get_mut("org.test/owner")
        .unwrap()
        .definition;
    let mut extra = definition.features[0].clone();
    extra.id = "unused".into();
    extra.effects = vec![crate::manifest::ConfigurationEffect::Dependency {
        target: "app:org.test/dep".into(),
    }];
    definition.features.push(extra);
    assert!(
        read(&unselected).is_ok(),
        "unselected effect does not assert foreign target kind"
    );
    let mut compatible = original.clone();
    compatible
        .configurations
        .get_mut("org.test/owner")
        .unwrap()
        .definition
        .features[0]
        .effects[0] = crate::manifest::ConfigurationEffect::Dependency {
        target: "flow:org.test/dep".into(),
    };
    assert!(read(&compatible).is_ok());
    let mut wildcard = original.clone();
    wildcard.packages[0].dependencies[0].version = VersionSpec::Latest;
    assert!(
        read(&wildcard)
            .unwrap_err()
            .to_string()
            .contains("exactly pin")
    );
    let mut wrong = original;
    wrong.packages[0].dependencies[0].version = VersionSpec::parse("=2.0.0").unwrap();
    assert!(
        read(&wrong)
            .unwrap_err()
            .to_string()
            .contains("exactly pin")
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#LF-CONFIGURED-STATE"
)]
fn rich_under_old_schema_and_dangling_or_mismatched_association_refuse() {
    for epoch in [6, 7] {
        let mut lock = rich();
        lock.meta.schema_version = epoch;
        assert!(matches!(
            read(&lock),
            Err(crate::Error::InvalidLockfile { .. })
        ));
    }
    let mut lock = rich();
    let record = lock.configurations.remove("org.test/owner").unwrap();
    lock.configurations
        .insert("org.test/missing".into(), record);
    assert!(
        read(&lock)
            .unwrap_err()
            .to_string()
            .contains("exactly one locked package")
    );
    let mut lock = rich();
    lock.packages[0].dependencies.clear();
    assert!(
        read(&lock)
            .unwrap_err()
            .to_string()
            .contains("features/dependency graph")
    );
}
