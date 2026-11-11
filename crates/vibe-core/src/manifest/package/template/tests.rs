//! The manifest-layer rules of `[package] template` and `[template]`.

use crate::manifest::Manifest;

use super::{DigestSource, Ttl, VersionSource};

const TEMPLATE_TABLE: &str = r#"
[template.versions]
kind = "json"
url = "https://nodejs.org/dist/index.json"
select = ""
strip_prefix = "v"

[template.digests]
kind = "sha256sums"
url = "https://nodejs.org/dist/v{version}/SHASUMS256.txt"

[[template.channel]]
rule = "truthy"
name = "lts"
from = "lts"

[[template.channel]]
rule = "value"
prefix = "lts"
from = "lts"
"#;

/// `[package]` of a template application, with `template` present or not.
fn package(kind: &str, flag: bool) -> String {
    format!(
        "[package]\nname = \"node\"\ngroup = \"org.vibevm.node\"\nkind = \"{kind}\"\n\
         version = \"1.0.0\"\nepoch = 1\n{}\n[application]\nid = \"node\"\n",
        if flag { "template = true" } else { "" }
    )
}

/// A whole template manifest with one installer option.
fn template_manifest(flag: bool, option_extra: &str) -> String {
    format!(
        "{}{TEMPLATE_TABLE}\n[[installer_options]]\nid = \"linux-x64\"\n\
         name = \"Node.js {{version}}\"\nplatform = \"linux\"\n\
         platform_name = \"Linux\"\narch = \"x86_64\"\npath_dirs = [\"bin\"]\n{option_extra}\n\
         [[installer_options.steps]]\nid = \"node\"\ntitle = \"Unpack\"\nkind = \"archive\"\n\
         format = \"tar_gz\"\n\
         source = \"https://nodejs.org/dist/v{{version}}/node-v{{version}}-linux-x64.tar.gz\"\n\
         sha256 = \"{{sha256:node-v{{version}}-linux-x64.tar.gz}}\"\n\
         payload_root = \"node-v{{version}}-linux-x64\"\n",
        package("app", flag)
    )
}

#[test]
fn the_flag_and_the_table_round_trip() {
    let manifest = Manifest::parse_str(&template_manifest(true, "")).expect("parses");
    assert!(manifest.is_template());
    let section = manifest.template.as_ref().expect("[template] present");
    assert!(matches!(
        section.versions,
        VersionSource::Json { ref select, .. } if select.is_empty()
    ));
    assert!(matches!(
        section.digests,
        Some(DigestSource::Sha256Sums { .. })
    ));
    assert_eq!(section.channels.len(), 2);
    assert!(!section.has_sizes());
    // The round trip keeps both, and an ordinary manifest keeps neither.
    let text = toml::to_string(&manifest).expect("serialises");
    assert!(text.contains("template = true"), "{text}");
    assert!(text.contains("[template.versions]"), "{text}");
    let plain = Manifest::parse_str(&format!(
        "{}\n[[installer_options]]\nid = \"a\"\nname = \"A\"\nplatform = \"linux\"\n\
         platform_name = \"Linux\"\narch = \"x86_64\"\npath_dirs = [\"bin\"]\n\
         [[installer_options.steps]]\nid = \"s\"\ntitle = \"t\"\nkind = \"copy\"\nfrom = \"x\"\n",
        package("app", false)
    ))
    .expect("parses");
    assert!(!plain.is_template());
    assert!(!toml::to_string(&plain).unwrap().contains("template"));
}

#[test]
fn the_table_without_the_flag_refuses_naming_its_path() {
    let error = Manifest::parse_str(&template_manifest(false, "")).expect_err("refuses");
    let message = error.to_string();
    assert!(message.contains("[template] without"), "{message}");
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("template"),
        "{message}"
    );
}

#[test]
fn the_flag_without_the_table_refuses_naming_its_path() {
    let text = format!(
        "{}\n[[installer_options]]\nid = \"a\"\nname = \"A\"\nplatform = \"linux\"\n\
         platform_name = \"Linux\"\narch = \"x86_64\"\npath_dirs = [\"bin\"]\n\
         [[installer_options.steps]]\nid = \"s\"\ntitle = \"t\"\nkind = \"copy\"\nfrom = \"x\"\n",
        package("app", true)
    );
    let error = Manifest::parse_str(&text).expect_err("refuses");
    assert!(
        error.to_string().contains("without a [template] table"),
        "{error}"
    );
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("package.template")
    );
}

#[test]
fn the_flag_on_another_kind_refuses_naming_the_restriction() {
    let text = format!("{}{TEMPLATE_TABLE}", package("flow", true))
        .replace("\n[application]\nid = \"node\"\n", "\n");
    let error = Manifest::parse_str(&text).expect_err("refuses");
    let message = error.to_string();
    assert!(message.contains("kind = \"app\""), "{message}");
    assert!(message.contains("kind = \"flow\""), "{message}");
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("package.template")
    );
}

#[test]
fn a_digest_variable_is_admitted_only_in_a_template() {
    // The template manifest above holds `{sha256:…}` and validates.
    Manifest::parse_str(&template_manifest(true, "")).expect("a template may hold the variable");
    // The same document with `template` and `[template]` removed must not.
    let plain = template_manifest(true, "")
        .replace("template = true\n", "")
        .replace(TEMPLATE_TABLE, "\n");
    let error = Manifest::parse_str(&plain).expect_err("refuses outside a template");
    assert!(
        error
            .to_string()
            .contains("must be a 64-digit hexadecimal digest"),
        "{error}"
    );
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("installer_options[0].steps[0].sha256")
    );
}

#[test]
fn template_optional_outside_a_template_refuses() {
    let plain = template_manifest(true, "template_optional = true")
        .replace("template = true\n", "")
        .replace(TEMPLATE_TABLE, "\n")
        .replace(
            "sha256 = \"{sha256:node-v{version}-linux-x64.tar.gz}\"",
            &format!("sha256 = \"{}\"", "a".repeat(64)),
        );
    let error = Manifest::parse_str(&plain).expect_err("refuses");
    assert!(
        error.to_string().contains("declares template_optional"),
        "{error}"
    );
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("installer_options[0].template_optional")
    );
    // Beside the flag and the table it is admitted.
    let manifest = Manifest::parse_str(&template_manifest(true, "template_optional = true"))
        .expect("a template admits it");
    assert!(manifest.installer_options[0].template_optional);
}

#[test]
fn a_selector_that_is_not_a_json_pointer_refuses() {
    let text = template_manifest(true, "").replace("select = \"\"", "select = \"$.versions[*]\"");
    let error = Manifest::parse_str(&text).expect_err("refuses");
    assert!(error.to_string().contains("RFC 6901"), "{error}");
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("template.versions.select")
    );
}

#[test]
fn a_version_list_url_carrying_a_variable_refuses() {
    let text = template_manifest(true, "").replace(
        "url = \"https://nodejs.org/dist/index.json\"",
        "url = \"https://n.invalid/{version}.json\"",
    );
    let error = Manifest::parse_str(&text).expect_err("refuses");
    assert!(
        error.to_string().contains("cannot depend on a version"),
        "{error}"
    );
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("template.versions.url")
    );
}

#[test]
fn a_sizes_claim_a_sha256sums_source_cannot_keep_refuses() {
    let text = template_manifest(true, "").replace(
        "kind = \"sha256sums\"",
        "kind = \"sha256sums\"\nsizes = true",
    );
    let error = Manifest::parse_str(&text).expect_err("refuses");
    assert!(error.to_string().contains("carries no sizes"), "{error}");
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("template.digests.sizes")
    );
}

#[test]
fn a_channel_name_outside_the_grammar_refuses() {
    let text = template_manifest(true, "").replace("name = \"lts\"", "name = \"LTS\"");
    let error = Manifest::parse_str(&text).expect_err("refuses");
    assert!(
        error.to_string().contains("is not a channel name"),
        "{error}"
    );
    assert_eq!(
        error.manifest_field().map(|f| f.to_string()).as_deref(),
        Some("template.channel[0].name")
    );
}

#[test]
fn an_unknown_key_in_the_table_refuses() {
    let text =
        template_manifest(true, "").replace("select = \"\"", "select = \"\"\nfilter = \"x\"");
    let error = Manifest::parse_str(&text).expect_err("refuses");
    assert!(error.to_string().contains("filter"), "{error}");
}

#[test]
fn a_list_source_needs_at_least_one_item() {
    let text = template_manifest(true, "").replace(
        "kind = \"json\"\nurl = \"https://nodejs.org/dist/index.json\"\nselect = \"\"\nstrip_prefix = \"v\"",
        "kind = \"list\"",
    );
    let error = Manifest::parse_str(&text).expect_err("refuses");
    assert!(error.to_string().contains("at least one"), "{error}");
}

#[test]
fn the_ttl_round_trips_through_the_table() {
    let text = template_manifest(true, "").replace(
        "[template.versions]",
        "[template]\nversion_list_ttl = \"2h\"\n\n[template.versions]",
    );
    let manifest = Manifest::parse_str(&text).expect("parses");
    assert_eq!(
        manifest
            .template
            .as_ref()
            .and_then(|section| section.version_list_ttl),
        Some("2h".parse::<Ttl>().unwrap())
    );
    assert!(
        toml::to_string(&manifest)
            .unwrap()
            .contains("version_list_ttl = \"2h\""),
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn package_channels_round_trip_and_refuse_by_name() {
    let concrete = concat!(
        "[package]\ngroup = \"org.vibevm.node\"\nname = \"node\"\nkind = \"app\"\n",
        "version = \"22.23.4\"\nchannels = [\"lts\", \"lts/jod\"]\n\n",
        "[application]\nid = \"node\"\n",
    );
    let manifest = crate::manifest::Manifest::parse_str(concrete).unwrap();
    assert_eq!(
        manifest.package.as_ref().unwrap().channels,
        vec!["lts".to_string(), "lts/jod".to_string()]
    );
    // Absent means byte-identical: nothing is written for an empty list.
    let rendered = manifest.render_with_comments(None).unwrap();
    assert!(rendered.contains("channels = ["), "{rendered}");
    assert!(rendered.contains("\"lts/jod\""), "{rendered}");
    let plain = crate::manifest::Manifest::parse_str(
        "[package]\ngroup = \"org.e\"\nname = \"p\"\nkind = \"flow\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let rendered = plain.render_with_comments(None).unwrap();
    assert!(!rendered.contains("channels"), "{rendered}");

    // A label that is not a channel name refuses, naming its TOML path.
    let bad = crate::manifest::Manifest::parse_str(&concrete.replace("\"lts/jod\"", "\"LTS\""))
        .unwrap_err();
    assert!(bad.to_string().contains("[package].channels"), "{bad}");
    assert!(bad.to_string().contains("LTS"), "{bad}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn a_template_may_not_also_label_itself_with_channels() {
    let text = concat!(
        "[package]\ngroup = \"org.e\"\nname = \"tool\"\nkind = \"app\"\nversion = \"0.1.0\"\n",
        "template = true\nchannels = [\"lts\"]\n\n[application]\nid = \"tool\"\n\n",
        "[template.versions]\nkind = \"list\"\n\n",
        "[[template.versions.item]]\nversion = \"1.0.0\"\n\n",
        "[template.digests]\nkind = \"sha256sums\"\n",
        "url = \"https://e.invalid/v{version}/SHASUMS256.txt\"\n",
    );
    let error = crate::manifest::Manifest::parse_str(text).unwrap_err();
    assert!(error.to_string().contains("[package].channels"), "{error}");
    assert!(
        error.to_string().contains("[[template.channel]]"),
        "{error}"
    );
}
