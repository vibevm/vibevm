# ADV-FIX-LEGACY-FLAGS-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** freshness_prepared returns Freshness::Stale("legacy feature flags require resolution") whenever --features, --no-default-features or --all-features is given on a project with authored root declarations and no configured input, so a fresh lock no longer swallows the flags.

**Write.** `crates/vibe-install/src/plan/fresh.rs`, `crates/vibe-install/tests/prepared_planning.rs`, `vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-install --test prepared_planning legacy_feature_flags_make_a_declared_world_stale`

**Test.** crates/vibe-install/tests/prepared_planning.rs, new #[test] fn legacy_feature_flags_make_a_declared_world_stale. Fixture: tempdir with vibe.toml = "[project]\nname = \"demo\"\ngroup = \"org.demo\"\nversion = \"0.1.0\"\n\n[requires.packages]\n'org.demo/tools' = '^1.0'\n"; (manifest, workspace) via the file's existing prepare(root); lock = Lockfile::empty("vibe test", "2026-01-01T00:00:00Z"). It calls vibe_install::freshness_prepared(root, &manifest, &workspace, &req, &lock, vibe_core::manifest::SpecFormat::Mixed) four times. (a) With request() and features.explicit = vec!["extra".into()] it asserts the result is Ok(Freshness::Stale(r)) with r == "legacy feature flags require resolution". (b) The same with features.no_defaults = true. (c) The same with features.all = true. (d) Control: with plain request() the result is Stale with a reason != "legacy feature flags require resolution". Freshness is vibe_workspace::freshness::Freshness. Before the change (a)-(c) return a different Stale reason (the empty lock does not satisfy [requires]), so the test fails.

**Read first.**
- crates/vibe-install/src/plan/fresh.rs:53-124
- crates/vibe-install/tests/prepared_planning.rs:1-60
- crates/vibe-install/tests/prepared_planning.rs:236-270
- crates/vibe-install/src/plan/configured.rs:220-226
- vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml:35-56

**Steps.**
1. In freshness_prepared, directly after the `if configured && (...) { return Err(configured::mixed_flags()); }` block (fresh.rs l.91-97) and before `crate::submission::verify_submitted_replay`, insert: `if request.features.all || request.features.no_defaults || !request.features.explicit.is_empty() { return Ok(Freshness::Stale("legacy feature flags require resolution".into())); }`.
2. Add the test legacy_feature_flags_make_a_declared_world_stale to crates/vibe-install/tests/prepared_planning.rs. Reuse the file's prepare() and request() helpers, write the fixture vibe.toml given in `test`, and add any missing `use` lines (vibe_workspace::freshness::Freshness).
3. Apply spec_edit verbatim in PROP-011-incremental-install.xml: insert the new <LEGACY-FLAGS-FORCE-RESOLVE> line directly after the <EMPTY-REQUIRES-IS-A-NO-OP .../> line (l.54), inside the same <facts ordered="false"> block, with the same indentation.
4. Run the check command; it must pass. Run nothing wider (lead-bounded rule 10).

**Decisions already made.**
- Policy: legacy flags force Stale (the solve runs), never a refusal. The existing mixed_flags refusal for configured input stays first and unchanged.
- Exact reason text: "legacy feature flags require resolution" (no trailing period). The CLI already prints it as "re-resolving — legacy feature flags require resolution" through PlanEvent::Reresolving (events.rs l.42).
- The new rule sits after the mixed_flags check so configured projects still get the refusal. It sits before verify_submitted_replay and the lock checks so it does not depend on lock contents.
- Do not change project_fresh_node_roots: it already treats any non-Fresh answer as not admitted.
- Do not write flags into vibe.toml.
- The spec edit is part of this entry and is the only spec unit it adds.

**Not in this entry.** Not in this entry: making `dep:` feature activations (FeatureExpansion.active_deps) add dependencies to the graph, or touching configuration_freshness and mixed_flags. Both are redesigns of legacy features.

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-workspace/PROP-011-incremental-install.xml. Inside <skip-resolution>, in the last <facts ordered="false"> block, directly after the <EMPTY-REQUIRES-IS-A-NO-OP .../> line (l.54), add:
        <LEGACY-FLAGS-FORCE-RESOLVE fact="true" status="impl/plan">**Legacy feature flags force resolution.** A `vibe install` given `--features`, `--no-default-features` or `--all-features` over a project without configured input is never answered by the fast path: the freshness check reports the lock stale ("legacy feature flags require resolution") and the run re-resolves with untouched roots held (§5.3), recording the active features in `vibe.lock` only. Mixing these flags with configured input remains a refusal; durable intent belongs in `[requires.configurations]`.</LEGACY-FLAGS-FORCE-RESOLVE>

Entries that write the same file run one after another, never in parallel.
