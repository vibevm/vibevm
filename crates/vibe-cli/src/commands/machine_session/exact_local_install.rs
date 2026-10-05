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
        exact_local_install_grant::ExactLocalInstallGrantService,
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
    let target_os =
        TargetOs::current().context("fixed Install requires a supported target platform")?;
    let project = Arc::new(Project::open(root)?);
    let scope = SubmissionScope {
        context: "vibe-cli-exact-local-install/e1".into(),
        target: format!("physical-root:{}", project.root_correlation_token()?),
        principal: "vibe-cli-project-local/e1".into(),
    };
    let service = ExactLocalInstallGrantService::from_trusted_host(
        project,
        scope,
        target_os,
        TrustedPolicySource::from_host_identifier("vibe-cli/exact-local-install/e1".into())?,
        HostPolicyFacts {
            approve_captured_script: false,
            approve_exact_local_install,
            allow_detach: false,
            allow_cancel_pre_dispatch: false,
            allow_cancel_deferred_boundary: false,
        },
        source,
    )?;
    let grant = match mode {
        ExactLocalInstallHostMode::Issue(ttl_ms) => service.issue(ttl_ms)?,
        ExactLocalInstallHostMode::Reattach => service.reattach()?,
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
}
