use super::*;
use crate::cli::{AgentModeArg, ProgressCheckArgs, ProgressReportArgs};
use std::path::Path;

fn fixture(root: &Path) {
    let specs = root.join(vibe_core::layout::current_specs_root());
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("a.md"),
        "# A {#a}\n\n@fact:A body @requires:specification @spec/done\n",
    )
    .unwrap();
    std::fs::write(root.join(specmap_core::index::INDEX_REL_PATH), "{ broken").unwrap();
}

fn common(root: &Path, campaign: Option<&Path>) -> ProgressCommonArgs {
    ProgressCommonArgs {
        path: root.to_path_buf(),
        campaign: campaign.map(Path::to_path_buf),
        no_cache: false,
    }
}

#[test]
fn malformed_specmap_affects_only_evidence_consuming_operations() {
    let tmp = tempfile::tempdir().unwrap();
    fixture(tmp.path());
    let ctx = Context::from_flags(true, false, None, true, AgentModeArg::Auto);

    check(
        &ctx,
        &ProgressCheckArgs {
            common: common(tmp.path(), None),
            exhaustive: false,
            write_state: false,
        },
    )
    .expect("read-only facts check never loads specmap");

    let report_ground = ground(&common(tmp.path(), None)).expect("generic grounding");
    let report_error = report_body(
        &report_ground,
        &ProgressReportArgs {
            common: common(tmp.path(), None),
            md: false,
            view: None,
            audience: None,
        },
        false,
    )
    .expect_err("report consumes evidence and stays loud");
    assert!(format!("{report_error:#}").contains("specmap.json"));

    let campaign = tmp.path().join("campaigns/c");
    std::fs::create_dir_all(campaign.join("run")).unwrap();
    let write_args = common(tmp.path(), Some(&campaign));
    assert!(mirror(&ctx, &write_args).is_err());
    assert!(!campaign.join("run/mirror").exists());
    assert!(resume(&ctx, &write_args).is_err());
    assert!(!campaign.join("run/RESUME.md").exists());
    assert!(scan(&ctx, &write_args).is_err());
    assert!(!campaign.join("run/cache.json").exists());
    assert!(
        check(
            &ctx,
            &ProgressCheckArgs {
                common: common(tmp.path(), Some(&campaign)),
                exhaustive: false,
                write_state: true,
            },
        )
        .is_err()
    );
    assert!(!campaign.join("run/cache.json").exists());
    let mut write_ground = ground(&write_args).unwrap();
    let write_error = refresh_state(&mut write_ground)
        .expect_err("write-state consumes evidence before any write");
    assert!(format!("{write_error:#}").contains("specmap.json"));
    assert!(!campaign.join("run/cache.json").exists());
    assert!(!campaign.join("run/state/terminal.json").exists());
}

fn write_specmap(root: &Path, with_edge: bool) {
    let file = format!(
        "{}/a.md",
        vibe_core::machine_json_path(&vibe_core::layout::current_specs_root())
    );
    let edges = if with_edge {
        serde_json::json!([{
            "file": "crates/a/src/lib.rs",
            "from_symbol": "a",
            "line": 7,
            "provenance": "authored",
            "uri": "spec://p/A#A",
            "verb": "implements"
        }])
    } else {
        serde_json::json!([])
    };
    let map = serde_json::json!({
        "code_items": [],
        "edges": edges,
        "schema": 2,
        "spec_units": [{
            "anchor": "A",
            "content_hash": "sha256:aa",
            "doc_path": "A",
            "file": file,
            "heading": "A",
            "line": 1,
            "uri": "spec://p/A#A"
        }],
        "suspects": [],
        "warnings": []
    });
    std::fs::write(
        root.join(specmap_core::index::INDEX_REL_PATH),
        serde_json::to_vec(&map).unwrap(),
    )
    .unwrap();
}

#[test]
fn provider_change_moves_terminal_projection_without_moving_cache() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let specs = root.join(vibe_core::layout::current_specs_root());
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("a.md"),
        "@fact:A body @requires:implementation @impl/done\n",
    )
    .unwrap();
    let campaign = root.join("campaigns/c");
    std::fs::create_dir_all(campaign.join("run")).unwrap();
    write_specmap(root, false);

    let args = common(root, Some(&campaign));
    let mut first = ground(&args).unwrap();
    refresh_state(&mut first).unwrap();
    let cache_path = campaign.join("run/cache.json");
    let terminal_path = campaign.join("run/state/terminal.json");
    let cache_before = std::fs::read(&cache_path).unwrap();
    let terminal_before = std::fs::read_to_string(&terminal_path).unwrap();
    assert!(terminal_before.contains("\"pending\": 1"));

    write_specmap(root, true);
    let mut second = ground(&args).unwrap();
    let refreshed = refresh_state(&mut second).unwrap();
    let terminal_after = std::fs::read_to_string(&terminal_path).unwrap();
    assert_eq!(std::fs::read(&cache_path).unwrap(), cache_before);
    assert_eq!(refreshed.writes.get("cache.json"), Some(&false));
    assert_eq!(refreshed.writes.get("terminal.json"), Some(&true));
    assert!(terminal_after.contains("\"terminal\": 1"));
    assert_ne!(terminal_after, terminal_before);
}

#[test]
fn terminal_view_is_exposed_and_requirements_fold_is_fatal() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let specs = root.join(vibe_core::layout::current_specs_root());
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("a.md"),
        "# A {#a}\n\n## S {#s}\n\n<status stage=\"spec\" state=\"done\"/>\n\n@fact:A closed @requires:specification @spec/done\n\n@fact:B legacy @spec/done\n",
    )
    .unwrap();
    let common_args = common(root, None);
    let grounded = ground(&common_args).unwrap();
    let body = report_body(
        &grounded,
        &ProgressReportArgs {
            common: common(root, None),
            md: false,
            view: Some("terminal".into()),
            audience: None,
        },
        true,
    )
    .unwrap();
    assert!(body.contains("\"address\": \"vibevm/vibespecs/a.md#A\""));
    assert!(
        !body.contains("a.md#B"),
        "terminal view excludes unclassified rows"
    );

    let ctx = Context::from_flags(true, false, None, true, AgentModeArg::Auto);
    let error = check(
        &ctx,
        &ProgressCheckArgs {
            common: common_args,
            exhaustive: false,
            write_state: false,
        },
    )
    .expect_err("a section fold cannot erase fact-owned requirements");
    assert!(format!("{error:#}").contains("1 error"), "{error:#}");
}

#[test]
fn empty_terminal_cli_view_keeps_terminal_shape() {
    let tmp = tempfile::tempdir().unwrap();
    let specs = tmp.path().join(vibe_core::layout::current_specs_root());
    std::fs::create_dir_all(&specs).unwrap();
    std::fs::write(
        specs.join("a.md"),
        "@fact:A waiting @requires:implementation @impl/done\n",
    )
    .unwrap();
    let common_args = common(tmp.path(), None);
    let grounded = ground(&common_args).unwrap();
    let args = |md| ProgressReportArgs {
        common: common(tmp.path(), None),
        md,
        view: Some("terminal".into()),
        audience: None,
    };
    let xml = report_body(&grounded, &args(false), false).unwrap();
    assert!(xml.contains("<progress-report schema=\"2\">"), "{xml}");
    assert!(!xml.contains("<marker "), "the only fact is pending: {xml}");
    let md = report_body(&grounded, &args(true), false).unwrap();
    assert!(md.contains("| requires | artifacts | terminal |"), "{md}");
}
