# ADV-FIX-LEGACY-FLAGS-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** After a successful `vibe install` that used a legacy feature flag, vibe prints a note that the features live only in vibe.lock and that [requires.configurations] is the durable place. An end-to-end test shows that a fresh-lock `--features` install records the feature and that `vibe reinstall` keeps it.

**Write.** `crates/vibe-cli/src/commands/install/direct.rs`, `crates/vibe-cli/tests/cli_registry_mgmt.rs`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --test cli_registry_mgmt fresh_lock_install_with_features_resolves_and_reinstall_keeps_them`

**Test.** crates/vibe-cli/tests/cli_registry_mgmt.rs, new #[test] fn fresh_lock_install_with_features_resolves_and_reinstall_keeps_them, placed after install_no_default_features_drops_default_feature_from_lockfile. It reuses make_features_fixture_registry(outer.path()) (package org.vibevm/feat-pkg 0.1.0 with non-default feature `with-rust`), plus UserScratch::new() and user.init_project(project.path()). Step 1: `install org.vibevm/feat-pkg --registry R --path P --assume-yes` succeeds. Step 2 (control): `install --registry R --path P --assume-yes` succeeds and stdout contains "vibe.lock is fresh". Step 3: `install --registry R --path P --features with-rust --assume-yes` succeeds. Its stdout+stderr do not contain "vibe.lock is fresh", do contain "legacy feature flags require resolution", and do contain "recorded only in vibe.lock". The lock read from P/vibe.lock as vibe_core::manifest::Lockfile has its feat-pkg entry's features containing "with-rust". Step 4: `reinstall P --assume-yes` (path is positional) succeeds. The re-read lock's feat-pkg entry still contains "with-rust", and P.join(common::slot_dir("org.vibevm.feat-pkg", "0.1.0")) is a directory. Before ADV-FIX-LEGACY-FLAGS-1, step 3 prints "vibe.lock is fresh" and leaves the lock unchanged, so the test fails.

**Read first.**
- crates/vibe-cli/src/commands/install/direct.rs:270-320
- crates/vibe-cli/src/output/lines.rs:11-33
- crates/vibe-cli/tests/cli_registry_mgmt.rs:1-20
- crates/vibe-cli/tests/cli_registry_mgmt.rs:1040-1280
- crates/vibe-cli/src/commands/install/events.rs:35-50

**Steps.**
1. In crates/vibe-cli/src/commands/install/direct.rs add `const LEGACY_FEATURES_NOTE: &str = "note: legacy feature flags are recorded only in vibe.lock; declare durable choices under [requires.configurations] in vibe.toml";`.
2. In run_observed (l.272), before `args` is moved into execute_after_open, compute `let legacy_flags = !args.features.is_empty() || args.no_default_features || args.all_features;`.
3. In run_observed, after the final `match (&rendered, command_failed, parked) { ... }` (l.314) and before `rendered` is returned, add `if legacy_flags && rendered.is_ok() && !command_failed && !parked { ctx.heading(LEGACY_FEATURES_NOTE); }`.
4. Add the test fresh_lock_install_with_features_resolves_and_reinstall_keeps_them exactly as described in `test`. Combine stdout and stderr with String::from_utf8_lossy on the Output from user.vibe()...output().
5. Run the check command and nothing wider (lead-bounded rule 10).
6. If, with ADV-FIX-LEGACY-FLAGS-1 merged, step 3 re-resolves but the lock's feat-pkg entry still lacks "with-rust", set this entry `blocked` and name crates/vibe-install/src/apply/lock.rs and crates/vibe-install/src/plan/metadata.rs in the note. Do not widen the write area.

**Decisions already made.**
- Note text, exact: "note: legacy feature flags are recorded only in vibe.lock; declare durable choices under [requires.configurations] in vibe.toml". It goes out through ctx.heading, so --json and --quiet suppress it.
- The note appears after any successful project install that used a legacy flag, with or without a pkgref, because the flags live only in the lock either way.
- Reinstall code is not changed. reinstall/inputs.rs resolver_args keeps `features: Vec::new()`. That value only reaches build_install_resolver_deferred, which ignores features. Reinstall never re-resolves and never writes vibe.lock, so locked features already survive. The test pins this behaviour.
- Expected path after ADV-FIX-LEGACY-FLAGS-1 (verified by reading, not by a build): Stale -> prepare.rs l.203-212 hold_pins -> metadata.rs l.180-187 expands root features via tailor_feature_request(request.features) -> record.rs l.332 copies feature_expansion.active_features into the locked entry -> apply/lock.rs record_lock rewrites vibe.lock whenever the rebuilt lock differs from the previous one.
- The acceptance does not ask for a dependency slot activated by the feature: `dep:` activations have no consumer today, and redesigning legacy features is out of scope.
- The test fixture is the existing make_features_fixture_registry. No new fixture files.

**Not in this entry.** Not in this entry: changing reinstall to re-read or replay features into resolver_args, adding feature flags to `vibe reinstall`, writing flags into vibe.toml, or editing the apply/lock-write code.

**Depends on.** ADV-FIX-LEGACY-FLAGS-1

Entries that write the same file run one after another, never in parallel.
