//! § "Apply uses a per-destination lock and staging where the destination
//! supports atomic replacement" — the sentence's own cell.
//!
//! The lock is proven as a HELD property (a probe from inside `apply`,
//! non-blocking, through a second independent capability), not as a file
//! that once existed; staging is proven as an offer that follows the
//! descriptor. One §7.2 sentence, one cell.

use specmark::verifies;

use super::apply_selection;
use super::state::{DeployState, DeploymentHome};
use super::support::{Fixture, FixtureProvider, selected, selection, target};

/// The engine's own lock-file name for one resource spelling — written
/// once, so a test cannot accidentally pin a different rule than the one
/// [`DeployState::lock_destinations`] applies.
fn lock_name_of(resource: &str) -> String {
    let mut digest = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(
        &mut digest,
        vibe_safefs::path_identity_key(resource).as_bytes(),
    );
    format!("{:x}.lock", sha2::Digest::finalize(digest))
}

/// Every per-DESTINATION lock file this state home currently holds, sorted.
///
/// §6.3.1.3's stable deployment-state lock lives in the same directory under
/// its own `deployment-` prefix and is a different family entirely — one per
/// deployment rather than one per destination — so it is filtered out here
/// and asserted where it belongs.
fn lock_files(state_home: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(state_home.join(".vibe"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.ends_with(".lock") && !name.starts_with("deployment-"))
                .collect()
        })
        .unwrap_or_default();
    names.sort_unstable();
    names
}

/// One PHYSICAL destination takes exactly one lock, whatever spelling
/// names it.
///
/// §6.3.0.10's pre-apply judgement already rules `Shared.json` and
/// `shared.json` one destination and admits two reference owners of it. A
/// lock keyed on the raw bytes would then hand those two participants two
/// different lock files and let them edit one document at once — the very
/// race the shared lock exists to prevent. So the lock name goes through
/// the SAME `vibe_safefs::path_identity_key` the judgement uses: one
/// identity law, one lock.
///
/// Both alias families are covered, because both are one file on the hosts
/// this project supports: ASCII case, and Unicode composition (NFC `é`
/// against NFD `e` + combining acute).
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn one_physical_destination_takes_one_lock_whatever_its_spelling() {
    let home = crate::mechanism::package::support::temp();
    let state = DeployState::open(home.path()).expect("the state home opens");

    // Four spellings, two physical destinations.
    let case_aliases = [
        "config/Shared.json".to_owned(),
        "config/shared.json".to_owned(),
    ];
    let composition_aliases = [
        "config/caf\u{e9}.json".to_owned(),   // NFC:  é
        "config/cafe\u{301}.json".to_owned(), // NFD:  e + combining acute
    ];

    let guards = state
        .lock_destinations(&case_aliases)
        .expect("the case aliases lock");
    assert_eq!(
        guards.len(),
        1,
        "`config/Shared.json` and `config/shared.json` are ONE destination",
    );
    assert_eq!(
        lock_files(home.path()),
        [lock_name_of("config/shared.json")]
    );
    drop(guards);

    let guards = state
        .lock_destinations(&composition_aliases)
        .expect("the composition aliases lock");
    assert_eq!(
        guards.len(),
        1,
        "NFC and NFD spellings of one name are ONE destination",
    );
    drop(guards);

    // Two spellings, two identities, two locks — the law does not merge
    // destinations that really are different.
    assert_eq!(
        lock_files(home.path()),
        {
            let mut both = [
                lock_name_of("config/shared.json"),
                lock_name_of("config/caf\u{e9}.json"),
            ];
            both.sort_unstable();
            both
        },
        "two genuinely different destinations keep two locks",
    );

    // And the whole mixed set still resolves to exactly those two.
    let guards = state
        .lock_destinations(&[case_aliases.as_slice(), composition_aliases.as_slice()].concat())
        .expect("the mixed set locks");
    assert_eq!(guards.len(), 2, "four spellings, two physical destinations");
}

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
    // The engine's own lock-name spelling:
    // `<sha256(path_identity_key(resource))>.lock`.
    let lock_name = lock_name_of("bin/helper");
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
    // One lock per destination, under the state home's own lock directory —
    // beside the deployment's own state lock, which is a different family.
    let locks = lock_files(&state_home);
    assert_eq!(locks.len(), 1, "one destination, one lock: {locks:?}");
    assert_eq!(locks[0], lock_name_of("bin/helper"), "{locks:?}");
    assert!(
        std::fs::read_dir(state_home.join(".vibe"))
            .expect("the lock directory exists")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(&format!("deployment-{}", home.id()))),
        "and §6.3.1.3's stable deployment-state lock is beside it",
    );

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
