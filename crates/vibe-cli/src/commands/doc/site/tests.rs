use std::path::Path;

use super::*;

fn args(config: &Path, out: &Path) -> DocBuildSiteArgs {
    DocBuildSiteArgs {
        config: config.to_path_buf(),
        out: out.to_path_buf(),
        dry_run: true,
        web: None,
        no_web: true,
    }
}

/// A checkout shaped like a host's: a `[project]` root and nothing else
/// a poll needs.
fn checkout(root: &Path) {
    std::fs::create_dir_all(root).expect("the checkout");
    std::fs::write(
        root.join("vibe.toml"),
        "[project]\nname = \"vibevm\"\ngroup = \"org.vibevm.core\"\nversion = \"1.0.0\"\n",
    )
    .expect("the project manifest");
    std::fs::write(root.join("README.md"), "# vibevm\n\nA tool.\n").expect("the README");
}

/// A configuration naming one host and nothing else — the only source a
/// test can hold still, and the only one that reaches no network.
fn config(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("site.toml");
    std::fs::write(
        &path,
        "schema = 1\n\n\
         [source.host]\n\
         git = \"https://example.invalid/host\"\n\
         checkout = \"checkout\"\n",
    )
    .expect("writing the configuration");
    path
}

/// A configuration nobody wrote is a refusal that names the file, not a
/// build of the defaults: a renderer pointed at the wrong path would
/// otherwise publish the public site's own defaults over whatever it was
/// meant to build.
#[test]
fn a_configuration_that_is_not_there_is_refused_by_name() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let path = tmp.path().join("site.toml");
    let error = run(args(&path, tmp.path()), DocEnv::default())
        .expect_err("a refusal")
        .to_string();
    assert!(error.contains("site.toml"), "{error}");
}

#[test]
fn a_host_checkout_is_polled_and_queued_without_touching_the_network() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    checkout(&tmp.path().join("checkout"));
    let path = config(tmp.path());
    run(args(&path, &tmp.path().join("out")), DocEnv::default()).expect("a polled and queued run");
}

/// A checkout the deploy did not prepare stops the run and says what the
/// deploy was meant to do — a source that will not answer must never read
/// as a source with nothing in it.
#[test]
fn a_host_checkout_that_is_not_there_stops_the_run() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let path = config(tmp.path());
    let error = run(args(&path, tmp.path()), DocEnv::default())
        .expect_err("a refusal")
        .to_string();
    assert!(error.contains("holds no `vibe.toml`"), "{error}");
}

/// The whole loop over one source: poll, queue, render, record — and
/// then a second run that finds nothing to do.
#[test]
fn a_second_run_over_an_unchanged_source_renders_nothing() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    checkout(&tmp.path().join("checkout"));
    let path = config(tmp.path());
    let out = tmp.path().join("out");
    let live = DocBuildSiteArgs {
        dry_run: false,
        ..args(&path, &out)
    };

    run(live, DocEnv::default()).expect("the first run");
    let first = state::read(&out).expect("the state");
    assert_eq!(first.rendered.len(), 1, "{:?}", first.rendered);
    assert!(!first.rendered[0].failed);
    assert!(first.rendered[0].files > 0);
    assert!(first.host_rendered_at.is_some());

    // The debounce holds the host back, and holding it back is exactly
    // «nothing was rendered»: the queue is empty of work either way,
    // which is what a second run over an unchanged source has to report.
    let again = DocBuildSiteArgs {
        dry_run: false,
        ..args(&path, &out)
    };
    run(again, DocEnv::default()).expect("the second run");
    let second = state::read(&out).expect("the state");
    assert_eq!(second.rendered, first.rendered);
}

/// The trees are what the static build is handed, so they have to be
/// where the next run can still find them.
#[test]
fn the_rendered_trees_stay_where_the_next_run_can_hand_them_over() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    checkout(&tmp.path().join("checkout"));
    let path = config(tmp.path());
    let out = tmp.path().join("out");
    run(
        DocBuildSiteArgs {
            dry_run: false,
            ..args(&path, &out)
        },
        DocEnv::default(),
    )
    .expect("the run");

    let work = out.join(state::STATE_DIR).join(TREES_DIR);
    let trees = every_tree(&state::read(&out).expect("the state"), &work);
    assert_eq!(trees.len(), render::FORMATS.len(), "{trees:?}");
    assert!(
        trees[0]
            .join("org.vibevm.core/vibevm/1.0.0/readme/index.html")
            .is_file(),
        "the host's README is not at its address"
    );
}

/// One version of one pair, in the shape the report's own lines are
/// formatted from.
fn pair(origin: vibe_doc::site::Origin) -> vibe_doc::site::Pair {
    vibe_doc::site::Pair {
        source: "vibespecs".into(),
        group: "org.vibevm.core".into(),
        name: "vibevm-docs".into(),
        version: "1.0.0".into(),
        content_hash: "sha256:aa".into(),
        origin,
        entry: None,
    }
}

/// The gap census as the report says it: nothing when a version left no
/// gap, one word for a registry pair, and a sentence for the host's own
/// documentation, which is rendered with every input in hand.
///
/// Neither line is a failure, and that is the point being pinned: the
/// checks gate these blocks and the site renders every version whatever
/// happens, so the census is a measurement an operator reads rather than
/// a verdict a run acts on.
#[test]
fn a_version_that_left_marked_gaps_says_so_in_one_line() {
    let counted = Unresolved {
        examples: 62,
        rules: 0,
        derived: 0,
    };
    assert_eq!(
        gaps(
            &pair(vibe_doc::site::Origin::Registry),
            Unresolved::default()
        ),
        None
    );

    let registry = gaps(&pair(vibe_doc::site::Origin::Registry), counted).expect("a line");
    assert!(registry.starts_with("  gaps   "), "{registry}");
    assert!(
        registry.contains("62 unresolved block(s) (62 example, 0 rule, 0 derived)"),
        "{registry}"
    );
    assert!(!registry.contains("defect"), "{registry}");

    let own = gaps(
        &pair(vibe_doc::site::Origin::HostProject {
            root: std::path::PathBuf::from("checkout"),
        }),
        counted,
    )
    .expect("a line");
    assert!(own.starts_with("  warn   "), "{own}");
    assert!(own.contains("so each one is a defect"), "{own}");
}

/// Every name the renderer sets on the static build is a name the site
/// package reads.
///
/// It is a contract across two languages and nothing else compares its
/// two ends: a rename on either side leaves a build that runs, a site
/// that renders and a value that quietly never arrives. That is the
/// exact failure X-058 recorded — the analytics host and the default
/// theme were configured, resolved, printed in the run's own report, and
/// then read by nobody.
///
/// It returns rather than fails when the package is not beside the
/// crate: this crate is compiled outside a checkout of the host too, and
/// there the question has no subject.
#[test]
fn the_environment_names_are_the_ones_the_site_package_reads() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(web::WEB_PACKAGE);
    let Ok(shell) = std::fs::read_to_string(package.join("site/src/config.ts")) else {
        return;
    };
    let surfaces =
        std::fs::read_to_string(package.join("tools/doc-surfaces.mjs")).expect("the tree reader");

    for name in [
        web::ORIGIN,
        web::WEBSITE_ID,
        web::HOST_URL,
        web::DEFAULT_THEME,
    ] {
        assert!(
            shell.contains(name),
            "`{name}` is set by the renderer and read by nobody in `site/src/config.ts`"
        );
    }
    assert!(
        surfaces.contains(web::TREES),
        "`{}` is set by the renderer and read by nobody in `tools/doc-surfaces.mjs`",
        web::TREES
    );
}
