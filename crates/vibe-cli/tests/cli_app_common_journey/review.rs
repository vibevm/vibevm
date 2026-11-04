//! Actual update refuses loss of a former automatic input before any effect.
use super::*;
fn definition(r: &AppRegistry, version: &str, parameter: bool, old: bool) {
    let automatic = if !old {
        String::new()
    } else if parameter {
        "[[installer_options.configuration.parameters]]\nid='n'\nname='Count'\ntype='integer'\ndefault=1\n".into()
    } else {
        "[[installer_options.configuration.groups]]\nmode='choice'\nid='g'\nname='Group'\ncardinality='exactly-one'\nmembers=['a']\ndefaults=['a']\n".into()
    };
    let package = r.package(
        "review",
        version,
        "review",
        &format!(
            r#"
result='owned'
[[installer_options.steps]]
id='build'
title='Build'
kind='recipe'
script='build.sh'
[[installer_options.steps]]
id='feature-effect'
title='Feature effect'
kind='shell'
when={{feature='a'}}
command="echo selected >> '{}'"
[[installer_options.configuration.features]]
id='a'
name='A'
required={parameter}
{automatic}
"#,
            r.root.path().join("selected").display()
        ),
    );
    fs::write(package.join("build.sh"),format!("set -e\nprintf '%s\\n' \"${{VIBE_PARAM_N-unset}}\" >> '{}'\nmkdir -p \"$VIBE_OUT\"\necho built > \"$VIBE_OUT/receipt\"\n",r.root.path().join("effects").display())).unwrap();
}
#[test]
fn deleting_the_last_automatic_input_requires_real_update_review_before_effects() {
    for parameter in [false, true] {
        let r = AppRegistry::new();
        definition(&r, "1.0.0", parameter, true);
        let input = r.root.path().join("input.json");
        let request = json!({"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":if parameter{"require-selection"}else{"use-declared-defaults"},"features":[],"defaultFeatures":false,"selections":{},"parameters":{}},"disabledFeatures":[]}});
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        ok(&r.app(
            &["install", "-g"],
            "org.example/review",
            &[
                "--configuration-input",
                input.to_str().unwrap(),
                "--json",
                "--offline",
                "--unattended",
            ],
        ));
        let before = fs::read(r.settings().join("applications/index.json")).unwrap();
        let effects = fs::read(r.root.path().join("effects")).unwrap();
        let selected = fs::read(r.root.path().join("selected")).unwrap();
        fs::remove_file(&input).unwrap();
        definition(&r, "2.0.0", parameter, false);
        let output = r.app(
            &["update", "-g"],
            "org.example/review",
            &["--json", "--offline", "--unattended"],
        );
        let (status, error) = common::fake_agent::refused(&output);
        assert_eq!(status, Some(2));
        assert_eq!(error["code"], "configuration_unresolved");
        assert_eq!(
            fs::read(r.settings().join("applications/index.json")).unwrap(),
            before
        );
        assert_eq!(fs::read(r.root.path().join("effects")).unwrap(), effects);
        assert_eq!(fs::read(r.root.path().join("selected")).unwrap(), selected);
        assert!(
            !r.settings()
                .join("cache/app-work/org.example.review@2.0.0")
                .exists()
        );
        fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
        ok(&r.app(
            &["update", "-g"],
            "org.example/review",
            &[
                "--configuration-input",
                input.to_str().unwrap(),
                "--json",
                "--offline",
                "--unattended",
            ],
        ));
        assert_eq!(
            fs::read_to_string(r.root.path().join("effects"))
                .unwrap()
                .lines()
                .count(),
            2
        );
        assert_eq!(
            r.index()["installations"]["org.example/review@2.0.0"]["configuration"]["evaluation"]["reliance"],
            json!([])
        );
    }
}
