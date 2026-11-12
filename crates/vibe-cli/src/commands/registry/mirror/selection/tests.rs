//! What a selection admits, and the two refusals that need the run's own
//! context (PROP-075 `#MIRROR-CONFIG`).
use std::path::Path;

use super::*;

/// A validated `mirror.toml` from `body`, held together with the directory
/// it was written in so the path in a refusal is a real one.
fn read_config(body: &str) -> (tempfile::TempDir, Validated) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mirror.toml");
    std::fs::write(&path, body).unwrap();
    let validated = config::read(&path).unwrap();
    (dir, validated)
}

fn of(body: &str) -> Selection {
    let (_dir, validated) = read_config(body);
    Selection::from_config(&validated, &[], None).unwrap()
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG")]
fn all_admits_everything_and_exclude_subtracts_on_both_axes() {
    let selection = of("schema = 1\n[select]\nall = true\nexclude = ['org.old', 'org.a/skip']\n");
    assert!(selection.admits("org.a", "keep"));
    assert!(!selection.admits("org.a", "skip"));
    assert!(!selection.admits("org.old", "anything"));
}

#[test]
fn groups_and_packages_intersect_exactly_as_the_flags_did() {
    let selection =
        of("schema = 1\n[select]\ngroups = ['org.a']\npackages = ['org.a/x', 'org.b/y']\n");
    assert!(selection.admits("org.a", "x"));
    // `org.b/y` is named, but the group axis does not admit `org.b`.
    assert!(!selection.admits("org.b", "y"));
    assert!(!selection.admits("org.a", "z"));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-TEMPLATE-SNAPSHOT"
)]
fn a_template_row_selects_the_template_package_itself() {
    let selection = of("schema = 1\n[[template]]\npackage = 'org.vibevm.node/node'\n");
    assert!(selection.admits("org.vibevm.node", "node"));
    assert!(!selection.admits("org.vibevm.node", "other"));
    assert_eq!(selection.templates().len(), 1);
}

#[test]
fn an_exclude_beats_a_template_row_that_names_the_same_package() {
    let selection =
        of("schema = 1\n[[template]]\npackage = 'org.a/b'\n[select]\nexclude = ['org.a/b']\n");
    assert!(!selection.admits("org.a", "b"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG-SCHEMA")]
fn a_selection_of_nothing_is_a_refusal_not_a_silent_empty_run() {
    let (_dir, validated) = read_config("schema = 1\n[select]\nall = false\n");
    let message = Selection::from_config(&validated, &[], None)
        .unwrap_err()
        .to_string();
    assert!(message.contains("selects nothing"), "{message}");
    // An empty `[select]` with a `[[template]]` row is not empty.
    let (_dir, with_row) = read_config("schema = 1\n[[template]]\npackage = 'org.a/b'\n");
    assert!(Selection::from_config(&with_row, &[], None).is_ok());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
fn payloads_default_to_the_first_configured_drop_and_refuse_without_one() {
    let body = "schema = 1\n[[template]]\npackage = 'org.a/b'\npayloads = true\n";
    let (_dir, validated) = read_config(body);
    let drops = vec![PathBuf::from("/srv/first"), PathBuf::from("/srv/second")];
    let selection = Selection::from_config(&validated, &drops, None).unwrap();
    assert_eq!(
        selection.payloads().unwrap().into,
        PathBuf::from("/srv/first")
    );
    assert_eq!(selection.payloads().unwrap().max_total, None);

    let message = Selection::from_config(&validated, &[], None)
        .unwrap_err()
        .to_string();
    assert!(message.contains("[artifacts] packages"), "{message}");
}

#[test]
fn a_row_without_payloads_asks_for_no_projection_at_all() {
    let selection =
        of("schema = 1\n[[template]]\npackage = 'org.a/b'\n[payloads]\nmax_total = 1\n");
    assert!(selection.payloads().is_none());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
fn a_drop_inside_the_registry_target_is_refused_by_name() {
    let target = tempfile::tempdir().unwrap();
    let inside = target.path().join("payloads");
    std::fs::create_dir_all(&inside).unwrap();
    let body = format!(
        "schema = 1\n[[template]]\npackage = 'org.a/b'\npayloads = true\n\
         [payloads]\ninto = '{}'\n",
        inside.display()
    );
    let (_dir, validated) = read_config(&body);
    let message = Selection::from_config(&validated, &[], Some(target.path()))
        .unwrap_err()
        .to_string();
    assert!(message.contains("is the registry target"), "{message}");
    assert!(message.contains("forty gigabytes"), "{message}");

    // The target itself, and a directory holding it, are refused too.
    for into in [target.path().to_path_buf(), target.path().join("..")] {
        let body = format!(
            "schema = 1\n[[template]]\npackage = 'org.a/b'\npayloads = true\n\
             [payloads]\ninto = '{}'\n",
            into.display()
        );
        let (_dir, validated) = read_config(&body);
        assert!(
            Selection::from_config(&validated, &[], Some(target.path())).is_err(),
            "{}",
            into.display()
        );
    }

    // A sibling directory is fine.
    let sibling = tempfile::tempdir().unwrap();
    let body = format!(
        "schema = 1\n[[template]]\npackage = 'org.a/b'\npayloads = true\n\
         [payloads]\ninto = '{}'\n",
        sibling.path().display()
    );
    let (_dir, validated) = read_config(&body);
    assert!(Selection::from_config(&validated, &[], Some(target.path())).is_ok());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG")]
fn flags_override_the_file_wholesale_and_the_file_is_still_validated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mirror.toml");
    std::fs::write(&path, "schema = 1\n[select]\nall = true\n").unwrap();
    let selection = resolve(Some(&path), &["org.a/x".to_string()], &[], None, &[], None).unwrap();
    assert_eq!(selection.origin(), Origin::Flags);
    assert!(selection.admits("org.a", "x"));
    // `all = true` of the file would have admitted this; the flags won.
    assert!(!selection.admits("org.b", "y"));

    // A document the flags override is still refused when it is wrong.
    std::fs::write(&path, "schema = 9\n").unwrap();
    assert!(resolve(Some(&path), &["org.a/x".to_string()], &[], None, &[], None).is_err());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG")]
fn a_bare_run_repeats_the_recorded_selection_and_does_not_rewrite_it() {
    let recorded = record::SelectRecord {
        all: false,
        groups: vec!["org.a".into()],
        packages: vec!["org.a/x".into()],
        exclude: vec!["org.a/old".into()],
        templates: vec![record::TemplateRecord {
            package: "org.vibevm.node/node".into(),
            versions: vec![">=22".into()],
            channels: vec!["lts".into()],
            newest: Some(2),
            platforms: vec!["linux".into()],
            archs: vec![],
            installers: vec![],
            payloads: true,
            version_list: Default::default(),
        }],
    };
    let selection = resolve(None, &[], &[], Some(&recorded), &[], None).unwrap();
    assert_eq!(selection.origin(), Origin::Recorded);
    assert!(selection.admits("org.a", "x"));
    assert!(!selection.admits("org.a", "old"));
    assert!(selection.admits("org.vibevm.node", "node"));
    assert_eq!(selection.templates()[0].newest, Some(2));
    // A repeat records nothing: the record already says this.
    assert!(selection.to_record().is_none());
    // And it asks for no payload projection of its own.
    assert!(selection.payloads().is_none());
}

#[test]
fn with_nothing_at_all_the_selection_is_everything_and_records_nothing() {
    let selection = resolve(None, &[], &[], None, &[], None).unwrap();
    assert_eq!(selection.origin(), Origin::Flags);
    assert!(selection.admits("org.anything", "at-all"));
    assert!(selection.to_record().is_none());
}

/// Only a declared selection is recorded. `--package` has a second job —
/// making a source that cannot be listed mirrorable one package at a time
/// — so recording it would narrow every later bare run of that source
/// for good.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG")]
fn a_flag_override_is_a_per_run_choice_and_records_nothing() {
    let flags = Selection::from_flags(&["org.a/x".into()], &["org.a".into()]).unwrap();
    assert!(flags.to_record().is_none());

    // Not even when it overrode a `--config` whose declaration stays.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mirror.toml");
    std::fs::write(&path, "schema = 1\n[select]\ngroups = ['org.b']\n").unwrap();
    let selection = resolve(Some(&path), &["org.a/x".to_string()], &[], None, &[], None).unwrap();
    assert!(selection.to_record().is_none());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-CONFIG")]
fn a_config_run_records_the_selection_it_used() {
    let selection = of(
        "schema = 1\n[select]\ngroups = ['org.a']\nexclude = ['org.a/old']\n\
         [[template]]\npackage = 'org.vibevm.node/node'\nversions = ['>=22']\nchannels = ['lts']\n",
    );
    let record = selection.to_record().unwrap();
    assert!(!record.all);
    assert_eq!(record.groups, ["org.a"]);
    assert_eq!(record.exclude, ["org.a/old"]);
    assert_eq!(record.templates[0].package, "org.vibevm.node/node");
    assert_eq!(record.templates[0].versions, [">=22"]);
    assert_eq!(record.templates[0].channels, ["lts"]);
    // The record reads back as the same selection.
    let repeated = Selection::from_record(&record);
    assert!(repeated.admits("org.a", "keep"));
    assert!(!repeated.admits("org.a", "old"));
    assert!(repeated.admits("org.vibevm.node", "node"));
}

#[test]
fn flag_packages_reach_the_per_package_lister() {
    let selection = Selection::from_flags(&["org.a/x".into()], &[]).unwrap();
    assert_eq!(
        selection.packages(),
        [("org.a".to_string(), "x".to_string())]
    );
    // So do a configuration's, which is what lets `[select] packages`
    // serve a source without an index.
    let configured = of("schema = 1\n[select]\npackages = ['org.a/x']\n");
    assert_eq!(
        configured.packages(),
        [("org.a".to_string(), "x".to_string())]
    );
}

#[test]
fn a_recorded_requirement_this_vibe_cannot_parse_is_dropped_not_fatal() {
    let recorded = record::SelectRecord {
        all: true,
        templates: vec![record::TemplateRecord {
            package: "org.a/b".into(),
            versions: vec!["=1.0.0".into(), "<<nonsense".into()],
            channels: vec![],
            newest: None,
            platforms: vec![],
            archs: vec![],
            installers: vec![],
            payloads: false,
            version_list: Default::default(),
        }],
        ..record::SelectRecord::default()
    };
    let selection = Selection::from_record(&recorded);
    assert_eq!(selection.templates()[0].versions.len(), 1);
}

#[test]
fn the_local_root_of_a_target_is_only_a_directory_one() {
    use vibe_publish::layout_publish::LayoutTarget;
    assert_eq!(
        super::super::local_root(&LayoutTarget::Localdir {
            root: PathBuf::from("/srv/reg")
        }),
        Some(Path::new("/srv/reg"))
    );
    assert_eq!(
        super::super::local_root(&LayoutTarget::Monorepo {
            url: "ssh://host/reg.git".into(),
            branch: "main".into()
        }),
        None
    );
}
