//! The system-prompt production roles inside the one boot transaction
//! (`PROP-076#SYS-TRANSACTION-ROLES`).
//!
//! Three statements are kept apart here because conflating any two of them
//! would be a real bug: `prompt: None` touches nothing, `PromptWrite::REMOVE`
//! retires the four owned paths, and a payload publishes them. The rest is
//! the interruption matrix over the new roles, which is why this phase
//! extends the existing safety tests rather than bypassing them.

use super::*;

#[test]
fn an_opted_in_generation_commits_all_seven_targets_and_the_ignore_file_covers_them() {
    let temp = TempDir::new().unwrap();
    let fixture = fixture(temp.path(), Direction::MarkdownToXml).opted_in();
    write_production(fixture.write()).unwrap();
    fixture.assert_new();
    fixture.assert_prompt_published();
    let ignore = fs::read_to_string(fixture.prompt_path(vibe_core::layout::BOOT_GITIGNORE))
        .expect("the ignore file vibe writes into the boot directory it owns");
    for name in [
        vibe_core::layout::BOOT_GITIGNORE,
        vibe_core::layout::SYSTEM_MD,
        vibe_core::layout::SYSTEM_META_JSON,
        vibe_core::layout::AGENT_LANE_MD,
    ] {
        assert!(ignore.contains(&format!("/{name}\n")), "{name}");
    }
    // The boot lane keeps the opposite rule, so no pattern line names it.
    for lane in [
        vibe_core::layout::INDEX_MD,
        vibe_core::layout::STATIC_MD,
        vibe_core::layout::STATIC_XML,
    ] {
        assert!(!ignore.contains(&format!("\n/{lane}\n")), "{lane}");
    }
    assert!(!intent_exists(temp.path()));
}

#[test]
fn a_second_identical_generation_writes_nothing_and_moves_no_mtime() {
    let temp = TempDir::new().unwrap();
    let fixture = fixture(temp.path(), Direction::MarkdownToXml).opted_in();
    write_production(fixture.write()).unwrap();
    let before: Vec<_> = fixture
        .prompt_pairs()
        .iter()
        .chain([(vibe_core::layout::INDEX_MD, [].as_slice())].iter())
        .map(|(name, _)| {
            let path = fixture.prompt_path(name);
            (
                path.clone(),
                fs::symlink_metadata(&path).unwrap().modified().unwrap(),
            )
        })
        .collect();
    write_production(fixture.write()).unwrap();
    for (path, modified) in before {
        assert_eq!(
            fs::symlink_metadata(&path).unwrap().modified().unwrap(),
            modified,
            "{} was rewritten by an idempotent regeneration",
            path.display()
        );
    }
    fixture.assert_prompt_published();
}

#[test]
fn opting_out_removes_exactly_the_four_owned_paths_and_nothing_else() {
    let temp = TempDir::new().unwrap();
    let fixture = fixture(temp.path(), Direction::MarkdownToXml).opted_in();
    write_production(fixture.write()).unwrap();
    // A file in the same directory that is nobody's business but its own.
    let foreign = fixture.prompt_path("90-user.xml");
    fs::write(&foreign, b"# mine\n").unwrap();

    let out = fixture.opted_out();
    write_production(out.write()).unwrap();
    out.assert_prompt_absent();
    out.assert_new();
    assert_eq!(fs::read(&foreign).unwrap(), b"# mine\n");
    assert!(!intent_exists(temp.path()));
}

#[test]
fn a_node_that_never_opted_in_touches_no_prompt_path_at_all() {
    let temp = TempDir::new().unwrap();
    let fixture = fixture(temp.path(), Direction::MarkdownToXml);
    write_production(fixture.write()).unwrap();
    fixture.assert_prompt_absent();
    // `prompt: None` is not `REMOVE`: a pre-existing file by one of those
    // names is not this publication's business and is left exactly alone.
    let stray = fixture.prompt_path(vibe_core::layout::SYSTEM_MD);
    fs::write(&stray, b"not ours\n").unwrap();
    write_production(fixture.write()).unwrap();
    assert_eq!(fs::read(&stray).unwrap(), b"not ours\n");
}

#[test]
fn a_failure_in_any_prompt_role_restores_every_previous_artifact() {
    // Each arm republishes over an already-published generation, so there is
    // a previous coherent set to lose. `PromptRemove` only fires on an
    // opt-out, and `PromptWrite` only on a publication, so the second
    // generation's mode is part of the case rather than a constant.
    for (point, opt_out) in [
        (WritePoint::PromptStage, false),
        (WritePoint::PromptWrite, false),
        (WritePoint::PromptRemove, true),
        (WritePoint::PostStagePreReplace, false),
        (WritePoint::StaticReplace, false),
        (WritePoint::IndexReplace, true),
    ] {
        let temp = TempDir::new().unwrap();
        let fixture = fixture(temp.path(), Direction::MarkdownToXml).opted_in();
        write_production(fixture.write()).unwrap();
        let after_first = fs::read(&fixture.index).unwrap();

        let mut next = if opt_out {
            fixture.clone().opted_out()
        } else {
            fixture.clone()
        };
        next.new_index = b"NEXT-INDEX\n".to_vec();
        let result = write_with_faults(next.write(), &FailAt(point));
        assert!(result.is_err(), "{point:?} did not fire");
        // The previous usable set is intact: the prefix set is still the
        // published one and INDEX still describes the world it belongs to.
        fixture.assert_prompt_published();
        assert_eq!(fs::read(&fixture.index).unwrap(), after_first, "{point:?}");
        assert!(!intent_exists(temp.path()), "{point:?}: {result:?}");
    }
}

#[test]
fn a_child_interruption_inside_a_prompt_role_recovers_to_one_coherent_set() {
    for point in [
        WritePoint::PromptWrite,
        WritePoint::PostStagePreReplace,
        WritePoint::PostIndexPreStaleCleanup,
    ] {
        let temp = TempDir::new().unwrap();
        let fixture = fixture(temp.path(), Direction::MarkdownToXml).opted_in();
        assert_interrupted(spawn_child_with_prompt(
            temp.path(),
            Direction::MarkdownToXml,
            point,
            false,
            true,
        ));
        assert!(intent_exists(temp.path()), "{point:?}");
        recover_pending(temp.path()).unwrap();
        // Recovery rolls the whole generation forward, prompt roles and all;
        // the retry then converges on exactly one STATIC carrier.
        fixture.assert_prompt_published();
        write_production(fixture.write()).unwrap();
        fixture.assert_new();
        fixture.assert_prompt_published();
        assert!(!intent_exists(temp.path()), "{point:?}");
    }
}

#[test]
fn a_journal_prompt_role_pointed_at_another_file_is_refused() {
    let temp = TempDir::new().unwrap();
    let fixture = fixture(temp.path(), Direction::MarkdownToXml).opted_in();
    write_production(fixture.write()).unwrap();
    let corpus =
        include_str!("../../../../../formats/corpora/boot-artifact-transaction/e1/commit.toml");
    let mut journal: Journal = toml::from_str(corpus).unwrap();
    let mut entry = journal.index.clone();
    entry.target = "SYSTEM.md".to_owned();
    entry.before = absent_state();
    entry.after = absent_state();
    journal.system = Some(entry.clone());
    validate_journal(&journal, Path::new(JOURNAL_NAME), JournalMode::Commit).unwrap();
    entry.target = "AGENTS.md".to_owned();
    journal.system = Some(entry);
    let error =
        validate_journal(&journal, Path::new(JOURNAL_NAME), JournalMode::Commit).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("must target exactly `SYSTEM.md`"),
        "{error}"
    );
}
