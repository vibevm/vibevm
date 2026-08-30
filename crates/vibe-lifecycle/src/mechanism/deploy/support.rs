//! The deploy suite's prepared WORLD — the temp trees, the recorded
//! artifact, the execution a test hands the executor, and the borrowing
//! shim that lets a test keep the provider it passed in.
//!
//! Split from the hermetic provider next door because they answer two
//! different questions: [`fixture`](super::fixture) is *what a provider
//! does*, and this cell is *what it is given*. The atom that injected a
//! user home and three client executables is what made the difference
//! worth a seam — the world now has to name five isolated roots, and none
//! of them is a property of the provider.
//!
//! Every path this world names is under a `TempDir` the test owns. The
//! operator's real settings directory, real home and real clients are
//! unreachable from this suite by construction: nothing here resolves a
//! home, and no test names one.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use vibe_core::manifest::{ArtifactKind, DeployTarget, MechanismRoutes};
use vibe_extension_registry::{MechanismRegistry, SelectionStep};
use vibe_wire::generated::artifact_record::ArtifactShape;

use super::model::ClientExecutables;
use super::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource, RemoveReport,
};
use super::state::CheckpointLedger;
use super::{DeployExecution, DeploySelection, Selected};
use crate::mechanism::package::support::{key, pin, registry as collect, temp, write};
use crate::mechanism::record::{RecordFreshness, RecordInputs, build_record, write_record};
use crate::mechanism::{DeployProvider, DeployTargetRequest, MechanismError};

// The provider cell's own names, re-exported so every existing
// `use super::support::{…}` site keeps one spelling: the split is a
// responsibility seam, not a rename.
pub(crate) use super::fixture::{FIXTURE_PIN, Faults, FixtureProvider};

/// One `[[deploy.target]]` row over an artifact id.
pub(crate) fn target(id: &str, artifact: &str, depends_on: &[&str]) -> DeployTarget {
    DeployTarget {
        id: id.to_owned(),
        artifact: artifact.to_owned(),
        mechanism: key("deploy:vibe-bin"),
        provider: Some(pin(FIXTURE_PIN)),
        depends_on: Some(depends_on.iter().map(|name| (*name).to_owned()).collect()),
        config: None,
    }
}

/// A prepared project with one produced artifact and its A2 record.
pub(crate) struct Fixture {
    pub(crate) project: TempDir,
    pub(crate) settings: TempDir,
    pub(crate) destination: TempDir,
    /// The injected user home — a FOURTH temp tree, deliberately not
    /// `settings`, so the two cannot be confused into agreeing.
    pub(crate) home: TempDir,
    /// The injected client executables — three paths that name nothing.
    pub(crate) clients: ClientExecutables,
    pub(crate) registry: MechanismRegistry,
    pub(crate) routes: MechanismRoutes,
}

impl Fixture {
    /// One project holding a produced `helper.exe` artifact, an empty
    /// deployment state home and an empty destination — three separate
    /// temp trees, so nothing in this suite can reach a real home.
    pub(crate) fn new(body: &str) -> Self {
        let project = temp();
        write(project.path(), "target/debug/helper.exe", body);
        let mut hash = Sha256::new();
        hash.update(body.as_bytes());
        let digest = format!("{:x}", hash.finalize());
        let absolute = crate::mechanism::contain::forward_slashed(
            &project.path().join("target/debug/helper.exe"),
        );
        let record = build_record(&RecordInputs {
            target: "helper",
            mechanism: &key("build:cargo"),
            provider_key: "org.vibevm/vibe#cargo",
            provider_version: None,
            provider_hash: None,
            output_id: "helper.exe",
            kind: ArtifactKind::Executable,
            shape: ArtifactShape::File,
            digest: &digest,
            path_absolute: &absolute,
            path_relative: "target/debug/helper.exe",
            freshness: RecordFreshness::default(),
            platform: None,
            media_type: None,
            created_at: "2026-08-30T00:00:00Z",
            evidence: "fixture artifact".to_owned(),
        })
        .expect("the fixture record builds");
        write_record(project.path(), &record).expect("the fixture record writes");
        let world = crate::mechanism::package::support::empty_world();
        let home = temp();
        let clients = crate::mechanism::vibebin::support::fake_clients(home.path());
        Self {
            registry: collect(&world),
            routes: MechanismRoutes::default(),
            project,
            settings: temp(),
            destination: temp(),
            home,
            clients,
        }
    }

    /// Rebuild the fixture's `helper.exe` with new bytes and re-record it —
    /// the manifest shape of "the artifact this target deploys was rebuilt",
    /// which is what makes a second `execute_deploy_targets` a new
    /// GENERATION of the same deployment rather than a no-op.
    pub(crate) fn rebuild(&self, body: &str) {
        write(self.project.path(), "target/debug/helper.exe", body);
        let mut hash = Sha256::new();
        hash.update(body.as_bytes());
        let digest = format!("{:x}", hash.finalize());
        let absolute = crate::mechanism::contain::forward_slashed(
            &self.project.path().join("target/debug/helper.exe"),
        );
        let record = build_record(&RecordInputs {
            target: "helper",
            mechanism: &key("build:cargo"),
            provider_key: "org.vibevm/vibe#cargo",
            provider_version: None,
            provider_hash: None,
            output_id: "helper.exe",
            kind: ArtifactKind::Executable,
            shape: ArtifactShape::File,
            digest: &digest,
            path_absolute: &absolute,
            path_relative: "target/debug/helper.exe",
            freshness: RecordFreshness::default(),
            platform: None,
            media_type: None,
            created_at: "2026-08-30T00:00:00Z",
            evidence: "fixture artifact, rebuilt".to_owned(),
        })
        .expect("the rebuilt record builds");
        write_record(self.project.path(), &record).expect("the rebuilt record writes");
    }

    /// The deployment state home of this fixture — a temp root, named as
    /// data exactly as the command layer would name the settings dir.
    pub(crate) fn state_home(&self) -> PathBuf {
        super::deploy_state_home(self.settings.path())
    }

    /// An execution over this fixture's project and state home.
    pub(crate) fn execution<'a>(
        &'a self,
        targets: &'a [DeployTarget],
        selection: &'a DeploySelection,
        state_home: &'a Path,
    ) -> DeployExecution<'a> {
        DeployExecution {
            project_root: self.project.path(),
            targets,
            selection,
            registry: &self.registry,
            routes: &self.routes,
            state_home,
            settings_root: self.settings.path(),
            user_home: self.home.path(),
            clients: &self.clients,
            project: "org.example/demo",
            package: None,
            created_at: "2026-08-30T12:00:00Z",
        }
    }
}

/// One already-resolved target, as the executor's own selection step
/// would produce it — the seam the saga's laws are driven through.
pub(crate) fn selected<'a>(
    target: &'a DeployTarget,
    provider: Box<dyn DeployProvider>,
) -> Selected<'a> {
    Selected {
        target,
        provider,
        pin: FIXTURE_PIN.to_owned(),
        via: SelectionStep::TargetPin,
        displaced: None,
    }
}

/// One profile selection over the given target ids.
pub(crate) fn selection(profile: &str, targets: &[&str]) -> DeploySelection {
    DeploySelection {
        profile: profile.to_owned(),
        targets: targets.iter().map(|id| (*id).to_owned()).collect(),
    }
}

/// A borrowing shim so a test can keep the fixture and still hand the
/// executor an owned provider.
pub(crate) struct Witness(pub(crate) std::rc::Rc<FixtureProvider>);

impl DeployProvider for Witness {
    fn descriptor(&self) -> DeployDescriptor {
        self.0.descriptor()
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        self.0.plan(request)
    }

    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        self.0.fingerprint(request, plan)
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.0.apply(request, plan, checkpoint)
    }

    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        self.0.verify(request, resources)
    }

    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        self.0.remove(request, resources, prior_state_handle)
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        self.0.recover(request, plan, observed, checkpoint)
    }
}
