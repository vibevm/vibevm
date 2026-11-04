//! Environment consumers must not erase distinct feature or parameter IDs.
use super::{HEADER, Manifest};

#[cfg(test)]
fn declarations(namespace: &str, common: bool, ids: &[&str]) -> String {
    let prefix = if common {
        "installer_options.configuration"
    } else {
        "installer_options"
    };
    ids.iter()
        .map(|id| {
            format!(
                "[[{prefix}.{namespace}]]\nid={id:?}\nname={id:?}\n{}\n",
                if namespace == "parameters" {
                    "type='string'"
                } else {
                    ""
                }
            )
        })
        .collect()
}

#[test]
fn colliding_declared_feature_and_parameter_env_ids_refuse_in_both_orders() {
    let mut missed = Vec::new();
    for common in [false, true] {
        for namespace in ["features", "parameters"] {
            for ids in [["a-b", "a.b"], ["a.b", "a-b"]] {
                for kind in [
                    "kind='recipe'\nscript='build.sh'",
                    "kind='shell'\ncommand='true'\nwhen={offline=false}",
                ] {
                    let text = format!(
                        "{HEADER}result={{external='/tmp/external'}}\n{}[[installer_options.steps]]\nid='step'\ntitle='Step'\n{kind}\n",
                        declarations(namespace, common, &ids)
                    );
                    match Manifest::parse_str(&text) {
                        Ok(_) => missed.push((common, namespace, ids, kind)),
                        Err(e) => {
                            let e = e.to_string();
                            assert!(
                                e.contains("a-b") && e.contains("a.b") && e.contains("_A_B"),
                                "{e}"
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(
        missed.is_empty(),
        "ambiguous definitions admitted: {missed:?}"
    );
}

#[test]
fn common_literal_ids_refuse_only_for_declared_environment_consumers() {
    for id in ["space id", "日本語", "a=b"] {
        let declared = declarations("features", true, &[id]);
        let with = |kind: &str| {
            format!(
                "{HEADER}result={{external='/tmp/external'}}\n{declared}[[installer_options.steps]]\nid='step'\ntitle='Step'\n{kind}\n"
            )
        };
        assert!(Manifest::parse_str(&with("kind='agent'\nprompt='Public prompt'")).is_ok());
        assert!(Manifest::parse_str(&with("kind='env'\nset={DEMO='public'}")).is_ok());
        let error = Manifest::parse_str(&with("kind='recipe'\nscript='build.sh'"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(id) && error.contains("VIBE_FEATURE_"),
            "{error}"
        );
        let package = format!(
            "[package]\nname='data'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n[[configuration.features]]\nid={id:?}\nname='Literal'\n"
        );
        assert!(Manifest::parse_str(&package).is_ok());
    }
}

#[test]
fn expanded_recipe_sugar_and_shell_hooks_are_environment_consumers() {
    let declared = declarations("features", true, &["a-b", "a.b"]);
    for body in [
        format!(
            "{HEADER}{declared}[installer_options.installation]\ntype='source_build'\nrecipe={{script='build.sh'}}\n"
        ),
        format!(
            "{HEADER}result={{external='/tmp/external'}}\npre_install='true'\n{declared}[[installer_options.steps]]\nid='agent'\ntitle='Agent'\nkind='agent'\nprompt='Public'\n"
        ),
        format!(
            "{HEADER}result={{external='/tmp/external'}}\npost_install='true'\n{declared}[[installer_options.steps]]\nid='agent'\ntitle='Agent'\nkind='agent'\nprompt='Public'\n"
        ),
    ] {
        let e = Manifest::parse_str(&body).unwrap_err().to_string();
        assert!(
            e.contains("a-b") && e.contains("a.b") && e.contains("VIBE_FEATURE_A_B"),
            "{e}"
        );
    }
}

#[test]
fn distinct_prefixes_and_nonconsuming_options_keep_valid_ids() {
    let text = format!(
        "{HEADER}result={{external='/tmp/external'}}\n{}{}[[installer_options.steps]]\nid='recipe'\ntitle='Recipe'\nkind='recipe'\nscript='build.sh'\n",
        declarations("features", false, &["a-b"]),
        declarations("parameters", false, &["a.b"])
    );
    Manifest::parse_str(&text).unwrap();
    let other = "[[installer_options]]\nid='other'\nname='Other'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\nresult={external='/tmp/other'}\n[[installer_options.features]]\nid='a.b'\nname='Other feature'\n[[installer_options.parameters]]\nid='a-b'\nname='Other parameter'\ntype='string'\n[[installer_options.steps]]\nid='recipe'\ntitle='Recipe'\nkind='recipe'\nscript='build.sh'\n";
    Manifest::parse_str(&format!("{text}{other}")).unwrap();
    let no_consumer = format!(
        "{HEADER}result={{external='/tmp/external'}}\n{}{}[[installer_options.steps]]\nid='agent'\ntitle='Agent'\nkind='agent'\nprompt='Public'\n",
        declarations("features", true, &["a-b", "a.b"]),
        declarations("parameters", true, &["a-b", "a.b"])
    );
    Manifest::parse_str(&no_consumer).unwrap();
}

#[test]
fn diagnostics_keep_original_row_positions_and_common_case_underscore_aliases() {
    use crate::manifest::FieldPath;
    for common in [false, true] {
        for namespace in ["features", "parameters"] {
            let source = format!(
                "id='main'\nname='Main'\nplatform='linux'\nplatform_name='Linux'\narch='x86_64'\nresult={{external='/tmp/external'}}\n{}[[steps]]\nid='step'\ntitle='Step'\nkind='shell'\ncommand='true'\n",
                declarations(namespace, common, &["z-z", "a-b", "a.b"])
                    .replace("installer_options.", "")
            );
            let option: crate::manifest::InstallerOptionDecl = toml::from_str(&source).unwrap();
            let error = option
                .validate_step_environment(&FieldPath::root().table("installer_options").index(7))
                .unwrap_err();
            let middle = if common { "configuration." } else { "" };
            assert_eq!(
                error.field.to_string(),
                format!("installer_options[7].{middle}{namespace}[2].id")
            );
            assert!(error.message.contains("a-b") && error.message.contains("a.b"));
        }
    }
    let source = format!(
        "{HEADER}result={{external='/tmp/external'}}\n{}[[installer_options.steps]]\nid='step'\ntitle='Step'\nkind='recipe'\nscript='build.sh'\n",
        declarations("features", true, &["A-B", "a_b"])
    );
    assert!(
        Manifest::parse_str(&source)
            .unwrap_err()
            .to_string()
            .contains("VIBE_FEATURE_A_B")
    );
}

#[test]
fn archive_and_native_only_options_do_not_acquire_the_environment_restriction() {
    let sha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    for kind in [
        format!("kind='archive'\nformat='zip'\nsource='input.zip'\nsha256='{sha}'"),
        format!("kind='native'\nformat='exe'\nsource='input.exe'\nsha256='{sha}'"),
    ] {
        let source = format!(
            "{HEADER}result={{external='/tmp/external'}}\n{}[[installer_options.steps]]\nid='step'\ntitle='Step'\n{kind}\n",
            declarations("features", true, &["space id", "a-b", "a.b"])
        );
        let source = if kind.contains("kind='native'") {
            source.replace("platform = \"linux\"", "platform = \"windows\"")
        } else {
            source
        };
        Manifest::parse_str(&source).unwrap();
    }
}
