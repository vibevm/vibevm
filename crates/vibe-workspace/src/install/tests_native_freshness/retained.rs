//! Native replay fixture with a genuinely consumed multi-fragment unit.

use super::*;

/// A second static contribution for `top`, spliced into its manifest right
/// after `[boot_snippet]`.
const TOP_SECOND_CONTRIBUTION: &str = "[[boot_snippet.fragment]]\nsource='boot/top-more.md'\n\n";

/// The native graph whose `top` ships two static contributions.
///
/// Reach `top` through a dynamic wrapper, so it is transitively suggested
/// static rather than the root of a directly selected static zone. Its two
/// fragments retain the compiled unit at de-substitution; this fixture tests
/// replay of an actually consumed unit rather than authored-zone projection.
pub(crate) fn native_graph_keeping_top(
    root_extension: &str,
    top_extension: &str,
    middle_extension: &str,
) -> NativeGraph {
    let mut graph = native_graph_projecting_top(root_extension, top_extension, middle_extension);
    let root = &graph.workspace.root;
    write(
        &root.join("vibe.toml"),
        &format!(
            "[project]\ngroup='org.demo'\nname='host'\nversion='0.1.0'\n\n\
            [requires.packages]\n'org.lock/wrapper'={{version='=1.0.0',link='dynamic'}}\n\n{root_extension}"
        ),
    );
    slot(
        root,
        "wrapper",
        "[package]\ngroup='org.lock'\nname='wrapper'\nkind='tool'\nversion='1.0.0'\n\n\
        [requires.packages]\n'org.lock/top'={version='=1.0.0',link='dynamic'}\n\n\
        [boot_snippet]\nsource='boot/wrapper.md'\nlink='dynamic'\n",
    );
    write(
        &crate::vibedeps::slot_abs_path(root, &group(), "wrapper", &version())
            .join("boot/wrapper.md"),
        "# wrapper\n",
    );
    graph.resolution.push(resolved(
        root,
        "wrapper",
        "sha256:66",
        &["org.lock/top@=1.0.0"],
    ));
    let mut lock = test_ok!(Lockfile::read(root.join("vibe.lock")), "fixture lock");
    lock.packages
        .push(locked("wrapper", "sha256:66", &["org.lock/top@=1.0.0"]));
    test_ok!(lock.write(root.join("vibe.lock")), "wrapper lock");
    graph.workspace = test_ok!(Workspace::load(root), "wrapper workspace");
    let world = test_ok!(
        ExtensionWorldEpoch::from_resolution(&graph.workspace.root, &graph.resolution),
        "wrapper world"
    );
    graph.epoch = test_ok!(
        lower_owner_runtimes(
            &graph.workspace,
            &world,
            OwnerRuntimeLowering::compatibility_root_without_presets()
        ),
        "wrapper runtimes"
    )
    .bind_run(graph.epoch.run().clone());
    graph
}

/// The same two-fragment unit directly selected statically by the root.
pub(crate) fn native_graph_projecting_top(
    root_extension: &str,
    top_extension: &str,
    middle_extension: &str,
) -> NativeGraph {
    let graph = native_graph_full(
        root_extension,
        &format!("{TOP_SECOND_CONTRIBUTION}{top_extension}"),
        middle_extension,
        "",
    );
    write(
        &crate::vibedeps::slot_abs_path(&graph.workspace.root, &group(), "top", &version())
            .join("boot/top-more.md"),
        "# top, continued\n",
    );
    graph
}
