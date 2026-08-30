//! § "Apply uses a per-destination lock and staging where the destination
//! supports atomic replacement" — the sentence's own cell.
//!
//! The lock is proven as a HELD property (a probe from inside `apply`,
//! non-blocking, through a second independent capability), not as a file
//! that once existed; staging is proven as an offer that follows the
//! descriptor. One §7.2 sentence, one cell.

use specmark::verifies;

use super::apply_selection;
use super::state::DeploymentHome;
use super::support::{Fixture, FixtureProvider, selected, selection, target};

/// § "Apply uses a per-destination lock" — as a HELD property, not a file
/// that once existed: a lock file outlives its guard, so only a probe from
/// inside `apply` can tell a held lock from a released one.
///
/// The probe is a second, independent capability over the same state home
/// trying the same lock name (non-blocking). While the engine's guard is
/// held it must be refused; a guard dropped before the provider runs would
/// let it through, and this pin is what makes that a red.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_destination_lock_is_held_while_the_provider_applies() {
    struct LockProbe {
        inner: FixtureProvider,
        state_home: std::path::PathBuf,
        lock_name: String,
    }
    use super::protocol::{ApplyReport, DeployFingerprint, DeployPlan, ObservedResource};
    use crate::mechanism::{DeployProvider, DeployTargetRequest, MechanismError};
    impl DeployProvider for LockProbe {
        fn descriptor(&self) -> super::protocol::DeployDescriptor {
            self.inner.descriptor()
        }
        fn plan(&self, request: &DeployTargetRequest<'_>) -> Result<DeployPlan, MechanismError> {
            self.inner.plan(request)
        }
        fn fingerprint(
            &self,
            request: &DeployTargetRequest<'_>,
            plan: &DeployPlan,
        ) -> Result<DeployFingerprint, MechanismError> {
            self.inner.fingerprint(request, plan)
        }
        fn apply(
            &self,
            request: &DeployTargetRequest<'_>,
            plan: &DeployPlan,
            checkpoint: &mut super::state::CheckpointLedger<'_>,
        ) -> Result<ApplyReport, MechanismError> {
            let probe = vibe_safefs::Project::open(&self.state_home)
                .expect("the probe opens its own capability");
            assert!(
                probe
                    .try_lock(&self.lock_name)
                    .expect("the probe's try_lock itself works")
                    .is_none(),
                "the per-destination lock must be HELD while the provider applies",
            );
            self.inner.apply(request, plan, checkpoint)
        }
        fn verify(
            &self,
            request: &DeployTargetRequest<'_>,
            resources: &[String],
        ) -> Result<Vec<ObservedResource>, MechanismError> {
            self.inner.verify(request, resources)
        }
        fn remove(
            &self,
            request: &DeployTargetRequest<'_>,
            resources: &[String],
            prior_state_handle: Option<&str>,
        ) -> Result<super::protocol::RemoveReport, MechanismError> {
            self.inner.remove(request, resources, prior_state_handle)
        }
        fn recover(
            &self,
            request: &DeployTargetRequest<'_>,
            plan: &DeployPlan,
            observed: &[ObservedResource],
            checkpoint: &mut super::state::CheckpointLedger<'_>,
        ) -> Result<ApplyReport, MechanismError> {
            self.inner.recover(request, plan, observed, checkpoint)
        }
    }

    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    // The engine's own lock-name spelling: `<sha256(resource)>.lock`.
    let mut digest = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(&mut digest, b"bin/helper");
    let lock_name = format!("{:x}.lock", sha2::Digest::finalize(digest));
    let probe = LockProbe {
        inner: FixtureProvider::new(fixture.destination.path(), &["bin/helper"]),
        state_home: state_home.clone(),
        lock_name,
    };
    apply_selection(&execution, &[selected(&targets[0], Box::new(probe))])
        .expect("the probed deployment applies");
}

/// § "Apply uses a per-destination lock and staging where the destination
/// supports atomic replacement."
///
/// Both halves: a provider that declares atomic replacement is handed an
/// engine-prepared staging directory, one that does not is handed `None`,
/// and either way one lock file per DESTINATION exists afterwards.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn staging_is_offered_only_where_the_destination_supports_atomic_replacement() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let selection = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &selection, &state_home);
    let provider = FixtureProvider::new(fixture.destination.path(), &["bin/helper"]).staging();
    apply_selection(&execution, &[selected(&targets[0], Box::new(provider))])
        .expect("an atomically replaceable destination deploys through staging");

    let home = DeploymentHome::new(&state_home, "org.example/demo", None, "local-helper");
    assert!(
        home.staging().is_dir(),
        "the engine prepared the staging directory it offered",
    );
    // One lock per destination, under the state home's own lock directory.
    let locks: Vec<String> = std::fs::read_dir(state_home.join(".vibe"))
        .expect("the lock directory exists")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    assert_eq!(locks.len(), 1, "one destination, one lock: {locks:?}");
    assert!(locks[0].ends_with(".lock"), "{locks:?}");

    // The other posture: a provider that never declared atomic
    // replacement is not handed a staging directory at all.
    let plain = FixtureProvider::new(fixture.destination.path(), &["bin/plain"]);
    let plain_target = [target("plain-helper", "helper.exe", &[])];
    let plain_selection = super::support::selection("local", &["plain-helper"]);
    let plain_execution = fixture.execution(&plain_target, &plain_selection, &state_home);
    apply_selection(
        &plain_execution,
        &[selected(&plain_target[0], Box::new(plain))],
    )
    .expect("a plain destination deploys without staging");
    let plain_home = DeploymentHome::new(&state_home, "org.example/demo", None, "plain-helper");
    assert!(
        !plain_home.staging().exists(),
        "no staging directory is created for a provider that declared none",
    );
}
