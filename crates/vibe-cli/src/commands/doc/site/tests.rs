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
