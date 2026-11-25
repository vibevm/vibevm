//! Pure human projection fixtures; no filesystem or network operations.

use super::*;
use super::super::{Input, VersionInputs};
use vibe_core::progress::transfer::Purpose;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#MIRROR-DRY-RUN-OUTPUT")]
fn template_projection_has_measured_children() {
    let outcome = SnapshotOutcome {
        package: "org.x/tpl".into(),
        revision: "r3".into(),
        versions: (0..12).map(|minor| semver::Version::new(1, minor, 0)).collect(),
        channels: vec![("stable".into(), Some(semver::Version::new(1, 11, 0)))],
        unchanged: 10,
        dropped: Vec::new(),
        installers: Vec::new(),
        added: 2,
        renders: Vec::new(),
        version_list: VersionList::default(),
    };
    let inputs = (0..14)
        .map(|index| Input {
            option: format!("option-{index}"),
            name: format!("input-{index}"),
            declared: format!("https://fixture.invalid/input-{index}"),
            sha256: "0".repeat(64),
            size: Some(match index {
                0..=2 => 5_600_000,
                13 => 1_200_000,
                _ => 4_000_000,
            }),
            file: format!("input-{index}"),
            destination: std::path::PathBuf::from(format!("drop/input-{index}")),
            present: index < 3,
            purpose: Purpose::Archive,
        })
        .collect();
    let mut projection = Projection {
        rows: vec![RowProjection {
            package: outcome.package.clone(),
            versions: vec![VersionInputs {
                version: semver::Version::new(1, 0, 0),
                inputs,
            }],
        }],
    };
    let list = human_rows(std::slice::from_ref(&outcome), &projection);
    assert_eq!(list.rows.len(), 1);
    let row = &list.rows[0];
    assert_eq!(row.cells[0].text, "template org.x/tpl@r3");
    assert_eq!(row.glyph, Some((Glyph::Done, Role::Ok)));
    assert_eq!(row.meta, vec![
        Meta::Count("12 versions selected".into()),
        Meta::note("1.0.0 … 1.11.0"),
        Meta::Count("2 to snapshot".into()),
        Meta::note("10 already there"),
    ]);
    assert_eq!(row.children.len(), 3);
    assert_eq!(row.children[0], "channel stable  1.11.0");
    assert!(row.children[1].starts_with("1.0.0: 14 payload inputs · option-0, option-1"));
    assert_eq!(row.children[2], "payloads  14 inputs · 3 in the drop · 16.8 MB in the drop · 41.2 MB to fetch · 58.0 MB in all");
    let rendered = list.render_at(240);
    assert!(rendered.contains("12 versions selected · 1.0.0 … 1.11.0 · 2 to snapshot · 10 already there"));
    assert!(rendered.contains("  └ channel stable  1.11.0\n  └ 1.0.0: 14 payload inputs"));

    projection.rows[0].versions[0].inputs[12].size = None;
    projection.rows[0].versions[0].inputs[13].size = None;
    let unknown = human_rows(std::slice::from_ref(&outcome), &projection);
    assert_eq!(unknown.rows[0].children[2], "payloads  14 inputs · 3 in the drop · 16.8 MB in the drop · 36.0 MB to fetch · 52.8 MB in all · lower bound · 2 inputs without a size");
    projection.rows[0].versions[0].inputs[13].size = Some(1_200_000);
    let singular = human_rows(std::slice::from_ref(&outcome), &projection);
    assert!(singular.rows[0].children[2].ends_with(" · lower bound · 1 input without a size"));

    let absent = human_rows(std::slice::from_ref(&outcome), &Projection::default());
    assert_eq!(absent.rows[0].children, ["channel stable  1.11.0"]);
    let mut single = outcome.clone();
    single.versions.truncate(1);
    let single = human_row(&single, None);
    assert_eq!(single.meta[0], Meta::Count("1 version selected".into()));
    assert_eq!(single.meta[1], Meta::note("1.0.0"));
    let mut empty = outcome;
    empty.versions.clear();
    empty.channels.clear();
    let empty = human_row(&empty, None);
    assert_eq!(empty.meta.len(), 3);
    assert!(empty.children.is_empty());
}
