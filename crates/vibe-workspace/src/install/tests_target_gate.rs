//! The `target:` gate end to end, through a real regeneration.
//!
//! The unit tests beside `resolve_condition` prove the verdict; these prove
//! the consequence a user sees — which rows reach the generated artifacts,
//! and that no `target:` string ever reaches `INDEX.md`. The compatibility
//! half matters most: a project that declares no `[system_prompt] target`
//! must see every `target:` row included, so adding the spelling changes no
//! existing project's boot (PROP-076 `#SYS-HOST-AND-TARGET`).

specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HOST-AND-TARGET",
    r = 1
);

use std::fs;

use tempfile::TempDir;

use super::test_helpers::*;
use super::*;

/// A workspace whose one dependency carries three fragments — one per
/// operating system — each gated on the **product's** platform. `declared`
/// is written into the root manifest verbatim, so the test can pose both
/// "declares nothing" and "declares one platform".
fn fixture(declared: &str) -> (TempDir, Vec<ResolvedDep>, Vec<TempDir>) {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        &format!(
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\
             \"org.vibevm/carrier\" = {{ version = \"^1.0\", link = \"static\" }}\n\
             {declared}"
        ),
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
    if !declared.is_empty() {
        // `[system_prompt]` is legal only beside the authored template whose
        // presence is the opt-in (PROP-076 `#SYS-OPT-IN`). Nothing in this
        // phase reads it; it is written so the manifest is admissible.
        write(
            ws_dir.path(),
            boot_rel(vibe_core::layout::SYSTEM_TEMPLATE_MD),
            "You are working inside a project.\n",
        );
    }

    let (carrier, carrier_pkg) = dep_with_boot(
        "carrier",
        "1.0.0",
        r#"[boot_snippet]
source = "boot/main.md"

[[boot_snippet.fragment]]
source = "boot/winapi.md"
when = "target:windows"

[[boot_snippet.fragment]]
source = "boot/kernel.md"
when = "target:linux"

[[boot_snippet.fragment]]
source = "boot/darwin.md"
when = "target:macos"
"#,
        "boot/main.md",
        "MAIN-CARRIER-BYTES",
    );
    write(carrier_pkg.path(), "boot/winapi.md", "WINAPI-RULE-BYTES");
    write(carrier_pkg.path(), "boot/kernel.md", "KERNEL-RULE-BYTES");
    write(carrier_pkg.path(), "boot/darwin.md", "DARWIN-RULE-BYTES");

    (ws_dir, vec![carrier], vec![carrier_pkg])
}

fn regenerate(ws_dir: &Path, resolution: Vec<ResolvedDep>) -> (String, String) {
    let ws = Workspace::load(ws_dir).unwrap();
    apply_resolution(&ws, &resolution, SlotIntegrity::TrustPresence, None).unwrap();
    (
        fs::read_to_string(ws_dir.join(boot_rel("STATIC.md"))).unwrap(),
        fs::read_to_string(ws_dir.join(boot_rel("INDEX.md"))).unwrap(),
    )
}

#[test]
fn a_project_declaring_no_target_includes_every_target_row() {
    // The compatibility condition: absent means every OS, so a `target:`
    // row behaves as if it had no condition at all.
    let (ws_dir, resolution, _packages) = fixture("");
    let (static_lane, index) = regenerate(ws_dir.path(), resolution);
    for bytes in [
        "WINAPI-RULE-BYTES",
        "KERNEL-RULE-BYTES",
        "DARWIN-RULE-BYTES",
    ] {
        assert!(
            static_lane.contains(bytes),
            "`{bytes}` belongs to the lane under an absent selection:\n{static_lane}"
        );
    }
    assert!(!index.contains("target:"), "{index}");
}

#[test]
fn a_declared_target_keeps_its_own_platform_and_drops_the_others() {
    let (ws_dir, resolution, _packages) = fixture("\n[system_prompt]\ntarget = [\"windows\"]\n");
    let (static_lane, index) = regenerate(ws_dir.path(), resolution);
    assert!(static_lane.contains("WINAPI-RULE-BYTES"), "{static_lane}");
    for dropped in ["KERNEL-RULE-BYTES", "DARWIN-RULE-BYTES", "boot/kernel.md"] {
        assert!(
            !static_lane.contains(dropped),
            "`{dropped}` is for a platform this product is not for:\n{static_lane}"
        );
        assert!(!index.contains(dropped), "{index}");
    }
}

#[test]
fn no_target_condition_ever_reaches_a_generated_index_row() {
    // Decided at generation time, exactly like `installed:`, so
    // `prepare_index_body`'s `kind` expression keeps its present meaning and
    // the gated continuation band can hold `os:` rows only.
    for declared in [
        "",
        "\n[system_prompt]\ntarget = [\"linux\"]\n",
        "\n[system_prompt]\ntarget = [\"windows\", \"macos\", \"linux\"]\n",
    ] {
        let (ws_dir, resolution, _packages) = fixture(declared);
        let (_static_lane, index) = regenerate(ws_dir.path(), resolution);
        // The generated header prose documents the `when` grammar, so the
        // assertion is on the rows: no entry carries a condition at all,
        // because a `target:` gate is decided before an INDEX row exists.
        let document: toml::Value = toml::from_str(&index).unwrap();
        for entry in document["entry"].as_array().unwrap() {
            assert!(
                entry.get("when").is_none(),
                "a `target:` row is never residual:\n{index}"
            );
            assert_eq!(entry["kind"].as_str(), Some("static"), "{index}");
        }
    }
}
