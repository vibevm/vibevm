//! Shared fixtures for the per-cell unit tests: a frozen clock and a
//! minimal clean project tree. Split out of `lib.rs` along the
//! test-support seam to keep that file within the length budget.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#root");

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::CheckOptions;

pub(crate) fn fixed_now() -> u64 {
    // 2026-05-04T12:00:00Z — well after the various dated test
    // markers, so age math is positive.
    vibe_core::timestamp::parse_unix_utc("2026-05-04T12:00:00Z").unwrap()
}

pub(crate) fn opts() -> CheckOptions {
    CheckOptions {
        now_unix_utc: Some(fixed_now()),
        ..Default::default()
    }
}

pub(crate) fn write_minimal_project(root: &Path) {
    // The scaffold routes every layout name through the seam
    // (`vibe_core::layout`, PROP-052 L2): the fixture tree names
    // whichever layout is live, so the R4 flip moves the tests with
    // the product and no scaffold is rewritten then.
    use vibe_core::layout;
    // vibe.toml with default registry.
    fs::write(
        root.join("vibe.toml"),
        r#"[project]
name = "demo"
version = "0.0.1"

[[registry]]
name = "vibespecs"
url = "https://example/vibespecs"
"#,
    )
    .unwrap();
    fs::create_dir_all(root.join(layout::current_boot_dir())).unwrap();
    fs::write(
        root.join(layout::current_boot_dir()).join("00-core.md"),
        "# core\n",
    )
    .unwrap();
    fs::write(
        root.join(layout::current_boot_dir()).join("90-user.md"),
        "# user\n",
    )
    .unwrap();
    // WAL with all required sections.
    let wal = root.join(layout::current_wal_md());
    fs::write(
        &wal,
        "# WAL\n\n## Current phase\n\n## Constraints\n\n## Done\n\n## Next\n\n## Known issues\n",
    )
    .unwrap();
    // Pin the WAL mtime to 1h before `fixed_now()` so the freshness
    // check sees a deterministic positive age regardless of where the
    // host's wall-clock sits when the test runs. Otherwise
    // `clean_minimal_project_has_no_findings` flakes whenever real
    // wall-time crosses past `fixed_now()` (a fresh-write mtime
    // ahead of `now` surfaces as the "WAL mtime is in the future"
    // info finding).
    let one_hour_before_fixed_now =
        SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(fixed_now().saturating_sub(3600));
    fs::OpenOptions::new()
        .write(true)
        .open(&wal)
        .unwrap()
        .set_modified(one_hour_before_fixed_now)
        .unwrap();
}
