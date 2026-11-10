//! Registered global application CLI report wire.

use vibe_wire::generated::application_report::{
    ApplicationReport, ApplicationReportCommand, ApplicationReportSelectedMode, Recommendation,
    RecommendationOutcome, VersionFile,
};

#[test]
fn application_report_round_trips_exact_camel_case_members() {
    let report = ApplicationReport {
        application_id: "demo".into(),
        command: ApplicationReportCommand::Install,
        installation: "org.example/demo@1.0.0".into(),
        message: "ready".into(),
        ok: true,
        protocol: "vibe-application-command-report/2".into(),
        slot_default: Some("org.example/demo@1.0.0".into()),
        app_root: Some("C:/settings/opt/apps/demo/org.example.demo@1.0.0".into()),
        work: None,
        selected_mode: Some(ApplicationReportSelectedMode::Zip),
        version_file: None,
        recommended: Vec::new(),
        suggested: Vec::new(),
    };
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["applicationId"], "demo");
    assert_eq!(value["slotDefault"], "org.example/demo@1.0.0");
    assert_eq!(value["command"], "install");
    assert_eq!(value["selectedMode"], "zip");
    assert_eq!(
        serde_json::from_value::<ApplicationReport>(value).unwrap(),
        report
    );
    let cleared = ApplicationReport {
        slot_default: None,
        ..report
    };
    assert!(serde_json::to_value(&cleared).unwrap()["slotDefault"].is_null());
    // A kept work directory (PROP-065#work) travels only when there is one.
    let kept = ApplicationReport {
        work: Some("C:/settings/cache/app-work/org.example.demo@1.0.0".into()),
        ..cleared
    };
    let value = serde_json::to_value(&kept).unwrap();
    assert_eq!(
        value["work"],
        "C:/settings/cache/app-work/org.example.demo@1.0.0"
    );
    assert_eq!(
        serde_json::from_value::<ApplicationReport>(value).unwrap(),
        kept
    );
    // The written version file (PROP-075#VERSION-FILE-WRITE) travels the
    // same way: only when `--write-version-file` wrote one.
    let saved = ApplicationReport {
        version_file: Some(VersionFile {
            path: "C:/project/.sdkmanrc".into(),
            version: "22.23.4".into(),
        }),
        ..kept
    };
    let value = serde_json::to_value(&saved).unwrap();
    assert_eq!(value["versionFile"]["path"], "C:/project/.sdkmanrc");
    assert_eq!(value["versionFile"]["version"], "22.23.4");
    assert_eq!(
        serde_json::from_value::<ApplicationReport>(value).unwrap(),
        saved
    );
    // The author's recommendations and suggestions (PROP-077
    // `#APP-RECOMMENDS`, `#SUGGESTS-UNCHANGED`): both omitted when the
    // package declared none, so every report written before them is
    // byte-identical.
    assert!(
        serde_json::to_value(&saved)
            .unwrap()
            .get("recommended")
            .is_none()
    );
    let weak = ApplicationReport {
        recommended: vec![Recommendation {
            package: "org.example/maven".into(),
            by: "org.example/jdk@1.0.0".into(),
            key: Some("org.example/maven@3.9.9".into()),
            outcome: RecommendationOutcome::Installed,
            reason: None,
        }],
        suggested: vec!["org.example/ide".into()],
        ..saved
    };
    let value = serde_json::to_value(&weak).unwrap();
    assert_eq!(value["recommended"][0]["package"], "org.example/maven");
    assert_eq!(value["recommended"][0]["outcome"], "installed");
    assert!(value["recommended"][0].get("reason").is_none());
    assert_eq!(value["suggested"][0], "org.example/ide");
    assert_eq!(
        serde_json::from_value::<ApplicationReport>(value).unwrap(),
        weak
    );
}
