//! The arithmetic of a projection, against a rendered manifest: which
//! installer options a row keeps, which inputs they declare, and what the
//! budget does with a total it cannot know exactly.
use std::path::PathBuf;

use super::super::config::VersionList;
use super::*;

/// A two-option manifest: one declares its archive's size, the other does
/// not — which is the case that makes a projected total a lower bound.
const RENDERED: &str = r#"
[package]
group = "org.example"
name = "tool"
kind = "app"
version = "2.0.0"
description = "The quokkaword tool package"
keywords = ["quokkaword"]

[application]
id = "tool"

[[installer_options]]
id = "linux-x64"
name = "Tool 2.0.0"
platform = "linux"
platform_name = "Linux"
arch = "x86_64"

[[installer_options.steps]]
id = "tool"
title = "Download the tool"
kind = "archive"
format = "tar_gz"
source = "https://downloads.example/tool-2.0.0-linux-x64.tar.gz"
sha256 = "11"
size = 400

[[installer_options]]
id = "win-x64"
name = "Tool 2.0.0"
platform = "windows"
platform_name = "Windows"
arch = "x86_64"

[[installer_options.steps]]
id = "tool"
title = "Download the tool"
kind = "archive"
format = "zip"
source = "https://downloads.example/tool-2.0.0-win-x64.zip"
sha256 = "22"
"#;

fn render() -> FullRender {
    FullRender {
        version: "2.0.0".parse().unwrap(),
        toml: RENDERED
            .replace("\"11\"", &format!("\"{}\"", "a".repeat(64)))
            .replace("\"22\"", &format!("\"{}\"", "b".repeat(64))),
        sha256: "c".repeat(64),
        digest_source_sha256: Some("d".repeat(64)),
        digest_url: Some("https://downloads.example/SHA256SUMS".to_owned()),
        dropped: Vec::new(),
    }
}

fn row(installers: &[&str], platforms: &[&str]) -> TemplateSelection {
    TemplateSelection {
        group: "org.example".into(),
        name: "tool".into(),
        versions: Vec::new(),
        channels: Vec::new(),
        newest: None,
        platforms: platforms.iter().map(|p| (*p).to_owned()).collect(),
        archs: Vec::new(),
        installers: installers.iter().map(|i| (*i).to_owned()).collect(),
        payloads: true,
        version_list: VersionList::default(),
    }
}

fn budget(into: PathBuf, max_total: Option<u64>) -> PayloadPlan {
    PayloadPlan { max_total, into }
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
fn an_input_lands_at_the_first_rung_of_the_drop_lookup() {
    let drop = tempfile::tempdir().unwrap();
    let plan = budget(drop.path().to_path_buf(), None);
    let inputs = inputs_of(&row(&[], &[]), &render(), &plan, "org.example/tool").unwrap();
    assert_eq!(inputs.len(), 2);
    // Exactly the rung `package_lookup` tries first, for this coordinate
    // and this file name.
    let rungs = vibe_core::remap::drop::package_lookup(
        drop.path(),
        Some("org.example/tool@2.0.0"),
        &"a".repeat(64),
        Some("tool-2.0.0-linux-x64.tar.gz"),
    );
    assert_eq!(inputs[0].destination, rungs[0]);
    assert!(!inputs[0].present);
    assert_eq!(inputs[0].size, Some(400));
    assert_eq!(inputs[1].size, None);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
fn installers_win_over_platforms_and_neither_keeps_every_option() {
    let drop = tempfile::tempdir().unwrap();
    let plan = budget(drop.path().to_path_buf(), None);
    let one = inputs_of(&row(&["win-x64"], &["linux"]), &render(), &plan, "p").unwrap();
    assert_eq!(
        one.iter().map(|i| i.option.as_str()).collect::<Vec<_>>(),
        ["win-x64"]
    );
    let by_platform = inputs_of(&row(&[], &["linux"]), &render(), &plan, "p").unwrap();
    assert_eq!(
        by_platform
            .iter()
            .map(|i| i.option.as_str())
            .collect::<Vec<_>>(),
        ["linux-x64"]
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-BUDGET")]
fn the_budget_is_spent_in_selection_order_and_names_what_does_not_fit() {
    let drop = tempfile::tempdir().unwrap();
    let plan = budget(drop.path().to_path_buf(), Some(100));
    let projection = Projection {
        rows: vec![RowProjection {
            package: "org.example/tool".into(),
            versions: vec![VersionInputs {
                version: "2.0.0".parse().unwrap(),
                inputs: inputs_of(&row(&[], &[]), &render(), &plan, "org.example/tool").unwrap(),
            }],
        }],
    };
    let facts = projection.rows[0].facts();
    // One declared size, one unknown: the total is 400 and says so by
    // counting the unknown input rather than guessing at it.
    assert_eq!(facts.inputs, 2);
    assert_eq!(facts.fetch_bytes, 400);
    assert_eq!(facts.total_bytes, 400);
    assert_eq!(facts.unknown, 1);
    assert_eq!(facts.present, 0);

    let error = weigh(&projection, &plan).unwrap_err().to_string();
    assert!(error.contains("payload_budget_exceeded"), "{error}");
    assert!(
        error.contains("org.example/tool@2.0.0 (linux-x64)"),
        "{error}"
    );
    assert!(error.contains("nothing was written"), "{error}");
    // Under budget it is silent, and an unknown size never trips it.
    weigh(&projection, &budget(drop.path().to_path_buf(), Some(4096))).unwrap();
    weigh(&projection, &budget(drop.path().to_path_buf(), None)).unwrap();
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-PAYLOAD-PROJECTION"
)]
fn a_file_already_in_the_drop_is_present_and_a_wrong_one_refuses() {
    let drop = tempfile::tempdir().unwrap();
    let dir = drop.path().join("org.example/tool/2.0.0");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("tool-2.0.0-linux-x64.tar.gz");
    std::fs::write(&file, b"payload").unwrap();
    let digest = format!(
        "{:x}",
        <sha2::Sha256 as sha2::Digest>::digest(b"payload".as_slice())
    );
    assert!(present(&file, &digest).unwrap());
    assert!(present(&file, &digest.to_ascii_uppercase()).unwrap());
    assert!(!present(&dir.join("absent.tar.gz"), &digest).unwrap());

    let error = present(&file, &"f".repeat(64)).unwrap_err().to_string();
    assert!(error.contains("digest_mismatch"), "{error}");
    // Not replaced: a human put it there.
    assert_eq!(std::fs::read(&file).unwrap(), b"payload");
}
