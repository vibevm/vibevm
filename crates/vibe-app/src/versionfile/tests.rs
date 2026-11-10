//! The version-file formats, the upward search, the resolution cache and
//! the writer (PROP-075 `#VERSION-FILE-FORMATS`, `#VERSION-FILE-KEYED`,
//! `#VERSION-FILE-WRITE`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use vibe_core::manifest::{KeyedComment, KeyedExtra, KeyedSeparator, KeyedSyntax, VersionFileDecl};

use super::*;
use crate::settings::SettingsRoot;

fn plain(names: &[&str]) -> VersionFileDecl {
    VersionFileDecl::Plain {
        names: names.iter().map(|name| (*name).to_owned()).collect(),
    }
}

fn declared(rows: &[(&str, VersionFileDecl)]) -> Declared {
    let mut installations = BTreeMap::new();
    for (index, (slot, row)) in rows.iter().enumerate() {
        installations
            .entry(format!("org.example/{slot}@1.{index}.0"))
            .or_insert_with(|| ((*slot).to_owned(), Vec::new()))
            .1
            .push(row.clone());
    }
    Declared { installations }
}

// ---------------------------------------------------------------------------
// formats
// ---------------------------------------------------------------------------

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn a_plain_file_is_its_first_non_empty_non_comment_line() {
    assert_eq!(plain_line("22.23.4\n"), Some("22.23.4"));
    // A CR is tolerated, a `#` comment is stripped, blanks are skipped.
    assert_eq!(
        plain_line("\r\n# pinned by the team\r\n  22.23.4  \r\n"),
        Some("22.23.4")
    );
    assert_eq!(plain_line("22.23.4 # the LTS line\n"), Some("22.23.4"));
    // Only the first line counts.
    assert_eq!(plain_line("22.23.4\n24.22.2\n"), Some("22.23.4"));
    // Nothing but blanks and comments is no selection at all.
    assert_eq!(plain_line(""), None);
    assert_eq!(plain_line("\n\n"), None);
    assert_eq!(plain_line("# nothing here\n"), None);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
fn tool_versions_takes_the_first_version_of_the_key_and_reports_the_rest() {
    let asdf = KeyedSyntax::TOOL_VERSIONS;
    let read = |text: &str, key: &str| keyed(text, key, asdf).unwrap();
    let text = "# the team's tools\nruby 3.3.0\nnodejs 22.23.4 20.11.0 18.19.0\npython 3.12.1\n";
    let found = read(text, "nodejs").unwrap();
    assert_eq!(found.taken, "22.23.4");
    assert_eq!(found.unused, ["20.11.0", "18.19.0"]);
    assert_eq!(found.line, 3);
    assert_eq!(read(text, "ruby").unwrap().taken, "3.3.0");
    assert!(read(text, "ruby").unwrap().unused.is_empty());
    // A key the file does not mention, a commented-out key and a key with no
    // version are all "this file says nothing about it".
    assert_eq!(read(text, "java"), None);
    assert_eq!(read("# nodejs 22.23.4\n", "nodejs"), None);
    assert_eq!(read("nodejs\n", "nodejs"), None);
    // CRLF and arbitrary spacing.
    let found = read("nodejs\t22.23.4  20.11.0 # pinned\r\n", "nodejs").unwrap();
    assert_eq!(found.taken, "22.23.4");
    assert_eq!(found.unused, ["20.11.0"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
fn a_keyed_file_reads_its_own_key_under_its_own_line_syntax() {
    let equals = KeyedSyntax {
        separator: KeyedSeparator::Equals,
        comment: KeyedComment::Hash,
        extra: KeyedExtra::Refuse,
    };
    let text = "# the team's JDK\njava=21.0.4-tem\nmaven = 3.9.9  # pinned\n";
    let found = keyed(text, "java", equals).unwrap().unwrap();
    assert_eq!(found.taken, "21.0.4-tem");
    assert_eq!(found.line, 2);
    // Another slot's key in a shared file is not a defect: it is read for
    // that slot and ignored for this one, with no fault either way.
    let maven = keyed(text, "maven", equals).unwrap().unwrap();
    assert_eq!(maven.taken, "3.9.9");
    assert_eq!(keyed(text, "gradle", equals), Ok(None));
    // A colon separator and a semicolon comment are the other two words of
    // the closed vocabulary.
    let colon = KeyedSyntax {
        separator: KeyedSeparator::Colon,
        comment: KeyedComment::Semicolon,
        extra: KeyedExtra::First,
    };
    let found = keyed("java: 21 17  ; both installed\n", "java", colon)
        .unwrap()
        .unwrap();
    assert_eq!(
        (found.taken.as_str(), found.unused.as_slice()),
        ("21", &["17".to_owned()][..])
    );
    // `comment = "none"` makes a `#` part of the value rather than a
    // comment's start.
    let none = KeyedSyntax {
        separator: KeyedSeparator::Equals,
        comment: KeyedComment::None,
        extra: KeyedExtra::First,
    };
    assert_eq!(
        keyed("java=21#x\n", "java", none).unwrap().unwrap().taken,
        "21#x"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
fn a_second_value_is_taken_under_first_and_refused_under_refuse() {
    let text = "java=21.0.4-tem extra\n";
    let refusing = KeyedSyntax {
        separator: KeyedSeparator::Equals,
        comment: KeyedComment::Hash,
        extra: KeyedExtra::Refuse,
    };
    let refusal = keyed(text, "java", refusing).expect_err("the line is unusable");
    assert!(refusal.contains("line 1"), "{refusal}");
    assert!(refusal.contains("`extra`"), "{refusal}");
    let taking = KeyedSyntax {
        extra: KeyedExtra::First,
        ..refusing
    };
    let found = keyed(text, "java", taking).unwrap().unwrap();
    assert_eq!(found.taken, "21.0.4-tem");
    assert_eq!(found.unused, ["extra"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#CHANNEL-SYNTAX")]
fn a_line_is_a_requirement_or_a_channel_and_a_requirement_is_tried_first() {
    for requirement in ["22", "^22", "=22.23.4", "22.x", ">=22, <24", "v24.22.2"] {
        let parsed = VersionRequest::parse(requirement).unwrap();
        assert!(
            parsed.spec().is_some(),
            "`{requirement}` must stay a requirement: {parsed:?}"
        );
        assert_eq!(parsed.authored, requirement);
    }
    // A leading `v` is tolerated but the authored spelling is kept.
    let tolerated = VersionRequest::parse("v24.22.2").unwrap();
    assert_eq!(tolerated.authored, "v24.22.2");
    assert!(
        tolerated
            .spec()
            .unwrap()
            .matches(&semver::Version::parse("24.22.2").unwrap())
    );
    for channel in ["lts", "lts/jod", "latest", "stable", "node", "iojs-v1.0"] {
        let parsed = VersionRequest::parse(channel).unwrap();
        assert_eq!(parsed.channel(), Some(channel), "{parsed:?}");
    }
    for bad in ["", "LTS", "lts/jod/extra", "~~", "/lts"] {
        assert!(
            VersionRequest::parse(bad).is_err(),
            "`{bad}` must be refused"
        );
    }
    assert!(is_channel_name("lts") && !is_channel_name("LTS") && !is_channel_name("1lts"));
}

// ---------------------------------------------------------------------------
// the upward search
// ---------------------------------------------------------------------------

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_nearest_directory_wins_and_the_search_stops_at_the_project_root() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project");
    let deep = project.join("a/b");
    fs::create_dir_all(&deep).unwrap();
    fs::write(dir.path().join(".nvmrc"), "18.19.0\n").unwrap();
    fs::write(project.join(".nvmrc"), "20.11.0\n").unwrap();
    let rows = declared(&[("node", plain(&[".nvmrc", ".node-version"]))]);

    // Outside a project the walk reaches above the project directory.
    let free = search(&deep, None, &rows);
    assert_eq!(free.of_slot("node").unwrap().request.authored, "20.11.0");

    // The nearer file wins over the project root's.
    fs::write(deep.join(".nvmrc"), "22.23.4\n").unwrap();
    let near = search(&deep, Some(&project), &rows);
    assert_eq!(near.of_slot("node").unwrap().request.authored, "22.23.4");
    assert_eq!(near.of_slot("node").unwrap().path, deep.join(".nvmrc"));

    // With the project root as the stop, the file above it is never read.
    fs::remove_file(deep.join(".nvmrc")).unwrap();
    fs::remove_file(project.join(".nvmrc")).unwrap();
    let stopped = search(&deep, Some(&project), &rows);
    assert!(stopped.found.is_empty(), "{stopped:?}");
    assert!(
        !stopped
            .consulted
            .iter()
            .any(|consulted| consulted.path.starts_with(dir.path().join(".nvmrc"))),
        "the search must not look above the project root: {stopped:?}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_declaration_order_breaks_a_tie_inside_one_directory() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".nvmrc"), "22.23.4\n").unwrap();
    fs::write(dir.path().join(".node-version"), "20.11.0\n").unwrap();
    let first = declared(&[("node", plain(&[".nvmrc", ".node-version"]))]);
    assert_eq!(
        search(dir.path(), Some(dir.path()), &first)
            .of_slot("node")
            .unwrap()
            .request
            .authored,
        "22.23.4"
    );
    let reversed = declared(&[("node", plain(&[".node-version", ".nvmrc"]))]);
    assert_eq!(
        search(dir.path(), Some(dir.path()), &reversed)
            .of_slot("node")
            .unwrap()
            .request
            .authored,
        "20.11.0"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn two_slots_read_two_formats_in_one_walk() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".nvmrc"), "22.23.4\n").unwrap();
    fs::write(dir.path().join(".tool-versions"), "ruby 3.3.0\n").unwrap();
    let rows = declared(&[
        ("node", plain(&[".nvmrc"])),
        ("ruby", VersionFileDecl::tool_versions("ruby")),
    ]);
    let found = search(dir.path(), Some(dir.path()), &rows);
    assert_eq!(found.of_slot("node").unwrap().request.authored, "22.23.4");
    assert_eq!(found.of_slot("ruby").unwrap().request.authored, "3.3.0");
    assert!(found.faults.is_empty(), "{found:?}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#ACTIVATION-HOOK")]
fn an_unusable_version_file_is_a_reported_fault_and_never_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let outer = dir.path();
    let inner = outer.join("inner");
    fs::create_dir_all(&inner).unwrap();
    fs::write(inner.join(".nvmrc"), "not a version!\n").unwrap();
    fs::write(outer.join(".nvmrc"), "22.23.4\n").unwrap();
    let rows = declared(&[("node", plain(&[".nvmrc"]))]);
    let found = search(&inner, Some(outer), &rows);
    assert_eq!(found.faults.len(), 1, "{found:?}");
    assert!(found.faults[0].contains("not a version!"), "{found:?}");
    // The walk continues past the unusable file rather than failing.
    assert_eq!(found.of_slot("node").unwrap().path, outer.join(".nvmrc"));
    // An empty file is no selection and no fault at all.
    fs::write(inner.join(".nvmrc"), "\n# nothing\n").unwrap();
    let empty = search(&inner, Some(outer), &rows);
    assert!(empty.faults.is_empty(), "{empty:?}");
    assert_eq!(empty.of_slot("node").unwrap().path, outer.join(".nvmrc"));
}

#[test]
fn a_home_that_declares_nothing_consults_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let found = search(dir.path(), None, &Declared::default());
    assert_eq!(found, Search::default());
}

// ---------------------------------------------------------------------------
// the declared sidecar
// ---------------------------------------------------------------------------

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_declared_sidecar_round_trips_and_an_empty_set_keeps_no_document() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    fs::create_dir_all(settings.applications_dir()).unwrap();
    assert_eq!(Declared::load(&settings), Declared::default());

    let rows = declared(&[
        ("node", plain(&[".nvmrc", ".node-version"])),
        ("node", VersionFileDecl::tool_versions("nodejs")),
    ]);
    rows.save(&settings).unwrap();
    assert_eq!(Declared::load(&settings), rows);
    assert_eq!(rows.slots(), ["node"]);
    assert_eq!(rows.names(), [".nvmrc", ".node-version", ".tool-versions"]);
    assert_ne!(rows.fingerprint(), Declared::default().fingerprint());

    // An empty set removes the document rather than writing an empty one.
    Declared::default().save(&settings).unwrap();
    assert!(!declared_path(&settings).exists());
    // A corrupt document is an empty set, never an error: a prompt hook
    // must not fail.
    fs::write(declared_path(&settings), b"{").unwrap();
    assert_eq!(Declared::load(&settings), Declared::default());
}

// ---------------------------------------------------------------------------
// the resolution cache
// ---------------------------------------------------------------------------

/// Copy `from`'s modification time onto `onto`, so a rewritten file looks
/// untouched to the cache and a hit can be proven.
fn copy_times(from: &Path, onto: &Path) {
    let meta = fs::metadata(from).unwrap();
    let times = fs::FileTimes::new()
        .set_accessed(meta.accessed().unwrap())
        .set_modified(meta.modified().unwrap());
    fs::OpenOptions::new()
        .write(true)
        .open(onto)
        .unwrap()
        .set_times(times)
        .unwrap();
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
fn the_cache_answers_while_every_consulted_file_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(&dir.path().join("settings"));
    let project = dir.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let file = project.join(".nvmrc");
    fs::write(&file, "22.23.4\n").unwrap();
    let rows = declared(&[("node", plain(&[".nvmrc"]))]);

    let cold = resolve(&settings, &project, Some(&project), &rows);
    assert_eq!(cold.of_slot("node").unwrap().request.authored, "22.23.4");
    assert!(cache_path(&settings).is_file(), "the cache is written");

    // Same bytes, same length, same modification time: the cache answers,
    // so the new content is NOT seen.
    let stamp = dir.path().join("stamp");
    fs::write(&stamp, "x").unwrap();
    copy_times(&file, &stamp);
    fs::write(&file, "24.22.2\n").unwrap();
    copy_times(&stamp, &file);
    let hit = resolve(&settings, &project, Some(&project), &rows);
    assert_eq!(
        hit.of_slot("node").unwrap().request.authored,
        "22.23.4",
        "an unchanged mtime and size is a cache hit"
    );

    // A real touch invalidates it.
    fs::write(&file, "24.22.2\n").unwrap();
    let fresh = resolve(&settings, &project, Some(&project), &rows);
    assert_eq!(fresh.of_slot("node").unwrap().request.authored, "24.22.2");

    // A file that appears where the search recorded an absence invalidates
    // it too.
    let deep = project.join("deep");
    fs::create_dir_all(&deep).unwrap();
    let from_deep = resolve(&settings, &deep, Some(&project), &rows);
    assert_eq!(from_deep.of_slot("node").unwrap().path, file);
    fs::write(deep.join(".nvmrc"), "20.11.0\n").unwrap();
    let nearer = resolve(&settings, &deep, Some(&project), &rows);
    assert_eq!(nearer.of_slot("node").unwrap().path, deep.join(".nvmrc"));

    // A changed declared set invalidates it as well.
    let other = declared(&[("node", plain(&[".node-version", ".nvmrc"]))]);
    fs::write(deep.join(".node-version"), "18.19.0\n").unwrap();
    assert_eq!(
        resolve(&settings, &deep, Some(&project), &other)
            .of_slot("node")
            .unwrap()
            .request
            .authored,
        "18.19.0"
    );

    clear_version_file_cache_for_test(&settings);
    assert!(!cache_path(&settings).exists());
}

fn clear_version_file_cache_for_test(settings: &SettingsRoot) {
    super::clear_cache(settings);
}
