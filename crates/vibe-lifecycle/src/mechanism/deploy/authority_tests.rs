//! §6.3.0.6's laws: the home and client authority the SURFACE resolved,
//! and the fence that keeps a relative command word out of every cell
//! below it.
//!
//! Its own cell because it is its own responsibility. The suite next door
//! proves what the pre-apply epoch REFUSES; this one proves what arrives —
//! and the two are different genres of evidence, one about a set of
//! resources and one about an injected value.

use std::rc::Rc;

use specmark::verifies;

use super::support::{Fixture, FixtureProvider, Witness, selected, selection, target};
use super::{ClientExecutable, ClientExecutables, apply_selection};

/// §6.3.0.6, at the seam the packet names: the home and the three client
/// executables the SURFACE resolved arrive on the provider's request, and
/// the user home is demonstrably not the settings root.
///
/// The negative half of the law — no cell below the surface resolves
/// either — is what the positive arrival buys: a provider handed the value
/// has nothing left to resolve, and one that resolved anyway would have to
/// write the ambient call in its own body.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn the_injected_home_and_client_paths_reach_the_provider_request() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let execution = fixture.execution(&targets, &chosen, &state_home);
    let provider = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));

    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(Witness(Rc::clone(&provider))),
        )],
    )
    .expect("the deployment applies");

    let seen = provider.authority();
    assert_eq!(seen.len(), 1, "one plan, one injected authority");
    let (home, clients) = &seen[0];
    assert_eq!(home, fixture.home.path(), "the surface's own user home");
    assert_ne!(
        home.as_path(),
        fixture.settings.path(),
        "the user home is NOT the settings root; a cell that confused them \
         would write a user's client state inside vibevm's own directory",
    );
    assert_eq!(clients, &fixture.clients);
    for client in clients.all() {
        assert!(
            client
                .resolved_path()
                .is_some_and(|path| path.starts_with(fixture.home.path())),
            "`{client:?}` must be the injected fake, never a client found on PATH",
        );
    }
}

/// The ARRIVAL FENCE for §6.3.0.6's negative half: whatever reaches a
/// provider, it is never a relative command word.
///
/// A bare `claude` in the lower value is not an injected authority at all —
/// `Command::new` would search `PATH` for it inside the provider, which is
/// the resolution the surface was supposed to have done. So this walks
/// every member of what really arrived and holds each to the total
/// contract: an ABSOLUTE resolved path, or a named absence with no path at
/// all. There is no third answer, and in particular no relative one.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn no_relative_command_word_can_reach_a_provider() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let mut execution = fixture.execution(&targets, &chosen, &state_home);
    // One resolved and two absent — the shape a machine with a partial
    // client install really produces, so the fence is proven over both
    // variants rather than only over the happy one.
    let mixed = ClientExecutables {
        claude: fixture.clients.claude.clone(),
        codex: ClientExecutable::Missing {
            command: "codex".to_owned(),
        },
        opencode: ClientExecutable::Missing {
            command: "opencode".to_owned(),
        },
    };
    execution.clients = &mixed;
    let provider = Rc::new(FixtureProvider::new(
        fixture.destination.path(),
        &["bin/helper"],
    ));

    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(Witness(Rc::clone(&provider))),
        )],
    )
    .expect("a run that selects no client deploys with two clients absent");

    let (_, clients) = &provider.authority()[0];
    for client in clients.all() {
        match client {
            ClientExecutable::Resolved { command, path } => {
                assert!(
                    path.is_absolute(),
                    "`{command}` arrived as `{}`, which is not an absolute executable",
                    path.display(),
                );
                assert_ne!(
                    path.as_os_str(),
                    std::ffi::OsStr::new(command.as_str()),
                    "`{command}` arrived as the bare command word, which is a PATH search \
                     deferred into the provider",
                );
            }
            ClientExecutable::Missing { command } => {
                assert!(
                    !command.is_empty(),
                    "an absence still names the command an operator must install",
                );
                assert!(client.resolved_path().is_none());
            }
        }
    }
}

/// Missing client CLIs are data, not a global deploy prerequisite. A target
/// that selects no client provider still runs when all three are absent; only
/// the future provider that needs one may turn its named absence into a
/// refusal.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS")]
fn an_unrelated_deploy_runs_when_every_client_is_missing() {
    let fixture = Fixture::new("helper-bytes");
    let targets = [target("local-helper", "helper.exe", &[])];
    let chosen = selection("local", &["local-helper"]);
    let state_home = fixture.state_home();
    let mut execution = fixture.execution(&targets, &chosen, &state_home);
    let missing = ClientExecutables {
        claude: ClientExecutable::Missing {
            command: "claude".to_owned(),
        },
        codex: ClientExecutable::Missing {
            command: "codex".to_owned(),
        },
        opencode: ClientExecutable::Missing {
            command: "opencode".to_owned(),
        },
    };
    execution.clients = &missing;

    apply_selection(
        &execution,
        &[selected(
            &targets[0],
            Box::new(FixtureProvider::new(
                fixture.destination.path(),
                &["bin/helper"],
            )),
        )],
    )
    .expect("a non-client deployment ignores three named client absences");

    assert!(fixture.destination.path().join("bin/helper").is_file());
}
