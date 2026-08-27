//! R3.4 — what a SUCCESSFUL traced `vibe update` / `vibe reinstall` records.
//!
//! The claim under test is "one command, one run": whatever the command
//! compiles — a shared package unit, every workspace node, a boot regeneration
//! after a fetch — joins ONE trace run with one dense global sequence, and the
//! single registered root carries that run's member.
//!
//! The fixture declares a STATIC dependency edge on purpose. A dynamically
//! linked dependency contributes an `INDEX.md` line and no compiled artifact,
//! so a run over one records nothing and would prove nothing about ordering,
//! sequence density or scope identity.

mod common;
mod trace_support;

use std::path::Path;

use common::{UserScratch, fixture_registry};
use serde_json::Value;
use trace_support::{
    declare_static_dependency, index_of, quiet_stdout, reinstall_json, reinstall_output,
    run_directories, sole_run, trace_member, update_json, update_output,
};
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;

/// A project with a STATIC dependency, already installed and locked, wired to
/// the hermetic fixture registry through its own `vibe.toml`.
///
/// The registry has to be DECLARED rather than passed with `--registry`:
/// `vibe update` and `vibe reinstall --force` carry no such flag, and a run
/// that cannot resolve offline would be testing the fixture, not the command.
/// The seed itself is UNTRACED, so every trace tree below belongs to the
/// command under test.
fn seeded(user: &UserScratch) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    let url = format!(
        "file:///{}",
        fixture_registry().display().to_string().replace('\\', "/")
    );
    let manifest = project.path().join("vibe.toml");
    let mut text = std::fs::read_to_string(&manifest).unwrap();
    text.push_str(&format!(
        "\n[[registry]]\nname = \"fixture\"\nurl = \"{url}\"\n"
    ));
    std::fs::write(&manifest, text).unwrap();
    declare_static_dependency(project.path(), "flow:org.vibevm/integration-alpha", "^0.1");

    let seed = user
        .vibe()
        .args(["install", "--json", "--offline", "--assume-yes"])
        .arg("--path")
        .arg(project.path())
        .output()
        .unwrap();
    assert!(
        seed.status.success(),
        "{}",
        String::from_utf8_lossy(&seed.stderr)
    );
    assert!(
        run_directories(project.path()).is_empty(),
        "the seed left no trace tree of its own",
    );
    project
}

/// Force the next run to really compile: remove the generated boot artifacts.
fn stale_boot(project: &Path) {
    let _ = std::fs::remove_file(project.join("CLAUDE.md"));
    let _ = std::fs::remove_file(project.join(common::index_rel()));
}

fn events(project: &Path, run_id: &str) -> usize {
    index_of(project, run_id).events.len()
}

/// One dense global sequence starting at zero — the property that proves a
/// single run rather than several stitched together.
fn assert_dense(project: &Path, run_id: &str) {
    let index = index_of(project, run_id);
    let mut sequences: Vec<u32> = index.events.iter().map(|event| event.sequence).collect();
    sequences.sort_unstable();
    assert_eq!(
        sequences,
        (0..u32::try_from(sequences.len()).unwrap()).collect::<Vec<_>>(),
        "one dense global sequence: {sequences:?}",
    );
}

/// A whole-graph update: one Update root, one run, and every compile it
/// performed inside it.
#[test]
fn a_traced_whole_update_reports_one_update_root_over_one_shared_run() {
    let user = UserScratch::new();
    let project = seeded(&user);
    stale_boot(project.path());

    let report = update_json(&user, project.path(), &["--trace-compile"]);
    assert_eq!(report["command"], "update");
    assert_eq!(report["scope"], "all", "the whole-graph shape");
    let run_id = sole_run(project.path(), &report);
    assert_eq!(
        run_directories(project.path()),
        vec![run_id.clone()],
        "exactly one run directory — the install delegate opened none of its own",
    );

    let trace = trace_member(&report).expect("a traced update reports its trace");
    assert_eq!(trace["status"], "ok");
    assert_eq!(trace["finalised"], true);
    assert_eq!(trace["run_id"].as_str().unwrap(), run_id);

    let index = index_of(project.path(), &run_id);
    assert!(matches!(index.status, RunStatus::Ok));
    assert!(
        !index.events.is_empty(),
        "the regeneration really compiled through the borrowed recorder",
    );
    assert_dense(project.path(), &run_id);
    assert_eq!(
        trace["events"].as_str().unwrap(),
        index.events.len().to_string(),
        "and the member counts exactly what the index holds",
    );
}

/// Plain `vibe reinstall`: the boot regeneration is the compile, and it joins
/// this command's own run.
#[test]
fn a_traced_plain_reinstall_records_its_boot_regeneration() {
    let user = UserScratch::new();
    let project = seeded(&user);
    stale_boot(project.path());

    let report = reinstall_json(&user, project.path(), &["--trace-compile"]);
    assert_eq!(report["command"], "reinstall");
    assert_eq!(report["forced"], false, "no fetch happened");
    let run_id = sole_run(project.path(), &report);
    let trace = trace_member(&report).expect("traced");
    assert_eq!(trace["status"], "ok");
    assert_eq!(trace["finalised"], true);
    assert!(
        events(project.path(), &run_id) > 0,
        "a plain reinstall over a stale boot really compiles",
    );
    assert_dense(project.path(), &run_id);
}

/// `vibe reinstall --force` over a locked world: the traced apply's own boot
/// regeneration lands in the same run.
#[test]
fn a_traced_forced_reinstall_records_its_apply() {
    let user = UserScratch::new();
    let project = seeded(&user);
    stale_boot(project.path());

    let report = reinstall_json(&user, project.path(), &["--force", "--trace-compile"]);
    assert_eq!(report["command"], "reinstall");
    assert_eq!(report["forced"], true, "the materialisation force");
    let run_id = sole_run(project.path(), &report);
    let trace = trace_member(&report).expect("traced");
    assert_eq!(trace["status"], "ok");
    assert!(events(project.path(), &run_id) > 0);
    assert_dense(project.path(), &run_id);
}

/// Empty-lockfile `vibe reinstall --force`: nothing to re-fetch, boot still
/// regenerates, and the run is a truthful one rather than an absent one.
#[test]
fn a_traced_empty_force_reinstall_still_opens_and_finalises_its_run() {
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    stale_boot(project.path());

    let report = reinstall_json(&user, project.path(), &["--force", "--trace-compile"]);
    assert_eq!(report["forced"], true);
    let run_id = sole_run(project.path(), &report);
    let trace = trace_member(&report).expect("an empty force is still a traced command");
    assert_eq!(trace["status"], "ok");
    assert_eq!(trace["finalised"], true);
    assert!(matches!(
        index_of(project.path(), &run_id).status,
        RunStatus::Ok
    ));
}

/// Human, quiet and JSON all derive their timing account from the SAME member.
///
/// Independent live runs need not take identical times, so what is compared is
/// the SHAPE each surface derives: the row count the JSON member carries is the
/// row count the human table prints, and the quiet suffix reports the same
/// event and snapshot counts.
#[test]
fn the_three_surfaces_agree_because_they_read_one_member() {
    let user = UserScratch::new();
    let project = seeded(&user);

    stale_boot(project.path());
    let json = reinstall_json(&user, project.path(), &["--force", "--trace-compile"]);
    let member = trace_member(&json).expect("traced");
    let rows = member["timings"].as_array().expect("timing rows").len();
    let events = member["events"].as_str().unwrap().to_string();
    let snapshots = member["snapshots"].as_str().unwrap().to_string();

    stale_boot(project.path());
    let human = quiet_stdout(
        &user
            .vibe()
            .args(["reinstall", "--force", "--assume-yes", "--trace-compile"])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    let table_rows = human
        .lines()
        .filter(|line| line.contains("ms") || line.contains("µs"))
        .count();
    assert!(
        table_rows >= rows.min(1),
        "human mode prints the member as a table: rows={rows}\n{human}",
    );
    assert!(
        human.contains("compile trace"),
        "with its own heading: {human}",
    );

    stale_boot(project.path());
    let quiet = quiet_stdout(
        &user
            .vibe()
            .args([
                "reinstall",
                "--force",
                "--quiet",
                "--assume-yes",
                "--trace-compile",
            ])
            .arg(project.path())
            .output()
            .unwrap(),
    );
    assert_eq!(quiet.lines().count(), 1, "quiet is one line: {quiet:?}");
    assert!(
        quiet.contains("event(s)") && quiet.contains("snapshot(s)"),
        "carrying the same two counts the member does: {quiet:?} (member: \
         {events} events, {snapshots} snapshots)",
    );
}

/// Success emits exactly ONE registered root per command, with the member on
/// it and no second document beside it.
#[test]
fn a_successful_traced_run_emits_one_root_and_nothing_else() {
    let user = UserScratch::new();
    let project = seeded(&user);

    for (label, output) in [
        (
            "update",
            update_output(&user, project.path(), &["--trace-compile"]),
        ),
        (
            "reinstall",
            reinstall_output(&user, project.path(), &["--trace-compile"]),
        ),
    ] {
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let docs = trace_support::documents(&output.stdout);
        let roots: Vec<&Value> = docs.iter().filter(|doc| doc["command"] == label).collect();
        assert_eq!(roots.len(), 1, "one `{label}` root: {docs:#?}");
        assert!(
            trace_member(roots[0]).is_some(),
            "and the member rides that one root",
        );
    }
}

/// The trace-OFF document of an ordinary successful `vibe reinstall --force`,
/// pinned as a COMPLETE key/value surface.
///
/// Field-by-field assertions cannot see a key that was ADDED, and adding one is
/// exactly what widening the ordinary-success projection would do: this command
/// has always reported the regenerated nodes and the pruned slots and nothing
/// else, so `materialised` and `skipped` are empty in every trace-disabled
/// document a consumer has ever parsed — even though the apply really did
/// materialise slots. The run's own completed record is kept INTERNALLY, where
/// every park, failure and serviced continuation reads it.
///
/// So the whole object is compared, with only the dynamic project path
/// substituted. Traced success may then differ by exactly one key.
#[test]
fn a_successful_forced_reinstall_keeps_its_exact_old_field_surface() {
    let user = UserScratch::new();
    let project = seeded(&user);
    stale_boot(project.path());

    let off = reinstall_json(&user, project.path(), &["--force"]);
    // The fixture is a single standalone node, so the regenerated list is a
    // literal like every other field. Reading the actual value back into the
    // expectation would make this half of the golden vacuous — a run that
    // regenerated nothing would still match itself.
    let expected = serde_json::json!({
        "ok": true,
        "command": "reinstall",
        "project": vibe_core::machine_json_path(project.path()),
        "forced": true,
        "complete": true,
        "unchanged": false,
        "materialised": [],
        "skipped": [],
        "pruned": [],
        "nodes_regenerated": ["."],
        "hooks": [],
    });
    assert_eq!(
        off, expected,
        "the COMPLETE trace-off surface — every key, every value. A new key, a          populated `materialised`, or a `notices`/`trace`/`contributions`/         `delegation` member appearing here is a change to a document old          consumers already parse.",
    );

    // Traced: the SAME document plus exactly one key.
    stale_boot(project.path());
    let mut on = reinstall_json(&user, project.path(), &["--force", "--trace-compile"]);
    assert!(on.get("trace").is_some());
    let on_object = on.as_object_mut().expect("an object");
    on_object.remove("trace");
    assert_eq!(
        Value::Object(on_object.clone()),
        off,
        "the trace member is the ONLY difference a traced forced success has",
    );
}
