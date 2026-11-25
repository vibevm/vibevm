use super::*;

#[test]
fn alias_writes_and_lists_use_display_language() {
    let site = Site::new();
    let target = "org.vibevm.qwen/qwen-code";
    let quiet = |args: &[&str]| {
        let mut all = vec!["--quiet", "registry", "alias", "corp"];
        all.extend_from_slice(args);
        site.vibe(&all)
    };
    let empty = site.alias(&["list"]);
    ok(&empty, "empty list");
    assert_eq!(
        stdout(&empty),
        "○ registry corp issues no short names\n  vibe registry alias corp add <alias> <group>/<name>\n"
    );
    assert!(stderr(&empty).is_empty(), "{}", stderr(&empty));
    let zero = quiet(&["list"]);
    ok(&zero, "quiet empty list");
    assert_eq!(stdout(&zero), "vibe registry alias: 0 aliases in corp\n");

    let preview = site.alias(&["--dry-run", "add", "qwen", target]);
    ok(&preview, "preview add");
    trailer(&preview, "would add qwen meaning org.vibevm.qwen/qwen-code");
    assert!(!stdout(&preview).contains("now means"));
    assert!(!site.path("reg/_index/aliases.toml").exists());

    let added = site.alias(&["add", "qwen", target]);
    ok(&added, "add");
    trailer(&added, "qwen now means org.vibevm.qwen/qwen-code");
    let table = fs::read(site.path("reg/_index/aliases.toml")).unwrap();
    let unchanged = site.alias(&["add", "qwen", target]);
    ok(&unchanged, "unchanged");
    trailer(&unchanged, "qwen already means org.vibevm.qwen/qwen-code");
    assert!(!stdout(&unchanged).contains("now means"));
    assert!(
        !format!("{}{}", stdout(&unchanged), stderr(&unchanged))
            .contains("Writing the alias table")
    );
    assert_eq!(fs::read(site.path("reg/_index/aliases.toml")).unwrap(), table);

    let listed = site.alias(&["list"]);
    ok(&listed, "list");
    assert_eq!(stdout(&listed), "  qwen  →  org.vibevm.qwen/qwen-code\n");
    assert!(stderr(&listed).is_empty(), "{}", stderr(&listed));
    let one = quiet(&["list"]);
    ok(&one, "quiet list");
    assert_eq!(stdout(&one), "vibe registry alias: 1 alias in corp\n");

    ok(&site.alias(&["add", "qw", target]), "second alias");
    let two = site.alias(&["list"]);
    ok(&two, "two-row list");
    assert_eq!(
        stdout(&two),
        "  qw    →  org.vibevm.qwen/qwen-code\n  qwen  →  org.vibevm.qwen/qwen-code\n"
    );
    let count = quiet(&["list"]);
    ok(&count, "quiet two-row list");
    assert_eq!(stdout(&count), "vibe registry alias: 2 aliases in corp\n");

    let preview = site.alias(&["--dry-run", "remove", "qwen"]);
    ok(&preview, "preview remove");
    trailer(&preview, "would withdraw qwen");
    assert!(!stdout(&preview).contains("withdrew"));
    assert!(fs::read_to_string(site.path("reg/_index/aliases.toml"))
        .unwrap().contains("qwen = "));
    let removed = site.alias(&["remove", "qwen"]);
    ok(&removed, "remove");
    trailer(&removed, "withdrew qwen");
    ok(&site.alias(&["remove", "qw"]), "remove last");
    let empty_again = site.alias(&["list"]);
    ok(&empty_again, "empty again");
    assert_eq!(stdout(&empty_again), stdout(&empty));
}

fn trailer(output: &Output, what: &str) {
    let said = stdout(output);
    let lines: Vec<_> = said.lines().collect();
    assert_eq!(lines.iter().filter(|line| line.starts_with("✻ ")).count(), 1, "{said}");
    let last = lines.last().unwrap();
    assert!(last.starts_with(&format!("✻ {what} · registry corp · ")), "{said}");
    assert!(!last.contains('`'), "{said}");
    assert!(!stderr(output).contains("✻ "), "{}", stderr(output));
}

/// The project install path obeys the same law: `vibe install <bare>`
/// resolves an issued spelling and refuses a package's own name, with
/// the qualified name offered to copy.
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn the_project_install_path_resolves_only_issued_names() {
    let site = Site::new();
    fs::create_dir_all(site.path("proj")).unwrap();
    fs::write(
        site.path("proj/vibe.toml"),
        "[package]\nname = \"consumer\"\ngroup = \"com.example\"\n\
         kind = \"flow\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let install = |spelling: &str| {
        site.vibe(&[
            "install",
            "--path",
            site.path("proj").to_str().unwrap(),
            "--plan-only",
            spelling,
        ])
    };

    // The package's own name is not a short name.
    let refused = install("qwen-code");
    assert!(
        !refused.status.success(),
        "a package name resolves nothing: {}",
        stdout(&refused)
    );
    let said = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(
        said.contains("not issued by any consulted registry"),
        "{said}"
    );
    assert!(
        said.contains("org.vibevm.qwen/qwen-code"),
        "the refusal hands over the qualified name: {said}"
    );

    // Issued, and the bare spelling now resolves through it. The
    // install itself may still fail further down (an application
    // package is not a project requirement), so this asserts on the
    // provenance line rather than on the exit code.
    ok(
        &site.alias(&["add", "qc", "org.vibevm.qwen/qwen-code"]),
        "alias add",
    );
    let attempted = install("qc");
    let said = format!("{}{}", stdout(&attempted), stderr(&attempted));
    assert!(
        said.contains("qc → org.vibevm.qwen/qwen-code (alias in registry corp)"),
        "the install boundary must resolve the issued spelling: {said}"
    );
}
