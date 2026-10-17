use super::*;
use vibe_core::manifest::{Manifest, ManifestMigrationOutcome as Outcome};
const OLD: &str = include_str!("../../../vibe-install/tests/fixtures/m_05_f_2_old.toml");

#[test]
fn saved_apply_preserves_comments_metadata_and_independent_roots() {
    let Outcome::Planned(plan) = propose_redbook_consumer(OLD) else {
        panic!("planned");
    };
    assert_eq!(
        propose_redbook_consumer(OLD),
        Outcome::Planned(plan.clone())
    );
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("vibe.toml"), OLD).unwrap();
    let grant =
        OwnedManifestMigrationFile::new(&root, "vibe.toml", &temp.path().join("recovery")).unwrap();
    let report = apply_redbook_proposal(&grant, "redbook-migration", &plan).unwrap();
    assert_eq!(report["outcome"], "completed", "{report}");
    assert_eq!(
        std::fs::read_to_string(root.join("vibe.toml")).unwrap(),
        plan.after
    );
    let before = Manifest::parse_str(OLD).unwrap();
    let after = Manifest::parse_str(&plan.after).unwrap();
    assert_eq!(before.requires.packages, after.requires.packages);
    assert_eq!(before.requires.links, after.requires.links);
    assert_eq!(before.requires.accesses, after.requires.accesses);
    assert_eq!(before.requires.friend_flags, after.requires.friend_flags);
    for comment in OLD.lines().filter(|s| s.trim_start().starts_with('#')) {
        assert!(plan.after.contains(comment));
    }
    assert_eq!(after.project, before.project);
    assert_eq!(
        after.consumer_configuration[0].selections["continuity"].members,
        Some(vec!["mup".into()])
    );
}

#[test]
fn both_and_neither_return_needs_input_without_writes() {
    assert!(matches!(
        manifest::plan_manifest_migration(
            OLD,
            MigrationSelection {
                source_profile: Some(manifest::redbook::LEGACY),
                target_profile: None,
                recipe: Some(manifest::redbook::RECIPE),
                source_decisions: &[],
            }
        ),
        Outcome::NeedsInput(_)
    ));
    for text in [
        OLD.replace("\"org.vibevm.world/wal\",", ""),
        OLD.replace(
            "\"org.vibevm.world/wal\",",
            "\"org.vibevm.world/wal\", \"org.vibevm.world/multi-user-planning\",",
        ),
    ] {
        assert!(matches!(
            propose_redbook_consumer(&text),
            Outcome::NeedsInput(_)
        ));
    }
}

#[test]
fn saved_apply_refuses_stale_before_image() {
    let Outcome::Planned(plan) = propose_redbook_consumer(OLD) else {
        panic!("planned");
    };
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let changed = format!("{OLD}\n# independent edit\n");
    std::fs::write(root.join("vibe.toml"), &changed).unwrap();
    let grant =
        OwnedManifestMigrationFile::new(&root, "vibe.toml", &temp.path().join("recovery")).unwrap();
    let report = apply_redbook_proposal(&grant, "stale-migration", &plan).unwrap();
    assert_ne!(report["outcome"], "completed");
    assert_eq!(
        std::fs::read_to_string(root.join("vibe.toml")).unwrap(),
        changed
    );
}
