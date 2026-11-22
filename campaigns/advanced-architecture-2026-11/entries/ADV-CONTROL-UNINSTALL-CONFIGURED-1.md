# ADV-CONTROL-UNINSTALL-CONFIGURED-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe uninstall org.vibevm/oracle-driver` on a configured owner exits 0 and removes its requires entry, its configuration input, its vibe.lock configurations record, its slot and the slots of dependencies it selected that no other root holds, and the regenerated boot no longer loads them.

**Write.** `crates/vibe-cli/src/commands/uninstall.rs`, `crates/vibe-cli/tests/cli_common_package_optional/boundaries.rs`, `vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml`

**Check.** `cargo test -p vibe-cli --test cli_common_package_optional configured_uninstall_ 2>&1 | grep -F 'test result: ok. 2 passed'`

**Test.** crates/vibe-cli/tests/cli_common_package_optional/boundaries.rs: (a) new `configured_uninstall_owner_removes_requires_record_and_selected_closure`: Fixture::new(true); leaf(&fixture.registry, "oracle-unrelated", "UNRELATED_MARKER"); add_root(&fixture, "oracle-unrelated", None); fixture.install_ok(); assert fixture.static_boot() contains DRIVER_MARKER; save bytes of fixture.slot("oracle-unrelated").join("payload.txt"); output = uninstall(&fixture, DRIVER); assert output.status.success() (print stdout/stderr on failure); read Manifest from project vibe.toml: assert no requires.iter_pkgrefs() item has name DRIVER and requires.configurations has no key coordinate(DRIVER); fixture.lock(): configurations has no key coordinate(DRIVER) and the set of package names equals {"oracle-unrelated"}; fixture.slot(x).exists() is false for DRIVER, BASE and EXTRA; unrelated payload bytes unchanged; build boot text by concatenating fs::read_to_string(project.join(p)).ok() for p in [common::static_md_rel(), common::static_xml_rel()] (do NOT call fixture.static_boot(), it asserts a file exists) and assert it contains none of DRIVER_MARKER, BASE_MARKER, EXTRA_MARKER; finally fixture.install_ok(). Fails today: the uninstall exits non-zero with the 'configuration-aware removal is not connected' refusal. (b) the existing `affected_uninstall_refuses_but_unrelated_legacy_root_removal_preserves_configuration` renamed to `configured_uninstall_unrelated_root_preserves_owner_record`, with its DRIVER-refusal half deleted (behaviour intentionally replaced by (a)); its unrelated-root half stays byte-for-byte. The check requires exactly 2 passing tests, so a missing or misnamed test fails it.

**Read first.**
- crates/vibe-cli/src/commands/uninstall.rs:43-120
- crates/vibe-cli/src/commands/uninstall/plan.rs:156-214
- crates/vibe-cli/tests/cli_common_package_optional/boundaries.rs:1-141
- crates/vibe-cli/tests/cli_common_package_optional.rs:15-241
- crates/vibe-core/src/manifest/configuration/package.rs:21-40
- crates/vibe-core/src/manifest/lockfile.rs:553-565

**Steps.**
1. Write test (a) first at the end of boundaries.rs with attribute #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#UNINSTALL-CLOSURE")], and rename/trim test (b): rename `affected_uninstall_refuses_but_unrelated_legacy_root_removal_preserves_configuration` to `configured_uninstall_unrelated_root_preserves_owner_record`; delete its lines `let lock = ...`, `let authored = ...`, `let boot = ...` and the whole block from `let output = uninstall(&fixture, DRIVER);` through the `assert_eq!(fixture.hooks(), hooks);` that precedes `let output = uninstall(&fixture, unrelated);`. Keep the `configuration` and `hooks` bindings and everything after. Run the check and confirm it does NOT print `2 passed` (test (a) fails) before touching uninstall.rs.
2. In uninstall.rs replace the block from `if let Some((owner, _)) = lockfile.configurations.iter().find(` through its closing `}` (today lines 76-86; keep the `let selected_key = ...;` line above it) with: build `let removed_keys: std::collections::BTreeSet<String>` = plan.removed mapped to format!("{}/{}", row.group, row.name); then `for (owner, record) in &lockfile.configurations`: `continue` when removed_keys.contains(owner); let stale = *owner == selected_key || record.active_dependencies.iter().any(|d| removed_keys.contains(d)) || record.forwarded_inputs.iter().any(|i| removed_keys.contains(&format!("{}/{}", i.source_group, i.source_name))); if stale return Err(vibe_core::manifest::PackageConfigurationError::Unsupported { package: owner.clone(), reason: <exact text in decisions>.into() }.into()).
3. Change nothing else in uninstall.rs: lockfile.remove() already drops lock.configurations[key] for every removed row (lockfile.rs:562), plan::drop_requires already drops requires.configurations (plan.rs:212), and the existing closure over lock edges already includes the configuration-selected dependencies.
4. Make the spec_edit below in PROP-009-loading-model.xml exactly as written; change no other unit.
5. Run `cargo fmt -p vibe-cli` then the check.

**Decisions already made.**
- No new flag and no --force: uninstalling a configured owner just works.
- Closure is unchanged: lock edges are the authority (verified: the driver's lock row carries edges to oracle-base and, when enabled, oracle-extra); active_dependencies are NOT added as seeds.
- Refusal is kept only for a configured owner that is NOT itself removed and whose record would go stale: it is the uninstall target but another root retains it, or its active_dependencies or forwarded_inputs sources name a removed package. Records of removed packages simply leave with them.
- Exact refusal reason string: "uninstall would leave this retained configured package's lock record naming a removed package or declaration; removing part of a retained configured package is not connected". It stays the existing PackageConfigurationError::Unsupported variant, exit code 1.
- Missing-owner-record case gets no special refusal: removal reads only lock edges, so nothing is guessed.
- Test (a) calls add_root with `None` on purpose: the unrelated root then has no `requires.optional` record, so the test asserts nothing about requires.optional. Test (b) keeps its existing `Some(false)` and its existing optional assertion unchanged; do not copy one into the other.
- The worker makes the spec edit (PROP-009 is in the write area); the unit text is fixed below.
- Probe result (planner ran this patch in a copy): both new tests passed, including the follow-up install_ok, with no change outside uninstall.rs.

**Not in this entry.** Moving uninstall into vibe-install, a lifecycle lease, recovery journal, or rewriting another consumer's record to drop forwarded inputs; also the selected-dependency refusal message (entry 2).

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml, unit UNINSTALL-CLOSURE: append before `</UNINSTALL-CLOSURE>` the sentences: " Removing a configured package's own declaration also drops its configuration input and its lock configuration record, and removes the dependencies its configuration selected where no surviving declaration reaches them. Uninstall refuses before any write when a retained configured package's lock record would still name a removed package or declaration."

Entries that write the same file run one after another, never in parallel.
