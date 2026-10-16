use super::*;
use crate::manifest::{feature_constraints::*, feature_effects::*};
use std::collections::BTreeSet;

fn source(epoch1: bool) -> String {
    format!(
        "# café\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.1.0'\r\n{}description='keep exact' # untouched\r\n[[requires_any]]\r\none_of=['flow:org.example/one@^1.0','flow:org.example/two@^1.0'] # technical OR\r\n[requires.packages]\r\n'flow:org.example/base' = {{ version='^1.0', link='static', access='private', friend=false, exclude=['docs/**'] }} # metadata\r\n[features]\r\ndefault = ['a'] # declared default\r\na = [\r\n 'dep:org.example/base', # activation\r\n 'org.example/base?/fast', # weak\r\n 'org.example/base/strong', # strong\r\n]\r\nb = []\r\n[features.exclusive]\r\nmode = [\r\n 'a', # first\r\n 'b', # second\r\n] # exclusive\r\n",
        if epoch1 {
            "epoch = 1 # preserve marker\r\n"
        } else {
            ""
        }
    )
}
fn selection(epoch1: bool) -> MigrationSelection<'static> {
    MigrationSelection {
        source_profile: Some(if epoch1 {
            FEATURES_LEGACY_EPOCH1
        } else {
            FEATURES_LEGACY_ABSENT
        }),
        target_profile: Some(if epoch1 {
            FEATURES_TYPED_EPOCH1
        } else {
            FEATURES_TYPED_ABSENT
        }),
        recipe: Some(if epoch1 {
            FEATURES_TO_TYPED_EPOCH1
        } else {
            FEATURES_TO_TYPED
        }),
        source_decisions: &[],
    }
}
fn planned(text: &str, epoch1: bool) -> ManifestMigrationPlan {
    match plan_manifest_migration(text, selection(epoch1)) {
        ManifestMigrationOutcome::Planned(plan) => plan,
        other => panic!("expected plan: {other:#?}"),
    }
}

#[test]
fn commented_features_preserve_meaning_and_epoch_coordinates() {
    for epoch1 in [false, true] {
        let before = source(epoch1);
        let old = Manifest::parse_str(&before).unwrap();
        let declarations =
            FeatureDeclarations::from_requires(&old.requires, &BTreeSet::new()).unwrap();
        let program = FeatureProgram::lower_legacy(&old.features, &declarations).unwrap();
        let plan = planned(&before, epoch1);
        let checked = plan.checked_features.as_ref().unwrap();
        assert_eq!(checked.declarations, declarations);
        assert_eq!(checked.program, program);
        assert_eq!(checked.selection, Selection::Omitted);
        assert!(
            checked
                .declarations
                .dependencies
                .values()
                .all(|d| !d.optional)
        );
        let group = &checked.constraints.groups[&LocalFeatureRef::new("mode").unwrap()];
        assert_eq!(group.cardinality, Cardinality::ZeroOrOne);
        assert!(group.cardinality.accepts(0));
        assert!(!group.cardinality.accepts(2));
        assert_eq!(group.defaults, None);
        assert_eq!(
            checked.manifest.package.as_ref().unwrap().epoch,
            epoch1.then_some(1)
        );
        assert_eq!(checked.manifest.requires, old.requires);
        assert_eq!(checked.manifest.requires_any, old.requires_any);
        assert_eq!(checked.manifest.requires_any[0].one_of.len(), 2);
        assert_eq!(
            checked.manifest.features.defaults(),
            old.features.defaults()
        );
        for marker in [
            "# café",
            "# untouched",
            "# metadata",
            "# activation",
            "# weak",
            "# strong",
            "# first",
            "# second",
            "# exclusive",
        ] {
            assert_eq!(plan.after.matches(marker).count(), 1);
        }
        assert!(plan.after.contains("mode = { cardinality = \"zero-or-one\", members = [\r\n 'a', # first\r\n 'b', # second\r\n] } # exclusive"));
        assert_eq!(replay_edits(&before, &plan.edits).unwrap(), plan.after);
        let mut source_cursor = 0;
        let mut target_cursor = 0;
        for edit in &plan.edits {
            let prefix = &before[source_cursor..edit.before_span.start];
            assert_eq!(
                &plan.after[target_cursor..target_cursor + prefix.len()],
                prefix
            );
            target_cursor += prefix.len() + edit.after_token.len();
            source_cursor = edit.before_span.end;
        }
        assert_eq!(&plan.after[target_cursor..], &before[source_cursor..]);
        assert_eq!(
            read_feature_manifest(&plan.after, plan.target_profile).unwrap(),
            *checked
        );
        assert!(
            Manifest::parse_str(&plan.after).is_err(),
            "ordinary reader must stay legacy"
        );
        for source_metadata in [
            "version='^1.0', path='../base'",
            "version='^1.0', git='https://example.test/base.git', tag='v1.0', auth='none'",
            "version={var='base_version'}",
        ] {
            let variant = before.replace("version='^1.0'", source_metadata);
            let old = Manifest::parse_str(&variant).unwrap();
            let converted = planned(&variant, epoch1);
            assert_eq!(converted.checked_manifest.requires, old.requires);
            assert_eq!(converted.after.matches(source_metadata).count(), 1);
        }
        let inline = format!(
            "package = {{name='demo',group='org.example',kind='flow',version='1.0.0'{}}}\nfeatures = {{default=['a'],a=[],b=[],exclusive={{mode=['a','b']}}}} # inline\n",
            if epoch1 { ",epoch=1" } else { "" }
        );
        assert!(planned(&inline, epoch1).after.ends_with(" # inline\n"));
    }
}

#[test]
fn ambiguous_feature_references_and_unknown_profiles_have_zero_edits() {
    for epoch1 in [false, true] {
        let base = source(epoch1);
        for text in [
            base.replace("'dep:org.example/base'", "'dep:missing'"),
            base.replace("'dep:org.example/base'", "'subskill:stack/rust'"),
            base.replace("a = [", "'slash/name' = ["),
            base.clone() + "other=['a']\r\n",
            base.replace("mode = [", "empty = []\r\nmode = ["),
            base.clone() + "unknown=['missing']\r\n",
            base.replace(
                "[features]",
                "'stack:org.example/base' = '^1.0'\r\n[features]",
            ),
            base.replace("version='^1.0'", "version='^1.0', optional=true"),
            base.replace("version='^1.0'", "version='^1.0', features=['x']"),
            base.replace("'dep:org.example/base'", "'org.example/base'"),
            base.replace("[features]", "'org.other/base' = '^1.0'\r\n[features]")
                .replace("'dep:org.example/base'", "'dep:base'"),
        ] {
            let ManifestMigrationOutcome::NeedsInput(questions) =
                plan_manifest_migration(&text, selection(epoch1))
            else {
                panic!("must ask: {text}")
            };
            assert!(!questions.is_empty());
            assert!(questions.iter().all(|q| q.field == "feature_reference"
                && q.authored_reference.is_some()
                && q.resource_path.is_none()
                && !q.choices.is_empty()));
        }
        for text in [
            base.replace("[package]", "[package]\nepoch=9"),
            base.replace("[package]", "[package]\nepoch='unknown'"),
        ] {
            assert!(matches!(
                plan_manifest_migration(&text, selection(epoch1)),
                ManifestMigrationOutcome::Refused(_)
            ));
        }
        let mut unknown = selection(epoch1);
        unknown.source_profile = Some("features-unknown-r1");
        assert!(matches!(
            plan_manifest_migration(&base, unknown),
            ManifestMigrationOutcome::Refused(_)
        ));
    }
}

#[test]
fn typed_target_identity_and_repeated_planning_are_noop() {
    for epoch1 in [false, true] {
        let before = source(epoch1);
        let first = planned(&before, epoch1);
        assert_eq!(first, planned(&before, epoch1));
        let choice = MigrationSelection {
            source_profile: Some(first.target_profile),
            target_profile: Some(first.target_profile),
            recipe: Some(IDENTITY),
            source_decisions: &[],
        };
        let ManifestMigrationOutcome::NoOp(second) = plan_manifest_migration(&first.after, choice)
        else {
            panic!("identity")
        };
        assert!(second.edits.is_empty());
        assert_eq!(second.before, second.after);
        assert_eq!(second.after, first.after);
        assert_eq!(second.checked_features, first.checked_features);
        assert_eq!(second.before_identity, second.after_identity);
        assert!(
            read_feature_manifest(
                &first.after.replace("zero-or-one", "exactly-one"),
                first.target_profile
            )
            .is_err()
        );
        assert!(
            read_feature_manifest(
                &first
                    .after
                    .replace("strength = \"weak\"", "strength = \"unknown\""),
                first.target_profile
            )
            .is_err()
        );
        let alias = before.replace("org.example/base?/fast", "base?/fast");
        assert_eq!(
            planned(&alias, epoch1).checked_features,
            first.checked_features
        );
        let no_default = before.replace("default = ['a'] # declared default\r\n", "");
        let checked = planned(&no_default, epoch1).checked_features.unwrap();
        assert!(
            !checked
                .program
                .features
                .contains_key(&LocalFeatureRef::new("default").unwrap())
        );
        assert_eq!(checked.selection, Selection::Omitted);
    }
}
