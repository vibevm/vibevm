//! `[[application.ecosystem]]` grammar, refusals and the
//! recommends → feature synthesis (PROP-075 `#ECOSYSTEM-ADAPTER`,
//! `#ECOSYSTEM-RECOMMENDS`, `#ECOSYSTEM-MIGRATION`).

use super::Manifest;
use crate::manifest::{ECOSYSTEM_MIGRATION_PREFIX, EcosystemVerb, FeatureOrigin};

/// An `app` package with one `[[installer_options]]` in the steps form,
/// so a feature row is admitted and the fixture exercises the real reach.
const PACKAGE: &str = r#"
[package]
name = "demo"
group = "org.vibevm"
kind = "app"
version = "1.1.0"

[application]
id = "demo"

[[installer_options]]
id = "linux"
name = "Linux"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"
result = "owned"

[[installer_options.steps]]
id = "place"
title = "Place it"
kind = "copy"
from = "payload"
"#;

/// One adapter body: `install` as a recipe, plus whatever `extra` adds.
fn adapter(id: &str, extra: &str) -> String {
    format!(
        "\n[[application.ecosystem]]\nid = \"{id}\"\nname = \"{id} packages\"\n{extra}\n\
         [application.ecosystem.install]\nid = \"install\"\ntitle = \"Install items\"\n\
         kind = \"recipe\"\nscript = \"eco/install.sh\"\n"
    )
}

/// `install` plus a `list` verb, which is what the synthesised migration
/// entry needs.
fn with_list(id: &str) -> String {
    format!(
        "{}\n[application.ecosystem.list]\nid = \"list\"\ntitle = \"List items\"\n\
         kind = \"recipe\"\nscript = \"eco/list.sh\"\n",
        adapter(id, "")
    )
}

fn document(body: &str) -> String {
    format!("{PACKAGE}{body}")
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
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn ecosystem_rows_parse_declare_their_verbs_and_round_trip() {
    let manifest = Manifest::parse_str(&document(&format!(
        "{}\n[application.ecosystem.remove]\nid = \"remove\"\ntitle = \"Remove items\"\n\
         kind = \"recipe\"\nscript = \"eco/remove.sh\"\n\
         \n[application.ecosystem.self_upgrade]\nid = \"up\"\ntitle = \"Upgrade manager\"\n\
         kind = \"shell\"\ncommand = \"npm install -g npm@latest\"\n",
        with_list("npm")
    )))
    .unwrap();
    manifest.validate().unwrap();
    let application = manifest.application.as_ref().unwrap();
    assert_eq!(application.ecosystem.len(), 1);
    let row = &application.ecosystem[0];
    assert_eq!(row.id, "npm");
    // `migration` defaults to true, so an adapter with a `list` verb
    // carries its ecosystem state between versions unless it opts out.
    assert!(row.migration);
    assert_eq!(
        row.declared_verbs(),
        [
            EcosystemVerb::Install,
            EcosystemVerb::List,
            EcosystemVerb::Remove,
            EcosystemVerb::SelfUpgrade
        ]
    );
    assert_eq!(
        row.verb(EcosystemVerb::SelfUpgrade).unwrap().kind.name(),
        "shell"
    );
    // Absent is the default, so every existing manifest is byte-identical.
    assert!(
        Manifest::parse_str(PACKAGE)
            .unwrap()
            .application
            .unwrap()
            .ecosystem
            .is_empty()
    );
    assert_eq!(
        Manifest::parse_str(&toml::to_string(&manifest).unwrap()).unwrap(),
        manifest
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn a_shell_install_or_remove_is_refused_naming_the_field_and_the_reason() {
    let shell_install = document(
        "\n[[application.ecosystem]]\nid = \"npm\"\nname = \"npm packages\"\n\
         [application.ecosystem.install]\nid = \"install\"\ntitle = \"Install items\"\n\
         kind = \"shell\"\ncommand = \"npm install -g\"\n",
    );
    let message = refusal(&shell_install);
    assert!(
        message.contains("application.ecosystem[0].install.kind")
            && message.contains("kind = \"recipe\"")
            && message.contains("argv"),
        "{message}"
    );
    let shell_remove = document(&format!(
        "{}\n[application.ecosystem.remove]\nid = \"remove\"\ntitle = \"Remove items\"\n\
         kind = \"shell\"\ncommand = \"npm uninstall -g\"\n",
        adapter("npm", "")
    ));
    let message = refusal(&shell_remove);
    assert!(
        message.contains("application.ecosystem[0].remove.kind") && message.contains("argv"),
        "{message}"
    );
    // `list` and `self_upgrade` take no items, so either kind is admitted.
    let shell_list = document(&format!(
        "{}\n[application.ecosystem.list]\nid = \"list\"\ntitle = \"List items\"\n\
         kind = \"shell\"\ncommand = \"npm ls -g\"\n",
        adapter("npm", "")
    ));
    Manifest::parse_str(&shell_list)
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn each_verb_takes_only_recipe_or_shell_and_nothing_the_runner_cannot_give_it() {
    let archive = document(
        "\n[[application.ecosystem]]\nid = \"npm\"\nname = \"npm packages\"\n\
         [application.ecosystem.install]\nid = \"install\"\ntitle = \"Install items\"\n\
         kind = \"archive\"\nformat = \"tar_gz\"\nsource = \"https://example.invalid/a.tgz\"\n\
         sha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
    );
    let message = refusal(&archive);
    assert!(
        message.contains("application.ecosystem[0].install.kind")
            && message.contains("`recipe` or `shell`"),
        "{message}"
    );
    let conditional = document(&format!(
        "{}\n[application.ecosystem.list]\nid = \"list\"\ntitle = \"List items\"\n\
         kind = \"recipe\"\nscript = \"eco/list.sh\"\nwhen = {{ feature = \"x\" }}\n",
        adapter("npm", "")
    ));
    let message = refusal(&conditional);
    assert!(
        message.contains("application.ecosystem[0].list.when"),
        "{message}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn two_rows_with_one_id_refuse_and_several_adapters_are_admitted() {
    let twice = document(&format!("{}{}", adapter("npm", ""), adapter("npm", "")));
    let message = refusal(&twice);
    assert!(
        message.contains("application.ecosystem[1].id") && message.contains("declared twice"),
        "{message}"
    );
    let several = document(&format!(
        "{}{}",
        adapter("rustup-component", ""),
        adapter("cargo", "")
    ));
    let manifest = Manifest::parse_str(&several).unwrap();
    manifest.validate().unwrap();
    let application = manifest.application.as_ref().unwrap();
    assert!(application.ecosystem_adapter("cargo").is_some());
    assert!(application.ecosystem_adapter("npm").is_none());
    let uppercase = document(&adapter("NPM", ""));
    let message = refusal(&uppercase);
    assert!(
        message.contains("application.ecosystem[0].id") && message.contains("portable"),
        "{message}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS")]
fn a_recommends_row_synthesises_one_ordinary_feature_in_every_option() {
    let body = adapter(
        "npm",
        "\n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"corepack\"\n\
         item = \"corepack@latest\"\ndefault = true\nreason = \"Package-manager shims\"\n\
         \n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"typescript\"\n\
         item = \"typescript\"\nname = \"TypeScript\"\n",
    );
    // A second option, in the method form, which carries no authored
    // feature at all: the recommendation reaches it too, because it is a
    // property of the package and not of one platform's archive.
    let second = "\n[[installer_options]]\nid = \"mac\"\nname = \"macOS\"\nplatform = \"macos\"\n\
                  platform_name = \"macOS\"\narch = \"aarch64\"\n\
                  [installer_options.installation]\ntype = \"tar_gz\"\n\
                  source = \"https://example.invalid/a.tgz\"\n\
                  sha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n";
    let text = format!("{}{body}", document(second));
    let manifest = Manifest::parse_str(&text).unwrap();
    manifest.validate().unwrap();
    for option in &manifest.installer_options {
        let ids: Vec<&str> = option
            .features
            .iter()
            .map(|feature| feature.id.as_str())
            .collect();
        assert_eq!(ids, ["corepack", "typescript"], "option {}", option.id);
        assert!(
            option
                .features
                .iter()
                .all(|feature| feature.origin == FeatureOrigin::EcosystemRecommends)
        );
    }
    let corepack = &manifest.installer_options[0].features[0];
    assert_eq!(corepack.name, "corepack@latest");
    assert_eq!(
        corepack.description.as_deref(),
        Some("Package-manager shims")
    );
    assert!(corepack.default);
    assert!(corepack.parent.is_none() && !corepack.required && corepack.requires.is_empty());
    // The row's own `name` wins over the item string when it is given.
    assert_eq!(manifest.installer_options[0].features[1].name, "TypeScript");
    // The synthesised rows never reach the document, so the round trip is
    // the author's own spelling and the two cannot drift.
    let written = toml::to_string(&manifest).unwrap();
    assert!(!written.contains("installer_options.features"), "{written}");
    assert_eq!(Manifest::parse_str(&written).unwrap(), manifest);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS")]
fn a_recommends_row_naming_an_undeclared_ecosystem_refuses() {
    let text = document(&adapter(
        "npm",
        "\n[[application.ecosystem.recommends]]\necosystem = \"pip\"\nid = \"corepack\"\n\
         item = \"corepack\"\n",
    ));
    let message = refusal(&text);
    assert!(
        message.contains("application.ecosystem[0].recommends[0].ecosystem")
            && message.contains("`pip`")
            && message.contains("`npm`"),
        "{message}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS")]
fn a_recommends_id_colliding_with_an_authored_feature_refuses_naming_both_paths() {
    let authored = "\n[[installer_options.features]]\nid = \"corepack\"\nname = \"Corepack\"\n";
    let text = format!(
        "{}{authored}{}",
        PACKAGE,
        adapter(
            "npm",
            "\n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"corepack\"\n\
             item = \"corepack@latest\"\n",
        )
    );
    let message = refusal(&text);
    assert!(
        message.contains("application.ecosystem[0].recommends[0].id")
            && message.contains("installer_options[0].features")
            && message.contains("collides"),
        "{message}"
    );
    // A recommends id that collides with nothing is admitted beside the
    // authored rows, and the authored row keeps its place in front.
    let text = format!(
        "{}{authored}{}",
        PACKAGE,
        adapter(
            "npm",
            "\n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"typescript\"\n\
             item = \"typescript\"\n",
        )
    );
    let manifest = Manifest::parse_str(&text).unwrap();
    manifest.validate().unwrap();
    let ids: Vec<&str> = manifest.installer_options[0]
        .features
        .iter()
        .map(|feature| feature.id.as_str())
        .collect();
    assert_eq!(ids, ["corepack", "typescript"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS")]
fn a_duplicate_recommendation_id_refuses() {
    let text = document(&adapter(
        "npm",
        "\n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"corepack\"\n\
         item = \"corepack\"\n\
         \n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"corepack\"\n\
         item = \"corepack@latest\"\n",
    ));
    let message = refusal(&text);
    assert!(
        message.contains("application.ecosystem[0].recommends") && message.contains("twice"),
        "{message}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-MIGRATION")]
fn an_adapter_with_a_list_verb_synthesises_one_migration_entry() {
    let manifest = Manifest::parse_str(&document(&with_list("npm"))).unwrap();
    manifest.validate().unwrap();
    let application = manifest.application.as_ref().unwrap();
    let entries = application.migration_entries();
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.id, format!("{ECOSYSTEM_MIGRATION_PREFIX}npm"));
    assert_eq!(entry.name, "npm packages");
    // Its export is the adapter's `list` verb and its import the
    // `install` verb; `default = true`, so an attended offer preselects it.
    assert!(entry.default);
    assert_eq!(entry.export.id, "list");
    assert_eq!(entry.import.id, "install");
    // No `list` verb: nothing to observe, so nothing is synthesised.
    let manifest = Manifest::parse_str(&document(&adapter("npm", ""))).unwrap();
    assert!(manifest.application.unwrap().migration_entries().is_empty());
    // `migration = false` suppresses the synthesis.
    let manifest = Manifest::parse_str(&document(&with_list("npm").replace(
        "name = \"npm packages\"",
        "name = \"npm packages\"\nmigration = false",
    )))
    .unwrap();
    manifest.validate().unwrap();
    assert!(manifest.application.unwrap().migration_entries().is_empty());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-MIGRATION")]
fn an_authored_migration_id_in_the_reserved_namespace_refuses() {
    let text = document(
        "\n[[application.migration]]\nid = \"ecosystem:npm\"\nname = \"Globals\"\n\
         [application.migration.export]\nid = \"out\"\ntitle = \"Out\"\nkind = \"recipe\"\n\
         script = \"m/out.sh\"\n\
         [application.migration.import]\nid = \"in\"\ntitle = \"In\"\nkind = \"recipe\"\n\
         script = \"m/in.sh\"\n",
    );
    let message = refusal(&text);
    assert!(
        message.contains("application.migration[0].id")
            && message.contains("reserved")
            && message.contains(ECOSYSTEM_MIGRATION_PREFIX),
        "{message}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER")]
fn an_adapter_with_no_install_verb_is_refused_by_name() {
    let text = document(
        "\n[[application.ecosystem]]\nid = \"npm\"\nname = \"npm packages\"\n\
         [application.ecosystem.list]\nid = \"list\"\ntitle = \"List items\"\n\
         kind = \"recipe\"\nscript = \"eco/list.sh\"\n",
    );
    let message = refusal(&text);
    assert!(message.contains("install"), "{message}");
}
