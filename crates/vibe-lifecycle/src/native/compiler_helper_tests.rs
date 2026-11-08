//! NATIVE-STOP-LAW: controlled compiler-native calls run in one operation
//! helper process that an acknowledged stop kills and reaps.
use std::ffi::OsString;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use vibe_core::cancellation::CancellationToken;
use vibe_wire::generated::native::e2::compile_reply::CompileReply;

use super::*;
use crate::native::helper::{NativeRuntime, running_helper};

const ENTRY: &str = "native::compiler_tests::helper_tests::native_helper_child_entry";

/// The re-exec target: under helper argv this test binary serves one session.
#[test]
fn native_helper_child_entry() {
    let args = std::env::args_os().collect::<Vec<_>>();
    if let Some(outcome) = native_helper_entry(&args) {
        outcome.expect("the helper session serves until its parent closes it");
    }
}

fn controlled() -> CancellationToken {
    let token = CancellationToken::new();
    let command = NativeHelperCommand::trusted(std::env::current_exe().unwrap())
        .unwrap()
        .with_prefix_args(
            ["--exact", ENTRY, "--nocapture", "--test-threads=1", "--"]
                .into_iter()
                .map(OsString::from)
                .collect(),
        );
    attach_native_helper(&token, command);
    token
}

struct Host {
    root: tempfile::TempDir,
    registry: ExtensionRegistry,
    mechanisms: MechanismRegistry,
}

impl Host {
    fn new(ids: &[&str]) -> Self {
        let root = tempdir().unwrap();
        let relative = fixture(root.path());
        let (registry, mechanisms) = registries(
            root.path(),
            ids.iter()
                .map(|id| native(id, Some(&relative), None))
                .collect(),
        );
        Self {
            root,
            registry,
            mechanisms,
        }
    }

    /// Invoke row `order` with an invoker bound to `token`.
    fn invoke(
        &self,
        token: &CancellationToken,
        order: usize,
    ) -> Result<Vec<u8>, vibe_spec::CompilerNativeInvokerError> {
        let all = self.registry.rows().iter().collect::<Vec<_>>();
        let routes = MechanismRoutes::default();
        let project = project(self.root.path());
        let world = world();
        let invoker = make_invoker(
            &all,
            &all,
            self.root.path(),
            &self.mechanisms,
            &routes,
            &project,
            &world,
            RUN_ID,
        )
        .with_cancellation_for_test(token.clone());
        let row = all[order];
        let config = effective_config(row).unwrap();
        invoker.invoke(call(row, order as u32, &config, CompilePoint::Pass, row))
    }
}

fn message(bytes: &[u8]) -> String {
    match serde_json::from_slice::<CompileReply>(bytes).unwrap() {
        CompileReply::Ok(reply) => reply.message.unwrap_or_default(),
        other => panic!("expected an ok compile reply, got {other:?}"),
    }
}

fn reaped(pid: u32) -> bool {
    if cfg!(target_os = "linux") {
        !Path::new(&format!("/proc/{pid}")).exists()
    } else {
        true
    }
}

#[test]
fn controlled_calls_match_in_process_bytes_through_one_reused_helper() {
    let host = Host::new(&["compiler-ok", "compiler-counter"]);
    let in_process = host
        .invoke(&CancellationToken::default(), 0)
        .expect("uncontrolled in-process call");
    let token = controlled();
    let hosted = host.invoke(&token, 0).expect("controlled helper call");
    assert_eq!(
        hosted, in_process,
        "the helper returns identical reply bytes"
    );
    let (pid, scratch) = running_helper(&token).expect("one running helper");
    let count = |bytes: Vec<u8>| -> usize {
        message(&bytes)
            .strip_prefix("raw invocations: ")
            .unwrap()
            .parse()
            .unwrap()
    };
    let first = count(host.invoke(&token, 1).unwrap());
    let second = count(host.invoke(&token, 1).unwrap());
    assert_eq!(second, first + 1, "both calls reached one plugin instance");
    assert_eq!(running_helper(&token).map(|(pid, _)| pid), Some(pid));
    assert_ne!(
        pid,
        std::process::id(),
        "the plugin ran outside this process"
    );
    assert_eq!(native_plugin_effects(&token), "unknown");
    drop(token);
    assert!(reaped(pid), "closing the operation reaps its helper");
    assert!(
        !scratch.exists(),
        "closing the operation removes its scratch"
    );
}

#[test]
fn stop_during_a_blocking_native_call_kills_and_reaps_the_helper_quickly() {
    let host = Host::new(&["compiler-ok", "compiler-sleep"]);
    let token = controlled();
    host.invoke(&token, 0).expect("warm helper");
    let (pid, scratch) = running_helper(&token).expect("one running helper");
    let requested = Arc::new(Mutex::new(None));
    let stopper = {
        let token = token.clone();
        let requested = Arc::clone(&requested);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(400));
            *requested.lock().unwrap() = Some(Instant::now());
            token.cancel();
        })
    };
    let error = host
        .invoke(&token, 1)
        .expect_err("the stop ends the blocking call");
    let returned = Instant::now();
    stopper.join().unwrap();
    let requested = requested.lock().unwrap().expect("the stop was requested");
    assert!(
        returned.duration_since(requested) < Duration::from_secs(2),
        "stopped within the bound"
    );
    let text = error.to_string();
    assert!(text.contains("native plugin call stopped"), "{text}");
    assert!(text.contains("native plugin effects are unknown"), "{text}");
    assert!(token.acknowledged());
    assert_eq!(token.publication_state(), "recovery-required");
    assert_eq!(native_plugin_effects(&token), "unknown");
    assert!(reaped(pid), "the helper process tree was reaped");
    assert!(
        !scratch.exists(),
        "scratch is removed after the confirmed reap"
    );
    assert!(running_helper(&token).is_none());
    let again = host
        .invoke(&token, 0)
        .expect_err("a stopped operation stays stopped");
    assert!(!again.to_string().is_empty());
}

#[test]
fn uncontrolled_selection_keeps_the_process_loader_and_unhelped_control_refuses() {
    let process = process_loader();
    match NativeRuntime::select(&CancellationToken::default(), process) {
        Some(NativeRuntime::InProcess(loader)) => assert!(std::ptr::eq(loader, process)),
        _ => panic!("an uncontrolled call keeps the in-process loader"),
    }
    let unhelped = CancellationToken::new();
    assert!(NativeRuntime::select(&unhelped, process).is_none());
    let host = Host::new(&["compiler-ok"]);
    let refusal = host.invoke(&unhelped, 0).unwrap_err().to_string();
    assert!(
        refusal.contains("no cooperative stop capability"),
        "{refusal}"
    );
    assert_eq!(native_plugin_effects(&unhelped), "not-started");
    assert_eq!(unhelped.publication_state(), "not-started");
}
