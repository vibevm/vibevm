//! The vibe-bin suite's shared world — four temp trees, one request
//! builder, and the two verb drivers every law below uses.
//!
//! Its own cell for the same reason the deploy engine's fixture home is
//! one: two test cells prove this provider's laws (the pure half next
//! door, the reconciling half beside it) and a second copy of the world
//! would be a second thing to drift. Nothing here resolves a home — the
//! settings root, the project, the staging directory and the deployment
//! state home are four temp trees the test owns, so the operator's real
//! `~/.vibe` is unreachable by construction.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use vibe_core::manifest::{ArtifactKind, DeployTarget};
use vibe_wire::generated::artifact_record::ArtifactShape;

use super::VibeBinProvider;
use super::launcher::LauncherFlavour;
use super::store;
use crate::mechanism::deploy::protocol::{ApplyReport, DeployPlan, ResolvedDeployArtifact};
use crate::mechanism::deploy::state::{CheckpointLedger, DeployState, DeploymentHome};
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::package::support::{config, key, temp};
use crate::mechanism::{DeployProvider, DeployTargetRequest, MechanismError};

/// The deploy-role section of one provider refusal.
///
/// Every refusal this provider raises is a [`DeployProviderError`] carried
/// transparently by the shared layer enum, so the suite unwraps it once
/// here rather than matching two levels at every assertion.
pub(crate) fn refusal(error: &MechanismError) -> &DeployProviderError {
    match error {
        MechanismError::Deploy(deploy) => deploy,
        other => panic!("expected a deploy-role refusal, got: {other}"),
    }
}

/// The launcher's own file name on this host.
pub(crate) fn launcher_name(command: &str) -> String {
    format!("bin/{command}{}", LauncherFlavour::NATIVE.launcher_suffix())
}

/// One isolated world: a settings root, a project, an engine-owned staging
/// directory and a deployment state home — four temp trees.
pub(crate) struct World {
    pub(crate) settings: TempDir,
    pub(crate) project: TempDir,
    pub(crate) staging: TempDir,
    pub(crate) state: TempDir,
}

impl World {
    pub(crate) fn new() -> Self {
        Self {
            settings: temp(),
            project: temp(),
            staging: temp(),
            state: temp(),
        }
    }

    /// One proven artifact, as the engine would have resolved it.
    pub(crate) fn artifact(
        &self,
        id: &str,
        body: &str,
        kind: ArtifactKind,
    ) -> ResolvedDeployArtifact {
        let relative = format!("target/debug/{id}");
        let absolute = self.project.path().join(&relative);
        if let Some(parent) = absolute.parent() {
            std::fs::create_dir_all(parent).expect("the fixture directory creates");
        }
        std::fs::write(&absolute, body).expect("the fixture artifact writes");
        ResolvedDeployArtifact {
            id: id.to_owned(),
            kind,
            shape: ArtifactShape::File,
            absolute,
            relative,
            digest: format!("{:x}", Sha256::digest(body.as_bytes())),
            bytes: body.len() as u64,
        }
    }

    /// The absolute path of one settings-relative resource identity.
    pub(crate) fn at(&self, relative: &str) -> PathBuf {
        store::join(self.settings.path(), relative)
    }
}

/// One `[[deploy.target]]` row over an artifact id and a config table.
pub(crate) fn target(id: &str, artifact: &str, config_text: Option<&str>) -> DeployTarget {
    DeployTarget {
        id: id.to_owned(),
        artifact: artifact.to_owned(),
        mechanism: key("deploy:vibe-bin"),
        provider: None,
        depends_on: None,
        config: config_text.map(config),
    }
}

/// The request one verb receives, with staging offered as an apply would.
pub(crate) fn request<'a>(
    world: &'a World,
    row: &'a DeployTarget,
    artifact: Option<&'a ResolvedDeployArtifact>,
    staged: bool,
) -> DeployTargetRequest<'a> {
    DeployTargetRequest {
        target: row,
        profile: "local",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        artifact,
        staging: staged.then(|| world.staging.path()),
    }
}

/// Apply one target and hand back what the provider reported.
pub(crate) fn apply(
    world: &World,
    row: &DeployTarget,
    artifact: &ResolvedDeployArtifact,
) -> ApplyReport {
    let request = request(world, row, Some(artifact), true);
    let plan = VibeBinProvider
        .plan(&request)
        .expect("the fixture target plans");
    let state = DeployState::open(world.state.path()).expect("the state home opens");
    let home = DeploymentHome::new(world.state.path(), "org.example/demo", None, &row.id);
    let mut ledger = CheckpointLedger::open(&state, &home, "plan-hash").expect("the ledger opens");
    VibeBinProvider
        .apply(&request, &plan, &mut ledger)
        .expect("the fixture target applies")
}

/// The plan one target would produce.
pub(crate) fn plan_of(
    world: &World,
    row: &DeployTarget,
    artifact: &ResolvedDeployArtifact,
) -> DeployPlan {
    VibeBinProvider
        .plan(&request(world, row, Some(artifact), false))
        .expect("the fixture target plans")
}

/// How many entries one directory holds; zero when it is not there.
pub(crate) fn count(directory: &Path) -> usize {
    std::fs::read_dir(directory).map_or(0, |entries| entries.flatten().count())
}
