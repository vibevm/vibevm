//! Actual recipe environment: identity survives or admission refuses before effects.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-ENV-IDENTITY");

#[cfg(test)]
mod common;

use common::app_registry::AppRegistry;

#[cfg(test)]
fn fixture(ids: &[&str], namespace: &str) -> AppRegistry {
    let r = AppRegistry::new();
    let mut option = String::new();
    for id in ids {
        option.push_str(&format!(
            "[[installer_options.{namespace}]]\nid={id:?}\nname={id:?}\n{}\n",
            if namespace == "parameters" {
                "type='string'"
            } else {
                ""
            }
        ));
    }
    option.push_str("[[installer_options.steps]]\nid='probe'\ntitle='Probe'\nkind='recipe'\nscript='probe.sh'\n");
    let p = r.package("env-identity", "1.0.0", "env-identity", &option);
    std::fs::write(p.join("probe.sh"),"mkdir -p \"$VIBE_OUT\"\nprintf '%s\\n' \"${VIBE_FEATURE_A_B-unset}\" \"${VIBE_FEATURE_A_C-unset}\" \"${VIBE_PARAM_A_B-unset}\" \"${VIBE_PARAM_A_C-unset}\" > \"$VIBE_OUT/observed.txt\"\n").unwrap();
    r
}

#[test]
fn colliding_feature_or_parameter_ids_refuse_global_install_in_both_orders() {
    let mut admitted = Vec::new();
    for namespace in ["features", "parameters"] {
        for ids in [["a-b", "a.b"], ["a.b", "a-b"]] {
            let r = fixture(&ids, namespace);
            let flags = if namespace == "features" {
                vec!["--offline", "--unattended", "--json", "--feature", "a-b"]
            } else {
                vec![
                    "--offline",
                    "--unattended",
                    "--json",
                    "--set",
                    "a-b=LEFT_VALUE",
                    "--set",
                    "a.b=RIGHT_VALUE",
                ]
            };
            let output = r.app(&["install", "-g"], "org.example/env-identity", &flags);
            let text = format!(
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            if output.status.success() {
                admitted.push((namespace, ids));
                continue;
            }
            assert!(
                text.contains("a-b") && text.contains("a.b") && text.contains("_A_B"),
                "{text}"
            );
            assert!(
                text.contains(&format!("installer_options[0].{namespace}[1].id")),
                "{text}"
            );
            assert!(
                !text.contains("LEFT_VALUE") && !text.contains("RIGHT_VALUE"),
                "{text}"
            );
            assert!(!r.settings().join("applications/index.json").exists());
            assert!(!r.settings().join("cache/app-work").exists());
        }
    }
    assert!(
        admitted.is_empty(),
        "ambiguous global installs succeeded: {admitted:?}"
    );
}

#[test]
fn distinct_projection_preserves_all_child_feature_and_parameter_values() {
    let r = fixture(&["a-b", "a-c"], "features");
    let p = r
        .registry()
        .join("org.example/env-identity/v1.0.0/vibe.toml");
    let s=std::fs::read_to_string(&p).unwrap().replace("[[installer_options.steps]]","[[installer_options.parameters]]\nid='a-b'\nname='Left'\ntype='string'\n[[installer_options.parameters]]\nid='a-c'\nname='Right'\ntype='string'\n[[installer_options.steps]]");
    std::fs::write(p, s).unwrap();
    let o = r.app(
        &["install", "-g"],
        "org.example/env-identity",
        &[
            "--offline",
            "--unattended",
            "--json",
            "--feature",
            "a-b",
            "--set",
            "a-b=LEFT_VALUE",
            "--set",
            "a-c=RIGHT_VALUE",
        ],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let bytes = std::fs::read_to_string(
        r.settings()
            .join("opt/apps/env-identity/org.example.env-identity@1.0.0/observed.txt"),
    )
    .unwrap();
    assert_eq!(bytes, "1\n0\nLEFT_VALUE\nRIGHT_VALUE\n");
}
