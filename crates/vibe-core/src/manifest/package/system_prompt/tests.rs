//! `[system_prompt]` round-trips, and every refusal names its key.
use super::*;
use crate::manifest::Manifest;

fn parse(body: &str) -> Result<SystemPromptSection, toml::de::Error> {
    toml::from_str(body)
}

#[test]
fn an_absent_table_leaves_every_existing_manifest_byte_identical() {
    // The whole compatibility claim of SP-FMT-002, as a round-trip: a
    // manifest that sets nothing serialises without the table at all.
    let text = "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\n\
                kind = \"flow\"\nversion = \"0.1.0\"\n";
    let manifest = Manifest::parse_str(text).unwrap();
    assert_eq!(manifest.system_prompt, None);
    let written = toml::to_string(&manifest).unwrap();
    assert!(
        !written.contains("system_prompt"),
        "an unset table must not appear in the output:\n{written}"
    );
}

#[test]
fn the_table_round_trips_through_the_wire_and_back() {
    let text = "[project]\nname = \"host\"\nversion = \"1.0.0\"\n\
                [system_prompt]\nentrypoint = \"CLAUDE.md\"\n\
                lane_form = \"verbatim\"\nrecipe = 1\n";
    let manifest = Manifest::parse_str(text).unwrap();
    let section = manifest.system_prompt.clone().expect("table parsed");
    assert_eq!(
        section.entrypoint,
        EntrypointChoice::File(EntrypointFile::Claude)
    );
    assert_eq!(section.lane_form, LaneForm::Verbatim);
    assert_eq!(section.recipe, Some(1));
    // Through `Serialize` (which goes via ManifestWire) and back.
    let written = toml::to_string(&manifest).unwrap();
    let again = Manifest::parse_str(&written).unwrap();
    assert_eq!(again.system_prompt, manifest.system_prompt);
}

#[test]
fn an_empty_table_is_an_explicit_opt_in_with_every_default() {
    let section = parse("").unwrap();
    assert!(section.is_empty());
    assert_eq!(section.entrypoint, EntrypointChoice::Default);
    assert_eq!(section.entrypoint.file(), Some(EntrypointFile::Agents));
    assert_eq!(section.lane_form, LaneForm::Verbatim);
    assert_eq!(section.recipe, None);
}

#[test]
fn entrypoint_false_means_no_entrypoint_segment() {
    let section = parse("entrypoint = false\n").unwrap();
    assert_eq!(section.entrypoint, EntrypointChoice::None);
    assert_eq!(section.entrypoint.file(), None);
    assert_eq!(section.entrypoint.file_name(), None);
}

#[test]
fn every_spelling_of_the_closed_set_is_admitted_and_nothing_else_is() {
    for file in EntrypointFile::ALL {
        let section = parse(&format!("entrypoint = \"{file}\"\n")).unwrap();
        assert_eq!(section.entrypoint, EntrypointChoice::File(file));
        assert_eq!(section.entrypoint.file_name(), Some(file.as_str()));
    }
    // A plausible near-miss, a case alias and a path are each refused, and
    // the message names all three legal spellings so the fix is readable
    // off the refusal itself.
    for bad in ["COPILOT.md", "agents.md", "docs/AGENTS.md", "AGENTS", ""] {
        let error = parse(&format!("entrypoint = \"{bad}\"\n"))
            .expect_err(&format!("`{bad}` must be refused"))
            .to_string();
        assert!(error.contains("entrypoint"), "names its key: {error}");
        for file in EntrypointFile::ALL {
            assert!(
                error.contains(file.as_str()),
                "names the legal spelling `{file}`: {error}"
            );
        }
    }
}

#[test]
fn entrypoint_true_is_refused_because_it_says_nothing() {
    let error = parse("entrypoint = true\n").unwrap_err().to_string();
    assert!(error.contains("entrypoint"), "{error}");
    assert!(error.contains("false"), "offers the legal boolean: {error}");
}

#[test]
fn lane_form_projected_refuses_as_reserved_rather_than_behaving_as_verbatim() {
    // PROP-076 #SYS-LANE-VERBATIM: the alternative is reserved, and a
    // silent fallback to `verbatim` would make the manifest lie.
    let error = parse("lane_form = \"projected\"\n")
        .unwrap_err()
        .to_string();
    assert!(error.contains("lane_form"), "names its key: {error}");
    assert!(error.contains("projected"), "{error}");
    assert!(
        error.contains("verbatim"),
        "names the accepted value: {error}"
    );
    let unknown = parse("lane_form = \"xml\"\n").unwrap_err().to_string();
    assert!(
        unknown.contains("lane_form") && unknown.contains("verbatim"),
        "{unknown}"
    );
}

#[test]
fn neither_platform_key_changes_anything_when_both_are_absent() {
    // The owner's compatibility condition: a table with neither key behaves
    // exactly as before the keys existed — portable host, every OS a target.
    let section = parse("").unwrap();
    assert_eq!(section.host, HostChoice::Portable);
    assert_eq!(section.host.fixed(), None);
    assert_eq!(section.target, TargetSelection::AllPlatforms);
    for os in [TargetOs::Windows, TargetOs::Macos, TargetOs::Linux] {
        assert!(
            section.target.includes(os),
            "an absent target selects every OS, including {os}"
        );
    }
    assert!(
        section.is_empty(),
        "both keys at default leave the table empty"
    );
}

#[test]
fn host_admits_portable_and_each_os_and_nothing_else() {
    let portable = parse("host = \"portable\"\n").unwrap();
    assert_eq!(portable.host, HostChoice::Portable);
    for os in [TargetOs::Windows, TargetOs::Macos, TargetOs::Linux] {
        let section = parse(&format!("host = \"{os}\"\n")).unwrap();
        assert_eq!(section.host, HostChoice::Fixed(os));
        assert_eq!(section.host.fixed(), Some(os));
        assert_eq!(section.host.as_str(), os.as_str());
    }
    // An unknown or mis-cased OS refuses, naming the field, every legal OS
    // and `portable` — which is not an OS and is easy to miss.
    for bad in ["beos", "Linux", "win", "x86_64-pc-windows-gnu", ""] {
        let error = parse(&format!("host = \"{bad}\"\n"))
            .expect_err(&format!("host = `{bad}` must be refused"))
            .to_string();
        assert!(error.contains("host"), "names its key: {error}");
        assert!(error.contains("portable"), "offers portable: {error}");
        for os in ["windows", "macos", "linux"] {
            assert!(error.contains(os), "names `{os}`: {error}");
        }
    }
}

#[test]
fn target_admits_a_nonempty_selection_and_refuses_the_rest() {
    let one = parse("target = [\"linux\"]\n").unwrap();
    assert!(one.target.includes(TargetOs::Linux));
    assert!(!one.target.includes(TargetOs::Windows));
    assert!(!one.target.includes(TargetOs::Macos));
    assert_eq!(one.target.platforms(), vec![TargetOs::Linux]);

    // The author's own order is kept, so a round-trip is byte-stable.
    let two = parse("target = [\"macos\", \"linux\"]\n").unwrap();
    assert_eq!(
        two.target.platforms(),
        vec![TargetOs::Macos, TargetOs::Linux]
    );

    // An empty list is refused rather than silently read as "every OS":
    // the two mean opposite things, and `[]` would drop every `target:` row.
    let empty = parse("target = []\n").unwrap_err().to_string();
    assert!(empty.contains("target"), "names its key: {empty}");
    assert!(
        empty.contains("omit the key"),
        "says how to mean every OS: {empty}"
    );

    let unknown = parse("target = [\"linux\", \"solaris\"]\n")
        .unwrap_err()
        .to_string();
    assert!(unknown.contains("target"), "names its key: {unknown}");
    assert!(
        unknown.contains("solaris"),
        "names the bad value: {unknown}"
    );

    let duplicate = parse("target = [\"linux\", \"linux\"]\n")
        .unwrap_err()
        .to_string();
    assert!(
        duplicate.contains("target") && duplicate.contains("once"),
        "{duplicate}"
    );
}

#[test]
fn the_two_platform_keys_are_independent_axes() {
    // `host` is where the agent runs; `target` is what the product is for.
    // A Linux workstation building Windows software is the case that proves
    // they must not be one key.
    let section = parse("host = \"linux\"\ntarget = [\"windows\"]\n").unwrap();
    assert_eq!(section.host.fixed(), Some(TargetOs::Linux));
    assert!(section.target.includes(TargetOs::Windows));
    assert!(!section.target.includes(TargetOs::Linux));
}

#[test]
fn the_platform_keys_round_trip_through_the_manifest_wire() {
    let text = "[project]\nname = \"host\"\nversion = \"1.0.0\"\n\
                [system_prompt]\nhost = \"windows\"\ntarget = [\"windows\", \"linux\"]\n";
    let manifest = Manifest::parse_str(text).unwrap();
    let written = toml::to_string(&manifest).unwrap();
    let again = Manifest::parse_str(&written).unwrap();
    assert_eq!(again.system_prompt, manifest.system_prompt);
    let section = again.system_prompt.unwrap();
    assert_eq!(section.host, HostChoice::Fixed(TargetOs::Windows));
    assert_eq!(
        section.target.platforms(),
        vec![TargetOs::Windows, TargetOs::Linux]
    );
}

#[test]
fn an_unknown_member_is_refused_by_name() {
    let error = parse("inclusion_policy = \"all\"\n")
        .unwrap_err()
        .to_string();
    assert!(error.contains("inclusion_policy"), "{error}");
}

#[test]
fn the_table_is_refused_without_the_template_that_is_the_opt_in() {
    // The refusal needs a directory, so it is `read`'s, not `parse_str`'s.
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(Manifest::FILENAME);
    std::fs::write(
        &path,
        "[project]\nname = \"host\"\nversion = \"1.0.0\"\n[system_prompt]\n",
    )
    .unwrap();
    let error = Manifest::read(&path).unwrap_err().to_string();
    assert!(
        error.contains("system_prompt"),
        "names the TOML path: {error}"
    );
    assert!(
        error.contains("SYSTEM-TEMPLATE.md"),
        "names the file that is the opt-in: {error}"
    );

    // With the template beside it, the same document is admitted.
    let template = root
        .path()
        .join(crate::layout::current_boot_system_template());
    std::fs::create_dir_all(template.parent().unwrap()).unwrap();
    std::fs::write(&template, "").unwrap();
    let manifest = Manifest::read(&path).unwrap();
    assert!(manifest.system_prompt.is_some());
}

#[test]
fn a_manifest_without_the_table_never_looks_for_a_template() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(Manifest::FILENAME);
    std::fs::write(&path, "[project]\nname = \"host\"\nversion = \"1.0.0\"\n").unwrap();
    let manifest = Manifest::read(&path).unwrap();
    assert_eq!(manifest.system_prompt, None);
}

#[test]
fn the_table_is_legal_on_a_project_role_not_only_on_a_package() {
    // The prefix is a property of the SELECTED NODE, which may be a plain
    // `[project]` — so `[system_prompt]` is not a package-role section and
    // must not join the "package-role section without [package]" refusal.
    let manifest = Manifest::parse_str(
        "[project]\nname = \"host\"\nversion = \"1.0.0\"\n[system_prompt]\nrecipe = 2\n",
    )
    .unwrap();
    assert_eq!(manifest.system_prompt.unwrap().recipe, Some(2));
}
