//! PROP-049 install-time predicate and fragment integration oracle.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-049#installed-predicate");

use std::fs;

use tempfile::TempDir;

use super::test_helpers::*;
use super::*;

#[test]
fn apply_resolution_resolves_installed_fragments_before_emission() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/carrier\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

    let (carrier, carrier_pkg) = dep_with_requires(
        "carrier",
        "1.0.0",
        r#"[requires.packages]
"org.vibevm/guard" = "^1.0"

[boot_snippet]
source = "boot/main.md"

[[boot_snippet.fragment]]
source = "boot/guarded.md"
when = "installed:org.vibevm/guard"

[[boot_snippet.fragment]]
source = "boot/absent.md"
when = "installed:org.vibevm/not-installed"
"#,
        "boot/main.md",
        "MAIN-CARRIER-BYTES",
        &["guard"],
    );
    write(
        carrier_pkg.path(),
        "boot/guarded.md",
        "GUARDED-FRAGMENT-BYTES",
    );
    write(
        carrier_pkg.path(),
        "boot/absent.md",
        "ABSENT-FRAGMENT-BYTES",
    );
    let (guard, _guard_pkg) = dep_with_boot("guard", "1.0.0", "", "unused.md", "unused");

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(&ws, &[carrier, guard], SlotIntegrity::TrustPresence, None).unwrap();

    let static_lane = fs::read_to_string(ws_dir.path().join(boot_rel("STATIC.md"))).unwrap();
    let index = fs::read_to_string(ws_dir.path().join(boot_rel("INDEX.md"))).unwrap();
    assert!(static_lane.contains("MAIN-CARRIER-BYTES"), "{static_lane}");
    assert!(
        static_lane.contains("GUARDED-FRAGMENT-BYTES"),
        "{static_lane}"
    );
    assert!(
        !static_lane.contains("ABSENT-FRAGMENT-BYTES"),
        "{static_lane}"
    );
    assert!(!static_lane.contains("boot/absent.md"), "{static_lane}");
    assert!(!index.contains("boot/absent.md"), "{index}");
}
