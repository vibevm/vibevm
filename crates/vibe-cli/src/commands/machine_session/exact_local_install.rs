//! Host-controlled exact local Install composition for CLI and MCP reaches.

use anyhow::{Context, Result, bail};
use std::{path::Path, sync::Arc};
use vibe_core::{PackageRef, manifest::TargetOs};
use vibe_lifecycle::{SubmissionScope, process::capture_package_tree_witness};
use vibe_mcp::ServerContext;
use vibe_orchestrator::{
    machine_operation::exact_local_install::ExactLocalInstallRoute,
    machine_submission::{
        HostPolicyFacts, TrustedPolicySource,
        exact_local_install_grant::{ExactLocalInstallGrantError, ExactLocalInstallGrantService},
    },
    ports::BoundOnePackageSource,
};
use vibe_package_source::PinnedLocalFactory;
use vibe_safefs::Project;

pub(crate) enum ExactLocalInstallHostMode {
    Issue(u64),
    Reattach,
}

/// The same host composition is used by explicit framed and MCP startup.
/// Only CLI flags choose the source; no frame, tool request, CWD or HOME does.
pub(crate) fn context(
    context: ServerContext,
    root: &Path,
    registry_root: &Path,
    package_pin: &str,
    source_files: &[String],
    approve_exact_local_install: bool,
    mode: ExactLocalInstallHostMode,
) -> Result<ServerContext> {
    if !registry_root.is_absolute() {
        bail!("fixed Install registry root must be absolute");
    }
    if !(1..=8).contains(&source_files.len())
        || !source_files.iter().any(|file| file == "vibe.toml")
    {
        bail!("fixed Install requires a bounded --source-file set containing vibe.toml");
    }
    let pin = PackageRef::parse(package_pin).context("fixed Install package pin")?;
    if !pin.is_qualified() || !pin.version.is_exact_pin() || pin.kind.is_none() {
        bail!("fixed Install requires an exact kind/group/name/version package pin");
    }
    let group = pin.group.as_ref().context("fixed Install package group")?;
    let version = pin.version.to_string();
    let version = version
        .strip_prefix('=')
        .context("fixed Install exact version")?;
    let target_os =
        TargetOs::current().context("fixed Install requires a supported target platform")?;
    let project = Arc::new(Project::open(root)?);
    let scope = SubmissionScope {
        context: "vibe-cli-exact-local-install/e1".into(),
        target: format!("physical-root:{}", project.root_correlation_token()?),
        principal: "vibe-cli-project-local/e1".into(),
    };
    let policy_source =
        TrustedPolicySource::from_host_identifier("vibe-cli/exact-local-install/e1".into())?;
    let facts = HostPolicyFacts {
        approve_captured_script: false,
        approve_exact_local_install,
        allow_detach: false,
        allow_cancel_pre_dispatch: false,
        allow_cancel_deferred_boundary: false,
    };
    let grant = match mode {
        ExactLocalInstallHostMode::Issue(ttl_ms) => {
            let source_root = registry_root
                .join(group.as_str())
                .join(pin.name.as_str())
                .join(format!("v{version}"));
            let files: Vec<&str> = source_files.iter().map(String::as_str).collect();
            let witness = capture_package_tree_witness(&source_root, &pin, &files)
                .context("capturing explicit exact local Install source")?;
            let source: Arc<dyn BoundOnePackageSource> = Arc::new(
                PinnedLocalFactory::new(registry_root, &pin, &witness)
                    .context("binding explicit local Install source")?,
            );
            ExactLocalInstallGrantService::from_trusted_host(
                project,
                scope,
                target_os,
                policy_source,
                facts,
                source,
            )?
            .issue(ttl_ms)?
        }
        ExactLocalInstallHostMode::Reattach => {
            let cold = ExactLocalInstallGrantService::from_trusted_recovery_host(
                project.clone(),
                scope.clone(),
                target_os,
                policy_source.clone(),
                facts.clone(),
                registry_root,
                &pin,
                source_files,
            )?;
            match cold.reattach() {
                Ok(grant) => grant,
                Err(ExactLocalInstallGrantError::InvalidPointer(reason))
                    if reason == "legacy pointer has no cold source selection" =>
                {
                    let source_root = registry_root
                        .join(group.as_str())
                        .join(pin.name.as_str())
                        .join(format!("v{version}"));
                    let files: Vec<&str> = source_files.iter().map(String::as_str).collect();
                    let witness = capture_package_tree_witness(&source_root, &pin, &files)
                        .context("legacy Install recovery requires original source bytes")?;
                    let source: Arc<dyn BoundOnePackageSource> = Arc::new(
                        PinnedLocalFactory::new(registry_root, &pin, &witness)
                            .context("legacy Install recovery source binding")?,
                    );
                    ExactLocalInstallGrantService::from_trusted_host(
                        project,
                        scope,
                        target_os,
                        policy_source,
                        facts,
                        source,
                    )?
                    .reattach_readonly_legacy()?
                }
                Err(error) => return Err(error.into()),
            }
        }
    };
    let (host, namespace) = grant.into_parts();
    Ok(context
        .with_exact_local_install_route(Arc::new(ExactLocalInstallRoute::new(host, namespace)))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("consumer");
        let registry = temp.path().join("registry");
        let package = registry.join("org.example/fixture/v1.1.0");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(project.join("vibe.toml"),
            "[project]\nname = 'consumer'\nversion = '1.1.0'\n[requires.packages]\n'flow:org.example/fixture' = '=1.1.0'\n").unwrap();
        std::fs::write(package.join("vibe.toml"),
            "[package]\ngroup = 'org.example'\nname = 'fixture'\nkind = 'flow'\nversion = '1.1.0'\n").unwrap();
        std::fs::write(package.join("README.md"), b"# Fixture\n").unwrap();
        (temp, project, registry)
    }

    #[test]
    fn explicit_install_host_issues_and_reattaches_only_same_source_and_policy() {
        let (_temp, root, registry) = fixture();
        let files = vec!["vibe.toml".into(), "README.md".into()];
        let compose = |mode, approve| {
            context(
                ServerContext::with_store_root(&root, root.join(".vibe/cache")),
                &root,
                &registry,
                "flow:org.example/fixture@=1.1.0",
                &files,
                approve,
                mode,
            )
        };
        let issued = compose(ExactLocalInstallHostMode::Issue(60_000), true).unwrap();
        let mut issued = vibe_mcp::machine_session::exact_local_install::ExactLocalInstallMachineSessionRunner::new(issued).unwrap();
        assert_eq!(
            issued
                .run(&mut std::io::Cursor::new(Vec::<u8>::new()), &mut Vec::new())
                .unwrap(),
            0
        );
        let attached = compose(ExactLocalInstallHostMode::Reattach, true).unwrap();
        let mut attached = vibe_mcp::machine_session::exact_local_install::ExactLocalInstallMachineSessionRunner::new(attached).unwrap();
        assert_eq!(
            attached
                .run(&mut std::io::Cursor::new(Vec::<u8>::new()), &mut Vec::new())
                .unwrap(),
            0
        );
        assert!(compose(ExactLocalInstallHostMode::Reattach, false).is_err());
        assert!(compose(ExactLocalInstallHostMode::Issue(60_000), true).is_err());
        let foreign = context(
            ServerContext::with_store_root(&root, root.join(".vibe/cache")),
            &root,
            &registry,
            "flow:org.example/foreign@=1.1.0",
            &files,
            true,
            ExactLocalInstallHostMode::Reattach,
        );
        assert!(foreign.is_err());
        assert!(!root.join("vibe.lock").exists());
    }

    #[test]
    fn source_selection_refuses_unknown_file_before_host_issue() {
        let (_temp, root, registry) = fixture();
        let error = context(
            ServerContext::with_store_root(&root, root.join(".vibe/cache")),
            &root,
            &registry,
            "flow:org.example/fixture@=1.1.0",
            &["vibe.toml".into()],
            true,
            ExactLocalInstallHostMode::Issue(60_000),
        )
        .err()
        .unwrap()
        .to_string();
        assert!(
            error.contains("capturing explicit exact local Install source"),
            "{error}"
        );
        assert!(!root.join(".vibe/machine-submissions.json").exists());
    }

    #[test]
    fn reattach_after_source_drift_skips_effect_capable_factory_and_returns_cold_runner() {
        let (_temp, root, registry) = fixture();
        let files = vec!["vibe.toml".into(), "README.md".into()];
        let build = |mode| {
            context(
                ServerContext::with_store_root(&root, root.join("unused")),
                &root,
                &registry,
                "flow:org.example/fixture@=1.1.0",
                &files,
                true,
                mode,
            )
        };
        build(ExactLocalInstallHostMode::Issue(60_000)).unwrap();
        let before = std::fs::read(root.join(".vibe/machine-submissions.json")).unwrap();
        std::fs::write(
            registry.join("org.example/fixture/v1.1.0/README.md"),
            b"changed after issue\n",
        )
        .unwrap();
        let cold = build(ExactLocalInstallHostMode::Reattach).unwrap();
        let mut runner = vibe_mcp::machine_session::exact_local_install::ExactLocalInstallMachineSessionRunner::new(cold).unwrap();
        assert_eq!(
            runner
                .run(&mut std::io::Cursor::new(Vec::<u8>::new()), &mut Vec::new())
                .unwrap(),
            0
        );
        assert_eq!(
            std::fs::read(root.join(".vibe/machine-submissions.json")).unwrap(),
            before
        );
        assert!(!root.join("vibe.lock").exists());
    }

    #[test]
    fn completed_install_is_cold_inspectable_after_source_drift_without_second_effect() {
        assert!(vibe_test_support::isolated_home().is_some());
        use vibe_orchestrator::machine_operation::input_decision as decision;
        use vibe_wire::generated::machine::e1::lifecycle_action_reply::{
            LifecycleActionReply, Reply,
        };
        let (_temp, root, registry) = fixture();
        let pin = PackageRef::parse("flow:org.example/fixture@=1.1.0").unwrap();
        let source_root = registry.join("org.example/fixture/v1.1.0");
        let witness =
            capture_package_tree_witness(&source_root, &pin, &["README.md", "vibe.toml"]).unwrap();
        let source: Arc<dyn BoundOnePackageSource> =
            Arc::new(PinnedLocalFactory::new(&registry, &pin, &witness).unwrap());
        let project = Arc::new(Project::open(&root).unwrap());
        let scope = SubmissionScope {
            context: "vibe-cli-exact-local-install/e1".into(),
            target: format!(
                "physical-root:{}",
                project.root_correlation_token().unwrap()
            ),
            principal: "vibe-cli-project-local/e1".into(),
        };
        let service = ExactLocalInstallGrantService::from_trusted_host(
            project,
            scope.clone(),
            TargetOs::current().unwrap(),
            TrustedPolicySource::from_host_identifier("vibe-cli/exact-local-install/e1".into())
                .unwrap(),
            HostPolicyFacts {
                approve_captured_script: false,
                approve_exact_local_install: true,
                allow_detach: false,
                allow_cancel_pre_dispatch: false,
                allow_cancel_deferred_boundary: false,
            },
            source,
        )
        .unwrap();
        let grant = service.issue(60_000).unwrap();
        let (host, namespace) = grant.into_parts();
        let route = ExactLocalInstallRoute::new(host, namespace.clone());
        let action = |value: serde_json::Value| {
            let bytes =
                serde_json::to_vec(&serde_json::json!({"schema":1,"action":value})).unwrap();
            ExactLocalInstallRoute::dispatch(Some(&route), &bytes, &root).unwrap()
        };
        let prepared = action(serde_json::json!({"kind":"prepare","phase":"install",
            "requested_options":{"force":false,"trace":false,"deploy_profile":null,"control":"none"}}));
        let Reply::Question(question) = prepared.reply else {
            panic!("question");
        };
        let mut question: decision::InputDecisionMessage =
            serde_json::from_str(&question.payload_json).unwrap();
        let reference = question.question_id.clone().unwrap();
        question.body = decision::Body::ApprovalAnswer(Box::new(decision::BodyApprovalAnswer {
            approved: true,
        }));
        let answered = action(
            serde_json::json!({"kind":"answer","message_json":serde_json::to_string(&question).unwrap()}),
        );
        assert!(matches!(answered.reply, Reply::Validated(_)));
        let submitted =
            action(serde_json::json!({"kind":"submit","accepted_ref":reference,"key":"completed"}));
        assert!(matches!(submitted.reply, Reply::Submitted(_)));
        let lease = vibe_lifecycle::LifecycleLease::acquire(&root).unwrap();
        let reserved = vibe_lifecycle::SubmissionReservations::inspect_existing_key(
            &lease,
            &namespace,
            &scope,
            "completed",
        )
        .unwrap()
        .unwrap();
        drop(lease);
        let before = std::fs::read(root.join(".vibe/machine-submissions.json")).unwrap();
        let lock = std::fs::read(root.join("vibe.lock")).unwrap();
        drop(route);
        drop(service);
        std::fs::write(
            source_root.join("README.md"),
            b"changed after completed Install\n",
        )
        .unwrap();
        let files = vec!["vibe.toml".into(), "README.md".into()];
        let cold = context(
            ServerContext::with_store_root(&root, root.join("unused")),
            &root,
            &registry,
            "flow:org.example/fixture@=1.1.0",
            &files,
            true,
            ExactLocalInstallHostMode::Reattach,
        )
        .unwrap();
        let request = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"lifecycle_operation","arguments":{"schema":1,
                "operation_id":vibe_orchestrator::machine_operation::exact_local_install::OPERATION_ID,
                "action":{"kind":"inspect","key":"completed"}}}});
        let response: serde_json::Value =
            serde_json::from_str(&vibe_mcp::dispatch_one(cold, &request.to_string()).unwrap())
                .unwrap();
        assert_eq!(response["result"]["isError"], false, "{response}");
        let reply: LifecycleActionReply =
            serde_json::from_value(response["result"]["structuredContent"].clone()).unwrap();
        let Reply::View(view) = reply.reply else {
            panic!("cold view");
        };
        let view: serde_json::Value = serde_json::from_str(&view.payload_json).unwrap();
        assert_eq!(view["run_id"], reserved.domain_run_id);
        assert_eq!(view["status"]["state"], "result_available");
        let cold_again = context(
            ServerContext::with_store_root(&root, root.join("unused")),
            &root,
            &registry,
            "flow:org.example/fixture@=1.1.0",
            &files,
            true,
            ExactLocalInstallHostMode::Reattach,
        )
        .unwrap();
        let refused_request = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"lifecycle_operation","arguments":{"schema":1,
                "operation_id":vibe_orchestrator::machine_operation::exact_local_install::OPERATION_ID,
                "action":{"kind":"submit","accepted_ref":"forged","key":"completed"}}}});
        let refused: serde_json::Value = serde_json::from_str(
            &vibe_mcp::dispatch_one(cold_again, &refused_request.to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(refused["result"]["isError"], true);
        assert_eq!(
            refused["result"]["structuredContent"]["code"],
            "decision_refused"
        );
        assert_eq!(
            refused["result"]["structuredContent"]["known_run_id"],
            reserved.domain_run_id
        );
        assert!(refused["result"]["structuredContent"]["view_json"].is_string());
        assert_eq!(
            std::fs::read(root.join(".vibe/machine-submissions.json")).unwrap(),
            before
        );
        assert_eq!(std::fs::read(root.join("vibe.lock")).unwrap(), lock);
    }
}
