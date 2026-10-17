use super::*;
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
const TEXT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../formats/corpora/change-manifest/e1/valid/representative.toml"
));
fn context() -> ChangeContext {
    ChangeContext::for_node(
        &Manifest::parse_str("[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n")
            .unwrap(),
        ".",
        Some(Path::new("docs/custom-specs")),
    )
    .unwrap()
}
#[test]
fn connected_epoch_one_parse() {
    let context = context();
    let id = ChangeId::parse(ID).unwrap();
    let directory = context.directory(&id);
    let change = ChangeManifest::parse_str(TEXT, &context, &directory).unwrap();
    assert_eq!(change.id().as_str(), ID);
    assert_eq!(change.origins().len(), 1);
    assert_eq!(change.documents().len(), 3);
    assert_eq!(
        change.metadata()["user_value"].as_str(),
        Some("Preserve me")
    );
    assert_eq!(
        change.directory(),
        Path::new("docs/custom-specs/changes/chg-0123456789abcdef0123456789abcdef")
    );
}
#[test]
fn title_slug_rename_preserves_directory_and_address() {
    let c = context();
    let id = ChangeId::parse(ID).unwrap();
    let dir = c.directory(&id);
    let a = ChangeManifest::parse_str(TEXT, &c, &dir).unwrap();
    let changed = TEXT
        .replace("first-title", "renamed-slug")
        .replace("Initial user title", "A new display title");
    let b = ChangeManifest::parse_str(&changed, &c, &dir).unwrap();
    assert_ne!(a.title(), b.title());
    assert_ne!(a.slug(), b.slug());
    assert_eq!(a.id(), b.id());
    assert_eq!(a.directory(), b.directory());
    assert_eq!(
        c.document_address(a.id(), "intent.md").unwrap(),
        c.document_address(b.id(), "intent.md").unwrap()
    );
}
#[test]
fn unknown_epochs_and_duplicate_ids_refuse() {
    let c = context();
    let dir = c.directory(&ChangeId::parse(ID).unwrap());
    let bad = TEXT.replacen("schema = 1", "schema = 9", 1);
    assert_eq!(
        ChangeManifest::parse_str(&bad, &c, &dir).unwrap_err().kind,
        ChangeErrorKind::UnknownEpoch
    );
    let a = ChangeManifest::parse_str(TEXT, &c, &dir).unwrap();
    assert_eq!(
        validate_change_catalog(&[a.clone(), a]).unwrap_err().kind,
        ChangeErrorKind::DuplicateId
    );
    for invalid_id in [
        "mutable slug",
        "chg-0123456789abcdef0123456789abcdeF",
        "../outside",
    ] {
        assert_eq!(
            ChangeId::parse(invalid_id).unwrap_err().kind,
            ChangeErrorKind::Identity
        );
    }
}
#[test]
fn escaping_and_closure_input_roles_refuse() {
    let c = context();
    let dir = c.directory(&ChangeId::parse(ID).unwrap());
    for path in ["../intent.md", "/absolute.md", "C:/intent.md"] {
        let bad = TEXT.replace("path = \"intent.md\"", &format!("path = {path:?}"));
        assert_eq!(
            ChangeManifest::parse_str(&bad, &c, &dir).unwrap_err().kind,
            ChangeErrorKind::Path
        );
    }
    let bad = TEXT.replace(
        "path = \"closure.md\"\nnormative = false\ninput = false",
        "path = \"closure.md\"\nnormative = true\ninput = true",
    );
    assert_eq!(
        ChangeManifest::parse_str(&bad, &c, &dir).unwrap_err().kind,
        ChangeErrorKind::Role
    );
    let alias = TEXT.replace("path = \"closure.md\"", "path = \"intent.md\"");
    assert_eq!(
        ChangeManifest::parse_str(&alias, &c, &dir)
            .unwrap_err()
            .kind,
        ChangeErrorKind::DuplicatePath
    );
}
#[test]
fn migration_preserves_comments_and_user_values() {
    let c = context();
    let dir = c.directory(&ChangeId::parse(ID).unwrap());
    let pilot = TEXT.replacen("schema = 1", "schema = 0", 1);
    let p = plan_change_migration(
        &pilot,
        &c,
        &dir,
        ChangeMigrationSelection {
            source_epoch: 0,
            target_epoch: 1,
            recipe: CHANGE_UPGRADE,
        },
    )
    .unwrap();
    assert_eq!(p.after, TEXT);
    assert_eq!(p.before, pilot);
    assert_eq!(p.edits.len(), 1);
    assert_eq!(p.checked.metadata()["number"].as_integer(), Some(7));
    let identity = plan_change_migration(
        &p.after,
        &c,
        &dir,
        ChangeMigrationSelection {
            source_epoch: 1,
            target_epoch: 1,
            recipe: CHANGE_IDENTITY,
        },
    )
    .unwrap();
    assert_eq!(identity.before, identity.after);
    assert!(identity.edits.is_empty());
}
#[test]
fn migration_refuses_invalid_target_before_proposal() {
    let c = context();
    let dir = c.directory(&ChangeId::parse(ID).unwrap());
    let pilot = TEXT
        .replacen("schema = 1", "schema = 0", 1)
        .replace("path = \"intent.md\"", "path = \"../intent.md\"");
    assert_eq!(
        plan_change_migration(
            &pilot,
            &c,
            &dir,
            ChangeMigrationSelection {
                source_epoch: 0,
                target_epoch: 1,
                recipe: CHANGE_UPGRADE
            }
        )
        .unwrap_err()
        .kind,
        ChangeErrorKind::Path
    );
    assert_eq!(
        plan_change_migration(
            TEXT,
            &c,
            &dir,
            ChangeMigrationSelection {
                source_epoch: 1,
                target_epoch: 1,
                recipe: "unknown"
            }
        )
        .unwrap_err()
        .kind,
        ChangeErrorKind::Migration
    );
}

#[test]
fn registered_authored_corpus_matches_shared_validator() {
    let c = context();
    let dir = c.directory(&ChangeId::parse(ID).unwrap());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let registry: toml::Value =
        toml::from_str(&std::fs::read_to_string(root.join("formats/REGISTRY.toml")).unwrap())
            .unwrap();
    let record = &registry["format"]["change-manifest"];
    assert_eq!(record["epoch"].as_integer(), Some(1));
    assert_eq!(record["schema"].as_str(), Some("none"));
    assert_eq!(record["unknown_fields"].as_str(), Some("deny"));
    let corpus = root.join(record["corpus"].as_str().unwrap());
    assert!(
        ChangeManifest::parse_str(
            &std::fs::read_to_string(corpus.join("valid/representative.toml")).unwrap(),
            &c,
            &dir
        )
        .is_ok()
    );
    for (file, kind) in [
        (
            "semantic-gap/unknown-epoch.toml",
            ChangeErrorKind::UnknownEpoch,
        ),
        ("semantic-gap/escaping-role.toml", ChangeErrorKind::Path),
        ("semantic-gap/closure-input.toml", ChangeErrorKind::Role),
        (
            "structural-invalid/unknown-field.toml",
            ChangeErrorKind::Syntax,
        ),
    ] {
        let body = std::fs::read_to_string(corpus.join(file)).unwrap();
        assert_eq!(
            ChangeManifest::parse_str(&body, &c, &dir).unwrap_err().kind,
            kind
        );
    }
}

#[test]
fn raw_origin_uri_cannot_normalize_into_selected_address() {
    let c = context();
    let dir = c.directory(&ChangeId::parse(ID).unwrap());
    let address = "spec://org.fixture/reusable-intent/FEAT-001#DELIVERY";
    for refused in [
        "spec://org.fixture/reusable-intent/./FEAT-001#DELIVERY",
        "spec://org.fixture/reusable-intent/other/../FEAT-001#DELIVERY",
        "spec://org.fixture/reusable-intent//FEAT-001#DELIVERY",
        "spec://org.fixture/reusable-intent/%46EAT-001#DELIVERY",
    ] {
        assert_eq!(TEXT.matches(address).count(), 1);
        let error =
            ChangeManifest::parse_str(&TEXT.replace(address, refused), &c, &dir).unwrap_err();
        assert_eq!(error.kind, ChangeErrorKind::Origin, "{refused}");
        assert_eq!(error.field, "origins.documents.address", "{refused}");
    }
}
