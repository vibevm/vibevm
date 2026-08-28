//! The `run.selected` law: the portable spelling grammar judged on the RAW
//! string, the planted-file read refusal, and the legacy upgrade path (an
//! idle pre-A6 file reads without it; its next begin names the current
//! node). The delegated-presence half of the law is exercised through
//! planted state in `adoption.rs`.

use std::fs;

use vibe_wire::generated::lifecycle_state::{LifecycleState, StateRun};

use super::{RUN_ID, lease};
use crate::LifecycleStateStore;

fn state_with_selected(selected: Option<&str>) -> LifecycleState {
    LifecycleState {
        schema: 1,
        run: StateRun {
            requested: "create".into(),
            chain: vec!["validate".into(), "install".into(), "create".into()],
            started: "2026-08-26T00:00:00Z".into(),
            run_id: None,
            selected: selected.map(str::to_string),
            slot_continuation: None,
            compile_trace: false,
        },
        execution: Default::default(),
    }
}

/// The grammar table, judged through the one semantic gate over hand-built
/// states — pure, no I/O. The load-bearing negative is `""`:
/// `vibe_core::RelPath::new` is infallible and normalising, so it would
/// silently repair an empty stored `selected` to `"."` — workspace-ROOT
/// ownership — and a root invocation would then compare equal and ADOPT a
/// park it does not own. That is a forged adoption, not a refused file,
/// which is why the spelling is judged on the raw string and never through
/// a normalising constructor.
#[test]
fn selected_spelling_is_judged_on_the_raw_string() {
    let validate = super::super::validate::validate_state;
    for valid in [".", "members/tool", "members/nested/deeply"] {
        assert!(
            validate(&state_with_selected(Some(valid))).is_ok(),
            "`{valid}` is a portable workspace-relative node rel",
        );
    }
    for invalid in [
        // The ownership forgery: `RelPath::new` would fold this to ".".
        "",
        "..\\m",
        "m:\\x",
        "members\\x",
        "members//x",
        "/members",
        "members/",
        "./members",
        "members/./x",
        "members/../x",
    ] {
        let error = validate(&state_with_selected(Some(invalid)))
            .expect_err("an unportable spelling refuses");
        assert!(
            error.contains("not the portable workspace-relative identity"),
            "`{invalid}`: {error}"
        );
        // The message renders the spelling debug-quoted, so a backslash is
        // named as `\\` — assert against the same rendering.
        assert!(
            error.contains(&format!("{invalid:?}")),
            "`{invalid}` is named: {error}"
        );
    }
    assert!(
        validate(&state_with_selected(None)).is_ok(),
        "an IDLE legacy state may omit `selected` entirely",
    );
}

/// The same law at the read boundary: a planted state file with an
/// unportable spelling refuses with the erasable-cache remediation, never
/// repairs the spelling to guess at ownership.
#[test]
fn a_planted_state_with_an_unportable_selected_refuses_on_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(LifecycleStateStore::FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "schema = 1\n\
         [run]\nrequested = 'create'\nchain = []\nstarted = 't'\nselected = 'members\\tool'\n\
         [execution]\n",
    )
    .unwrap();
    let error = LifecycleStateStore::peek(dir.path())
        .expect_err("an unportable selected refuses at the read boundary")
        .to_string();
    assert!(
        error.contains("not the portable workspace-relative identity"),
        "{error}"
    );
    assert!(
        error.contains("remove this erasable cache"),
        "the remediation is the erasable-cache one: {error}"
    );
}

/// A legacy idle file (no `run_id`, no `selected`, no delegated row) reads
/// as-is; its next begin from any node writes THAT node into the header —
/// the upgrade is the ordinary header refresh, not a migration.
#[test]
fn a_legacy_idle_state_reads_and_its_next_begin_names_the_current_node() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(LifecycleStateStore::FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "schema = 1\n\
         [run]\nrequested = 'create'\nchain = []\nstarted = '2026-08-20T09:00:00Z'\n\
         [execution]\n",
    )
    .unwrap();
    let legacy = LifecycleStateStore::peek(dir.path())
        .expect("a legacy idle state still reads")
        .expect("state is present");
    assert_eq!(legacy.run.selected, None);

    let store = LifecycleStateStore::begin(
        lease(dir.path()),
        "create".into(),
        vec![],
        "2026-08-26T09:00:00Z".into(),
        RUN_ID.into(),
        "members/tool".into(),
        false,
    )
    .unwrap();
    let written: LifecycleState =
        toml::from_str(&fs::read_to_string(store.path()).unwrap()).unwrap();
    assert_eq!(
        written.run.selected.as_deref(),
        Some("members/tool"),
        "begin names the node that is running now",
    );
}
