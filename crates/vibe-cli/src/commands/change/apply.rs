//! Read-only explicit composition preparation, never scaffold application.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-FULL-PASS-CARRIAGE");
use clap::Args;
use std::{path::PathBuf, process::ExitCode};
#[derive(Debug, Args)]
pub struct ApplyArgs {
    pub id: String,
    #[arg(long)]
    pub work: String,
    #[arg(long, default_value = ".")]
    pub path: PathBuf,
    #[arg(long)]
    pub spec_root: Option<PathBuf>,
}
fn prepare(args: &ApplyArgs) -> anyhow::Result<serde_json::Value> {
    // Exact immutable ID syntax refuses before project discovery.
    vibe_core::manifest::change::ChangeId::parse(&args.id)?;
    let root = super::super::resolve_project_root(&args.path)?;
    let captured = vibe_orchestrator::change::query::CapturedChangeQuery::prepare(
        &root,
        args.spec_root.as_deref(),
        &args.id,
    )?;
    let plan = captured.full_pass_plan(&args.work)?;
    let selection = plan.selection();
    Ok(serde_json::json!({
        "status":"prepared", "execution_carriage":plan.execution_carriage(),
        "change":args.id, "selected_node":selection.selected_node(), "consumer":selection.consumer(),
        "capture":plan.capture_id(), "selected_work":selection.work(),
        "intent":{"recipe":selection.intent().recipe(),"digest":selection.intent().digest()},
        "contribution":{"recipe":selection.contribution().recipe(),"digest":selection.contribution().digest()},
        "operations":["production","current-source-verification"],
        "unbound_inputs":plan.unbound_inputs(), "production_request":null,
        "machine_operation_ids":null
    }))
}
pub(super) fn run(args: &ApplyArgs, json: bool) -> ExitCode {
    let (report, exit) = match prepare(args) {
        Ok(value) => (value, ExitCode::SUCCESS),
        Err(error) => (
            serde_json::json!({"status":"refused","reason":error.to_string(),"execution_carriage":"unavailable:m16-change-full-pass"}),
            ExitCode::from(1),
        ),
    };
    println!(
        "{}",
        if json {
            report.to_string()
        } else {
            serde_json::to_string_pretty(&report).unwrap()
        }
    );
    exit
}
#[cfg(test)]
mod m_03_a_2_cases {
    use super::*;
    use clap::Parser;
    #[test]
    fn explicit_apply_parses_one_work_and_invalid_id_refuses_before_discovery() {
        let parsed = crate::cli::Cli::try_parse_from([
            "vibe",
            "change",
            "apply",
            "chg-0123456789abcdef0123456789abcdef",
            "--work",
            "work-a",
        ])
        .unwrap();
        let crate::cli::Command::Change(change) = parsed.command else {
            panic!("wrong route")
        };
        let super::super::ChangeCommand::Apply(args) = change.command else {
            panic!("not preparation")
        };
        assert_eq!(args.work, "work-a");
        assert_eq!(args.path, PathBuf::from("."));
        assert!(
            crate::cli::Cli::try_parse_from([
                "vibe",
                "change",
                "apply",
                "chg-0123456789abcdef0123456789abcdef",
                "--work",
                "work-a",
                "--execute"
            ])
            .is_err()
        );
        let invalid = ApplyArgs {
            id: "bad".into(),
            work: "work-a".into(),
            path: "/deliberately-absent-root".into(),
            spec_root: None,
        };
        assert!(
            prepare(&invalid)
                .unwrap_err()
                .to_string()
                .contains("change.id")
        );
    }
    #[test]
    fn actual_saved_preparation_reports_two_operations_without_effects() {
        let root = tempfile::tempdir().unwrap();
        let id = "chg-0123456789abcdef0123456789abcdef";
        let directory = root.path().join("docs/changes").join(id);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            root.path().join("vibe.toml"),
            "[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n",
        )
        .unwrap();
        let mut manifest = format!(
            "schema=1\n[change]\nid='{id}'\nselected_node='.'\nslug='test'\ntitle='Test'\n[[origins]]\npackage='feat:org.demo/origin@=1.0.0'\nmanifest_sha256='sha256:{}'\n[[origins.documents]]\naddress='spec://org.demo/origin/FEAT#ONE'\nsha256='sha256:{}'\n",
            "a".repeat(64),
            "b".repeat(64)
        );
        for (role, path, normative) in [
            ("intent", "intent.md", true),
            ("acceptance", "acceptance.md", true),
            ("work", "work.toml", false),
        ] {
            manifest.push_str(&format!(
                "[[documents]]\nrole='{role}'\npath='{path}'\nnormative={normative}\ninput=true\n"
            ));
        }
        std::fs::write(directory.join("change.toml"), manifest).unwrap();
        std::fs::write(directory.join("intent.md"), b"Selected fixture intent").unwrap();
        std::fs::write(directory.join("acceptance.md"), b"Independent acceptance").unwrap();
        std::fs::write(
            directory.join("work.toml"),
            "schema=1\n[[items]]\nid='work-a'\n[[items]]\nid='work-b'\n",
        )
        .unwrap();
        fn inventory(
            root: &std::path::Path,
        ) -> std::collections::BTreeMap<PathBuf, Option<Vec<u8>>> {
            fn visit(
                base: &std::path::Path,
                at: &std::path::Path,
                rows: &mut std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>,
            ) {
                for entry in std::fs::read_dir(at).unwrap() {
                    let path = entry.unwrap().path();
                    let key = path.strip_prefix(base).unwrap().into();
                    if path.is_dir() {
                        rows.insert(key, None);
                        visit(base, &path, rows);
                    } else {
                        rows.insert(key, Some(std::fs::read(path).unwrap()));
                    }
                }
            }
            let mut rows = Default::default();
            visit(root, root, &mut rows);
            rows
        }
        let before = inventory(root.path());
        let args = ApplyArgs {
            id: id.into(),
            work: "work-a".into(),
            path: root.path().into(),
            spec_root: Some("docs".into()),
        };
        let report = prepare(&args).unwrap();
        assert_eq!(
            report["operations"],
            serde_json::json!(["production", "current-source-verification"])
        );
        assert_eq!(report["selected_work"], "work-a");
        assert!(report["production_request"].is_null());
        assert_eq!(
            report["execution_carriage"],
            "unavailable:m16-change-full-pass"
        );
        assert_eq!(before, inventory(root.path()));
        assert!(!root.path().join(".vibe").exists());
        eprintln!("A2_CLI_PREPARATION {}", report);
    }
}
