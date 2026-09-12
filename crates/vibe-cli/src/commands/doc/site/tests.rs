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

#[test]
fn a_configuration_that_names_its_sources_resolves() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let config = tmp.path().join("site.toml");
    std::fs::write(
        &config,
        "schema = 1\n\n\
         [[source.registry]]\n\
         name = \"vibespecs\"\n\
         url = \"https://github.com/vibespecs\"\n",
    )
    .expect("writing the configuration");
    run(args(&config, tmp.path()), DocEnv::default()).expect("a resolved configuration");
}
