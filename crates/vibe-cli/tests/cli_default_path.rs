//! End-to-end for the *default* `vibe init` → `vibe install` path: the
//! package resolves through the machine-global registry
//! (`~/.vibe/registry.toml`), not through any `[[registry]]` the project
//! carries.
//!
//! This is the coverage gap audit finding `2026-05-23-01` called out — the
//! oldest open finding in the project, and the one that let finding `-02`
//! ship green through eight milestone phases. Every prior git-registry
//! install (`cli_pkg_cycle::install_from_git_registry`) declared its
//! registry in the *project* `vibe.toml`, so the global layer on the
//! install path — `merge_effective` in
//! `crates/vibe-cli/src/commands/install/resolver.rs`, fed by
//! `GlobalRegistryConfig::load()` in `install/mod.rs` — was exercised by no
//! test at all. Here the project `vibe.toml` carries NO `[[registry]]`; the
//! single registry lives in the isolated settings home, exactly where a
//! developer's machine-local registry would.

mod common;

use std::fs;

use common::{UserScratch, git_available, make_per_package_registry};
use specmark::verifies;

/// `vibe init` with no registry flag of any kind, then
/// `vibe install org.vibevm.world/wal` where the only registry is the
/// machine-global one. The project manifest stays clean of `[[registry]]`
/// (so this test cannot silently duplicate the project-registry path), and
/// the install resolves through the global layer — proven by the lockfile's
/// `source_url` (`git+file://…/org.vibevm.world.wal.git`) and the
/// materialised `vibedeps/flow-wal/0.2.0/` slot.
///
/// The short name `vibe install wal` is deliberately NOT covered here: a
/// hermetic per-package git registry carries no PROP-005 package index, so
/// `MultiRegistryResolver::resolve_name_candidates` skips it and short-name
/// qualification fails with "no package index" — measured, not a gap. See
/// the worker report for the file:line evidence.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#global-config")]
fn default_path_installs_via_global_registry() {
    if !git_available() {
        eprintln!("skipping default_path_installs_via_global_registry: git not on PATH");
        return;
    }

    // 1. A hermetic per-package git registry under a temp dir — one bare
    //    repo, `org.vibevm.world.wal.git`, seeded from the real
    //    `org.vibevm.world/wal@0.2.0` package tree (dogfood, not a fixture).
    let outer = tempfile::tempdir().unwrap();
    let org_root = make_per_package_registry(outer.path());

    // 2. An isolated settings home carrying ONE `[[registry]]` that points
    //    at it. `$VIBE_SETTINGS` (set by `UserScratch::vibe`) relocates the
    //    whole per-user home, and `registry_config_path()` resolves to
    //    `<settings>/registry.toml`, so this file is exactly what
    //    `GlobalRegistryConfig::load()` reads on the install path.
    let user = UserScratch::new();
    // Org URL = parent of `org.vibevm.world.wal.git`. The `git+file://`
    // prefix is the Cargo/pip lockfile convention; the resolver strips it
    // before invoking `git`, so it works with prefixed and bare forms.
    let url = format!(
        "git+file://{}",
        org_root.to_string_lossy().replace('\\', "/")
    );
    // A distinctive name (not "default") makes the lockfile proof
    // unambiguous: the package came from THIS global section, never from a
    // seeded default pair. `naming` defaults to `fqdn` (PROP-008 §3), so the
    // resolver composes `<group>.<name>.git` — matching the seeded repo.
    let global_registry_toml =
        format!("[[registry]]\nname = \"hermetic-global\"\nurl = \"{url}\"\n");
    fs::write(user.settings.join("registry.toml"), global_registry_toml).unwrap();

    // 3. `vibe init` with NO registry flag — not `--registry`, not
    //    `--registry-url`, not `--no-registry`. `VIBE_NO_DEFAULT_REGISTRY`
    //    (set by `vibe()`) suppresses seeding a default pair into the
    //    settings home, so the file written above is read verbatim.
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());

    // 4. The project manifest carries NO `[[registry]]` — otherwise this
    //    test would be exercising the project-registry path (already
    //    covered by `cli_pkg_cycle::install_from_git_registry`) and
    //    silently duplicating it. The registry must resolve through the
    //    global layer alone; this assertion is what keeps the test honest.
    let manifest_text = fs::read_to_string(project.path().join("vibe.toml")).unwrap();
    let parsed = vibe_core::manifest::Manifest::parse_str(&manifest_text).unwrap();
    assert_eq!(
        parsed.registries.len(),
        0,
        "default-path test: project vibe.toml must carry no [[registry]] \
         (the registry lives in the global settings home); got: {manifest_text}"
    );
    assert!(
        !manifest_text.contains("[[registry]]"),
        "project vibe.toml must not contain [[registry]]: {manifest_text}"
    );

    // 5. Install the *qualified* package. Resolution runs against the merged
    //    effective set = project (empty) + global (one git registry), built
    //    by `merge_effective` at the install path's composition root.
    user.vibe()
        .arg("install")
        .arg("org.vibevm.world/wal")
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();

    // 6. Assert by artifacts, not by exit code.
    let lock_text = fs::read_to_string(project.path().join("vibe.lock")).unwrap();
    let lock: vibe_core::manifest::Lockfile = toml::from_str(&lock_text).unwrap();
    assert_eq!(lock.packages.len(), 1, "exactly one package installed");
    let entry = &lock.packages[0];

    // Provenance: the package resolved through the GLOBAL registry section.
    // The name recorded here is the global section's `name`, proving the
    // effective set's only registry was the machine-global one.
    assert_eq!(
        entry.registry.as_deref(),
        Some("hermetic-global"),
        "lockfile registry must be the global section's name, not a default"
    );
    assert!(
        entry.source_url.starts_with("git+file://"),
        "expected git+file:// source_url, got: {}",
        entry.source_url
    );
    assert!(
        entry.source_url.ends_with("/org.vibevm.world.wal.git"),
        "expected per-package URL ending in /org.vibevm.world.wal.git, got: {}",
        entry.source_url
    );
    assert_eq!(entry.source_ref.as_deref(), Some("v0.2.0"));
    assert!(!entry.overridden);

    // The materialised `vibedeps/` slot — the real
    // `org.vibevm.world/wal@0.2.0` tree the registry was seeded from.
    let slot = project.path().join("vibedeps/flow-wal/0.2.0");
    assert!(
        slot.join("vibe.toml").is_file(),
        "expected vibedeps/flow-wal/0.2.0/vibe.toml after install"
    );
    assert!(
        slot.join("README.md").is_file(),
        "expected vibedeps/flow-wal/0.2.0/README.md after install"
    );

    // Cache: exactly one registry bucket. This is the assertion that is
    // *especially* meaningful on the global path — it confirms the merged
    // effective set contributed exactly one git registry and nothing else
    // leaked in (the default pair is suppressed by `VIBE_NO_DEFAULT_REGISTRY`,
    // which `vibe()` sets). Taken (not blindly copied) from
    // `install_from_git_registry`: the per-clone `.git`/`vibe.toml` internals
    // are skipped as redundant with the lockfile + slot assertions above.
    let buckets: Vec<_> = fs::read_dir(&user.cache)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        buckets.len(),
        1,
        "expected exactly one registry cache bucket (the single global registry)"
    );
}
