//! `[[application.migration]]` grammar and refusals
//! (PROP-075 `#MIGRATION-DECLARATION`).

use super::Manifest;

const PACKAGE: &str = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.1.0"

[application]
id = "demo"
"#;

/// One well-formed entry whose two sides are ordinary `recipe` steps.
const ENTRY: &str = r#"
[[application.migration]]
id = "global-packages"
name = "Global packages"
description = "Reinstall what the old version had"
default = true

[application.migration.export]
id = "list-globals"
title = "List the global packages"
kind = "recipe"
script = "migrate/export.sh"

[application.migration.import]
id = "install-globals"
title = "Install the global packages"
kind = "recipe"
script = "migrate/import.sh"
"#;

/// `PACKAGE` with `body` appended.
fn with(body: &str) -> String {
    format!("{PACKAGE}{body}")
}

/// One entry whose `export` table is `export`, the `import` left ordinary.
fn with_export(export: &str) -> String {
    with(&format!(
        "\n[[application.migration]]\nid = \"carry\"\nname = \"Carry\"\n\n\
         [application.migration.export]\n{export}\n\n\
         [application.migration.import]\nid = \"in\"\ntitle = \"In\"\nkind = \"recipe\"\n\
         script = \"m/in.sh\"\n"
    ))
}

/// The refusal of a document, whether it comes from parsing (an unknown
/// member of a closed table) or from validation (a field's own rule).
fn refusal(text: &str) -> String {
    match Manifest::parse_str(text) {
        Err(error) => error.to_string(),
        Ok(manifest) => manifest
            .validate()
            .expect_err("the document must be refused")
            .to_string(),
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
fn migration_rows_parse_round_trip_and_keep_both_sides() {
    let manifest = Manifest::parse_str(&with(ENTRY)).unwrap();
    manifest.validate().unwrap();
    let rows = &manifest.application.as_ref().unwrap().migration;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.id, "global-packages");
    assert_eq!(row.name, "Global packages");
    assert_eq!(
        row.description.as_deref(),
        Some("Reinstall what the old version had")
    );
    assert!(row.default);
    assert_eq!(row.export.id, "list-globals");
    assert_eq!(row.export.kind.name(), "recipe");
    assert_eq!(row.import.id, "install-globals");
    assert_eq!(
        row.sides()
            .map(|(side, step)| (side.as_str(), step.id.as_str())),
        [("export", "list-globals"), ("import", "install-globals")]
    );
    // Absent is the default, so every existing manifest is byte-identical.
    assert!(
        Manifest::parse_str(PACKAGE)
            .unwrap()
            .application
            .unwrap()
            .migration
            .is_empty()
    );
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&manifest).unwrap()).unwrap(),
        manifest
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
fn a_forbidden_kind_refuses_naming_the_kind_and_its_path() {
    for kind in [
        "kind = \"archive\"\nformat = \"zip\"\nsource = \"u\"\nsha256 = \"ab\"",
        "kind = \"sources\"",
        "kind = \"copy\"\nfrom = \"a\"",
        "kind = \"agent\"\nprompt = \"do it\"",
        "kind = \"env\"",
        "kind = \"native\"\nformat = \"exe\"\nsource = \"u\"\nsha256 = \"ab\"",
    ] {
        let error = refusal(&with_export(&format!(
            "id = \"out\"\ntitle = \"Out\"\n{kind}"
        )));
        assert!(
            error.contains("application.migration[0].export.kind"),
            "`{kind}` refused as `{error}`"
        );
        let named = kind
            .split_once('"')
            .and_then(|(_, rest)| rest.split_once('"'))
            .map(|(name, _)| name)
            .unwrap();
        assert!(error.contains(named), "`{kind}` refused as `{error}`");
    }
    // Both admitted kinds are admitted.
    for kind in [
        "kind = \"recipe\"\nscript = \"m/out.sh\"",
        "kind = \"shell\"\ncommand = \"echo out\"",
    ] {
        let manifest = Manifest::parse_str(&with_export(&format!(
            "id = \"out\"\ntitle = \"Out\"\n{kind}"
        )))
        .unwrap();
        manifest.validate().unwrap();
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
fn every_migration_refusal_names_its_field() {
    let body = |entry: &str| with(&format!("\n[[application.migration]]\n{entry}\n"));
    let sides = "\n[application.migration.export]\nid = \"out\"\ntitle = \"Out\"\n\
                 kind = \"recipe\"\nscript = \"m/out.sh\"\n\n\
                 [application.migration.import]\nid = \"in\"\ntitle = \"In\"\n\
                 kind = \"recipe\"\nscript = \"m/in.sh\"\n";
    for (entry, needle) in [
        (
            format!("id = \"Carry It\"\nname = \"Carry\"{sides}"),
            "application.migration[0].id",
        ),
        (
            format!("id = \"carry\"\nname = \"  \"{sides}"),
            "application.migration[0].name",
        ),
    ] {
        let error = refusal(&body(&entry));
        assert!(error.contains(needle), "`{entry}` refused as `{error}`");
    }
    for (export, needle) in [
        (
            "id = \"Out Loud\"\ntitle = \"Out\"\nkind = \"recipe\"\nscript = \"m/out.sh\"",
            "application.migration[0].export.id",
        ),
        (
            "id = \"out\"\ntitle = \" \"\nkind = \"recipe\"\nscript = \"m/out.sh\"",
            "application.migration[0].export.title",
        ),
        (
            "id = \"out\"\ntitle = \"Out\"\nkind = \"recipe\"\nscript = \"m/out.sh\"\n\
             requires = [{ package = \"org.example/other\" }]",
            "application.migration[0].export.requires",
        ),
        (
            "id = \"out\"\ntitle = \"Out\"\nkind = \"recipe\"\nscript = \"m/out.sh\"\n\
             when = { feature = \"x\" }",
            "application.migration[0].export.when",
        ),
        (
            "id = \"out\"\ntitle = \"Out\"\nkind = \"recipe\"\nscript = \"m/out.sh\"\n\
             unless = { offline = true }",
            "application.migration[0].export.unless",
        ),
        (
            "id = \"out\"\ntitle = \"Out\"\nkind = \"recipe\"\nscript = \"m/out.sh\"\n\
             phase = \"finish\"",
            "application.migration[0].export.phase",
        ),
    ] {
        let error = refusal(&with_export(export));
        assert!(error.contains(needle), "`{export}` refused as `{error}`");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION")]
fn two_entries_never_share_an_id() {
    let twice = refusal(&with(&format!("{ENTRY}{ENTRY}")));
    assert!(
        twice.contains("application.migration[1].id") && twice.contains("global-packages"),
        "{twice}"
    );
}
