//! The arithmetic of the user's own extras list, the stale-line notices,
//! the routing of a recommendation to its adapter, and what a run leaves
//! on the installation — none of which runs a child (PROP-075
//! `#SLOT-EXTRAS`).
//!
//! Split out of `ecosystem/tests.rs`, which keeps the selection, the work
//! directory and the record of a run; the seam is "what the list is"
//! against "where a verb runs".

use vibe_wire::generated::application_ecosystem as wire;

use super::ECOSYSTEM_PROTOCOL;
use super::extras::{ExtrasPolicy, UserExtras};
use super::tests::{adapter, extras_wire_verb, home, package};

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_listing_reads_the_enabled_state_off_the_recorded_feature_set() {
    let resolved = package(&adapter("npm", RECOMMENDS));
    let declared = &resolved.application.ecosystem[0];
    let none = UserExtras::default();
    let both = ExtrasPolicy::default();
    // A recommendation is in the list exactly when its synthesised
    // feature is in the set the installation recorded.
    let row = super::listing::describe(&resolved, declared, &["corepack".to_owned()], &none, both);
    assert_eq!(row.id, "npm");
    assert_eq!(row.verbs, [extras_wire_verb()]);
    assert_eq!(row.recommendations.len(), 2);
    assert!(row.recommendations[0].enabled && row.recommendations[0].default);
    assert!(!row.recommendations[0].dropped);
    assert_eq!(row.recommendations[1].name, "TypeScript");
    assert!(!row.recommendations[1].enabled);
    assert_eq!(row.items, ["corepack@latest"]);
    // Nothing chosen: nothing would be installed, and the adapter is then
    // not run at all.
    let row = super::listing::describe(&resolved, declared, &[], &none, both);
    assert!(row.items.is_empty());
    // Both chosen: declaration order, de-duplicated by exact string.
    let row = super::listing::describe(
        &resolved,
        declared,
        &["typescript".to_owned(), "corepack".to_owned()],
        &none,
        both,
    );
    assert_eq!(row.items, ["corepack@latest", "typescript"]);
}

/// Two recommendations, one chosen by default and one not — the fixture
/// the arithmetic is measured against.
const RECOMMENDS: &str = "\n[[application.ecosystem.recommends]]\necosystem = \"npm\"\n\
     id = \"corepack\"\nitem = \"corepack@latest\"\ndefault = true\nreason = \"Shims\"\n\
     \n[[application.ecosystem.recommends]]\necosystem = \"npm\"\nid = \"typescript\"\n\
     item = \"typescript\"\nname = \"TypeScript\"\n";

/// What this user asks for in `[slot.runtime.extras.npm]`.
fn user(add: &[&str], drop: &[&str]) -> UserExtras {
    let mut answer = UserExtras::default();
    answer.by_ecosystem.insert(
        "npm".to_owned(),
        vibe_core::user_config::SlotExtras {
            add: add.iter().map(|item| (*item).to_owned()).collect(),
            drop: drop.iter().map(|id| (*id).to_owned()).collect(),
        },
    );
    answer
}

/// **The arithmetic**, table-driven (PROP-075 `#SLOT-EXTRAS`): enabled
/// author recommendations, minus drops, plus the user's `add`,
/// de-duplicated first-wins — and the case the owner's wording left
/// ambiguous and the design decided, that a `drop` is honoured **under
/// `--no-extras`**.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_final_list_is_recommendations_minus_drops_plus_the_users_own_add() {
    let resolved = package(&adapter("npm", RECOMMENDS));
    let routed = resolved.application.ecosystem_recommends_of("npm");
    let all = ["corepack".to_owned(), "typescript".to_owned()];
    let both = ExtrasPolicy::default();
    let no_extras = ExtrasPolicy {
        no_extras: true,
        ..both
    };
    let no_recommends = ExtrasPolicy {
        no_recommends: true,
        ..both
    };
    let neither = ExtrasPolicy {
        no_recommends: true,
        no_extras: true,
    };
    let mine = user(&["pnpm@9", "corepack@latest"], &["typescript"]);
    let own = mine.by_ecosystem.get("npm");

    for (what, chosen, user, policy, expected) in [
        // No user list at all: the author's enabled rows, in declaration
        // order. This is the "behaves exactly as today" case.
        (
            "author only",
            &all[..],
            None,
            both,
            &["corepack@latest", "typescript"][..],
        ),
        (
            "author, one chosen",
            &all[..1],
            None,
            both,
            &["corepack@latest"][..],
        ),
        ("author, none chosen", &[][..], None, both, &[][..]),
        // The user's `add` comes after, in the user's own order, and an
        // item the author already contributed is not repeated: exact
        // string, first occurrence winning.
        (
            "drop, then add, de-duplicated first-wins",
            &all[..],
            own,
            both,
            &["corepack@latest", "pnpm@9"][..],
        ),
        // `--no-extras` skips the user's `add` — and **not** their `drop`.
        (
            "no-extras keeps the drop",
            &all[..],
            own,
            no_extras,
            &["corepack@latest"][..],
        ),
        // `--no-recommends` skips the author's rows; the user's own list
        // is theirs and stays.
        (
            "no-recommends keeps the add",
            &all[..],
            own,
            no_recommends,
            &["pnpm@9", "corepack@latest"][..],
        ),
        // Both flags: nothing, so the adapter is not run at all.
        ("both flags", &all[..], own, neither, &[][..]),
        // A drop with no author row to decline changes nothing — it is
        // reported once by `stale_notices` and otherwise ignored.
        (
            "a stale drop is inert",
            &all[..],
            Some(&user(&[], &["gone"]).by_ecosystem["npm"]),
            both,
            &["corepack@latest", "typescript"][..],
        ),
    ] {
        assert_eq!(
            super::extras::final_list(&routed, chosen, user, policy),
            expected,
            "{what}"
        );
    }
}

/// An unknown slot, an unknown ecosystem id and an unknown recommendation
/// id are each reported **once** and otherwise ignored: a settings file the
/// user wrote by hand outlives the packages it mentions (PROP-075
/// `#SLOT-EXTRAS`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn a_stale_line_in_the_users_settings_is_reported_once_and_never_fatal() {
    let resolved = package(&adapter("npm", RECOMMENDS));
    let mut mine = user(&[], &["typescript", "gone", "also-gone"]);
    mine.by_ecosystem
        .insert("cargo".to_owned(), Default::default());
    let notices = super::extras::stale_notices(&resolved, &mine);
    assert_eq!(notices.len(), 3, "{notices:#?}");
    assert!(
        notices.iter().any(|line| line.contains("`cargo`")),
        "{notices:#?}"
    );
    assert!(
        notices.iter().any(|line| line.contains("`gone`")),
        "{notices:#?}"
    );
    assert!(
        !notices.iter().any(|line| line.contains("`typescript`")),
        "a declared recommendation is not stale: {notices:#?}"
    );
}

/// The `vibe-app` → user-config seam (PROP-075 `#SLOT-EXTRAS`): the file
/// the caller names is the only one read, **no path means no list**, and a
/// home with no `[slot]` table behaves exactly as today.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_users_list_is_read_from_the_file_the_caller_names_and_nowhere_else() {
    let index = home(&[]);
    // No path: no read at all, which is how every command that does not
    // consume extras behaves and how a test stays isolated by default.
    assert_eq!(
        super::extras::user_extras(None, &index, "runtime"),
        super::extras::UserExtras::default()
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    // A missing file is the all-defaults config, never an error.
    assert!(
        super::extras::user_extras(Some(&path), &index, "runtime")
            .by_ecosystem
            .is_empty()
    );
    // A file with no `[slot]` table: byte-identical behaviour to today.
    std::fs::write(&path, "[net]\noffline = true\n").unwrap();
    assert_eq!(
        super::extras::user_extras(Some(&path), &index, "runtime"),
        super::extras::UserExtras::default()
    );
    // The slot's own tables come back; another slot's do not.
    std::fs::write(
        &path,
        "[slot.runtime.extras.npm]\nadd = [\"pnpm@9\"]\ndrop = [\"corepack\"]\n\
         [slot.python.extras.pip]\nadd = [\"wheel\"]\n",
    )
    .unwrap();
    let mine = super::extras::user_extras(Some(&path), &index, "runtime");
    assert_eq!(mine.by_ecosystem["npm"].add, ["pnpm@9"]);
    assert_eq!(mine.by_ecosystem["npm"].drop, ["corepack"]);
    assert_eq!(mine.by_ecosystem.len(), 1);
    // `python` is a slot nothing installs, so it is reported once.
    assert_eq!(mine.notices.len(), 1, "{:#?}", mine.notices);
    assert!(mine.notices[0].contains("`python`"), "{:#?}", mine.notices);
    // And a file that will not parse is a notice and an empty list: the
    // extras list must never be able to fail an install.
    std::fs::write(&path, "[slot.runtime.extras.npm\n").unwrap();
    let broken = super::extras::user_extras(Some(&path), &index, "runtime");
    assert!(broken.by_ecosystem.is_empty());
    assert_eq!(broken.notices.len(), 1, "{:#?}", broken.notices);
}

/// A recommendation row belongs to the adapter its `ecosystem` field
/// names, not to the table it was authored inside (`#P7B-AS-BUILT` (3)) —
/// so the item reaches that adapter's `install` verb and no other.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS")]
fn a_recommendation_is_routed_by_its_ecosystem_field_and_not_by_its_table() {
    // Authored inside `npm`, addressed to `pip`.
    let crossed = "\n[[application.ecosystem.recommends]]\necosystem = \"pip\"\n\
         id = \"wheel\"\nitem = \"wheel\"\ndefault = true\n";
    let resolved = package(&format!(
        "{}{}",
        adapter("npm", crossed),
        adapter("pip", "")
    ));
    assert!(
        resolved
            .application
            .ecosystem_recommends_of("npm")
            .is_empty()
    );
    let routed = resolved.application.ecosystem_recommends_of("pip");
    assert_eq!(routed.len(), 1);
    assert_eq!(
        super::extras::final_list(
            &routed,
            &["wheel".to_owned()],
            None,
            ExtrasPolicy::default()
        ),
        ["wheel"]
    );
}

/// What a run leaves on the installation (PROP-075 `#SLOT-EXTRAS`,
/// `AV-FMT-029`): an apply records what it asked for and how it ended, a
/// `list` records what it saw, a failed observation keeps the previous
/// list, and a `skipped` adapter writes nothing at all.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn a_run_is_remembered_per_adapter_and_a_skip_remembers_nothing() {
    use crate::index::ExtrasResult;

    let row = |id: &str, outcome, items: &[&str], observed: &[&str]| wire::Adapter {
        id: id.to_owned(),
        name: format!("{id} packages"),
        outcome,
        items: items.iter().map(|i| (*i).to_owned()).collect(),
        observed: observed.iter().map(|i| (*i).to_owned()).collect(),
        step: None,
        log: None,
        reason: None,
        retry: None,
        started: Some("2026-11-10T10:15:00Z".to_owned()),
        finished: Some("2026-11-10T10:15:06Z".to_owned()),
    };
    let document = |verb, adapters: Vec<wire::Adapter>| wire::ApplicationEcosystem {
        protocol: ECOSYSTEM_PROTOCOL.to_owned(),
        ok: true,
        slot: "runtime".to_owned(),
        key: "org.example/runtime@2.0.0".to_owned(),
        verb,
        work: String::new(),
        adapters,
        notices: Vec::new(),
    };

    let mut kept = Vec::new();
    // A skip is not work: nothing is remembered, so the home is not
    // pushed onto the `/8` reader for an adapter that did nothing.
    assert!(!super::extras::remember(
        &mut kept,
        &document(
            wire::Verb::Install,
            vec![row("npm", wire::Outcome::Skipped, &[], &[])]
        )
    ));
    assert!(kept.is_empty());

    // An apply records the list it asked for, verbatim, and its outcome.
    assert!(super::extras::remember(
        &mut kept,
        &document(
            wire::Verb::Install,
            vec![row(
                "npm",
                wire::Outcome::Failed,
                &["corepack@latest", "; rm -rf $HOME"],
                &[]
            )]
        )
    ));
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].applied, ["corepack@latest", "; rm -rf $HOME"]);
    assert_eq!(kept[0].applied_outcome, Some(ExtrasResult::Failed));
    assert_eq!(kept[0].applied_at.as_deref(), Some("2026-11-10T10:15:06Z"));
    assert!(kept[0].observed.is_empty());

    // A `list` fills the other half of the same record, by adapter id.
    super::extras::remember(
        &mut kept,
        &document(
            wire::Verb::List,
            vec![row("npm", wire::Outcome::Done, &[], &["corepack@latest"])],
        ),
    );
    assert_eq!(kept.len(), 1, "one record per adapter id");
    assert_eq!(kept[0].observed, ["corepack@latest"]);
    assert_eq!(kept[0].applied, ["corepack@latest", "; rm -rf $HOME"]);

    // A failed observation leaves the previous list standing and names
    // itself: a lost observation costs one re-run, never a wrong answer.
    super::extras::remember(
        &mut kept,
        &document(
            wire::Verb::List,
            vec![row("npm", wire::Outcome::Failed, &[], &[])],
        ),
    );
    assert_eq!(kept[0].observed, ["corepack@latest"]);
    assert_eq!(kept[0].observed_outcome, Some(ExtrasResult::Failed));

    // `remove` and `self_upgrade` change what a version carries without
    // being an apply, so the applied list is not rewritten by one.
    assert!(!super::extras::remember(
        &mut kept,
        &document(
            wire::Verb::Remove,
            vec![row("npm", wire::Outcome::Done, &["typescript"], &[])]
        )
    ));
    assert_eq!(kept[0].applied, ["corepack@latest", "; rm -rf $HOME"]);
}
