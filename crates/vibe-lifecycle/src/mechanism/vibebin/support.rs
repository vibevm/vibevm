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
use crate::mechanism::deploy::model::{ClientExecutable, ClientExecutables};
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
/// directory, a deployment state home and the invoking user's home — five
/// temp trees, plus three client executables that do not exist.
pub(crate) struct World {
    pub(crate) settings: TempDir,
    pub(crate) project: TempDir,
    pub(crate) staging: TempDir,
    pub(crate) state: TempDir,
    /// The injected user home — deliberately a DIFFERENT tree from
    /// `settings`, so a cell that confused the two is caught rather than
    /// accidentally right.
    pub(crate) home: TempDir,
    /// The injected client executables. Fake paths inside the fake home:
    /// nothing in this suite may spawn a client, and naming a real one
    /// would make that unprovable.
    pub(crate) clients: ClientExecutables,
}

impl World {
    pub(crate) fn new() -> Self {
        let home = temp();
        let clients = fake_clients(home.path());
        Self {
            settings: temp(),
            project: temp(),
            staging: temp(),
            state: temp(),
            home,
            clients,
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

/// Three client executables that are absolute paths and not programs.
///
/// Shared by every suite that has to build a request: the injected value is
/// mandatory, and a fixture that had to invent one per test would be three
/// chances to name a real client by accident. Each is spelled as the SURFACE
/// would spell a resolution — an absolute path under the fake home — so a
/// cell that leaked a bare command word downward is visible against them.
pub(crate) fn fake_clients(home: &Path) -> ClientExecutables {
    let resolved = |command: &str| ClientExecutable::Resolved {
        command: command.to_owned(),
        path: home.join("fake-clients").join(command),
    };
    ClientExecutables {
        claude: resolved("claude"),
        codex: resolved("codex"),
        opencode: resolved("opencode"),
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
///
/// The injected client authority is a FIFTH temp tree and three paths that
/// name nothing: this provider reads neither, and a fixture that handed it
/// the operator's real home or a real client binary would make that fact
/// unprovable.
///
/// `prior_receipt` is `None` for the same reason it is a request member at
/// all: §6.3.1.1 makes prior ownership the ENGINE's read, and this suite
/// drives the provider's own verbs directly, with no engine and therefore no
/// state home to have read one from. `deploy:vibe-bin` owns whole files that
/// no other deployment may hold, so it consults no prior receipt — the
/// engine's `refuse_foreign_ownership` and the launcher collision law
/// already answer the question the member exists for.
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
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt: None,
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
