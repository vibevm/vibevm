//! Registered global application CLI report wire.

use vibe_wire::generated::application_report::{
    ApplicationReport, ApplicationReportCommand, ApplicationReportSelectedMode,
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
}
