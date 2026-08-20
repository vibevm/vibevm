//! The live end-to-end suite: a REAL rust-analyzer against a scratch
//! cargo project. Installing this stack obliges the machine to carry
//! the component (ORACLE-RUST §1) — an absent rust-analyzer FAILS
//! these tests with the recipe, never skips (D11).

use std::time::Duration;

use rust_ai_native_tcg_bridge::position::OuterPosition;
use rust_ai_native_tcg_bridge::{RustOracle, resolve_rust_analyzer};

const CLEAN: &str = r#"fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

fn main() {
    let s = greet("world");
    println!("{s}");
}
"#;

fn scratch_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"live-scratch\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("Cargo.toml");
    std::fs::create_dir_all(dir.path().join("src")).expect("src");
    std::fs::write(dir.path().join("src/main.rs"), CLEAN).expect("main.rs");
    dir
}

#[test]
fn the_live_chain_answers_overlay_truth() {
    // Resolution first: the failure mode a fresh box hits, surfaced
    // with its recipe rather than skipped.
    let dir = scratch_project();
    resolve_rust_analyzer(dir.path()).expect(
        "rust-analyzer is a stack prerequisite (ORACLE-RUST §1): \
         `rustup component add rust-analyzer`",
    );

    let mut oracle =
        RustOracle::spawn(dir.path(), Duration::from_secs(60)).expect("spawn + handshake");
    assert!(
        oracle.capabilities().pull_diagnostics,
        "1.93.1 grants pull diagnostics"
    );

    // A seeded type error as a pure overlay — the disk stays clean.
    let seeded = CLEAN.replace("let s = greet", "let s: i32 = greet");
    let out = oracle
        .validate("src/main.rs", Some(seeded))
        .expect("validate overlay");
    assert!(
        out.diagnostics.iter().any(|d| d.code == "E0308"),
        "the E0308-class diagnostic surfaces through the overlay: {:?}",
        out.diagnostics
    );
    let disk = std::fs::read_to_string(dir.path().join("src/main.rs")).expect("read");
    assert_eq!(disk, CLEAN, "the overlay never touched disk");

    // The clean text clears it (version law: a NEW version, not a
    // reset).
    let clean = oracle
        .validate("src/main.rs", Some(CLEAN.to_string()))
        .expect("validate clean");
    assert!(
        clean.diagnostics.iter().all(|d| d.category != "error"),
        "clean text carries no error-grade diagnostics: {:?}",
        clean.diagnostics
    );

    // Quick info on the call site names the signature.
    let pos_line = 6u32; // 1-based: `    let s = greet("world");`
    let (display, _docs) = oracle
        .hover(
            "src/main.rs",
            OuterPosition {
                line: pos_line,
                character: 13,
            },
            None,
        )
        .expect("hover");
    assert!(
        display.contains("fn greet"),
        "hover names the signature: {display:?}"
    );

    // Completions at `let x = gre` include the in-scope fn with type
    // text.
    let entries = oracle
        .complete(
            "src/main.rs",
            OuterPosition {
                line: 6,
                character: 15,
            },
            Some(CLEAN.replace("let s = greet(\"world\")", "let x = gre")),
        )
        .expect("complete");
    let greet = entries
        .iter()
        .find(|e| e.name.starts_with("greet"))
        .expect("greet completes in scope");
    assert!(
        greet
            .type_text
            .as_deref()
            .is_some_and(|t| t.contains("String")),
        "the entry carries type text: {greet:?}"
    );

    // The graceful dance; kill-on-drop remains the backstop.
    oracle.shutdown().expect("shutdown");
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
             its transport dropped — the no-zombie property regressed (ORACLE-RUST §7)"
        );
        std::thread::sleep(Duration::from_millis(NO_ZOMBIE_POLL_MS));
    }
}

/// The no-zombie property, asked of the OS (ORACLE-RUST §7): the
/// shutdown dance is best-effort, kill-on-drop is the binding promise.
/// This spawns a real rust-analyzer, reads the child PID through the
/// observability seam, PROVES the process is live (else the test would
/// be checking emptiness), drops the transport — the production kill
/// path, NOT the graceful `shutdown()` (the guarantee must hold even for
/// an uncooperative child) — then polls the process table under a
/// deadline to prove the PID is dead. Capability-gated by
/// `resolve_rust_analyzer`, like the suite above: an absent
/// rust-analyzer FAILS here with the recipe, never skips.
#[test]
fn dropping_the_oracle_kills_the_child_process_no_zombie() {
    let dir = scratch_project();
    resolve_rust_analyzer(dir.path()).expect(
        "rust-analyzer is a stack prerequisite (ORACLE-RUST §1): \
         `rustup component add rust-analyzer`",
    );
    let oracle = RustOracle::spawn(dir.path(), Duration::from_secs(60)).expect("spawn + handshake");

    let pid = oracle.child_pid();
    // Mandatory live half + identity capture (PID-reuse defense).
    let started = process_start(pid, "rust-analyzer");

    // The production kill path: drop the transport, which kill-on-drops
    // the child. No graceful shutdown() — the backstop is what we test.
    drop(oracle);

    assert_child_dead_within(pid, started, NO_ZOMBIE_DEADLINE_SECS, "rust-analyzer");
}
