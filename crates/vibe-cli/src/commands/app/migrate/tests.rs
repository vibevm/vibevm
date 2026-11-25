//! The human report of vibe app migrate at a fixed width. Glyphs asserted here: ✓ ✗.

use super::*;

fn fixture() -> wire::ApplicationMigration {
    serde_json::from_value(serde_json::json!({
        "protocol": "vibe-app-migration/1", "ok": false, "slot": "stateful",
        "from": "org.example/stateful@1.0.0", "to": "org.example/stateful@2.0.0",
        "work": "/h/work/m1", "resumed": false,
        "entries": [
            {"id": "npmrc", "name": "npm settings", "outcome": "done",
             "started": "2026-11-23T08:00:00Z", "finished": "2026-11-23T08:00:01Z",
             "log": "/h/work/m1/npmrc.log"},
            {"id": "state", "name": "Carried state", "outcome": "import_failed",
             "started": "2026-11-23T08:00:00Z", "finished": "2026-11-23T08:00:03Z",
             "side": "import", "step": "import-state", "log": "/h/work/m1/state-import.log",
             "reason": "the `import-state` step exited with status 4",
             "retry": "vibe app migrate stateful 1.0.0 2.0.0 --only state --resume --yes"}
        ]
    }))
    .unwrap()
}

#[test]
fn migrate_report_draws_one_block_per_entry() {
    assert_eq!(
        migration_text(&fixture(), 80),
        concat!(
            "✓ npmrc  npm settings · 1s · log /h/work/m1/npmrc.log\n",
            "✗ state  Carried state · 3s · import failed · log /h/work/m1/state-import.log\n",
            "  why     the import-state step exited with status 4\n",
            "  retry   vibe app migrate stateful 1.0.0 2.0.0 --only state --resume --yes"
        )
    );
}

#[test]
fn migrate_partly_carried_has_alert_trailer() {
    let mut document = fixture();
    assert_eq!(migration_trailer(&document), (false,
        "migrated stateful 1.0.0 → 2.0.0 partly · 1 not carried".into()));
    document.ok = true;
    document.entries[1].outcome = wire::Outcome::Done;
    assert_eq!(migration_trailer(&document), (true,
        "migrated stateful 1.0.0 → 2.0.0".into()));
    document.ok = false;
    document.entries[0].outcome = wire::Outcome::ExportFailed;
    document.entries[1].outcome = wire::Outcome::ImportFailed;
    assert_eq!(migration_trailer(&document), (false,
        "migrated nothing of stateful 1.0.0 → 2.0.0 · 2 not carried".into()));
}

#[test]
fn migrate_offer_puts_the_command_on_its_own_line() {
    assert_eq!(offer_text("1.0.0", &["Carried state"],
        "vibe app migrate stateful 1.0.0 2.0.0 --only state --yes"),
        "  state from 1.0.0 can be carried over (Carried state); nothing was migrated\n  vibe app migrate stateful 1.0.0 2.0.0 --only state --yes");
}

#[test]
fn migrate_missing_timestamps_omit_duration() {
    for (started, finished) in [(None, Some("2026-11-23T08:00:01Z")),
        (Some("2026-11-23T08:00:00Z"), None), (None, None)] {
        let mut document = fixture();
        document.entries.truncate(1);
        document.entries[0].started = started.map(str::to_owned);
        document.entries[0].finished = finished.map(str::to_owned);
        assert_eq!(migration_text(&document, 80),
            "✓ npmrc  npm settings · log /h/work/m1/npmrc.log");
    }
}

#[test]
fn migrate_malformed_timestamps_omit_duration() {
    for (started, finished) in [("bad", "2026-11-23T08:00:01Z"),
        ("2026-11-23T08:00:00Z", "bad")] {
        let mut document = fixture();
        document.entries.truncate(1);
        document.entries[0].started = Some(started.into());
        document.entries[0].finished = Some(finished.into());
        assert_eq!(migration_text(&document, 80),
            "✓ npmrc  npm settings · log /h/work/m1/npmrc.log");
    }
}

#[test]
fn migrate_reversed_timestamps_omit_duration() {
    let mut document = fixture();
    document.entries.truncate(1);
    document.entries[0].started = Some("2026-11-23T08:00:02Z".into());
    assert_eq!(migration_text(&document, 80),
        "✓ npmrc  npm settings · log /h/work/m1/npmrc.log");
}
