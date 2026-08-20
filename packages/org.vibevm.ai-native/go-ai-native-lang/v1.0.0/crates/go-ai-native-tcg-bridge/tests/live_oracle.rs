//! The live end-to-end chain: a real gopls over a real temp module —
//! spawn, handshake, overlay validate (a seeded type error must
//! surface), hover, completion, shutdown with no zombie. Requires
//! gopls (PATH / GO_AI_NATIVE_GOPLS / GOPATH-bin) — a stack
//! obligation; absence FAILS with the recipe, never skips
//! (TCG-ORACLE-GO §1).

use std::time::Duration;

use go_ai_native_tcg_bridge::GoOracle;
use go_ai_native_tcg_bridge::position::OuterPosition;

fn write(root: &std::path::Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, body).expect("write");
}

#[test]
fn seeded_error_surfaces_through_an_overlay_and_the_session_shuts_down() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write(root, "go.mod", "module live\n\ngo 1.24\n");
    write(
        root,
        "main.go",
        "package main\n\nfunc main() { println(greet()) }\n\nfunc greet() string { return \"hi\" }\n",
    );

    let mut oracle =
        GoOracle::spawn(root, Duration::from_secs(60)).expect("gopls spawns (stack obligation)");

    // A pure overlay carrying a type error — never written to disk.
    let outcome = oracle
        .validate(
            "main.go",
            Some(
                "package main\n\nfunc main() { println(greet()) }\n\nfunc greet() string { return 42 }\n"
                    .into(),
            ),
        )
        .expect("validate answers");
    assert!(
        outcome.diagnostics.iter().any(|d| d.category == "error"),
        "a seeded type error must surface: {outcome:?}"
    );

    // The healthy overlay goes quiet again (error-grain).
    let healthy = oracle
        .validate(
            "main.go",
            Some(
                "package main\n\nfunc main() { println(greet()) }\n\nfunc greet() string { return \"hi\" }\n"
                    .into(),
            ),
        )
        .expect("validate healthy");
    assert!(
        healthy.diagnostics.iter().all(|d| d.category != "error"),
        "the healthy overlay must carry no errors: {healthy:?}"
    );

    // Hover over `greet` in the call (line 3, character 22 —
    // `func main() { println(greet()) }`).
    let (display, _docs) = oracle
        .hover(
            "main.go",
            OuterPosition {
                line: 3,
                character: 23,
            },
            None,
        )
        .expect("hover answers");
    assert!(display.contains("greet"), "hover display: {display}");

    oracle.shutdown().expect("the LSP exit dance");
}

/// How long to wait for the OS to reap a killed child. Generous on
/// purpose: death is asynchronous, and a false red is worse than a slow
/// green. The transport's `Drop` calls `kill()` + blocking `wait()`, so
/// reap is effectively complete before the first poll — the deadline is a
/// safety net, not the expected wait.
const NO_ZOMBIE_DEADLINE_SECS: u64 = 10;

/// The poll cadence — matches the fractality pod's loopback probe.
const NO_ZOMBIE_POLL_MS: u64 = 200;

/// Read the start time of a live `pid` from the OS process table, or
/// panic: the no-zombie test is VACUOUS unless the child is provably
/// alive right after spawn (a green on a PID that was never the child
/// means nothing).
fn process_start(pid: u32, who: &str) -> u64 {
    let mut system = sysinfo::System::new();
    let target = sysinfo::Pid::from_u32(pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[target]), true);
    system
        .process(target)
        .unwrap_or_else(|| {
            panic!(
                "{who} child pid {pid} is not alive immediately after spawn — \
                 the no-zombie test would assert nothing"
            )
        })
        .start_time()
}

/// True iff `pid` names a live process that started at `started`. A dead
/// PID reads as not-alive; so does a PID the OS reused for a DIFFERENT
/// process (its start time differs) — in both cases the child we spawned
/// is gone, which is exactly what the no-zombie property requires.
fn alive_with_identity(pid: u32, started: u64) -> bool {
    let mut system = sysinfo::System::new();
    let target = sysinfo::Pid::from_u32(pid);
    system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[target]), true);
    system
        .process(target)
        .is_some_and(|p| p.start_time() == started)
}

/// Poll the process table until `pid` is dead (absent, or reused by a
/// different process), failing the test — naming the PID — if it lingers
/// past the deadline.
fn assert_child_dead_within(pid: u32, started: u64, deadline_secs: u64, who: &str) {
    let deadline = std::time::Instant::now() + Duration::from_secs(deadline_secs);
    loop {
        if !alive_with_identity(pid, started) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{who} child pid {pid} (start {started}) still alive {deadline_secs}s after \
             its transport dropped — the no-zombie property regressed (ORACLE-GO §7)"
        );
        std::thread::sleep(Duration::from_millis(NO_ZOMBIE_POLL_MS));
    }
}

/// The no-zombie property, asked of the OS (ORACLE-GO §7): the shutdown
/// dance is best-effort, kill-on-drop is the binding promise. This
/// spawns a real gopls, reads the child PID through the observability
/// seam, PROVES the process is live (else the test would be checking
/// emptiness), drops the transport — the production kill path, NOT the
/// graceful `shutdown()` (the guarantee must hold even for an
/// uncooperative child) — then polls the process table under a deadline
/// to prove the PID is dead. Capability-gated by `GoOracle::spawn`: an
/// absent gopls FAILS here with the recipe, never skips.
#[test]
fn dropping_the_oracle_kills_the_gopls_child_no_zombie() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write(root, "go.mod", "module live\n\ngo 1.24\n");

    let oracle =
        GoOracle::spawn(root, Duration::from_secs(60)).expect("gopls spawns (stack obligation)");

    let pid = oracle.child_pid();
    // Mandatory live half + identity capture (PID-reuse defense).
    let started = process_start(pid, "gopls");

    // The production kill path: drop the transport, which kill-on-drops
    // the child. No graceful shutdown() — the backstop is what we test.
    drop(oracle);

    assert_child_dead_within(pid, started, NO_ZOMBIE_DEADLINE_SECS, "gopls");
}
