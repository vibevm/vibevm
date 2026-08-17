//! The `unavailable` half of the read-side surface guards (F62B): the
//! tests that pin WHAT THE ANSWER SAYS about a version this build
//! refuses to act on — where the parent file holds the tests that pin
//! the shapes of the answers that say nothing unusual.
//!
//! Out of line for the 600-line file budget, by the crate's own idiom
//! (`scanner_e2e.rs` → `scanner_e2e/journal_form.rs`): the parent
//! declares `mod unavailable;`, so the module-tree position — and
//! therefore `use super::*` — reaches the fixtures above: one
//! `cmd` / `populated_index` / `quarantine_version_in_catalog` set,
//! not a second copy.

use super::*;

/// The retargeted F62A guard: a catalog carrying an unusable version
/// must NAME it in `unavailable`, not hide it. F62A's core is pinned
/// unchanged — the refused version never appears among `versions` —
/// while the answer now speaks: every `unavailable` row carries the
/// full coordinate, the missing capabilities, and the one-home recipe.
#[test]
fn get_names_a_quarantined_version_instead_of_hiding_it() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    // wal's ONLY version becomes unusable; rust keeps 0.1.0 usable and
    // loses 0.2.0 to quarantine.
    quarantine_version_in_catalog(&data, "wal", "0.1.0");
    quarantine_version_in_catalog(&data, "rust", "0.2.0");

    // The name whose every version is refused still answers: the
    // (group, name) identity stands, no version is served, and the
    // refused one is named.
    let out = cmd()
        .args(["get", data.to_str().unwrap(), "org.vibevm", "wal", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["found"], true);
    assert_eq!(env["versions"].as_array().unwrap().len(), 0);
    let unavailable = env["unavailable"].as_array().unwrap();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0]["group"], "org.vibevm");
    assert_eq!(unavailable[0]["name"], "wal");
    assert_eq!(unavailable[0]["version"], "0.1.0");
    assert_eq!(
        unavailable[0]["missing"],
        serde_json::json!(["some-future-capability"])
    );
    assert!(
        unavailable[0]["recipe"]
            .as_str()
            .unwrap()
            .contains("this build does not understand `some-future-capability`"),
        "recipe: {}",
        unavailable[0]["recipe"]
    );

    // The mixed package answers with its usable version AND names the
    // refused one.
    let out = cmd()
        .args([
            "get",
            data.to_str().unwrap(),
            "org.vibevm",
            "rust",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["found"], true);
    let versions: Vec<&str> = env["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["version"].as_str().unwrap())
        .collect();
    assert_eq!(versions, vec!["0.1.0"]);
    let unavailable = env["unavailable"].as_array().unwrap();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0]["version"], "0.2.0");

    // Asking for the refused version by number: not served (`found`
    // stays false — the ask was not answered), and named.
    let out = cmd()
        .args([
            "get",
            data.to_str().unwrap(),
            "org.vibevm",
            "rust",
            "--version",
            "0.2.0",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["found"], false);
    assert_eq!(env["versions"].as_array().unwrap().len(), 0);
    let unavailable = env["unavailable"].as_array().unwrap();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0]["version"], "0.2.0");
}

/// §3.5.2 / §6.1 — a package with NO usable versions left: the exact
/// call that used to PANIC on `args.version.unwrap()` (get.rs §1.2,
/// proven red before the fix) now explains. JSON keeps the identity
/// (`found:true`, empty `versions`, the refusal rows); the TEXT branch
/// of the same call exits zero and prints the explanation instead of
/// panicking.
#[test]
fn get_explains_a_package_with_no_usable_versions() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    quarantine_version_in_catalog(&data, "wal", "0.1.0");

    // JSON branch.
    let out = cmd()
        .args(["get", data.to_str().unwrap(), "org.vibevm", "wal", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(env["found"], true);
    assert_eq!(env["versions"].as_array().unwrap().len(), 0);
    assert_eq!(env["unavailable"].as_array().unwrap().len(), 1);

    // Text branch — the former panic site: no `--version`, no `--json`,
    // nothing left usable. It must exit zero and explain.
    let out = cmd()
        .args(["get", data.to_str().unwrap(), "org.vibevm", "wal"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("unavailable   : 1"),
        "the text branch must count the refusal: {stdout}"
    );
    assert!(
        stdout.contains("- 0.1.0  missing: some-future-capability"),
        "the text branch must name the version and its missing list: {stdout}"
    );
    assert!(
        stdout.contains("this build does not understand"),
        "the text branch must carry the recipe: {stdout}"
    );
}

/// §3.5.3 / §6.3 — the difference this step exists for: a version this
/// build refuses to act on, versus a name that never existed. With the
/// old silence both answered `found:false, versions:[]` and were
/// byte-indistinguishable; now the refused version speaks through
/// `unavailable` while the absent name keeps its exact old shape. The
/// two answers are compared side by side in ONE test.
#[test]
fn a_refused_version_differs_from_a_name_that_never_existed() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    quarantine_version_in_catalog(&data, "rust", "0.2.0");

    // A: asking for the refused version — found:false, no versions,
    // and a refusal row.
    let out = cmd()
        .args([
            "get",
            data.to_str().unwrap(),
            "org.vibevm",
            "rust",
            "--version",
            "0.2.0",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let refused: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // B: asking a name that never existed, same version number.
    let out = cmd()
        .args([
            "get",
            data.to_str().unwrap(),
            "org.vibevm",
            "definitely-absent",
            "--version",
            "0.2.0",
            "--json",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let absent: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // The old shape is identical on both — and stays so.
    assert_eq!(refused["found"], absent["found"]);
    assert_eq!(
        refused["versions"].as_array().unwrap().len(),
        absent["versions"].as_array().unwrap().len()
    );
    // The difference is `unavailable` — and ONLY it.
    assert_eq!(
        refused["unavailable"]
            .as_array()
            .expect("the refused version must be named")
            .len(),
        1
    );
    assert!(
        absent.get("unavailable").is_none() || absent["unavailable"].as_array().unwrap().is_empty(),
        "a name that never existed keeps its silence-shaped answer"
    );
}

/// §3.5.4 — the shape proven beyond `get`: `list` names the refused
/// versions on the package's row, in both branches.
#[test]
fn list_names_refused_versions_on_the_row() {
    let Some((_work, data)) = populated_index() else {
        return;
    };
    quarantine_version_in_catalog(&data, "wal", "0.1.0");

    // JSON branch: the wal row serves nothing and names the refusal.
    let out = cmd()
        .args(["list", data.to_str().unwrap(), "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    let env: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let wal_row = env["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "wal")
        .expect("wal row");
    assert_eq!(wal_row["versions"].as_array().unwrap().len(), 0);
    let unavailable = wal_row["unavailable"].as_array().unwrap();
    assert_eq!(unavailable.len(), 1);
    assert_eq!(unavailable[0]["version"], "0.1.0");
    // A package with nothing refused carries no field at all.
    let sqlx_row = env["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "sqlx-skin")
        .expect("sqlx-skin row");
    assert!(sqlx_row.get("unavailable").is_none());

    // Text branch: one line under the package.
    let out = cmd()
        .args(["list", data.to_str().unwrap()])
        .assert()
        .success();
    let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("unavailable : 0.1.0 (missing: some-future-capability)"),
        "the text branch must name the refusal: {stdout}"
    );
}
