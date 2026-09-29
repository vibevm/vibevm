//! Actual MemoryTransport calls on isolated, explicitly granted temp roots.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL");
use super::*;
use crate::{MemoryTransport, Server, dispatch_one};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_core::manifest::{CURRENT_ABSENT, LEGACY_ABSENT, SNAPSHOT_TO_COPY};
use vibe_wire::generated::migration::e1::{
    application_journal::ApplicationJournal, plan_report::Outcome as PlanOutcome,
};
use vibe_workspace::{
    migration_application::identity::{self, StageAction},
    migration_planning::{MigrationPlanningResponse, plan_migration_value},
};

const BEFORE: &str = "# café\r\n[package]\r\nname='demo'\r\ngroup='org.example'\r\nkind='flow'\r\nversion='1.1.0'\r\nmaterialization='snapshot' # keep\r\n";
struct Fixture {
    temp: tempfile::TempDir,
    project: PathBuf,
    store: PathBuf,
    cache: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        std::fs::create_dir(&project).unwrap();
        std::fs::write(project.join("vibe.toml"), BEFORE).unwrap();
        let store = temp.path().join("store");
        let cache = temp.path().join("cache");
        Self {
            temp,
            project,
            store,
            cache,
        }
    }
    fn context(&self, granted: bool) -> ServerContext {
        let context = ServerContext::with_store_root(&self.project, &self.cache);
        if granted {
            context
                .with_manifest_migration_authority(&self.store, ["vibe.toml"])
                .unwrap()
        } else {
            context
        }
    }
    fn authority(&self) -> Value {
        json!({"project_root":self.project,"recovery_store_root":self.store,"target_relative":"vibe.toml","scope":"owned_authored_file"})
    }
    fn apply(&self) -> Value {
        let approved = json!({"schema":1,"source_text":BEFORE,"source_profile":LEGACY_ABSENT,"target_profile":CURRENT_ABSENT,"recipe":SNAPSHOT_TO_COPY});
        let MigrationPlanningResponse::Report(report) = plan_migration_value(approved.clone())
        else {
            panic!("report")
        };
        let PlanOutcome::Planned(row) = report.outcome else {
            panic!("plan")
        };
        json!({"schema":1,"operation":"apply","operation_id":"mcp-operation","authority":self.authority(),"approved_request_json":approved.to_string(),"before_content_id":row.plan.before_identity,"after_content_id":row.plan.after_identity,"diff_id":row.plan.diff_identity})
    }
    fn inspect(&self) -> Value {
        json!({"schema":1,"operation":"inspect","operation_id":"mcp-operation","authority":self.authority()})
    }
    fn journal(&self) -> ApplicationJournal {
        let grant = OwnedGrant::new(&self.project, "vibe.toml", &self.store).unwrap();
        let key = identity::operation_storage_key(
            &grant.project().identity_token().unwrap(),
            "mcp-operation",
        );
        serde_json::from_slice(
            &std::fs::read(self.store.join("migrations").join(key).join("journal.json")).unwrap(),
        )
        .unwrap()
    }
    fn restore(&self) -> Value {
        json!({"schema":1,"operation":"recover","operation_id":"mcp-operation","authority":self.authority(),"expected_intent_id":self.journal().intent_id,"action":"restore"})
    }
    fn call(&self, args: Value, granted: bool) -> Value {
        let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"manifest_migration_operation","arguments":args}});
        let response: Value = serde_json::from_str(
            &dispatch_one(self.context(granted), &request.to_string()).unwrap(),
        )
        .unwrap();
        assert!(response["error"].is_null());
        assert_eq!(response["result"]["content"].as_array().unwrap().len(), 1);
        assert!(!self.cache.exists());
        response["result"].clone()
    }
}
use vibe_workspace::migration_application::OwnedManifestMigrationFile as OwnedGrant;
fn tree(root: &Path) -> BTreeMap<PathBuf, (String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, (String, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            let key = path.strip_prefix(root).unwrap().to_owned();
            if meta.file_type().is_symlink() {
                out.insert(
                    key,
                    (
                        "link".into(),
                        std::fs::read_link(path)
                            .unwrap()
                            .to_string_lossy()
                            .as_bytes()
                            .to_vec(),
                    ),
                );
            } else if meta.is_dir() {
                out.insert(key, ("directory".into(), vec![]));
                walk(root, &path, out);
            } else {
                out.insert(key, ("file".into(), std::fs::read(path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn normal(result: &Value) -> OperationReport {
    serde_json::from_value(result["structuredContent"].clone()).unwrap()
}
fn preflight_result(result: &Value, code: Code) {
    assert_eq!(result["isError"], true);
    let report = normal(result);
    assert_eq!(report.error.unwrap().code, code);
    assert!(report.effects.is_empty());
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("structured preflight")
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn descriptor_and_strict_preflight_before_grants() {
    let f = Fixture::new();
    let before = tree(f.temp.path());
    let descriptor = ManifestMigrationOperationMcpTool.descriptor();
    assert_eq!(descriptor.name, "manifest_migration_operation");
    let branches = descriptor.input_schema["oneOf"].as_array().unwrap();
    assert_eq!(branches.len(), 3);
    for branch in branches {
        assert_eq!(branch["additionalProperties"], false);
        assert_eq!(
            branch["properties"]["authority"]["additionalProperties"],
            false
        );
    }
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
    let listed: Value =
        serde_json::from_str(&dispatch_one(f.context(false), &request.to_string()).unwrap())
            .unwrap();
    assert_eq!(
        listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["name"] == descriptor.name)
            .count(),
        1
    );
    for value in [Value::Null, json!({}), json!({"schema":"1"})] {
        preflight_result(&f.call(value, false), Code::InvalidRequest);
    }
    let mut value = f.apply();
    value["extra"] = json!(true);
    preflight_result(&f.call(value, false), Code::InvalidRequest);
    let mut value = f.apply();
    value["schema"] = json!(2);
    preflight_result(&f.call(value, false), Code::UnsupportedSchema);
    for embedded in [
        "{",
        "null",
        r#"{"schema":2,"source_text":""}"#,
        r#"{"schema":1,"source_text":"","recipe":null}"#,
    ] {
        let mut value = f.apply();
        value["approved_request_json"] = json!(embedded);
        value["authority"]["project_root"] = json!(f.temp.path().join("nonexistent"));
        let result = f.call(value, false);
        preflight_result(&result, Code::InvalidRequest);
        assert_eq!(
            normal(&result).error.unwrap().field,
            "approved_request_json"
        );
    }
    assert_eq!(tree(f.temp.path()), before);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#MANIFEST-MIGRATION-CELL",
    r = 1
)]
fn composed_authority_refuses_payload_overrides_without_writes() {
    let f = Fixture::new();
    let before = tree(f.temp.path());
    preflight_result(&f.call(f.apply(), false), Code::AuthorityRefused);
    for (field, value) in [
        (
            "project_root",
            f.temp.path().join("foreign").to_string_lossy().into_owned(),
        ),
        (
            "recovery_store_root",
            f.temp
                .path()
                .join("foreign-store")
                .to_string_lossy()
                .into_owned(),
        ),
        ("target_relative", "member/vibe.toml".into()),
    ] {
        let mut request = f.apply();
        request["authority"][field] = json!(value);
        preflight_result(&f.call(request, true), Code::AuthorityRefused);
    }
    let absent_store = f.call(f.inspect(), true);
    assert_eq!(absent_store["isError"], true);
    assert_eq!(
        normal(&absent_store).error.unwrap().code,
        Code::JournalInvalid
    );
    assert!(
        !absent_store["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("structured preflight")
    );
    assert_eq!(tree(f.temp.path()), before);
    assert!(!f.store.exists());
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
fn actual_apply_inspect_restore_and_terminal_replay() {
    let f = Fixture::new();
    let apply = f.apply();
    let result = f.call(apply.clone(), true);
    assert_eq!(result["isError"], false);
    let report = normal(&result);
    assert_eq!(report.outcome, Outcome::Completed);
    assert_eq!(
        report.before_content_id.as_deref(),
        apply["before_content_id"].as_str()
    );
    assert_eq!(
        std::fs::read(f.project.join("vibe.toml")).unwrap(),
        BEFORE.replace("'snapshot'", "'copy'").as_bytes()
    );
    let stable = tree(f.temp.path());
    let inspection = f.call(f.inspect(), true);
    assert_eq!(inspection["isError"], false);
    assert_eq!(normal(&inspection).outcome, Outcome::Completed);
    assert_eq!(tree(f.temp.path()), stable);
    let repeat = f.call(apply, true);
    assert_eq!(normal(&repeat).outcome, Outcome::NoOp);
    assert_eq!(tree(f.temp.path()), stable);
    let restore = f.restore();
    let result = f.call(restore.clone(), true);
    assert_eq!(result["isError"], false);
    assert_eq!(normal(&result).outcome, Outcome::Completed);
    assert_eq!(
        std::fs::read(f.project.join("vibe.toml")).unwrap(),
        BEFORE.as_bytes()
    );
    let stable = tree(f.temp.path());
    assert_eq!(normal(&f.call(restore, true)).outcome, Outcome::NoOp);
    assert_eq!(tree(f.temp.path()), stable);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
fn conflicts_frozen_and_link_targets_preserve_source() {
    let f = Fixture::new();
    let mut value = f.apply();
    let mut approved: Value =
        serde_json::from_str(value["approved_request_json"].as_str().unwrap()).unwrap();
    approved["source_text"] = json!(format!("{BEFORE}frozen=true\r\n"));
    let MigrationPlanningResponse::Report(report) = plan_migration_value(approved.clone()) else {
        panic!()
    };
    let PlanOutcome::Planned(row) = report.outcome else {
        panic!()
    };
    value["approved_request_json"] = json!(approved.to_string());
    value["before_content_id"] = json!(row.plan.before_identity);
    value["after_content_id"] = json!(row.plan.after_identity);
    value["diff_id"] = json!(row.plan.diff_identity);
    let before = tree(f.temp.path());
    preflight_result(&f.call(value, true), Code::AuthorityRefused);
    assert_eq!(tree(f.temp.path()), before);
    f.call(f.apply(), true);
    let restore = f.restore();
    std::fs::write(f.project.join("vibe.toml"), b"external edit").unwrap();
    let result = f.call(restore, true);
    assert_eq!(result["isError"], true);
    assert_eq!(normal(&result).target_state,vibe_wire::generated::migration::e1::operation_report::OperationReportTargetState::ThirdState);
    assert_eq!(
        std::fs::read(f.project.join("vibe.toml")).unwrap(),
        b"external edit"
    );
    #[cfg(unix)]
    {
        let f = Fixture::new();
        let victim = f.temp.path().join("victim");
        std::fs::write(&victim, b"foreign").unwrap();
        std::fs::remove_file(f.project.join("vibe.toml")).unwrap();
        std::os::unix::fs::symlink(&victim, f.project.join("vibe.toml")).unwrap();
        let result = f.call(f.apply(), true);
        assert_eq!(result["isError"], true);
        assert_eq!(std::fs::read(victim).unwrap(), b"foreign");
        assert!(
            std::fs::symlink_metadata(f.project.join("vibe.toml"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
fn failed_restore_has_real_pending_journal_and_retained_residue() {
    let f = Fixture::new();
    f.call(f.apply(), true);
    let j = f.journal();
    let stage = identity::transaction_stage_name(&j.intent_id, StageAction::Restore);
    std::fs::write(f.project.join(&stage), b"foreign stage").unwrap();
    let after = std::fs::read(f.project.join("vibe.toml")).unwrap();
    let result = f.call(f.restore(), true);
    assert_eq!(result["isError"], true);
    let failed = normal(&result);
    assert!(!failed.effects.is_empty());
    assert!(!failed.residue.is_empty());
    assert_eq!(f.journal().phase,vibe_wire::generated::migration::e1::application_journal::ApplicationJournalPhase::Restoring);
    assert_eq!(std::fs::read(f.project.join("vibe.toml")).unwrap(), after);
    let stable = tree(f.temp.path());
    let pending = f.call(f.inspect(), true);
    assert_eq!(pending["isError"], false);
    assert_eq!(normal(&pending).outcome, Outcome::Pending);
    assert!(!normal(&pending).residue.is_empty());
    assert_eq!(tree(f.temp.path()), stable);
    assert_eq!(
        std::fs::read(f.project.join(&stage)).unwrap(),
        b"foreign stage"
    );
    // Explicit fixture-owner resolution, not runtime removal of unknown data.
    std::fs::remove_file(f.project.join(stage)).unwrap();
    let result = f.call(f.restore(), true);
    assert_eq!(normal(&result).outcome, Outcome::Completed);
    assert_eq!(
        std::fs::read(f.project.join("vibe.toml")).unwrap(),
        BEFORE.as_bytes()
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-SURFACES",
    r = 2
)]
fn invariant_failure_is_executed_diagnostic_without_structured_report() {
    struct Probe;
    impl McpTool for Probe {
        fn descriptor(&self) -> ToolDescriptor {
            ToolDescriptor {
                name: "retained_invariant_probe".into(),
                description: "test-only diagnostic".into(),
                input_schema: json!({"type":"object"}),
            }
        }
        fn run(&self, _: &Value, _: &ServerContext) -> Result<ToolOutput, ToolError> {
            let mut builder = ReportBuilder::new(Some(Operation::Apply), Some("probe".into()));
            builder.evidence_mut().effects.push(vibe_wire::generated::migration::e1::operation_report::Effect{kind:vibe_wire::generated::migration::e1::operation_report::EffectKind::Journal,state:vibe_wire::generated::migration::e1::operation_report::EffectState::Uncertain});
            render_result(builder.into_report())
        }
    }
    let f = Fixture::new();
    let before = tree(f.temp.path());
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"retained_invariant_probe","arguments":{}}});
    let transport = MemoryTransport::with_input(request.to_string() + "\n");
    let mut server = Server::new(transport, f.context(false));
    server.register_tool(Box::new(Probe));
    server.run().unwrap();
    let response: Value = serde_json::from_str(server.transport.take_output().trim()).unwrap();
    let result = &response["result"];
    assert_eq!(result["isError"], true);
    assert!(result.get("structuredContent").is_none());
    let text = result["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("INTERNAL MIGRATION REPORT VALIDATION FAILURE"));
    assert!(text.contains("UNVALIDATED RETAINED EVIDENCE"));
    assert!(text.contains("uncertain"));
    assert_eq!(tree(f.temp.path()), before);
}
