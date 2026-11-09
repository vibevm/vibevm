use vibe_app::{PackageCoordinate, Plan, PlannedDependency, PlannedStep, RequiredBy};
use vibe_core::manifest::{RequirementScope, ResultDecl, StepDecl, StepKind};

use super::render;

fn required(package: &str, step: Option<&str>, installed: bool) -> PlannedDependency {
    PlannedDependency {
        package: package.into(),
        scope: RequirementScope::Build,
        step: step.map(str::to_owned),
        key: format!("{package}.0"),
        installed,
        hidden: true,
        commands: vec![],
        option: None,
        resolved: None,
    }
}

fn plan(steps: Vec<PlannedStep>, dependencies: Vec<PlannedDependency>) -> Plan {
    Plan {
        source_evidence: None,
        import_scope: None,
        configuration: None,
        secret_origins: Default::default(),
        secret_provided: Vec::new(),
        package: PackageCoordinate {
            group: "org.example".into(),
            name: "tool".into(),
            version: "1.0.0".into(),
        },
        option: "main".into(),
        platform: "linux".into(),
        arch: "x86_64".into(),
        result: ResultDecl::owned(),
        declared: vec![],
        profile: None,
        features: vec![],
        implied: vec![],
        parameters: vec![],
        license: None,
        download_bytes: 0,
        unknown_sizes: 0,
        drop_inputs: 0,
        dependencies,
        steps,
        warnings: vec![],
        required_by: None,
    }
}

fn copy(id: &str, title: &str, run: bool, reason: Option<&str>) -> PlannedStep {
    PlannedStep {
        agent_mode: None,
        prepared_prompt: None,
        step: StepDecl::new(
            id,
            title,
            StepKind::Copy {
                from: "payload".into(),
                to: String::new(),
            },
        ),
        run,
        reason: reason.map(str::to_owned),
        note: None,
    }
}

#[test]
fn the_plan_opens_with_its_rule_and_lists_its_steps_with_requirements_as_children() {
    let mut plan = plan(
        vec![
            copy("fetch", "Download the sources", true, None),
            copy("build", "Build", true, None),
            copy(
                "docs",
                "Copy the documentation",
                false,
                Some("feature docs off"),
            ),
        ],
        vec![
            required("org.example/mingw@15", Some("build"), false),
            required("org.example/ninja@1", Some("build"), true),
        ],
    );
    plan.required_by = Some(RequiredBy {
        package: "org.example/consumer@2.0.0".into(),
        step: Some("compile".into()),
    });
    let text = render(&plan, Some("vibevm-app"));
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines[0].starts_with(
            "── plan for org.example/tool@1.0.0 required by org.example/consumer@2.0.0 \
             for step compile ─"
        ),
        "{text}"
    );
    assert!(lines[0].ends_with("─ installed by vibevm-app"), "{text}");
    assert_eq!(lines[1], "");
    assert_eq!(lines[2], "  installer  main (linux/x86_64) · result owned");
    assert_eq!(lines[3], "");
    assert_eq!(lines[4], "  1.  fetch  (copy)  Download the sources");
    assert_eq!(lines[5], "  2.  build  (copy)  Build");
    assert_eq!(
        lines[6],
        "      └ requires org.example/mingw@15.0 (build, hidden) for org.example/mingw@15"
    );
    assert_eq!(
        lines[7],
        "      └ requires org.example/ninja@1.0 (build, hidden) for org.example/ninja@1  \
         [installed]"
    );
    assert_eq!(
        lines[8],
        "○ 3.  docs   (copy)  Copy the documentation  skipped: feature docs off"
    );
    assert!(!text.contains('`'), "no backticks in human output: {text}");
}

/// The download row prices only what will be transferred: an input the
/// `[artifacts] packages` drop already holds is named as the drop's, and a
/// plan whose inputs all come from the drop still says so instead of
/// silently pricing nothing (`PROP-066#drops`).
#[test]
fn the_download_row_names_the_inputs_a_drop_serves() {
    let mut served = plan(vec![], vec![]);
    served.drop_inputs = 2;
    let text = render(&served, None);
    assert!(
        text.contains("2 input(s) from the [artifacts] packages drop"),
        "{text}"
    );
    assert!(text.contains("download"), "{text}");

    // Nothing to download and no drop: the row stays absent.
    let quiet = render(&plan(vec![], vec![]), None);
    assert!(!quiet.contains("download"), "{quiet}");
}
