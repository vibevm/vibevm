use super::{constraints::*, effects::*, profile};

const REDBOOK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../vibevm/vibepacks/org.vibevm.world/redbook/v1.1.0/vibe.toml"
));
fn f(s: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(s).unwrap()
}

#[test]
fn actual_redbook_definition_preserves_declarations_and_local_groups() {
    let checked = profile::constrained::read(REDBOOK).unwrap().unwrap();
    assert_eq!(
        checked
            .manifest
            .require_package()
            .unwrap()
            .version
            .to_string(),
        "1.1.0"
    );
    assert_eq!(checked.selection, Selection::Omitted);
    assert_eq!(checked.constraints.groups.len(), 4);
    let continuity = &checked.constraints.groups[&f("continuity")];
    assert_eq!(continuity.cardinality, Cardinality::ExactlyOne);
    assert_eq!(continuity.members, [f("mup"), f("wal")].into());
    assert_eq!(continuity.defaults, Some([f("mup")].into()));
    assert_eq!(
        checked.constraints.groups[&f("additions")].cardinality,
        Cardinality::ZeroOrMore
    );
    for branch in ["mup", "wal"] {
        let group = &checked.constraints.groups[&f(&format!("{branch}-additions"))];
        assert_eq!(group.cardinality, Cardinality::ZeroOrMore);
        assert_eq!(group.guards, vec![Guard::Member(f(branch))]);
    }
    assert_eq!(checked.declarations.dependencies.len(), 23);
    assert_eq!(
        checked
            .declarations
            .dependencies
            .values()
            .filter(|d| d.optional)
            .count(),
        8
    );
    assert!(checked.manifest.conflicts.packages.is_empty());
    for effects in checked.program.features.values() {
        for effect in effects {
            let FeatureEffect::ActivateDependency(id) = effect else {
                panic!("unexpected effect")
            };
            assert!(checked.declarations.dependencies[id].optional);
        }
    }
    let text = REDBOOK.replace(
        "\"flow:org.vibevm.world/wal\" = \"=1.1.0\"",
        "\"flow:org.vibevm.world/wal\" = { version = \"=1.1.0\", link = \"static\", access = \"private\", friend = true, exclude = [\"org.example/unused\"] }",
    );
    let modified = profile::constrained::read(&text).unwrap().unwrap();
    let id = DependencyDeclarationId::new("org.vibevm.world/wal").unwrap();
    let declaration = &modified.declarations.dependencies[&id];
    assert!(declaration.optional);
    assert_eq!(declaration.link, Some(crate::manifest::LinkType::Static));
    assert_eq!(
        declaration.access,
        Some(crate::manifest::AccessLevel::Private)
    );
    assert_eq!(declaration.friend, Some(true));
    assert_eq!(declaration.exclude, Some(vec!["org.example/unused".into()]));
}

#[test]
fn absent_legacy_and_old_typed_profile_keep_semantics() {
    let text =
        "[package]\nname='app'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\nepoch=1\n";
    assert!(profile::constrained::read(text).unwrap().is_none());
    let legacy = format!("{text}[features]\na=[]\nb=[]\n[features.exclusive]\nchoice=['a','b']\n");
    assert!(profile::constrained::read(&legacy).unwrap().is_none());
    assert!(crate::manifest::Manifest::parse_str(&legacy).is_ok());
    let legacy_profile = format!("{text}[features]\nprofile=[]\n");
    assert!(
        profile::constrained::read(&legacy_profile)
            .unwrap()
            .is_none()
    );
    let parsed = crate::manifest::Manifest::parse_str(&legacy_profile).unwrap();
    assert!(parsed.features.features.contains_key("profile"));
    let typed = format!(
        "{text}[features]\na=[]\nb=[]\n[features.exclusive]\nchoice={{cardinality='zero-or-one',members=['a','b']}}\n"
    );
    let checked = crate::manifest::migration::read_feature_manifest(
        &typed,
        crate::manifest::migration::FEATURES_TYPED_EPOCH1,
    )
    .unwrap();
    assert_eq!(
        checked.constraints.groups[&f("choice")].cardinality,
        Cardinality::ZeroOrOne
    );
    assert_eq!(checked.constraints.groups[&f("choice")].defaults, None);
    assert!(checked.constraints.groups[&f("choice")].guards.is_empty());
    assert!(crate::manifest::Manifest::parse_str(REDBOOK).is_err());
}

#[test]
fn malformed_dormant_definitions_refuse() {
    for text in [
        REDBOOK.replace("constrained-r1", "constrained-r2"),
        REDBOOK.replace("member = \"wal\"", "member = \"foreign/wal\""),
        REDBOOK.replace("defaults = [\"mup\"]", "defaults = [\"missing\"]"),
        REDBOOK.replace(
            "members = [\"mup\", \"wal\"]",
            "members = [\"mup\", \"mup\"]",
        ),
        REDBOOK.replace("member = \"wal\"", "not = \"wal\""),
        REDBOOK.replace(
            "guards = [{ member = \"wal\" }]",
            "guards = [{ group = \"wal-additions\" }]",
        ),
    ] {
        assert!(profile::constrained::read(&text).is_err());
    }
}
