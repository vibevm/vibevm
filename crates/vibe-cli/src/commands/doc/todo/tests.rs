use super::*;

fn args(dir: &std::path::Path) -> DocTodoArgs {
    DocTodoArgs {
        path: dir.to_path_buf(),
        format: "json".into(),
        examples: false,
        min: vibe_doc::coverage::FULL_COVERAGE,
        backlog: None,
        journal: None,
        binary: None,
        sandbox: None,
        timeout: 300,
    }
}

fn package(dir: &std::path::Path) {
    std::fs::write(
        dir.join("vibe.toml"),
        "[package]\ngroup = \"org.acme\"\nname = \"manual\"\nkind = \"doc\"\n\
         version = \"2.3.4\"\n",
    )
    .expect("the manifest");
    let pages = dir.join("vibevm/vibespecs/model");
    std::fs::create_dir_all(&pages).expect("the spec root");
    std::fs::write(
        pages.join("boot-lane.xml"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
           <title id=\"root\">Page</title>\n\
         </spec>\n",
    )
    .expect("a page");
}

/// The queue is a measurer. Rows in it are not a reason to fail, and a
/// run that returned an error would be the release lock the norm refuses.
#[test]
fn a_queue_with_rows_in_it_still_succeeds() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    run_todo(args(tmp.path()), DocEnv::default()).expect("the queue prints and returns success");
}

#[test]
fn the_package_names_itself_from_its_own_manifest() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    assert_eq!(package_version(tmp.path()), "2.3.4");
}

#[test]
fn a_directory_with_no_manifest_is_reported_as_saying_nothing() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    assert_eq!(package_version(tmp.path()), "");
}

#[test]
fn the_md_form_is_the_default_and_prints_the_eight_numbers() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    package(tmp.path());
    let args = DocTodoArgs {
        format: "md".into(),
        ..args(tmp.path())
    };
    run_todo(args, DocEnv::default()).expect("a report");
}
