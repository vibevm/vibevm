//! Registered global application CLI report wire.

use vibe_wire::generated::application_report::{ApplicationReport, ApplicationReportCommand};

#[test]
fn application_report_round_trips_exact_camel_case_members() {
    let report = ApplicationReport {
        application_id: "demo".into(),
        command: ApplicationReportCommand::Install,
        host_root: "C:/settings/opt/apps/demo".into(),
        message: "ready".into(),
        ok: true,
        protocol: "vibe-application-command-report/1".into(),
    };
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["applicationId"], "demo");
    assert_eq!(value["hostRoot"], "C:/settings/opt/apps/demo");
    assert_eq!(value["command"], "install");
    assert_eq!(
        serde_json::from_value::<ApplicationReport>(value).unwrap(),
        report
    );
}
