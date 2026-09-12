use std::path::Path;

use super::*;

fn args(config: &Path, out: &Path) -> DocBuildSiteArgs {
    DocBuildSiteArgs {
        config: config.to_path_buf(),
        out: out.to_path_buf(),
        dry_run: true,
    }
}

/// A configuration nobody wrote is a refusal that names the file, not a
/// build of the defaults: a renderer pointed at the wrong path would
/// otherwise publish the public site's own defaults over whatever it was
/// meant to build.
#[test]
fn a_configuration_that_is_not_there_is_refused_by_name() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let config = tmp.path().join("site.toml");
    let error = run(args(&config, tmp.path()), DocEnv::default())
        .expect_err("a refusal")
        .to_string();
    assert!(error.contains("site.toml"), "{error}");
}

/// The whole run, over the one source a test can hold still: a checkout
/// on disk. Nothing here reaches the network, which is also why the
/// configuration names no registry.
#[test]
fn a_host_checkout_is_polled_and_queued_without_touching_the_network() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let checkout = tmp.path().join("checkout");
    std::fs::create_dir_all(&checkout).expect("the checkout");
    std::fs::write(
        checkout.join("vibe.toml"),
        "[project]\nname = \"vibevm\"\ngroup = \"org.vibevm.core\"\nversion = \"1.0.0\"\n",
    )
    .expect("the project manifest");

    let config = tmp.path().join("site.toml");
    std::fs::write(
        &config,
        "schema = 1\n\n\
         [source.host]\n\
         git = \"https://example.invalid/host\"\n\
         checkout = \"checkout\"\n",
    )
    .expect("writing the configuration");

    run(args(&config, &tmp.path().join("out")), DocEnv::default())
        .expect("a polled and queued run");
}

/// A checkout the deploy did not prepare stops the run and says what the
/// deploy was meant to do — a source that will not answer must never read
/// as a source with nothing in it.
#[test]
fn a_host_checkout_that_is_not_there_stops_the_run() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let config = tmp.path().join("site.toml");
    std::fs::write(
        &config,
        "schema = 1\n\n\
         [source.host]\n\
         git = \"https://example.invalid/host\"\n\
         checkout = \"absent\"\n",
    )
    .expect("writing the configuration");

    let error = run(args(&config, tmp.path()), DocEnv::default())
        .expect_err("a refusal")
        .to_string();
    assert!(error.contains("holds no `vibe.toml`"), "{error}");
}
