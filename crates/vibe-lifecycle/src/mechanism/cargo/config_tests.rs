//! The structured-config and selector laws — §5's law 4, and the strict
//! posture our own tables take (as against the lenient one the foreign
//! message stream gets next door).

use specmark::verifies;

use super::*;

fn table(toml_text: &str) -> ExtensionConfig {
    match toml_text.parse::<toml::Table>() {
        Ok(parsed) => ExtensionConfig::from_table(parsed),
        Err(error) => panic!("the fixture table parses: {error}"),
    }
}

fn parse(toml_text: &str) -> Result<CargoBuildConfig, MechanismError> {
    CargoBuildConfig::parse("helper", Some(&table(toml_text)))
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_absent_config_is_the_empty_config() {
    let parsed = match CargoBuildConfig::parse("helper", None) {
        Ok(parsed) => parsed,
        Err(error) => panic!("an absent config parses: {error}"),
    };

    assert_eq!(parsed, CargoBuildConfig::default());
    assert!(parsed.build_arguments().is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn every_law_four_member_is_read_and_rendered_in_a_fixed_order() {
    let parsed = match parse(concat!(
        "manifest_path = \"crates/helper/Cargo.toml\"\n",
        "package = \"vibe-helper\"\n",
        "target_kind = \"bin\"\n",
        "target_name = \"vibe-helper\"\n",
        "profile = \"release\"\n",
        "target_triple = \"x86_64-pc-windows-msvc\"\n",
        "features = [\"a\", \"b\"]\n",
        "no_default_features = true\n",
        "locked = true\n",
        "offline = true\n",
        "frozen = true\n",
    )) {
        Ok(parsed) => parsed,
        Err(error) => panic!("the full config parses: {error}"),
    };

    assert_eq!(
        parsed.manifest_path.as_deref(),
        Some("crates/helper/Cargo.toml")
    );
    assert_eq!(parsed.target_kind.as_deref(), Some("bin"));
    assert_eq!(
        parsed.build_arguments(),
        vec![
            "--package",
            "vibe-helper",
            "--bin",
            "vibe-helper",
            "--profile",
            "release",
            "--target",
            "x86_64-pc-windows-msvc",
            "--features",
            "a,b",
            "--no-default-features",
            "--locked",
            "--offline",
            "--frozen",
        ]
    );
    assert_eq!(
        parsed.posture_arguments(),
        vec!["--locked", "--offline", "--frozen"]
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_unknown_member_refuses_and_names_the_whole_vocabulary() {
    let refusal = parse("prfile = \"release\"").expect_err("our own table is strict");

    match &refusal {
        MechanismError::Config { member, reason, .. } => {
            assert_eq!(member, "prfile");
            assert!(reason.contains("target_triple"), "{reason}");
        }
        other => panic!("expected a config refusal, got {other}"),
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn an_engine_owned_member_refuses_with_its_reason_not_as_unknown() {
    for member in ["target_dir", "target-dir", "message_format", "env"] {
        let refusal = parse(&format!("{member} = \"x\""))
            .expect_err("the engine owns paths, the message format and the environment");
        match &refusal {
            MechanismError::Config { reason, .. } => {
                assert!(!reason.contains("unknown member"), "{member}: {reason}");
            }
            other => panic!("expected a config refusal, got {other}"),
        }
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_mistyped_member_refuses_naming_what_it_found() {
    let wrong_string = parse("profile = 7").expect_err("profile is a string");
    assert!(wrong_string.to_string().contains("found integer"));

    let wrong_flag = parse("locked = \"yes\"").expect_err("locked is a boolean");
    assert!(wrong_flag.to_string().contains("expected a boolean"));

    let wrong_list = parse("features = \"a\"").expect_err("features is an array");
    assert!(wrong_list.to_string().contains("array of strings"));

    let blank = parse("package = \"  \"").expect_err("a blank package names nothing");
    assert!(blank.to_string().contains("non-blank"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_comma_bearing_feature_refuses_because_cargo_joins_with_commas() {
    let refusal = parse("features = [\"a,b\"]").expect_err("a comma would smuggle two features");

    assert!(refusal.to_string().contains("carries a comma"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn contradictory_and_unbuildable_combinations_refuse_before_a_spawn() {
    let contradiction = parse("all_features = true\nfeatures = [\"a\"]")
        .expect_err("all features and some features contradict");
    assert!(contradiction.to_string().contains("contradict"));

    let wrong_kind = parse("target_kind = \"lib\"")
        .expect_err("this provider produces executables, so the kind is bin");
    assert!(wrong_kind.to_string().contains("the only kind is `bin`"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn the_network_answer_folds_the_run_posture_with_the_targets() {
    let open = match parse("profile = \"release\"") {
        Ok(parsed) => parsed,
        Err(error) => panic!("the config parses: {error}"),
    };
    assert!(open.reaches_network(false));
    assert!(
        !open.reaches_network(true),
        "a run-level offline posture wins"
    );

    let sealed = match parse("offline = true") {
        Ok(parsed) => parsed,
        Err(error) => panic!("the config parses: {error}"),
    };
    assert!(!sealed.reaches_network(false));

    let frozen = match parse("frozen = true") {
        Ok(parsed) => parsed,
        Err(error) => panic!("the config parses: {error}"),
    };
    assert!(!frozen.reaches_network(false), "frozen implies offline");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#ONE-MACHINE")]
fn a_selector_reads_package_and_bin_and_refuses_anything_else() {
    let both = match OutputSelect::parse(
        "helper",
        "helper.exe",
        Some(&table("package = \"vibe-helper\"\nbin = \"vibe-helper\"")),
    ) {
        Ok(parsed) => parsed,
        Err(error) => panic!("the selector parses: {error}"),
    };
    assert_eq!(both.package.as_deref(), Some("vibe-helper"));
    assert_eq!(both.bin.as_deref(), Some("vibe-helper"));
    assert_eq!(both.describe(), "package `vibe-helper` bin `vibe-helper`");

    let absent = match OutputSelect::parse("helper", "helper.exe", None) {
        Ok(parsed) => parsed,
        Err(error) => panic!("an absent selector parses: {error}"),
    };
    assert_eq!(absent, OutputSelect::default());
    assert_eq!(absent.describe(), "any executable artifact of this build");

    let unknown = OutputSelect::parse("helper", "helper.exe", Some(&table("example = \"x\"")))
        .expect_err("our own table is strict");
    assert!(unknown.to_string().contains("`package` and/or `bin`"));

    let mistyped = OutputSelect::parse("helper", "helper.exe", Some(&table("bin = 7")))
        .expect_err("a bin name is a string");
    assert!(mistyped.to_string().contains("found integer"));
}
