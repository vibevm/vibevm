//! The standalone-skill suite's shared WORLD — temp trees, the proven
//! skill artifact, the receipt a prior generation would have left, and
//! the request/verb drivers the law cells share.
//!
//! The same posture as the vibe-bin suite's world: the project, the
//! settings root, the deployment state home and the INJECTED user home
//! are separate temp trees the test owns, and the client executables are
//! three paths that name nothing — so the operator's real home, real
//! settings directory and real clients are unreachable from this suite
//! by construction, and a missing-client run is the DEFAULT shape rather
//! than a special case.
//!
//! The ENGINE-driven half of the suite shares this cell too: the world
//! that records real artifacts and drives the shipped executor, and the
//! after-write crash-point provider both engine lifecycle cells prove
//! §7.2's windows with — shared here so the first-deployment recovery
//! cell and the update recovery cell drive ONE machinery, not two
//! copies that could drift apart.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use vibe_core::manifest::{ArtifactKind, DeployTarget, MechanismRoutes};
use vibe_wire::behaviour::deploy_records::INTENT_EPOCH;
use vibe_wire::generated::artifact_record::ArtifactShape;
use vibe_wire::generated::deploy_intent::{
    DeployIntent, DeployTargetIdentity, PlannedResource as IntentResource,
};
use vibe_wire::generated::deploy_receipt::{
    DeployIdentity, DeployReceipt, DestinationScope, OwnedResource, ProviderIdentity, ReceiptStatus,
};

use super::SkillDeployProvider;
use super::client::SkillClient;
use crate::mechanism::deploy::model::{
    ClientExecutable, ClientExecutables, DeployExecution, DeploySelection, deploy_state_home,
};
use crate::mechanism::deploy::protocol::{
    ApplyReport, DeployDescriptor, DeployFingerprint, DeployPlan, ObservedResource, RemoveReport,
    ResolvedDeployArtifact,
};
use crate::mechanism::deploy::state::{CheckpointLedger, DeployState, DeploymentHome};
use crate::mechanism::error::DeployProviderError;
use crate::mechanism::package::support::{config, empty_world, key, registry, temp};
use crate::mechanism::record::{RecordFreshness, RecordInputs, build_record, write_record};
use crate::mechanism::{DeployProvider, DeployTargetRequest, MechanismError};

/// The canonical demo entry document.
pub(crate) const DEMO_ENTRY: &str =
    "---\nname: demo\ndescription: A demonstration skill.\n---\n\nBody.\n";

/// The updated generation's entry document.
pub(crate) const UPDATED_ENTRY: &str =
    "---\nname: demo\ndescription: A demonstration skill, updated.\n---\n\nNew body.\n";

/// The deploy-role section of one provider refusal.
pub(crate) fn refusal(error: &MechanismError) -> &DeployProviderError {
    match error {
        MechanismError::Deploy(deploy) => deploy,
        other => panic!("expected a deploy-role refusal, got: {other}"),
    }
}

/// Three client executables that are all MISSING — the suite's default,
/// because §6.3.0.5's skill destinations are documented filesystem
/// projections and never spawn a client.
pub(crate) fn missing_clients() -> ClientExecutables {
    ClientExecutables {
        claude: ClientExecutable::Missing {
            command: "claude".into(),
        },
        codex: ClientExecutable::Missing {
            command: "codex".into(),
        },
        opencode: ClientExecutable::Missing {
            command: "opencode".into(),
        },
    }
}

/// One isolated world: a project, a settings root, a deployment state
/// home and the injected user home — four temp trees, deliberately
/// distinct so a cell that confused any two is caught rather than
/// accidentally right.
pub(crate) struct World {
    pub(crate) project: TempDir,
    pub(crate) settings: TempDir,
    pub(crate) state: TempDir,
    /// The engine-owned staging scratch an apply would be offered.
    pub(crate) staging: TempDir,
    /// The injected user home — where the three clients' skill roots
    /// really land in a test.
    pub(crate) home: TempDir,
    pub(crate) clients: ClientExecutables,
}

impl World {
    pub(crate) fn new() -> Self {
        Self {
            project: temp(),
            settings: temp(),
            state: temp(),
            staging: temp(),
            home: temp(),
            clients: missing_clients(),
        }
    }

    /// One proven `skill`-kind file artifact, as the engine would have
    /// resolved it from a `package:static-skill` record.
    pub(crate) fn skill_artifact(&self, id: &str, body: &str) -> ResolvedDeployArtifact {
        let relative = format!("target/vibe-package/{id}/SKILL.md");
        let absolute = self.project.path().join(&relative);
        if let Some(parent) = absolute.parent() {
            std::fs::create_dir_all(parent).expect("the fixture directory creates");
        }
        std::fs::write(&absolute, body).expect("the fixture skill writes");
        ResolvedDeployArtifact {
            id: id.to_owned(),
            kind: ArtifactKind::Skill,
            shape: ArtifactShape::File,
            absolute,
            relative,
            digest: format!("{:x}", Sha256::digest(body.as_bytes())),
            bytes: body.len() as u64,
        }
    }

    /// The absolute path of one home-relative identity.
    pub(crate) fn at(&self, relative: &str) -> PathBuf {
        let mut path = self.home.path().to_path_buf();
        for part in relative.split('/') {
            path.push(part);
        }
        path
    }

    /// The absolute path of one owned resource identity.
    pub(crate) fn resource_at(&self, resource: &str) -> PathBuf {
        self.at(resource.strip_prefix("home:").unwrap_or(resource))
    }
}

/// One `[[deploy.target]]` row for one client's skill mechanism.
pub(crate) fn target(client: SkillClient, id: &str, artifact: &str, name: &str) -> DeployTarget {
    DeployTarget {
        id: id.to_owned(),
        artifact: artifact.to_owned(),
        mechanism: key(&format!("deploy:{}-skill", client.as_str())),
        provider: None,
        depends_on: None,
        config: Some(config(&format!("name = \"{name}\""))),
    }
}

/// A prior receipt owning exactly the named resources at their recorded
/// digests — what an earlier generation of the same deployment left.
pub(crate) fn receipt_owning(generation: u32, resources: &[(&str, &str)]) -> DeployReceipt {
    DeployReceipt {
        applied_at: "2026-08-30T00:00:00Z"
            .parse()
            .expect("the fixture stamp parses"),
        artifact_digest: "0".repeat(64),
        desired_config_digest: "0".repeat(64),
        generation,
        identity: DeployIdentity {
            project: "org.example/demo".to_owned(),
            package: None,
        },
        profile: "local".to_owned(),
        provider: ProviderIdentity {
            key: "org.vibevm/vibe#fixture".to_owned(),
            version: None,
            content_hash: None,
        },
        resources: resources
            .iter()
            .map(|(resource, digest)| OwnedResource {
                resource: (*resource).to_owned(),
                post_digest: (*digest).to_owned(),
            })
            .collect(),
        reversible: true,
        schema: 1,
        scope: DestinationScope::User,
        status: ReceiptStatus::Verified,
        target: "skill-target".to_owned(),
        evidence: Some("fixture prior receipt".to_owned()),
        finalized_at: None,
        prior_state_handle: None,
    }
}

/// The request one verb receives.
///
/// `prior_receipt` is the injected ownership a provider consults; the
/// engine hands the real one, and the provider-law suite hands exactly
/// the value a law needs to see. The staging scratch, when a verb takes
/// one, is a directory inside the suite's state tree. The recovery
/// intent is `None` here — the apply-time shape the engine builds.
pub(crate) fn request<'a>(
    world: &'a World,
    row: &'a DeployTarget,
    artifact: Option<&'a ResolvedDeployArtifact>,
    prior_receipt: Option<&'a DeployReceipt>,
    staged: bool,
) -> DeployTargetRequest<'a> {
    request_with_intent(world, row, artifact, prior_receipt, staged, None)
}

/// The planning request with §7.2's unretired durable intent injected —
/// the read-only-planner/pre-apply shape, so the interrupted-window laws
/// drive the exact evidence the engine hands a provider's `plan`.
pub(crate) fn request_with_intent<'a>(
    world: &'a World,
    row: &'a DeployTarget,
    artifact: Option<&'a ResolvedDeployArtifact>,
    prior_receipt: Option<&'a DeployReceipt>,
    staged: bool,
    recovery_intent: Option<&'a DeployIntent>,
) -> DeployTargetRequest<'a> {
    DeployTargetRequest {
        target: row,
        profile: "local",
        project_root: world.project.path(),
        settings_root: world.settings.path(),
        user_home: world.home.path(),
        clients: &world.clients,
        prior_receipt,
        recovery_intent,
        artifact,
        staging: staged.then(|| world.staging.path()),
    }
}

/// One validated durable intent desiring the named resources — the §7.2
/// journal an interrupted first-deployment apply would have left
/// unretired (no prior receipt, opened for generation 0).
pub(crate) fn intent_desiring(resources: &[(&str, &str)]) -> DeployIntent {
    intent_opened_over(0, None, resources)
}

/// One validated durable intent opened for `generation` over
/// `prior_generation` — the §7.2 journal an interrupted UPDATE apply
/// would have left unretired: the generation it was finalising, the
/// receipt generation it was superseding, and the resources it desired.
pub(crate) fn intent_opened_over(
    generation: u32,
    prior_generation: Option<u32>,
    resources: &[(&str, &str)],
) -> DeployIntent {
    DeployIntent {
        plan_hash: "1".repeat(64),
        resources: resources
            .iter()
            .map(|(resource, desired)| IntentResource {
                resource: (*resource).to_owned(),
                desired_digest: (*desired).to_owned(),
                prior_digest: None,
            })
            .collect(),
        schema: INTENT_EPOCH,
        started_at: "2026-08-30T00:00:00Z"
            .parse()
            .expect("the fixture stamp parses"),
        target: DeployTargetIdentity {
            generation,
            profile: "local".to_owned(),
            project: "org.example/demo".to_owned(),
            target: "skill-target".to_owned(),
            package: None,
        },
        prior_generation,
    }
}

/// Plan one target, with the exact inputs a law needs.
pub(crate) fn plan_of(
    provider: &SkillDeployProvider,
    world: &World,
    row: &DeployTarget,
    artifact: &ResolvedDeployArtifact,
    prior_receipt: Option<&DeployReceipt>,
) -> DeployPlan {
    provider
        .plan(&request(world, row, Some(artifact), prior_receipt, false))
        .expect("the fixture target plans")
}

/// Open the suite's checkpoint ledger over its own state home.
fn ledger_for(world: &World, row: &DeployTarget) -> (DeployState, DeploymentHome) {
    let state = DeployState::open(world.state.path()).expect("the state home opens");
    let home = DeploymentHome::new(world.state.path(), "org.example/demo", None, &row.id);
    (state, home)
}

/// Apply one target through a real checkpoint ledger, with the exact
/// injected ownership a law needs.
pub(crate) fn apply(
    provider: &SkillDeployProvider,
    world: &World,
    row: &DeployTarget,
    artifact: &ResolvedDeployArtifact,
    prior_receipt: Option<&DeployReceipt>,
) -> ApplyReport {
    apply_with_plan(
        provider,
        world,
        row,
        artifact,
        prior_receipt,
        &plan_of(provider, world, row, artifact, prior_receipt),
    )
    .expect("the fixture target applies")
}

/// Apply one already-made plan — the seam the post-plan recheck laws
/// drive: the plan and the destination state are set up by the test, and
/// the provider's own recheck runs inside the verb.
pub(crate) fn apply_with_plan(
    provider: &SkillDeployProvider,
    world: &World,
    row: &DeployTarget,
    artifact: &ResolvedDeployArtifact,
    prior_receipt: Option<&DeployReceipt>,
    plan: &DeployPlan,
) -> Result<ApplyReport, MechanismError> {
    let request = request(world, row, Some(artifact), prior_receipt, true);
    let (state, home) = ledger_for(world, row);
    let mut ledger = CheckpointLedger::open(&state, &home, "plan-hash").expect("the ledger opens");
    provider.apply(&request, plan, &mut ledger)
}

/// Recover one plan against what observation found, through a real
/// checkpoint ledger.
pub(crate) fn recover_with(
    provider: &SkillDeployProvider,
    world: &World,
    row: &DeployTarget,
    artifact: &ResolvedDeployArtifact,
    prior_receipt: Option<&DeployReceipt>,
    plan: &DeployPlan,
    observed: &[ObservedResource],
) -> Result<ApplyReport, MechanismError> {
    let request = request(world, row, Some(artifact), prior_receipt, true);
    let (state, home) = ledger_for(world, row);
    let mut ledger = CheckpointLedger::open(&state, &home, "plan-hash").expect("the ledger opens");
    provider.recover(&request, plan, observed, &mut ledger)
}

/// The owned resource identity of one client's `demo` skill.
pub(crate) fn resource_of(client: SkillClient) -> String {
    format!("home:{}/demo/SKILL.md", client.skills_relative())
}

/// Write one fixture file under the injected home, creating its parents.
pub(crate) fn write_home(world: &World, relative: &str, contents: &str) {
    let path = world.at(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the fixture directory creates");
    }
    std::fs::write(&path, contents).expect("the fixture file writes");
}

/// The SHA-256 of a file's exact bytes, in the 64-hex spelling the
/// records use.
pub(crate) fn digest_at(path: &Path) -> String {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("the file at {} reads: {error}", path.display()));
    format!("{:x}", Sha256::digest(&bytes))
}

/// One engine world: the project holding a recorded skill artifact, the
/// settings root carrying the deployment state home, the injected home
/// whose client roots are the destinations, and the collected mechanism
/// plane the executions share.
pub(crate) struct EngineWorld {
    pub(crate) project: TempDir,
    pub(crate) settings: TempDir,
    pub(crate) home: TempDir,
    pub(crate) state_home: PathBuf,
    pub(crate) clients: ClientExecutables,
    pub(crate) registry: vibe_extension_registry::MechanismRegistry,
    pub(crate) routes: MechanismRoutes,
}

impl EngineWorld {
    pub(crate) fn new() -> Self {
        let settings = temp();
        let state_home = deploy_state_home(settings.path());
        Self {
            project: temp(),
            settings,
            home: temp(),
            state_home,
            clients: missing_clients(),
            registry: registry(&empty_world()),
            routes: MechanismRoutes::default(),
        }
    }

    /// Write and RECORD one skill artifact, exactly as the package phase
    /// would have left it — the deploy role's one door for a consumed
    /// artifact. Answers the artifact's digest.
    pub(crate) fn record_skill(&self, id: &str, body: &str) -> String {
        let relative = format!("target/vibe-package/{id}/SKILL.md");
        let path = self.project.path().join(&relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the fixture directory creates");
        }
        std::fs::write(&path, body).expect("the fixture skill writes");
        let digest = match crate::mechanism::contain::digest_file(&path) {
            Ok((digest, _)) => digest,
            Err(fault) => panic!("the fixture skill digests: {}", fault.reason()),
        };
        let absolute = crate::mechanism::contain::forward_slashed(&path);
        let record = build_record(&RecordInputs {
            target: id,
            mechanism: &key("package:static-skill"),
            provider_key: "org.vibevm/vibe#static-skill",
            provider_version: None,
            provider_hash: None,
            output_id: id,
            kind: ArtifactKind::Skill,
            shape: ArtifactShape::File,
            digest: &digest,
            path_absolute: &absolute,
            path_relative: &relative,
            freshness: RecordFreshness::default(),
            platform: None,
            media_type: Some("text/markdown"),
            created_at: "2026-08-30T00:00:00Z",
            evidence: "fixture skill artifact".to_owned(),
        })
        .expect("the fixture record builds");
        write_record(self.project.path(), &record).expect("the fixture record writes");
        digest
    }

    /// The execution over this world, with MISSING clients.
    pub(crate) fn execution<'a>(
        &'a self,
        targets: &'a [DeployTarget],
        selection: &'a DeploySelection,
    ) -> DeployExecution<'a> {
        DeployExecution {
            project_root: self.project.path(),
            targets,
            selection,
            registry: &self.registry,
            routes: &self.routes,
            state_home: &self.state_home,
            settings_root: self.settings.path(),
            user_home: self.home.path(),
            clients: &self.clients,
            project: "org.example/demo",
            package: None,
            created_at: "2026-08-30T12:00:00Z",
        }
    }

    /// The absolute path of one home-relative member.
    pub(crate) fn at(&self, relative: &str) -> PathBuf {
        let mut path = self.home.path().to_path_buf();
        for part in relative.split('/') {
            path.push(part);
        }
        path
    }
}

/// One profile selection over the given target ids.
pub(crate) fn selection(ids: &[&str]) -> DeploySelection {
    DeploySelection {
        profile: "local".to_owned(),
        targets: ids.iter().map(|id| (*id).to_owned()).collect(),
    }
}

/// Write one fixture file under the injected home, creating its parents.
pub(crate) fn write_at(world: &EngineWorld, relative: &str, bytes: &[u8]) {
    let path = world.at(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the fixture directory creates");
    }
    std::fs::write(&path, bytes).expect("the fixture file writes");
}

/// The state capability and deployment home of one engine world's target,
/// for reading the engine's own durable records in a proof.
pub(crate) fn engine_state_of(
    world: &EngineWorld,
    row: &DeployTarget,
) -> (DeployState, DeploymentHome) {
    let state = DeployState::open(&world.state_home).expect("the state home opens");
    let home = DeploymentHome::new(&world.state_home, "org.example/demo", None, &row.id);
    (state, home)
}

/// A test provider that behaves exactly like the real skill provider of
/// the client it is built with, except that `apply` fails AFTER its real
/// work — the entry is genuinely published and checkpointed through the
/// real provider, and only then does the injected crash point fire. This
/// is §7.2's after-write window: desired bytes durable, intent unretired,
/// and whatever receipt existed before the run still the receipt after
/// it — the crash shape of a first deployment AND of an update.
pub(crate) struct FailingAfterWrite(pub(crate) SkillClient);

impl DeployProvider for FailingAfterWrite {
    fn descriptor(&self) -> DeployDescriptor {
        SkillDeployProvider::new(self.0).descriptor()
    }

    fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
        SkillDeployProvider::new(self.0).plan(request)
    }

    fn fingerprint(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
    ) -> Result<DeployFingerprint, MechanismError> {
        SkillDeployProvider::new(self.0).fingerprint(request, plan)
    }

    fn apply(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        SkillDeployProvider::new(self.0).apply(request, plan, checkpoint)?;
        Err(MechanismError::Deploy(DeployProviderError::Write {
            target: request.target.id.clone(),
            path: "sentinel-after-write".to_owned(),
            reason: "the injected crash point fired after the entry was published".to_owned(),
        }))
    }

    fn verify(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
    ) -> Result<Vec<ObservedResource>, MechanismError> {
        SkillDeployProvider::new(self.0).verify(request, resources)
    }

    fn remove(
        &self,
        request: &DeployTargetRequest<'_>,
        resources: &[String],
        prior_state_handle: Option<&str>,
    ) -> Result<RemoveReport, MechanismError> {
        SkillDeployProvider::new(self.0).remove(request, resources, prior_state_handle)
    }

    fn recover(
        &self,
        request: &DeployTargetRequest<'_>,
        plan: &DeployPlan,
        observed: &[ObservedResource],
        checkpoint: &mut CheckpointLedger<'_>,
    ) -> Result<ApplyReport, MechanismError> {
        SkillDeployProvider::new(self.0).recover(request, plan, observed, checkpoint)
    }
}
